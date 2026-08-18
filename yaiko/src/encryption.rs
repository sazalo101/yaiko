//! Authenticated envelope encryption and key rotation primitives.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedEnvelope {
    pub key_id: String,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncryptionError {
    InvalidKey,
    InvalidEnvelope,
    AuthenticationFailed,
    UnknownKey,
}

#[derive(Clone)]
pub struct KeyRing {
    keys: Arc<RwLock<BTreeMap<String, [u8; 32]>>>,
    current: Arc<RwLock<String>>,
}

impl KeyRing {
    pub fn new(key_id: impl Into<String>, key: [u8; 32]) -> Result<Self, EncryptionError> {
        let key_id = key_id.into();
        if key_id.trim().is_empty() {
            return Err(EncryptionError::InvalidKey);
        }
        let mut keys = BTreeMap::new();
        keys.insert(key_id.clone(), key);
        Ok(Self {
            keys: Arc::new(RwLock::new(keys)),
            current: Arc::new(RwLock::new(key_id)),
        })
    }
    pub fn rotate(&self, key_id: impl Into<String>, key: [u8; 32]) -> Result<(), EncryptionError> {
        let key_id = key_id.into();
        if key_id.trim().is_empty() {
            return Err(EncryptionError::InvalidKey);
        }
        self.keys
            .write()
            .expect("key ring poisoned")
            .insert(key_id.clone(), key);
        *self.current.write().expect("key ring poisoned") = key_id;
        Ok(())
    }
    pub fn encrypt(
        &self,
        plaintext: &[u8],
        associated_data: &[u8],
    ) -> Result<EncryptedEnvelope, EncryptionError> {
        let key_id = self.current.read().expect("key ring poisoned").clone();
        let key = *self
            .keys
            .read()
            .expect("key ring poisoned")
            .get(&key_id)
            .ok_or(EncryptionError::UnknownKey)?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| EncryptionError::InvalidKey)?;
        let nonce_bytes = *Uuid::new_v4().as_bytes();
        let nonce = Nonce::from_slice(&nonce_bytes[..12]);
        let ciphertext = cipher
            .encrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: plaintext,
                    aad: associated_data,
                },
            )
            .map_err(|_| EncryptionError::AuthenticationFailed)?;
        Ok(EncryptedEnvelope {
            key_id,
            nonce: URL_SAFE_NO_PAD.encode(&nonce_bytes[..12]),
            ciphertext: URL_SAFE_NO_PAD.encode(ciphertext),
        })
    }
    pub fn decrypt(
        &self,
        envelope: &EncryptedEnvelope,
        associated_data: &[u8],
    ) -> Result<Vec<u8>, EncryptionError> {
        let key = *self
            .keys
            .read()
            .expect("key ring poisoned")
            .get(&envelope.key_id)
            .ok_or(EncryptionError::UnknownKey)?;
        let nonce_bytes = URL_SAFE_NO_PAD
            .decode(&envelope.nonce)
            .map_err(|_| EncryptionError::InvalidEnvelope)?;
        if nonce_bytes.len() != 12 {
            return Err(EncryptionError::InvalidEnvelope);
        }
        let ciphertext = URL_SAFE_NO_PAD
            .decode(&envelope.ciphertext)
            .map_err(|_| EncryptionError::InvalidEnvelope)?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| EncryptionError::InvalidKey)?;
        cipher
            .decrypt(
                Nonce::from_slice(&nonce_bytes),
                aes_gcm::aead::Payload {
                    msg: &ciphertext,
                    aad: associated_data,
                },
            )
            .map_err(|_| EncryptionError::AuthenticationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypts_and_decrypts_with_authenticated_metadata() {
        let ring = KeyRing::new("v1", [7; 32]).unwrap();
        let envelope = ring.encrypt(b"secret", b"tenant-a").unwrap();
        assert_eq!(ring.decrypt(&envelope, b"tenant-a").unwrap(), b"secret");
        assert_eq!(
            ring.decrypt(&envelope, b"tenant-b"),
            Err(EncryptionError::AuthenticationFailed)
        );
    }

    #[test]
    fn rotates_keys_while_retaining_old_key_decryption() {
        let ring = KeyRing::new("v1", [1; 32]).unwrap();
        let old = ring.encrypt(b"old", b"").unwrap();
        ring.rotate("v2", [2; 32]).unwrap();
        let fresh = ring.encrypt(b"new", b"").unwrap();
        assert_eq!(old.key_id, "v1");
        assert_eq!(fresh.key_id, "v2");
        assert_eq!(ring.decrypt(&old, b"").unwrap(), b"old");
    }

    #[test]
    fn rejects_tampering_and_unknown_keys() {
        let ring = KeyRing::new("v1", [3; 32]).unwrap();
        let mut envelope = ring.encrypt(b"secret", b"").unwrap();
        envelope.ciphertext.replace_range(0..1, "A");
        assert_eq!(
            ring.decrypt(&envelope, b""),
            Err(EncryptionError::AuthenticationFailed)
        );
        envelope.key_id = "missing".into();
        assert_eq!(
            ring.decrypt(&envelope, b""),
            Err(EncryptionError::UnknownKey)
        );
    }
}
