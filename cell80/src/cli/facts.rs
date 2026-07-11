//! The fact-file verbs (docs/12 §4): `facts export|import|verify`.
use super::*;

/// `facts export|import|verify` — the fact-file verbs (docs/12 §4). Every beat of
/// the sharing demo is the default behaviour of these: export prints claims,
/// import spot-checks them (one caught lie fails the file), verify audits every
/// line and exits nonzero on any failure.
pub(super) fn cmd_facts(args: &[String]) -> Result<String, String> {
    let verb = args.first().map(String::as_str).ok_or(USAGE)?;
    match verb {
        "export" => {
            let dir = args.get(1).ok_or(USAGE)?;
            let mut calls_file = None;
            let mut producer = std::env::var("USER").unwrap_or_else(|_| "cell80".into());
            let mut it = args[2..].iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--calls" => calls_file = Some(it.next().ok_or("--calls needs a file")?),
                    "--producer" => producer = it.next().ok_or("--producer needs a name")?.clone(),
                    other => return Err(format!("unknown flag `{other}`")),
                }
            }
            let calls_file = calls_file.ok_or("facts export needs --calls <file>")?;
            let mut host = host_from_dir(dir)?;
            host.set_cache(true);
            let calls = std::fs::read_to_string(calls_file).map_err(|e| e.to_string())?;
            let mut handles: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for (n, line) in calls.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let mut parts = line.split_whitespace();
                let id = parts
                    .next()
                    .ok_or_else(|| format!("calls line {}", n + 1))?;
                let h = match handles.get(id) {
                    Some(h) => *h,
                    None => {
                        let h = host.load(id)?;
                        handles.insert(id.to_string(), h);
                        h
                    }
                };
                let rest: Vec<&str> = parts.collect();
                if rest.iter().any(|t| t.contains('=')) {
                    // Named state fields: `id k=v k=v ...`
                    let mut fields = Vec::new();
                    for t in &rest {
                        let (k, v) = t
                            .split_once('=')
                            .ok_or_else(|| format!("calls line {}: bad `{t}`", n + 1))?;
                        fields.push((
                            k.to_string(),
                            v.parse::<u64>()
                                .map_err(|_| format!("calls line {}: bad value `{v}`", n + 1))?,
                        ));
                    }
                    host.run_state_fast(h, &fields, DEFAULT_CYCLES)?;
                } else {
                    // Register args: `id a b c`
                    let a: Vec<u16> = rest
                        .iter()
                        .map(|t| {
                            t.parse::<u16>()
                                .map_err(|_| format!("calls line {}: bad arg `{t}`", n + 1))
                        })
                        .collect::<Result<_, _>>()?;
                    host.run_fast(h, &a, DEFAULT_CYCLES)?;
                }
            }
            let mut out = Vec::new();
            host.export_facts(&mut out, &producer)?;
            Ok(String::from_utf8(out).expect("facts are utf-8"))
        }
        "import" | "verify" => {
            let file = args.get(1).ok_or(USAGE)?;
            let dir = args.get(2).ok_or(USAGE)?;
            let mut policy = crate::ImportPolicy::default();
            let mut json = false;
            let mut it = args[3..].iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--verify-fraction" => {
                        policy.verify_fraction = it
                            .next()
                            .and_then(|v| v.parse().ok())
                            .ok_or("--verify-fraction needs a number")?
                    }
                    "--quarantine" => policy.quarantine = true,
                    "--json" => json = true,
                    other => return Err(format!("unknown flag `{other}`")),
                }
            }
            let mut host = host_from_dir(dir)?;
            host.set_cache(true);
            let f = std::fs::File::open(file).map_err(|e| format!("{file}: {e}"))?;
            let r = std::io::BufReader::new(f);
            let rep = if verb == "verify" {
                host.verify_facts(r)?
            } else {
                host.import_facts(r, &policy)?
            };
            let rendered = if json { rep.to_json() } else { rep.to_human() };
            // The audit contract: any failure exits nonzero (CI-able).
            if rep.file_failed || !rep.failures.is_empty() {
                return Err(rendered);
            }
            Ok(rendered)
        }
        other => Err(format!(
            "unknown facts verb `{other}` — export/import/verify"
        )),
    }
}
