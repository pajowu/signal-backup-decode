/// Write csv output of backup
pub struct SignalOutputCsv {
	written_frames: usize,
}

impl SignalOutputCsv {
	/// Creates new output object
	///
	/// `force_write` determines whether existing files will be overwritten.
	pub fn new() -> Self {
		Self {
			// we set read frames to 1 due to the header frame we will never write
			written_frames: 1,
		}
	}
}

impl crate::output::SignalOutput for SignalOutputCsv {
	fn write_statement(
		&mut self,
		statement: &str,
		parameters: &[rusqlite::types::Value],
	) -> Result<(), anyhow::Error> {
		if statement == "INSERT INTO sms VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)" {
			println!("{:?}", parameters);
		}

		self.written_frames += 1;
		Ok(())
	}

	fn write_attachment(
		&mut self,
		_data: &[u8],
		_attachmend_id: u64,
		_row_id: u64,
	) -> Result<(), anyhow::Error> {
		self.written_frames += 1;
		Ok(())
	}

	fn write_sticker(&mut self, _data: &[u8], _row_id: u64) -> Result<(), anyhow::Error> {
		self.written_frames += 1;
		Ok(())
	}

	fn write_avatar(&mut self, _data: &[u8], _name: &str) -> Result<(), anyhow::Error> {
		self.written_frames += 1;
		Ok(())
	}

	fn write_preference(
		&mut self,
		_pref: &crate::Backups::SharedPreference,
	) -> Result<(), anyhow::Error> {
		self.written_frames += 1;
		Ok(())
	}

	fn write_version(&mut self, _version: u32) -> Result<(), anyhow::Error> {
		self.written_frames += 1;
		Ok(())
	}

	fn get_written_frames(&self) -> usize {
		self.written_frames
	}
}
