use anyhow::anyhow;
use anyhow::Context;
use byteorder::ReadBytesExt;
use std::convert::TryInto;
use std::io::Read;

/// Read input file
pub struct InputFile {
	reader: std::io::BufReader<std::fs::File>,
	decrypter: crate::decrypter::Decrypter,
	count_frame: usize,
	count_byte: usize,
}

impl InputFile {
	pub fn new(
		path: &std::path::Path,
		password: &[u8],
		verify_mac: bool,
	) -> Result<Self, anyhow::Error> {
		// open file
		let file = std::fs::File::open(path)
			.with_context(|| format!("Could not open backup file: {}", path.to_string_lossy()))?;
		let mut reader = std::io::BufReader::new(file);

		// create decrypter
		// - read first frame
		let len: usize = reader
			.read_u32::<byteorder::BigEndian>()
			.unwrap()
			.try_into()
			.unwrap();
		let mut frame_content = vec![0u8; len];
		reader.read_exact(&mut frame_content)?;
		let frame = protobuf::parse_from_bytes::<crate::Backups::BackupFrame>(&frame_content)
			.with_context(|| format!("Could not parse frame from {:?}", frame_content))?;
		let frame = crate::frame::Frame::new(&frame);

		// check that frame is a header and return
		match frame {
			crate::frame::Frame::Header { salt, iv } => Ok(Self {
				reader,
				decrypter: crate::decrypter::Decrypter::new(&password, salt, iv, verify_mac),
				count_frame: 1,
				count_byte: len,
			}),
			_ => Err(anyhow!("first frame is not a header")),
		}
	}

	pub fn read_data(&mut self, length: usize) -> Result<Vec<u8>, anyhow::Error> {
		let mut bytes_left = length;
		let mut attachment_data = std::vec::Vec::with_capacity(length - 10);
		let mut attachment_hmac = [0u8; 10];

		self.decrypter.mac_update_with_iv();

		while bytes_left > 0 {
			let mut buffer = vec![0u8; std::cmp::min(bytes_left, 8192)];
			self.reader.read_exact(&mut buffer)?;
			bytes_left -= buffer.len();
			self.decrypter.decrypt(&mut buffer);
			attachment_data.append(&mut buffer);
		}

		self.reader.read_exact(&mut attachment_hmac)?;
		self.decrypter.verify_mac(&attachment_hmac)?;
		self.decrypter.increase_iv();

		self.count_byte += length;
		Ok(attachment_data)
	}

	pub fn read_frame(&mut self) -> Result<Vec<u8>, anyhow::Error> {
		let len: usize = self
			.reader
			.read_u32::<byteorder::BigEndian>()
			.unwrap()
			.try_into()
			.unwrap();
		let len = len as usize;
		let mut frame_content = vec![0u8; len - 10];
		let mut frame_hmac = [0u8; 10];

		self.reader.read_exact(&mut frame_content)?;
		self.reader.read_exact(&mut frame_hmac)?;
		self.decrypter.decrypt(&mut frame_content);
		self.decrypter.verify_mac(&frame_hmac)?;
		self.decrypter.increase_iv();

		self.count_frame += 1;
		self.count_byte += len;
		Ok(frame_content)
	}

	pub fn get_count_frame(&self) -> usize {
		self.count_frame
	}

	pub fn get_count_byte(&self) -> usize {
		self.count_byte
	}
}

//impl Iterator for InputFile {
//    type Item = Result<Vec<u8>, anyhow::Error>;
//
//    fn next(&mut self) -> Option<Self::Item> {
//        let len = self.reader.read_u32::<byteorder::BigEndian>()?.try_into()?;
//        let mut frame_content = std::vec::Vec::with_capacity(len as usize - 10);
//        let mut frame_hmac = [0u8; 10];
//
//        self.reader.read_exact(&mut frame_content)?;
//        self.reader.read_exact(&mut frame_hmac)?;
//        self.decrypter.decrypt(&mut frame_content);
//        self.decrypter.verify_mac(&frame_hmac)?;
//        self.decrypter.increase_iv();
//
//        self.count_frame += 1;
//        self.count_byte += len as usize;
//        Some(Ok(frame_content))
//
//        //let frame = protobuf::parse_from_bytes::<crate::Backups::BackupFrame>(&frame_content)
//        //    .with_context(|| format!("Could not parse frame: {:?}", frame_content))?;
//        //let frame = crate::frame::Frame::new(frame);
//
//        //match frame {
//        //    crate::frame::Frame::End => None,
//        //    _ => {
//        //        self.count_frame += 1;
//        //        self.count_byte += len as usize;
//        //        Some(Ok(frame.to_owned()))
//        //    }
//        //}
//    }
//}
