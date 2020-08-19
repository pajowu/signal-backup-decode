use log::info;

/// Write no output of backup
///
/// This output module does not write any backup files. This module can be used to check HMAC of
/// the backup file but not writing any output.
pub struct SignalOutputCsv {
	written_frames: usize,
}

impl SignalOutputCsv {
	/// Creates new output object
	///
	/// `force_write` determines whether existing files will be overwritten.
	pub fn new() -> Self {
		info!("No output will be written");

		Self {
			// we set 2 read frames in the beginning because we have 1) a header frame
			// and 2) a version frame we do not count in written frames.
			written_frames: 2,
		}
	}
}

impl crate::output::SignalOutput for SignalOutputCsv {
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
}
