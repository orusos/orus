//! Ed25519 keys and detached signatures for catalogs. Keys are stored as base64 text.

use std::path::Path;

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// A new key pair: (private key text, public key text).
pub fn generate() -> (String, String) {
    let key = SigningKey::generate(&mut rand_core::OsRng);
    (
        STANDARD.encode(key.to_bytes()),
        STANDARD.encode(key.verifying_key().to_bytes()),
    )
}

fn decode32(text: &str, what: &str) -> Result<[u8; 32]> {
    let bytes = STANDARD
        .decode(text.trim())
        .with_context(|| format!("{what} is not base64"))?;
    bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("{what} is not 32 bytes"))
}

pub fn signing_key_from_file(path: &Path) -> Result<SigningKey> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read the signing key {}", path.display()))?;
    Ok(SigningKey::from_bytes(&decode32(&text, "the signing key")?))
}

pub fn verifying_key_from_file(path: &Path) -> Result<VerifyingKey> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read the public key {}", path.display()))?;
    VerifyingKey::from_bytes(&decode32(&text, "the public key")?)
        .map_err(|e| anyhow::anyhow!("the public key is not valid: {e}"))
}

/// The base64 signature of `message`.
pub fn sign(key: &SigningKey, message: &[u8]) -> String {
    STANDARD.encode(key.sign(message).to_bytes())
}

/// Check `signature` (base64) over `message` against `key`.
pub fn verify(key: &VerifyingKey, message: &[u8], signature: &str) -> Result<()> {
    let bytes = STANDARD
        .decode(signature.trim())
        .context("the signature is not base64")?;
    let bytes: [u8; 64] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("the signature is not 64 bytes"))?;
    match key.verify(message, &Signature::from_bytes(&bytes)) {
        Ok(()) => Ok(()),
        Err(_) => bail!("the catalog signature does not match its key"),
    }
}
