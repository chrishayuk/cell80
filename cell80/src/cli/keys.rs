//! Signing verbs: `keygen` (fresh ed25519 seed) and `sign` (embed a signature
//! block over the artifact hash).
use super::*;

/// `keygen <out.key>` — write a fresh 32-byte ed25519 seed (from the OS entropy pool)
/// and print the public verifying key. Guard the file like any private key.
pub(super) fn cmd_keygen(args: &[String]) -> Result<String, String> {
    let out = args.first().ok_or(USAGE)?;
    use std::io::Read;
    let mut seed = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut seed))
        .map_err(|e| format!("no OS entropy source (/dev/urandom): {e}"))?;
    std::fs::write(out, seed).map_err(|e| format!("{out}: {e}"))?;
    let vk = ed25519_dalek::SigningKey::from_bytes(&seed).verifying_key();
    let hex: String = vk.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!(
        "wrote signing key to {out} (keep it private)\npublic key: ed25519:{hex}"
    ))
}

/// `sign <file.cell> --key <key>` — sign the artifact hash and rewrite the `.cell` with
/// the signature block embedded. Verifiers check it on every load.
pub(super) fn cmd_sign(args: &[String]) -> Result<String, String> {
    let mut it = args.iter();
    let file = it.next().ok_or(USAGE)?;
    let mut key_path: Option<&String> = None;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--key" => key_path = Some(it.next().ok_or("--key needs a path")?),
            other => return Err(format!("unknown option `{other}`\n{USAGE}")),
        }
    }
    let key_path = key_path.ok_or("sign needs --key <key file>")?;
    let key = std::fs::read(key_path).map_err(|e| format!("{key_path}: {e}"))?;
    let seed: [u8; 32] = key
        .as_slice()
        .try_into()
        .map_err(|_| format!("{key_path}: want exactly 32 bytes (from `keygen`)"))?;
    let bytes = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
    let mut cart = Cartridge::from_bytes(&bytes)?; // verified before signing
    cart.sign(&seed);
    std::fs::write(file, cart.to_bytes()).map_err(|e| format!("{file}: {e}"))?;
    let vk = cart.signature.expect("just signed").0;
    let hex: String = vk.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("signed {file} (key ed25519:{}…)", &hex[..16]))
}
