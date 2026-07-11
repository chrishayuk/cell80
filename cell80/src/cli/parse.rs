//! Token parsers shared by the CLI verbs: register args, behavioural examples
//! (positional and field forms), and the `--set`/`--read` typed-memory specs.
use super::*;

/// Parse a comma-separated arg list — decimal or `0x`-prefixed hex, each a `u16`.
pub fn parse_args(s: &str) -> Result<Vec<u16>, String> {
    s.split(',')
        .filter(|t| !t.trim().is_empty())
        .map(|t| {
            let t = t.trim();
            let v = match t.strip_prefix("0x") {
                Some(h) => u16::from_str_radix(h, 16),
                None => t.parse::<u16>(),
            };
            v.map_err(|_| format!("bad arg `{t}` (want a u16, decimal or 0x..)"))
        })
        .collect()
}

/// Parse `route` example tokens like `"3,7=7"` into `(inputs, expected_output)` pairs.
pub(super) fn parse_examples(toks: &[&str]) -> Result<Vec<(Vec<u16>, u16)>, String> {
    toks.iter()
        .map(|t| {
            let (lhs, rhs) = t
                .split_once('=')
                .ok_or_else(|| format!("bad example `{t}` (want in,..=out)"))?;
            let inputs = parse_args(lhs)?;
            let want = parse_args(rhs)?;
            match want.as_slice() {
                [out] => Ok((inputs, *out)),
                _ => Err(format!("bad example `{t}` (one output after `=`)")),
            }
        })
        .collect()
}

/// Parse field-form example tokens for state cells: `"x:3,y:4=7"` (expected return),
/// `"a:9,b:3=1,out:12"` / `"a:9,b:3=out:12"` (expected post-run fields — the status-flag
/// sibling separator, [`FieldExample::want_fields`]). LHS is named input fields; RHS is a
/// comma-separated mix of at most one bare number (the expected return) and `name:val`
/// pairs (expected post-run field values). At least one expectation is required.
pub(super) fn parse_field_examples(toks: &[&str]) -> Result<Vec<crate::FieldExample>, String> {
    toks.iter()
        .map(|t| {
            let (lhs, rhs) = t
                .split_once('=')
                .ok_or_else(|| format!("bad example `{t}` (want name:val,..=out)"))?;
            let fields = lhs
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|kv| {
                    let (k, v) = kv
                        .split_once(':')
                        .ok_or_else(|| format!("bad field `{kv}` (want name:val)"))?;
                    let v = v
                        .parse::<u64>()
                        .map_err(|_| format!("bad value `{v}` in `{t}`"))?;
                    Ok((k.trim().to_string(), v))
                })
                .collect::<Result<Vec<_>, String>>()?;
            let mut want_result = None;
            let mut want_fields = Vec::new();
            for item in rhs.split(',').filter(|s| !s.is_empty()) {
                match item.split_once(':') {
                    Some((k, v)) => {
                        let v = v
                            .parse::<u64>()
                            .map_err(|_| format!("bad value `{v}` in `{t}`"))?;
                        want_fields.push((k.trim().to_string(), v));
                    }
                    None if want_result.is_some() => {
                        return Err(format!("bad example `{t}` (one expected return after `=`)"))
                    }
                    None => {
                        want_result =
                            Some(item.trim().parse::<u16>().map_err(|_| {
                                format!("bad output `{item}` in `{t}` (want a u16)")
                            })?)
                    }
                }
            }
            if want_result.is_none() && want_fields.is_empty() {
                return Err(format!("bad example `{t}` (nothing expected after `=`)"));
            }
            Ok(crate::FieldExample {
                fields,
                want_result,
                want_fields,
            })
        })
        .collect()
}

/// Parse a `--set` spec — comma-separated `addr:ty=value` (addr/value decimal or `0x..`),
/// the typed inputs written into memory before the run.
pub(super) fn parse_sets(s: &str) -> Result<Vec<(u16, Ty, u64)>, String> {
    let num16 = |t: &str| match t.strip_prefix("0x") {
        Some(h) => u16::from_str_radix(h, 16),
        None => t.parse::<u16>(),
    };
    let num64 = |t: &str| match t.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16),
        None => t.parse::<u64>(),
    };
    s.split(',')
        .filter(|t| !t.trim().is_empty())
        .map(|t| {
            let t = t.trim();
            let bad = || format!("bad --set `{t}` (want addr:ty=value)");
            let (lhs, val_s) = t.split_once('=').ok_or_else(bad)?;
            let (addr_s, ty_s) = lhs.split_once(':').ok_or_else(bad)?;
            let addr = num16(addr_s).map_err(|_| format!("bad address in `{t}`"))?;
            let val = num64(val_s).map_err(|_| format!("bad value in `{t}`"))?;
            Ok((addr, Ty::parse(ty_s)?, val))
        })
        .collect()
}

/// Parse a `--read` spec — comma-separated `name@addr:ty` (addr decimal or `0x..`).
pub(super) fn parse_reads(s: &str) -> Result<Vec<(String, u16, Ty)>, String> {
    s.split(',')
        .filter(|t| !t.trim().is_empty())
        .map(|t| {
            let t = t.trim();
            let bad = || format!("bad --read `{t}` (want name@addr:ty)");
            let (name, rest) = t.split_once('@').ok_or_else(bad)?;
            let (addr_s, ty_s) = rest.split_once(':').ok_or_else(bad)?;
            let addr = match addr_s.strip_prefix("0x") {
                Some(h) => u16::from_str_radix(h, 16),
                None => addr_s.parse::<u16>(),
            }
            .map_err(|_| format!("bad address in `{t}`"))?;
            Ok((name.to_string(), addr, Ty::parse(ty_s)?))
        })
        .collect()
}
