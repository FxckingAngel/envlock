use age::x25519::{Identity, Recipient};
use age::secrecy::ExposeSecret;
use anyhow::{bail, Context, Result};

/// Generate a new age x25519 identity (private key).
pub fn generate_identity() -> Identity {
    Identity::generate()
}

/// Serialize an identity to its string form (AGE-SECRET-KEY-...).
/// Note: age wraps this in a Secret<String> for safety; we expose it
/// here because we need to write it to disk.
pub fn identity_to_string(id: &Identity) -> String {
    id.to_string().expose_secret().clone()
}

/// Parse an identity from its string form.
pub fn parse_identity(s: &str) -> Result<Identity> {
    s.trim()
        .parse::<Identity>()
        .map_err(|e| anyhow::anyhow!("Failed to parse age identity: {}", e))
}

/// Derive the public recipient from an identity.
pub fn identity_to_recipient(id: &Identity) -> Recipient {
    id.to_public()
}

/// Parse a recipient from its string form (age1...).
pub fn parse_recipient(s: &str) -> Result<Recipient> {
    s.trim()
        .parse::<Recipient>()
        .map_err(|e| anyhow::anyhow!("Failed to parse age recipient: {}", e))
}

/// Encrypt plaintext bytes to one or more recipients.
/// Returns the encrypted ciphertext.
pub fn encrypt_bytes(plaintext: &[u8], recipients: &[Recipient]) -> Result<Vec<u8>> {
    let recipient_boxes: Vec<Box<dyn age::Recipient + Send>> = recipients
        .iter()
        .map(|r| Box::new(r.clone()) as Box<dyn age::Recipient + Send>)
        .collect();

    let encryptor = age::Encryptor::with_recipients(recipient_boxes)
        .context("No valid recipients provided")?;

    let mut ciphertext = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut ciphertext)
        .context("Failed to initialise encryption stream")?;
    use std::io::Write;
    writer
        .write_all(plaintext)
        .context("Failed to write plaintext to encryption stream")?;
    writer
        .finish()
        .context("Failed to finalise encryption")?;

    Ok(ciphertext)
}

/// Decrypt ciphertext using a single identity.
/// Returns a generic "Decryption failed" error on any failure to avoid
/// oracle-style information leaks.
pub fn decrypt_bytes(ciphertext: &[u8], identity: &Identity) -> Result<Vec<u8>> {
    let decryptor = match age::Decryptor::new(ciphertext) {
        Ok(d) => d,
        Err(_) => bail!("Decryption failed"),
    };

    let recipients_decryptor = match decryptor {
        age::Decryptor::Recipients(d) => d,
        _ => bail!("Decryption failed"),
    };

    let mut plaintext = Vec::new();
    let reader = match recipients_decryptor.decrypt(std::iter::once(identity as &dyn age::Identity)) {
        Ok(r) => r,
        Err(_) => bail!("Decryption failed"),
    };

    let mut reader = reader;
    use std::io::Read;
    if reader.read_to_end(&mut plaintext).is_err() {
        bail!("Decryption failed");
    }

    Ok(plaintext)
}
