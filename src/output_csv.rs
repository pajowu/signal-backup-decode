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

		Self::set_directory(&path, "", force_overwrite)?;

		// determine csv path
		let path_csv = path.join("signal_backup.csv");

		if path_csv.exists() {
			if force_overwrite {
				std::fs::remove_file(&path_csv).with_context(|| {
					format!(
						"could not delete old file: {}",
						path_csv.to_string_lossy()
					)
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

	fn set_directory(
		base: &std::path::Path,
		name: &str,
		force_overwrite: bool,
	) -> Result<std::path::PathBuf, anyhow::Error> {
		let folder = base.join(name);

		// check output path
		if !force_overwrite && folder.exists() {
			return Err(anyhow!(
				"{} already exists and may not be overwritten. Try -f",
				folder.to_string_lossy()
			));
		}

		if folder.exists() && !folder.is_dir() {
			return Err(anyhow!(
				"{} exists and is not a directory",
				folder.to_string_lossy()
			));
		}

		if !folder.exists() {
			std::fs::create_dir(&folder)
				.with_context(|| format!("{} could not be created", folder.to_string_lossy()))?;
		}

		Ok(folder)
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

	fn get_written_frames(&self) -> usize {
		self.written_frames
	}
}
