use log::info;

/// Write no output of backup
///
/// This output module does not write any backup files. This module can be used to check HMAC of
/// the backup file but not writing any output.
pub struct SignalOutputNone {
	written_frames: usize,
}

impl SignalOutputNone {
	/// Creates new output object
	///
	/// `force_write` determines whether existing files will be overwritten.
	pub fn new() -> Self {
		info!("No output will be written");

		Self {
			// we set read frames to 1 due to the header frame we will never write
			written_frames: 1,
		}
	}
}

impl crate::output::SignalOutput for SignalOutputNone {
	fn write_statement(
		&mut self,
		_statement: &str,
		_parameters: &[rusqlite::types::Value],
	) -> Result<(), anyhow::Error> {
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

	fn write_key_value(&mut self, key_value: &crate::Backups::KeyValue) ->  Result<(), anyhow::Error>{
		self.written_frames += 1;
		Ok(())
	}

	fn finish(&mut self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}
