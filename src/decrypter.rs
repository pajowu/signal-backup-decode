use aes_ctr::stream_cipher::NewStreamCipher;
use aes_ctr::stream_cipher::SyncStreamCipher;
use anyhow::anyhow;
use hmac::crypto_mac::Mac;
use hmac::crypto_mac::NewMac;
use sha2::Digest;
use subtle::ConstantTimeEq;

/// Decrypt bytes
pub struct Decrypter {
    mac: Option<hmac::Hmac<sha2::Sha256>>,
    cipher: aes_ctr::Aes256Ctr,
    key: Vec<u8>,
    iv: Vec<u8>,
}

impl Decrypter {
    pub fn new(key: &[u8], salt: &[u8], iv: &[u8], verify_mac: bool) -> Self {
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

        // create hmac and cipher
        Self {
            mac: if verify_mac {
                Some(hmac::Hmac::<sha2::Sha256>::new_varkey(&okm[32..]).unwrap())
            } else {
                None
            },
            cipher: aes_ctr::Aes256Ctr::new(
                generic_array::GenericArray::from_slice(&okm[..32]),
                generic_array::GenericArray::from_slice(&iv),
            ),
            key: okm[..32].to_vec(),
            iv: iv.to_vec(),
        }
    }

    pub fn decrypt(&mut self, mut data_decrypt: &mut [u8]) {
        // check hmac?
        if let Some(ref mut hmac) = self.mac {
            // calculate hmac of frame data
            hmac.update(&data_decrypt);
        }

        // decrypt
        self.cipher.apply_keystream(&mut data_decrypt);
    }

    pub fn mac_update_with_iv(&mut self) {
        if let Some(ref mut hmac) = self.mac {
            hmac.update(&self.iv);
        }
    }

    pub fn verify_mac(&mut self, hmac_control: &[u8]) -> Result<(), anyhow::Error> {
        if let Some(ref mut hmac) = self.mac {
            let result = hmac.finalize_reset();
            let code_bytes = &result.into_bytes()[..10];

            // compare to given hmac
            let result = code_bytes.ct_eq(&hmac_control);

            if result.unwrap_u8() == 0 {
                return Err(anyhow!("HMAC verification failed"));
            }
        }

        Ok(())
    }

    // TODO what is happening here?
    pub fn increase_iv(&mut self) {
        let mut i = 3;

        loop {
            if self.iv[i] < 255 {
                self.iv[i] += 1;
                break;
            } else {
                self.iv[i] = 0;
                i -= 1;
            }
        }

        self.cipher = aes_ctr::Aes256Ctr::new(
            generic_array::GenericArray::from_slice(&self.key),
            generic_array::GenericArray::from_slice(&self.iv),
        );
    }
}