//! The detached Ed25519 seal over a corpus attestation (spec 023, FR-003).
//!
//! Signing is a **post-pass over an already-emitted hash**: this module is the
//! only place in spec-spine that handles a key, deliberately kept in the CLI so
//! the pure core (`spec-spine-core::attest`) and the type substrate stay
//! crypto-free and key-free. The signed message is the 32 raw bytes of the
//! attestation hash; `key_id` and `signed_at` live in the [`LedgerSeal`]
//! envelope, never in the attested payload.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use spec_spine_types::{Error, LedgerSeal};

/// Load an Ed25519 signing key (a 32-byte seed) from `path`. Accepts the raw 32
/// bytes or a 64-char lowercase/uppercase hex string (with optional trailing
/// whitespace).
pub fn load_signing_key(path: &Path) -> Result<SigningKey, Error> {
    let raw = fs::read(path)
        .map_err(|e| Error::Io(format!("read signing key {}: {e}", path.display())))?;
    let seed = parse_key_bytes(&raw, "signing key")?;
    Ok(SigningKey::from_bytes(&seed))
}

/// Load an Ed25519 public key (32 bytes, raw or hex) from `path`.
pub fn load_verifying_key(path: &Path) -> Result<VerifyingKey, Error> {
    let raw = fs::read(path)
        .map_err(|e| Error::Io(format!("read public key {}: {e}", path.display())))?;
    let bytes = parse_key_bytes(&raw, "public key")?;
    VerifyingKey::from_bytes(&bytes)
        .map_err(|e| Error::Config(format!("invalid ed25519 public key: {e}")))
}

/// The default `key_id` for a signing key: the lowercase-hex public key.
pub fn default_key_id(signing_key: &SigningKey) -> String {
    hex_encode(signing_key.verifying_key().as_bytes())
}

/// Sign `attestation_hash` (lowercase hex of the 32-byte hash) with `signing_key`,
/// producing the detached [`LedgerSeal`]. `signed_at` is the CLI-supplied
/// wall-clock instant (RFC3339); the core never reads the clock.
pub fn sign(
    attestation_hash: &str,
    signing_key: &SigningKey,
    key_id: String,
    signed_at: String,
) -> Result<LedgerSeal, Error> {
    let digest = hex_decode(attestation_hash)?;
    let signature = signing_key.sign(&digest);
    Ok(LedgerSeal {
        alg: "ed25519".to_string(),
        key_id,
        signed_at,
        sig: hex_encode(&signature.to_bytes()),
    })
}

/// Verify a [`LedgerSeal`] over `attestation_hash` against `verifying_key`.
/// Returns `Ok(true)` for a valid signature, `Ok(false)` for a mismatch, and
/// `Err` only when the seal is structurally unusable (unknown alg, malformed
/// hex). Uses `verify_strict` (rejects non-canonical / weak-key signatures).
pub fn verify(
    attestation_hash: &str,
    seal: &LedgerSeal,
    verifying_key: &VerifyingKey,
) -> Result<bool, Error> {
    if seal.alg != "ed25519" {
        return Err(Error::Config(format!(
            "unsupported seal algorithm '{}' (only ed25519 is supported)",
            seal.alg
        )));
    }
    let digest = hex_decode(attestation_hash)?;
    let sig_bytes = hex_decode(&seal.sig)?;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| Error::Parse("ed25519 signature must be 64 bytes".to_string()))?;
    let signature = Signature::from_bytes(&sig_arr);
    Ok(verifying_key.verify_strict(&digest, &signature).is_ok())
}

// --- small hex + key-bytes helpers (kept local; the core's hash.rs is private) ---

/// Parse a 32-byte key from raw bytes or a 64-char hex string.
fn parse_key_bytes(raw: &[u8], what: &str) -> Result<[u8; 32], Error> {
    if let Ok(text) = std::str::from_utf8(raw) {
        let trimmed = text.trim();
        if trimmed.len() == 64 && trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
            let bytes = hex_decode(trimmed)?;
            return bytes
                .try_into()
                .map_err(|_| Error::Config(format!("{what} must be 32 bytes")));
        }
    }
    if raw.len() == 32 {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(raw);
        return Ok(arr);
    }
    Err(Error::Config(format!(
        "{what} must be 32 raw bytes or a 64-character hex string"
    )))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn hex_decode(s: &str) -> Result<Vec<u8>, Error> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(Error::Parse("hex string has an odd length".to_string()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| Error::Parse(format!("invalid hex: {e}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // A fixed seed keeps the test deterministic (no RNG): the attestation is
    // pure, and so is this round-trip.
    fn fixed_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    const HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn sign_then_verify_round_trips() {
        let key = fixed_key();
        let seal = sign(
            HASH,
            &key,
            default_key_id(&key),
            "2026-06-16T00:00:00Z".to_string(),
        )
        .unwrap();
        assert_eq!(seal.alg, "ed25519");
        assert!(verify(HASH, &seal, &key.verifying_key()).unwrap());
    }

    #[test]
    fn a_tampered_hash_fails_verification() {
        let key = fixed_key();
        let seal = sign(HASH, &key, "k".to_string(), "t".to_string()).unwrap();
        let other = "00".repeat(32);
        assert!(!verify(&other, &seal, &key.verifying_key()).unwrap());
    }

    #[test]
    fn a_wrong_key_fails_verification() {
        let key = fixed_key();
        let seal = sign(HASH, &key, "k".to_string(), "t".to_string()).unwrap();
        let wrong = SigningKey::from_bytes(&[9u8; 32]);
        assert!(!verify(HASH, &seal, &wrong.verifying_key()).unwrap());
    }

    #[test]
    fn key_bytes_parse_hex_and_raw() {
        let hex = "00".repeat(32);
        assert_eq!(parse_key_bytes(hex.as_bytes(), "k").unwrap(), [0u8; 32]);
        assert_eq!(parse_key_bytes(&[1u8; 32], "k").unwrap(), [1u8; 32]);
        assert!(parse_key_bytes(b"short", "k").is_err());
    }
}
