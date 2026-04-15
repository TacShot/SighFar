use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Context, Result, anyhow, bail};
use argon2::Argon2;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rand::{Rng, rng};

use crate::models::SecureKeyPair;

pub struct SecureEnvelope;

impl SecureEnvelope {
    pub fn make_key_pair(&self, passphrase: &str) -> SecureKeyPair {
        SecureKeyPair {
            passphrase: passphrase.to_string(),
            companion_code: random_code(18),
        }
    }

    pub fn seal(&self, message: &str, key_pair: &SecureKeyPair) -> Result<String> {
        let key = derive_key(key_pair)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| anyhow!("invalid AES key length"))?;
        let nonce_bytes = random_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, message.as_bytes())
            .map_err(|_| anyhow!("failed to encrypt payload"))?;

        let mut payload = nonce_bytes.to_vec();
        payload.extend(ciphertext);
        Ok(BASE64.encode(payload))
    }

    pub fn open(&self, payload: &str, key_pair: &SecureKeyPair) -> Result<String> {
        let decoded = BASE64
            .decode(payload)
            .context("secure payload is not valid base64")?;
        if decoded.len() < 12 {
            bail!("secure payload is malformed");
        }

        let (nonce_bytes, ciphertext) = decoded.split_at(12);
        let key = derive_key(key_pair)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| anyhow!("invalid AES key length"))?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|_| anyhow!("failed to decrypt payload with the provided keys"))?;

        String::from_utf8(plaintext).context("decrypted payload is not valid UTF-8")
    }
}

pub fn derive_key(key_pair: &SecureKeyPair) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    let salt = normalized_salt(&key_pair.companion_code);
    Argon2::default()
        .hash_password_into(key_pair.passphrase.as_bytes(), &salt, &mut key)
        .map_err(|err| anyhow!("failed to derive secure key: {err}"))?;
    Ok(key)
}

fn normalized_salt(companion_code: &str) -> [u8; 16] {
    let bytes = companion_code.as_bytes();
    let mut salt = [0u8; 16];
    for (idx, slot) in salt.iter_mut().enumerate() {
        *slot = *bytes.get(idx % bytes.len()).unwrap_or(&b'X');
    }
    salt
}

fn random_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    rng().fill(&mut nonce);
    nonce
}

fn random_code(length: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut random = rng();
    (0..length)
        .map(|_| {
            let idx = random.random_range(0..ALPHABET.len());
            ALPHABET[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::SecureEnvelope;
    use crate::models::SecureKeyPair;

    #[test]
    fn secure_envelope_round_trip() {
        let envelope = SecureEnvelope;
        let pair = SecureKeyPair {
            passphrase: "alpha".to_string(),
            companion_code: "BRAVO987".to_string(),
        };

        let payload = envelope.seal("cipher-stack-output", &pair).unwrap();
        let opened = envelope.open(&payload, &pair).unwrap();

        assert_eq!(opened, "cipher-stack-output");
    }
}
