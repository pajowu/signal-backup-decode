use aes_ctr::stream_cipher::NewStreamCipher;
use hmac::crypto_mac::NewMac;
use sha2::Digest;

/// Decrypt bytes
pub struct Decrypter {
    mac: hmac::Hmac<sha2::Sha256>,
    cipher: aes_ctr::Aes256Ctr,
}

impl Decrypter {
    pub fn new(key: &[u8], salt: &[u8], iv: &[u8]) -> Self {
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

        //Self {
        //    cipher_key: okm[..32].to_vec(),
        //    mac_key: okm[32..].to_vec(),
        //}

        // create hmac and cipher
        Self {
            mac: hmac::Hmac::<sha2::Sha256>::new_varkey(&okm[32..]).unwrap(),
            cipher: aes_ctr::Aes256Ctr::new(
                generic_array::GenericArray::from_slice(&okm[..32]),
                generic_array::GenericArray::from_slice(&iv),
            ),
        }
    }
}
