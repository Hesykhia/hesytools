//! Access codes for the Dev tab. hesy signs a code for each person with a
//! private key that never leaves their PC; the app only checks it against
//! the public key below, so codes can't be made up or edited.
//!
//! On hesy's PC (where the private key lives) the app is the "owner": the
//! Dev tab is always unlocked and can make codes for friends.
//! `scripts/devcode.ps1 <name>` does the same from a terminal.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// Public half of hesy's dev key (%USERPROFILE%\.tauri\hesytools-dev.key).
const PUBLIC_KEY: &str = "59d895bc0f48d6d1acddfaeaff75c2cfa7ab6082f658a40e99e13206f8e30913";
const CONTEXT: &[u8] = b"hesytools dev access v1:";
const PREFIX: &str = "HT-";

/// The private key, if this PC has it (only hesy's does).
pub fn owner_seed() -> Option<[u8; 32]> {
    let path = std::path::PathBuf::from(std::env::var_os("USERPROFILE")?).join(".tauri").join("hesytools-dev.key");
    let hex = std::fs::read_to_string(path).ok()?;
    let hex = hex.trim();
    if hex.len() != 64 {
        return None;
    }
    let bytes: Vec<u8> = (0..64).step_by(2).filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok()).collect();
    let seed: [u8; 32] = bytes.try_into().ok()?;
    // Only count it if it really is the key the app trusts.
    (public_key_hex(&seed) == PUBLIC_KEY).then_some(seed)
}

fn message(name: &str) -> Vec<u8> {
    [CONTEXT, name.as_bytes()].concat()
}

/// A code that unlocks the Dev tab for `name`.
pub fn make_code(seed: &[u8; 32], name: &str) -> String {
    let sig = SigningKey::from_bytes(seed).sign(&message(name));
    let mut raw = vec![name.len() as u8];
    raw.extend_from_slice(name.as_bytes());
    raw.extend_from_slice(&sig.to_bytes());
    format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(raw))
}

pub fn public_key_hex(seed: &[u8; 32]) -> String {
    SigningKey::from_bytes(seed).verifying_key().to_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

/// Who a valid code belongs to; `None` for anything else.
pub fn check(code: &str) -> Option<String> {
    check_with(code, PUBLIC_KEY)
}

fn check_with(code: &str, public_hex: &str) -> Option<String> {
    let raw = URL_SAFE_NO_PAD.decode(code.trim().strip_prefix(PREFIX)?).ok()?;
    let (&len, rest) = raw.split_first()?;
    let len = len as usize;
    if rest.len() != len + 64 {
        return None;
    }
    let name = std::str::from_utf8(&rest[..len]).ok()?.to_string();
    let sig = Signature::from_bytes(rest[len..].try_into().ok()?);
    let key_bytes: Vec<u8> = (0..public_hex.len()).step_by(2).filter_map(|i| u8::from_str_radix(public_hex.get(i..i + 2)?, 16).ok()).collect();
    let key = VerifyingKey::from_bytes(key_bytes.as_slice().try_into().ok()?).ok()?;
    key.verify(&message(&name), &sig).ok()?;
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_check_out_and_cant_be_edited() {
        let seed = [7u8; 32];
        let public = public_key_hex(&seed);
        let code = make_code(&seed, "friend1");
        assert_eq!(check_with(&code, &public).as_deref(), Some("friend1"));
        // Same code, different name: rejected.
        let forged = make_code(&[9u8; 32], "friend1");
        assert_eq!(check_with(&forged, &public), None);
        let mut tampered = code.clone();
        tampered.pop();
        assert_eq!(check_with(&tampered, &public), None);
        assert_eq!(check_with("nonsense", &public), None);
    }
}
