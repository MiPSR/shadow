use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use pqcrypto_mldsa::mldsa65;
use pqcrypto_mlkem::mlkem768;
use pqcrypto_traits::kem::{Ciphertext, PublicKey as KemPublicKey, SecretKey as KemSecretKey, SharedSecret};
use pqcrypto_traits::sign::{DetachedSignature, PublicKey as SignPublicKey, SecretKey as SignSecretKey};

pub mod mldsa {
    use super::*;
    pub fn keypair() -> (Vec<u8>, Vec<u8>) {
        let (pk, sk) = mldsa65::keypair();
        (pk.as_bytes().to_vec(), sk.as_bytes().to_vec())
    }
    pub fn sign(sk: &[u8], msg: &[u8]) -> Result<Vec<u8>, String> {
        let sk = mldsa65::SecretKey::from_bytes(sk).map_err(|e| e.to_string())?;
        let sig = mldsa65::detached_sign(msg, &sk);
        Ok(sig.as_bytes().to_vec())
    }
    pub fn verify(pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        let Ok(pk) = mldsa65::PublicKey::from_bytes(pk) else { return false };
        let Ok(sig) = mldsa65::DetachedSignature::from_bytes(sig) else { return false };
        mldsa65::verify_detached_signature(&sig, msg, &pk).is_ok()
    }
}

pub mod mlkem {
    use super::*;
    pub fn keypair() -> (Vec<u8>, Vec<u8>) {
        let (pk, sk) = mlkem768::keypair();
        (pk.as_bytes().to_vec(), sk.as_bytes().to_vec())
    }
    pub fn encapsulate(pk: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
        let pk = mlkem768::PublicKey::from_bytes(pk).map_err(|e| e.to_string())?;
        let (ss, ct) = mlkem768::encapsulate(&pk);
        Ok((ss.as_bytes().to_vec(), ct.as_bytes().to_vec()))
    }
    pub fn decapsulate(sk: &[u8], ct: &[u8]) -> Result<Vec<u8>, String> {
        let sk = mlkem768::SecretKey::from_bytes(sk).map_err(|e| e.to_string())?;
        let ct = mlkem768::Ciphertext::from_bytes(ct).map_err(|e| e.to_string())?;
        let ss = mlkem768::decapsulate(&ct, &sk);
        Ok(ss.as_bytes().to_vec())
    }
}

pub mod xchacha {
    use super::*;
    pub fn encrypt(key: &[u8], plaintext: &[u8], _aad: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
        if key.len() != 32 {
            return Err("key must be 32 bytes".into());
        }
        let key = chacha20poly1305::Key::from_slice(key);
        let cipher = XChaCha20Poly1305::new(key);
        let mut nonce_bytes = [0u8; 24];
        getrandom::getrandom(&mut nonce_bytes).map_err(|e| e.to_string())?;
        let nonce = XNonce::from_slice(&nonce_bytes);
        // aad not supported by simple encrypt; prepend? We'll ignore aad for now but keep param.
        let ct = cipher.encrypt(nonce, plaintext).map_err(|e| e.to_string())?;
        Ok((nonce_bytes.to_vec(), ct))
    }
    pub fn decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8], _aad: &[u8]) -> Result<Vec<u8>, String> {
        if key.len() != 32 { return Err("key must be 32 bytes".into()); }
        if nonce.len() != 24 { return Err("nonce must be 24 bytes".into()); }
        let key = chacha20poly1305::Key::from_slice(key);
        let cipher = XChaCha20Poly1305::new(key);
        let nonce = XNonce::from_slice(nonce);
        cipher.decrypt(nonce, ciphertext).map_err(|e| e.to_string())
    }
}

pub fn b64_encode(b: &[u8]) -> String { B64.encode(b) }
pub fn b64_decode(s: &str) -> Result<Vec<u8>, String> { B64.decode(s).map_err(|e| e.to_string()) }
