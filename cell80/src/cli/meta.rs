//! Cell-source header parsing (`//!` metadata → manifest fields) and the one-line
//! manifest rendering shared by the library verbs.
use super::*;

/// A `//! scale:` value → fractional bits: a plain count (`8`) or a Q-format
/// (`q8.8` / `Q16.16`, taking the part after the point). Unparseable → `None` (the
/// annotation is optional).
pub(super) fn parse_scale(s: &str) -> Option<u8> {
    let s = s.trim().trim_start_matches(['q', 'Q']);
    s.rsplit('.').next().unwrap_or(s).trim().parse::<u8>().ok()
}

/// Parse a cell source's leading `//!` header →
/// `(summary, tags, entry, limits, scale, accuracy, finite_result, kernel_bank)`.
#[allow(clippy::type_complexity)]
pub(super) fn parse_meta(
    src: &str,
) -> (
    String,
    Vec<String>,
    Option<String>,
    Vec<String>,
    Option<u8>,
    Option<String>,
    Option<bool>,
    bool,
) {
    let (mut summary, mut tags, mut entry, mut limits, mut scale) =
        (String::new(), Vec::new(), None, Vec::new(), None);
    let mut accuracy = None;
    let mut finite_result = None;
    let mut kernel_bank = false;
    let csv = |s: &str| -> Vec<String> {
        s.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    for line in src.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("//!") {
            let rest = rest.trim();
            if let Some(t) = rest.strip_prefix("tags:") {
                tags = csv(t);
            } else if let Some(e) = rest.strip_prefix("entry:") {
                entry = Some(e.trim().to_string());
            } else if let Some(m) = rest.strip_prefix("limits:") {
                // The escalation contract, authorable from the source header — what this
                // cell can't do (`//! limits: floats, inputs > 65535`).
                limits = csv(m);
            } else if let Some(sv) = rest.strip_prefix("scale:") {
                // Fixed-point scale (fractional bits) — `//! scale: 8` for a Q8.8 cell.
                scale = parse_scale(sv);
            } else if let Some(av) = rest.strip_prefix("accuracy:") {
                // The F2 accuracy contract — a declared ULP bound over a domain,
                // e.g. `//! accuracy: <= 4 ulp over [-87.34, 88.72]`. Free-form;
                // the harness, not the parser, holds it to measurement.
                let a = av.trim();
                if !a.is_empty() {
                    accuracy = Some(a.to_string());
                }
            } else if let Some(kv) = rest.strip_prefix("kernel_bank:") {
                // Compile against the resident kernel bank — the image calls into
                // BANK_ORG and the manifest pins the bank hash (`.cell` v9).
                kernel_bank = matches!(kv.trim(), "on" | "true" | "1");
            } else if let Some(fv) = rest.strip_prefix("finite_result:") {
                // The F0.4 boundary contract — `off` opts an IEEE-plumbing cell out
                // of the non-finite-return escalation (default on).
                finite_result = match fv.trim() {
                    "off" | "false" | "0" => Some(false),
                    _ => Some(true),
                };
            } else if summary.is_empty() {
                summary = rest.to_string();
            }
        } else if !l.is_empty() && !l.starts_with("//") {
            break; // first code line — header done
        }
    }
    (
        summary,
        tags,
        entry,
        limits,
        scale,
        accuracy,
        finite_result,
        kernel_bank,
    )
}

/// Build a cartridge from a library `.rs` (id = file stem, metadata from the `//!` header)
/// or load a `.cell`. Returns `None` for any other extension. `pub(crate)` so the admission
/// gate (`crate::admission`) walks a directory the same way `cmd_index`/`host_from_dir` do.
pub fn library_cartridge(path: &std::path::Path) -> Option<Result<Cartridge, String>> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => Some((|| {
            let src =
                std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let (summary, tags, entry, limits, scale, accuracy, finite_result, kernel_bank) =
                parse_meta(&src);
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("cell")
                .to_string();
            Cartridge::compile(
                &src,
                CellConfig::sandboxed(),
                CartridgeOpts {
                    id: Some(id),
                    entry,
                    summary,
                    tags,
                    limits,
                    scale,
                    accuracy,
                    finite_result,
                    kernel_bank,
                    ..Default::default()
                },
            )
        })()),
        Some("cell") => Some(
            std::fs::read(path)
                .map_err(|e| format!("{}: {e}", path.display()))
                .and_then(|b| Cartridge::from_bytes(&b)),
        ),
        _ => None,
    }
}

/// Format a manifest as one library/search-result line: `id — summary  [tags]  (signature)`.
pub(super) fn render(m: &crate::Manifest) -> String {
    format!(
        "  {} — {}  [{}]  ({})",
        m.id,
        if m.summary.is_empty() {
            "(no summary)"
        } else {
            &m.summary
        },
        m.tags.join(", "),
        m.signature.to_decl(&m.entry),
    )
}
