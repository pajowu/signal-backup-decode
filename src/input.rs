use anyhow::anyhow;
use anyhow::Context;
use byteorder::ReadBytesExt;
use log::{debug, info};
use std::convert::TryInto;
use std::io::Read;

/// Read input file
pub struct InputFile {
	reader: std::io::BufReader<std::fs::File>,
	decrypter: crate::decrypter::Decrypter,
	count_frame: usize,
	count_byte: usize,
	file_bytes: u64,
}

impl InputFile {
	pub fn new(
		path: &std::path::Path,
		password: &[u8],
		verify_mac: bool,
	) -> Result<Self, anyhow::Error> {
		// open file
		info!("Input file: {}", &path.to_string_lossy());
		let file = std::fs::File::open(path)
			.with_context(|| format!("Could not open backup file: {}", path.to_string_lossy()))?;
		let file_bytes = file.metadata().unwrap().len();
		let mut reader = std::io::BufReader::new(file);

		// create decrypter
		// - read first frame
		let len: usize = reader
			.read_u32::<byteorder::BigEndian>()
			.unwrap()
			.try_into()
			.unwrap();
		let mut frame = vec![0u8; len];
		reader.read_exact(&mut frame)?;
		let frame: crate::frame::Frame = frame.try_into()?;

		// check that frame is a header and return
		match &frame {
			crate::frame::Frame::Header { salt, iv } => {
				debug!("Found header: {}", &frame);
				Ok(Self {
					reader,
					decrypter: crate::decrypter::Decrypter::new(&password, &salt, &iv, verify_mac),
					count_frame: 1,
					count_byte: len,
					file_bytes,
				})
			}
			_ => Err(anyhow!("first frame is not a header")),
		}
	}

        fn read_data(&mut self, length: usize, read_attachment: bool) -> Result<Vec<u8>, anyhow::Error> {
		let mut data = std::vec::Vec::with_capacity(length - 10);
		let mut hmac = [0u8; 10];

                if read_attachment {
                    self.decrypter.mac_update_with_iv();
                }

                // read data
                self.reader.read_exact(&mut data)?;
                self.reader.read_exact(&mut hmac)?;

                // decrypt
                self.decrypter.decrypt(&mut data);

                // verify mac
		self.decrypter.verify_mac(&hmac)?;
		self.decrypter.increase_iv();

                if !read_attachment {
                    // we add 4 bytes to take read frame length into account
                    self.count_byte += length + (u32::MAX / u8::MAX as u32) as usize;
                } else {
                    // we haven't read frame length here, so we don't need it!
                    self.count_byte += length;
                }
		Ok(data)
        }

	pub fn read_frame(&mut self) -> Result<crate::frame::Frame, anyhow::Error> {
		// read frame length from input file
		let len: usize = self
			.reader
			.read_u32::<byteorder::BigEndian>()
			.unwrap()
			.try_into()
			.unwrap();
                debug!("Read frame number {} with length {} (bytes)", self.count_frame, len);

		// create frame
                let frame = self.read_data(len, false)?;
                let mut frame: crate::frame::Frame = frame.try_into()?;

		match frame {
			crate::frame::Frame::Attachment { data_length, .. } => {
				frame.set_data(self.read_data(data_length, true)?);
			}
			crate::frame::Frame::Avatar { data_length, .. } => {
				frame.set_data(self.read_data(data_length, true)?);
			}
			crate::frame::Frame::Sticker { data_length, .. } => {
				frame.set_data(self.read_data(data_length, true)?);
			}
			crate::frame::Frame::Header { .. } => return Err(anyhow!("unexpected header found")),
			_ => (),
		};

		// clean up and return
		self.count_frame += 1;
		Ok(frame)
	}

	pub fn get_count_frame(&self) -> usize {
		self.count_frame
	}

	pub fn get_count_byte(&self) -> usize {
		self.count_byte
	}

	pub fn get_file_size(&self) -> u64 {
		self.file_bytes
	}
}

impl Iterator for InputFile {
	type Item = Result<crate::frame::Frame, anyhow::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		let ret = self.read_frame();

		if let Ok(x) = ret {
			match x {
				crate::frame::Frame::End => None,
				_ => Some(Ok(x)),
			}
		} else {
			Some(ret)
		}
	}
}
