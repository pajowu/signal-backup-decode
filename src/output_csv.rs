use anyhow::anyhow;
use anyhow::Context;
use log::info;

/// Write csv output of backup
pub struct SignalOutputCsv {
	writer: csv::Writer<std::fs::File>,
	written_frames: usize,
}

impl SignalOutputCsv {
	/// Creates new output object
	///
	/// `force_write` determines whether existing files will be overwritten.
	pub fn new(path: &std::path::Path, force_overwrite: bool) -> Result<Self, anyhow::Error> {
		info!("Output path: {}", &path.to_string_lossy());

		// check output path
		if path.exists() && !path.is_dir() {
			return Err(anyhow!(
				"{} exists and is not a directory",
				path.to_string_lossy()
			));
		} else {
			std::fs::create_dir_all(&path).with_context(|| {
				format!("Path could not be created: {}", path.to_string_lossy())
			})?;
		}

		// open csv connection
		let path_csv = path.join("signal_backup.csv");

		if path_csv.exists() {
			if force_overwrite {
				std::fs::remove_file(&path_csv).with_context(|| {
					format!("Could not delete old file: {}", path_csv.to_string_lossy())
				})?;
			} else {
				return Err(anyhow!(
					"Backup file already exists and may not be overwritten. Try -f"
				));
			}
		}

		Ok(Self {
			writer: csv::Writer::from_path(path_csv)?,
			// we set read frames to 1 due to the header frame we will never write
			written_frames: 1,
		})
	}
}

impl crate::output::SignalOutput for SignalOutputCsv {
	fn write_statement(
		&mut self,
		statement: &str,
		parameters: &[rusqlite::types::Value],
	) -> Result<(), anyhow::Error> {
		if statement.starts_with("INSERT INTO sms") {
			let mess = crate::message::Message::new(parameters);
			self.writer.serialize(mess)?;
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

	fn write_key_value(&mut self, key_value: &crate::Backups::KeyValue) ->  Result<(), anyhow::Error>{
		self.written_frames += 1;
		Ok(())
	}

	fn get_written_frames(&self) -> usize {
		self.written_frames
	}

	fn finish(&mut self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}
