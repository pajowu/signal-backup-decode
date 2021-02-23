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
		debug!("Frame type: {}", &frame);

		// check that frame is a header and return
		match &frame {
			crate::frame::Frame::Header { salt, iv } => Ok(Self {
				reader,
				decrypter: crate::decrypter::Decrypter::new(&password, &salt, &iv, verify_mac),
				count_frame: 1,
				// We already read `len` and 4 bytes with read_u32
				// There are 16 bytes missing somewhere independent of the input
				// file. However, I don't know why.
				count_byte: len + std::mem::size_of::<u32>() + 16,
				file_bytes,
			}),
			_ => Err(anyhow!("first frame is not a header")),
		}
	}

	fn read_data(
		&mut self,
		length: usize,
		read_attachment: bool,
	) -> Result<Vec<u8>, anyhow::Error> {
		let mut hmac = [0u8; crate::decrypter::LENGTH_HMAC];
		let mut data;

		// Reading files (attachments) need an update of MAC with IV.
		// And their given length corresponds to file length but frame length corresponds
		// to data length + hmac data.
		if read_attachment {
			self.decrypter.mac_update_with_iv();
			data = vec![0u8; length];
		} else {
			data = vec![0u8; length - crate::decrypter::LENGTH_HMAC];
		}

		// read data and decrypt
		self.reader.read_exact(&mut data)?;
		let data = self.decrypter.decrypt(&mut data);

		// read hmac
		self.reader.read_exact(&mut hmac)?;

		// verify mac
		self.decrypter.verify_mac(&hmac)?;
		self.decrypter.increase_iv();

		if read_attachment {
			// we got file length, so we have to add 10 bytes for hmac data
			self.count_byte += length + crate::decrypter::LENGTH_HMAC;
		} else {
			// in the case of frames, we add 4 bytes we have read to determine frame length
			// (hmac data is already in length included)
			self.count_byte += length + std::mem::size_of::<u32>();
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
		debug!(
			"Read frame number {} with length of {} bytes",
			self.count_frame, len
		);

		// create frame
		let frame = self.read_data(len, false)?;
		let mut frame: crate::frame::Frame = frame.try_into()?;
		debug!("Frame type: {}", &frame);

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
