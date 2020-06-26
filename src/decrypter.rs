use sha2::Digest;

/// Decrypt bytes
pub struct Decrypter {
    cipher_key: Vec<u8>,
    mac_key: Vec<u8>,
}

impl Decrypter {
    pub fn new(key: &[u8], salt: &[u8]) -> Self {
        // create hash
        let mut hash = key.to_vec();
        let mut hasher = sha2::Sha512::new();
        hasher.update(&salt);

        for _ in 0..250000 {
            hasher.update(&hash);
            hasher.update(key);
            hash = hasher.finalize_reset().to_vec();
        }

        // create secrets
        let info = b"Backup Export";
        let mut okm = [0u8; 64];
        let hk = hkdf::Hkdf::<sha2::Sha256>::new(None, &hash[..32]);
        hk.expand(info, &mut okm).unwrap();

        Self {
            cipher_key: okm[..32].to_vec(),
            mac_key: okm[32..].to_vec(),
        }

        //fn generate_keys(key: &[u8], salt: &[u8]) -> Result<([u8; 32], [u8; 32]), anyhow::Error> {
        //    let mut digest = Hasher::new(MessageDigest::sha512())?;
        //    digest.update(salt)?;
        //    let mut hash = key.to_vec();
        //    for _ in 0..250000 {
        //        digest.update(&hash)?;
        //        digest.update(key)?;
        //        hash = digest.finish()?.to_vec();
        //    }
        //    let backup_key = &hash[..32];
        //    Ok(derive_secrets(backup_key, b"Backup Export", 64))
        //}
        //fn derive_secrets(key: &[u8], info: &[u8], length: usize) -> ([u8; 32], [u8; 32]) {
        //    let mut prk = [0u8; 32];
        //    crypto::hkdf::hkdf_extract(crypto::sha2::Sha256::new(), &[0u8; 32], key, &mut prk);
        //    let mut sec = vec![0u8; length];
        //    crypto::hkdf::hkdf_expand(crypto::sha2::Sha256::new(), &prk, info, &mut sec);
        //    let mut sec1: [u8; 32] = Default::default();
        //    let mut sec2: [u8; 32] = Default::default();
        //    sec1.copy_from_slice(&sec[..32]);
        //    sec2.copy_from_slice(&sec[32..]);
        //    (sec1, sec2)
        //}
    }
}
