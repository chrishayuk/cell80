//! The `.cell` cartridge — a named, versioned, **self-describing** tool artifact: a
//! [`Manifest`] (id / summary / tags / entry / source-hash / compiler+ABI version) wrapping
//! a compiled [`CellProgram`] image. This is the portable object the CLI, a tool index, and
//! the MCP server pass around — the gate for "compile once → ship → discover → run."
use super::program::{put_string, ImageReader};
use super::report::sorted_symbols;
use super::*;
use rustz80::Signature;
use std::hash::{Hash, Hasher};

const MAGIC: &[u8; 4] = b"CELL";
// v2 added the typed I/O signature; v3 added state field addresses; v4 added a width
// (`Ty`) per state field, so a `u32` field is drivable/readable wide by name; v5 added
// the `limits` declaration (the escalation contract, roadmap 3.2) **and** content
// addressing (roadmap 3.1): a SHA-256 artifact hash over the serialized manifest +
// image, verified on load by default, plus an optional ed25519 signature over that
// hash. Pre-v5 cartridges carry no hash and load unverified (grandfathered).
const VERSION: u8 = 5;

/// Serialize / read a `(name, type)` pair list (signature params / state fields).
fn put_pairs(b: &mut Vec<u8>, v: &[(String, String)]) {
    b.extend_from_slice(&(v.len() as u16).to_le_bytes());
    for (n, t) in v {
        put_string(b, n);
        put_string(b, t);
    }
}
fn read_pairs(r: &mut ImageReader) -> Result<Vec<(String, String)>, String> {
    let n = r.u16()?;
    let mut v = Vec::with_capacity(n as usize);
    for _ in 0..n {
        v.push((r.string()?, r.string()?));
    }
    Ok(v)
}

/// Serialize / read a `(name, u16 address, ty)` list (the state field addresses). A v3
/// cartridge has no `ty` byte — its fields read back as `u16` (the only width v3 knew).
fn put_addrs(b: &mut Vec<u8>, v: &[(String, u16, Ty)]) {
    b.extend_from_slice(&(v.len() as u16).to_le_bytes());
    for (n, a, ty) in v {
        put_string(b, n);
        b.extend_from_slice(&a.to_le_bytes());
        b.push(ty.code());
    }
}
fn read_addrs(r: &mut ImageReader, ver: u8) -> Result<Vec<(String, u16, Ty)>, String> {
    let n = r.u16()?;
    let mut v = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let name = r.string()?;
        let addr = r.u16()?;
        let ty = if ver >= 4 {
            Ty::from_code(r.u8()?)?
        } else {
            Ty::U16
        };
        v.push((name, addr, ty));
    }
    Ok(v)
}

/// Self-describing metadata carried by a `.cell` cartridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// A stable identifier (e.g. `"grid.manhattan.v1"`; defaults to the entry name).
    pub id: String,
    /// One-line human/agent summary (for a tool index).
    pub summary: String,
    /// Free-form tags for search/filtering.
    pub tags: Vec<String>,
    /// The default entry to run.
    pub entry: String,
    /// A **non-cryptographic** hash of the source (provenance / cache key).
    pub source_hash: u64,
    /// The `rustz80` version that produced this cartridge.
    pub compiler_version: String,
    /// The [`ABI_VERSION`] the cartridge targets.
    pub abi_version: u32,
    /// The typed I/O signature of the entry — so a registry/MCP can present the interface
    /// and validate named inputs **without re-parsing** the source.
    pub signature: Signature,
    /// For a state-cell entry: the byte address **and width** of each scalar state field
    /// at [`STATE_BASE`], `(name, addr, ty)` in declaration order — a `u32` field is 4
    /// bytes / two slots. Lets a warm host (or a peer cell in a graph) drive the cell *by
    /// field name*, wide fields included, without the source. Empty for a free fn.
    pub state_addrs: Vec<(String, u16, Ty)>,
    /// The escalation contract (roadmap 3.2): what this cell **can't** do, declared
    /// machine-readably so an orchestrator can route around the kernel class *before*
    /// running (e.g. `["floats", "inputs > 65535"]`). The substrate-wide non-goals
    /// (strings / floats / I/O / network) hold for every cell and don't need repeating —
    /// this field is for the cell's *own* boundary, and pairs with the structured
    /// [`Halt::Escalate`](crate::Halt::Escalate) hand-off at run time.
    pub limits: Vec<String>,
}

/// Options for [`Cartridge::compile`] (all optional).
#[derive(Default)]
pub struct CartridgeOpts {
    pub id: Option<String>,
    pub entry: Option<String>,
    pub summary: String,
    pub tags: Vec<String>,
    /// The cell's declared boundary — see [`Manifest::limits`].
    pub limits: Vec<String>,
}

/// A compiled cell **plus** its manifest — the `.cell` artifact.
#[derive(Clone)]
pub struct Cartridge {
    pub manifest: Manifest,
    pub program: CellProgram,
    /// An optional ed25519 `(verifying key, signature)` over the [artifact
    /// hash](Cartridge::artifact_hash) — attached by [`sign`](Cartridge::sign), carried
    /// through serialization, and **verified on load** when present. Unsigned artifacts
    /// stay first-class: the hash alone already pins content.
    pub signature: Option<([u8; 32], [u8; 64])>,
}

impl Cartridge {
    /// Compile `src` under `cfg` and wrap it in a cartridge: resolves the entry (opts, then
    /// `run`/`main`), hashes the source, and stamps the compiler + ABI versions.
    pub fn compile(src: &str, cfg: CellConfig, opts: CartridgeOpts) -> Result<Self, String> {
        let program = CellProgram::compile_with_config(src, cfg)?;
        let syms = &program.program().symbols;
        let entry = match opts.entry {
            Some(e) if syms.contains_key(&e) => e,
            Some(e) => return Err(format!("no entry `{e}` in the program")),
            None if syms.contains_key("run") => "run".into(),
            None if syms.contains_key("main") => "main".into(),
            None => return Err("no `run`/`main` entry — pass an explicit entry".into()),
        };
        let mut h = std::collections::hash_map::DefaultHasher::new();
        src.hash(&mut h);
        let signature = rustz80::entry_signature(src, &entry)?;
        let state_addrs = super::state_field_addrs(src, &entry)?;
        Ok(Cartridge {
            manifest: Manifest {
                id: opts.id.unwrap_or_else(|| entry.clone()),
                summary: opts.summary,
                tags: opts.tags,
                entry,
                source_hash: h.finish(),
                compiler_version: env!("CARGO_PKG_VERSION").to_string(),
                abi_version: ABI_VERSION,
                signature,
                state_addrs,
                limits: opts.limits,
            },
            program,
            signature: None,
        })
    }

    /// The serialized manifest prefix (everything the artifact hash covers besides the
    /// image): MAGIC through the `limits` list.
    fn manifest_bytes(&self) -> Vec<u8> {
        let m = &self.manifest;
        let mut b = Vec::new();
        b.extend_from_slice(MAGIC);
        b.push(VERSION);
        b.extend_from_slice(&m.abi_version.to_le_bytes());
        put_string(&mut b, &m.id);
        put_string(&mut b, &m.summary);
        b.extend_from_slice(&(m.tags.len() as u16).to_le_bytes());
        for t in &m.tags {
            put_string(&mut b, t);
        }
        put_string(&mut b, &m.entry);
        b.extend_from_slice(&m.source_hash.to_le_bytes());
        put_string(&mut b, &m.compiler_version);
        put_pairs(&mut b, &m.signature.params);
        put_string(&mut b, &m.signature.ret);
        put_pairs(&mut b, &m.signature.state);
        put_addrs(&mut b, &m.state_addrs);
        b.extend_from_slice(&(m.limits.len() as u16).to_le_bytes());
        for l in &m.limits {
            put_string(&mut b, l);
        }
        b
    }

    /// The content address (roadmap 3.1): SHA-256 over the serialized manifest + the
    /// emitted image. Two artifacts with the same hash are the same tool — manifest text,
    /// entry, capability policy, code, all of it.
    pub fn artifact_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.manifest_bytes());
        h.update(self.program.to_bytes());
        h.finalize().into()
    }

    /// Sign the [artifact hash](Cartridge::artifact_hash) with an ed25519 seed (32
    /// bytes, e.g. from `cell80 keygen`). The `(verifying key, signature)` pair is
    /// embedded on the next [`to_bytes`](Cartridge::to_bytes) and verified on load.
    pub fn sign(&mut self, seed: &[u8; 32]) {
        use ed25519_dalek::{Signer, SigningKey};
        let key = SigningKey::from_bytes(seed);
        let sig = key.sign(&self.artifact_hash());
        self.signature = Some((key.verifying_key().to_bytes(), sig.to_bytes()));
    }

    /// Serialize to `.cell` bytes: manifest, the artifact hash, the optional signature
    /// block, then the [`CellProgram`] image.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = self.manifest_bytes();
        b.extend_from_slice(&self.artifact_hash());
        match &self.signature {
            Some((vk, sig)) => {
                b.push(1);
                b.extend_from_slice(vk);
                b.extend_from_slice(sig);
            }
            None => b.push(0),
        }
        let img = self.program.to_bytes();
        b.extend_from_slice(&(img.len() as u32).to_le_bytes());
        b.extend_from_slice(&img);
        b
    }

    /// Reload a `.cell` cartridge — no parse, no compile. **Verifies by default** (the
    /// roadmap-3.1 contract): the stored artifact hash must match the recomputed one,
    /// and the signature (when present) must verify against it. Pre-v5 cartridges carry
    /// no hash and load as before. For dev round-trips on intentionally edited bytes,
    /// [`from_bytes_unverified`](Cartridge::from_bytes_unverified) skips both checks.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        Self::from_bytes_impl(bytes, true)
    }

    /// [`from_bytes`](Cartridge::from_bytes) without the hash/signature check — the
    /// `--no-verify` dev path. The artifact is still parsed and structurally validated.
    pub fn from_bytes_unverified(bytes: &[u8]) -> Result<Self, String> {
        Self::from_bytes_impl(bytes, false)
    }

    fn from_bytes_impl(bytes: &[u8], verify: bool) -> Result<Self, String> {
        let mut r = ImageReader { b: bytes, i: 0 };
        if r.take(4)? != MAGIC {
            return Err("not a .cell cartridge".into());
        }
        let ver = r.u8()?;
        if !(2..=5).contains(&ver) {
            return Err(format!("unsupported .cell version {ver}"));
        }
        let abi_version = r.u32()?;
        let id = r.string()?;
        let summary = r.string()?;
        let ntags = r.u16()?;
        let mut tags = Vec::with_capacity(ntags as usize);
        for _ in 0..ntags {
            tags.push(r.string()?);
        }
        let entry = r.string()?;
        let source_hash = r.u64()?;
        let compiler_version = r.string()?;
        let signature = Signature {
            params: read_pairs(&mut r)?,
            ret: r.string()?,
            state: read_pairs(&mut r)?,
        };
        // v3+ carries the state field addresses (v4 adds a width per field); a v2
        // cartridge has none (named I/O unavailable until recompiled).
        let state_addrs = if ver >= 3 {
            read_addrs(&mut r, ver)?
        } else {
            Vec::new()
        };
        // v5+ carries the limits declaration (the escalation contract); older
        // cartridges have none declared.
        let limits = if ver >= 5 {
            let n = r.u16()?;
            let mut v = Vec::with_capacity(n as usize);
            for _ in 0..n {
                v.push(r.string()?);
            }
            v
        } else {
            Vec::new()
        };
        // v5+ is content-addressed: the stored hash covers bytes[..here] + the image.
        let manifest_end = r.i;
        let (stored_hash, cart_sig) = if ver >= 5 {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(r.take(32)?);
            let sig = match r.u8()? {
                0 => None,
                1 => {
                    let mut vk = [0u8; 32];
                    vk.copy_from_slice(r.take(32)?);
                    let mut s = [0u8; 64];
                    s.copy_from_slice(r.take(64)?);
                    Some((vk, s))
                }
                other => return Err(format!("bad signature marker {other}")),
            };
            (Some(hash), sig)
        } else {
            (None, None)
        };
        let img_len = r.u32()? as usize;
        let img = r.take(img_len)?;
        if verify {
            if let Some(stored) = stored_hash {
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(&bytes[..manifest_end]);
                h.update(img);
                let actual: [u8; 32] = h.finalize().into();
                if actual != stored {
                    return Err(
                        "artifact hash mismatch — the .cell is corrupted or was tampered \
                         with (recompile it, or load with --no-verify for dev)"
                            .into(),
                    );
                }
                if let Some((vk_bytes, sig_bytes)) = &cart_sig {
                    use ed25519_dalek::{Signature, VerifyingKey};
                    let vk = VerifyingKey::from_bytes(vk_bytes)
                        .map_err(|e| format!("bad signing key in .cell: {e}"))?;
                    vk.verify_strict(&stored, &Signature::from_bytes(sig_bytes))
                        .map_err(|_| {
                            "signature verification failed — the .cell was not signed by \
                             the key it claims (or was altered after signing)"
                                .to_string()
                        })?;
                }
            }
        }
        let program = CellProgram::from_bytes(img)?;
        Ok(Cartridge {
            manifest: Manifest {
                id,
                summary,
                tags,
                entry,
                source_hash,
                compiler_version,
                abi_version,
                signature,
                state_addrs,
                limits,
            },
            program,
            signature: cart_sig,
        })
    }

    /// A human-readable inspection summary.
    pub fn to_human(&self) -> String {
        let m = &self.manifest;
        let p = self.program.program();
        let c = &self.program.cfg;
        let entry_addr = p.symbols.get(&m.entry).copied().unwrap_or(0);
        let caps = format!(
            "raw_memory={} ports={} max_code={} max_touched={}",
            c.allow_raw_memory,
            c.allow_ports,
            c.max_code_bytes.map_or("∞".into(), |n| n.to_string()),
            c.max_touched.map_or("∞".into(), |n| n.to_string()),
        );
        let syms: Vec<String> = sorted_symbols(&p.symbols)
            .iter()
            .map(|(n, a)| format!("{n}@0x{a:04X}"))
            .collect();
        let state = if m.signature.state.is_empty() {
            String::new()
        } else {
            let fs: Vec<String> = m
                .signature
                .state
                .iter()
                .map(|(n, t)| format!("{n}: {t}"))
                .collect();
            format!("\n  state: {{ {} }}", fs.join(", "))
        };
        let limits = if m.limits.is_empty() {
            String::new()
        } else {
            format!("\n  limits: {} (escalates past these)", m.limits.join(", "))
        };
        let hash = self.artifact_hash();
        let artifact = match &self.signature {
            Some((vk, _)) => format!(
                "sha256:{}  (signed, key ed25519:{}…)",
                hex(&hash),
                &hex(&vk[..])[..16]
            ),
            None => format!("sha256:{}  (unsigned)", hex(&hash)),
        };
        format!(
            "cell `{}`  (abi {}, compiler {})\n  {}\n  tags: {}\n  signature: {}{}{limits}\n  \
             entry: {} @ 0x{:04X}\n  code: {} bytes, {} functions\n  capabilities: {}\n  \
             symbols: {}\n  source_hash: 0x{:016x}\n  artifact: {artifact}",
            m.id,
            m.abi_version,
            m.compiler_version,
            if m.summary.is_empty() {
                "(no summary)"
            } else {
                &m.summary
            },
            if m.tags.is_empty() {
                "—".into()
            } else {
                m.tags.join(", ")
            },
            m.signature.to_decl(&m.entry),
            state,
            m.entry,
            entry_addr,
            p.code.len(),
            p.size_report().len(),
            caps,
            syms.join(", "),
            m.source_hash,
        )
    }

    /// A JSON inspection summary (for tooling / a tool index).
    pub fn to_json(&self) -> String {
        let m = &self.manifest;
        let p = self.program.program();
        let c = &self.program.cfg;
        let tags: Vec<String> = m.tags.iter().map(|t| format!("\"{t}\"")).collect();
        let limits: Vec<String> = m.limits.iter().map(|l| format!("\"{l}\"")).collect();
        let syms: Vec<String> = sorted_symbols(&p.symbols)
            .iter()
            .map(|(n, a)| format!("\"{n}\":{a}"))
            .collect();
        let pairs_json = |v: &[(String, String)]| -> String {
            v.iter()
                .map(|(n, t)| format!("[\"{n}\",\"{t}\"]"))
                .collect::<Vec<_>>()
                .join(",")
        };
        format!(
            "{{\"id\":\"{}\",\"abi\":{},\"compiler\":\"{}\",\"summary\":\"{}\",\"tags\":[{}],\
             \"limits\":[{}],\
             \"entry\":\"{}\",\"signature\":{{\"params\":[{}],\"ret\":\"{}\",\"state\":[{}]}},\
             \"code_bytes\":{},\"functions\":{},\"source_hash\":\"0x{:016x}\",\
             \"artifact_hash\":\"sha256:{}\",\"signed\":{},\
             \"capabilities\":{{\"raw_memory\":{},\"ports\":{},\"max_code\":{},\"max_touched\":{}}},\
             \"symbols\":{{{}}}}}",
            m.id,
            m.abi_version,
            m.compiler_version,
            m.summary,
            tags.join(","),
            limits.join(","),
            m.entry,
            pairs_json(&m.signature.params),
            m.signature.ret,
            pairs_json(&m.signature.state),
            p.code.len(),
            p.size_report().len(),
            m.source_hash,
            hex(&self.artifact_hash()),
            self.signature.is_some(),
            c.allow_raw_memory,
            c.allow_ports,
            c.max_code_bytes.map_or("null".into(), |n| n.to_string()),
            c.max_touched.map_or("null".into(), |n| n.to_string()),
            syms.join(","),
        )
    }
}

/// Lowercase hex of a byte slice (the artifact-hash / key rendering).
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
