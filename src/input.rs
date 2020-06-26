use anyhow::Context;

/// Read input file
pub struct InputFile {
    reader: std::io::BufReader<std::fs::File>,
}

impl InputFile {
    pub fn new(path: &std::path::Path) -> Result<Self, anyhow::Error> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("Could not open backup file: {}", path.to_string_lossy()))?;

        Ok(Self {
            reader: std::io::BufReader::new(file),
        })
    }

    //pub fn read_frame(&mut self) -> Result<&crate::frame::Frame, anyhow::Error> {
    //    let len = self.reader.read_u32::<BigEndian>()?.try_into()?;
    //    let mut frame_content = vec![0u8; len as usize];
    //    self.reader.read_exact(&mut frame_content)?;

    //    match *cipher_data {
    //        None => Ok((len, frame_content)),
    //        Some(ref mut cipher_data) => {
    //            let frame_data = &frame_content[..frame_content.len() - 10];
    //            if verify_mac {
    //                let frame_mac = &frame_content[frame_content.len() - 10..];
    //                cipher_data.hmac.input(&frame_data);
    //                let hmac_result = cipher_data.hmac.result();
    //                let calculated_mac = &hmac_result.code()[..10];
    //                cipher_data.hmac.reset();
    //                if !crypto::util::fixed_time_eq(calculated_mac, frame_mac) {
    //                    return Err(anyhow!(
    //                        "MacVerificationError, {:?}, {:?}.",
    //                        calculated_mac.to_vec(),
    //                        frame_mac.to_vec(),
    //                    ));
    //                }
    //            }
    //            let plaintext = decrypt(&cipher_data.cipher_key, &cipher_data.counter, frame_data)?;
    //            increase_counter(&mut cipher_data.counter, None);
    //            Ok((len, plaintext))
    //        }
    //    }
    //}
}

//impl Iterator for InputFile {
//    type Item = super::frame::Frame;
//
//    fn next(&self) -> Option<Self::Item> {
//
//    }
//}
