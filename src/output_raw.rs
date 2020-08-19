use anyhow::anyhow;
use anyhow::Context;
use log::{debug, info};
use std::io::Write;

/// Write raw backup
///
/// This output module writes the backup in a sqlite database and media files in different
/// directories.
pub struct SignalOutputRaw {
	path_avatar: std::path::PathBuf,
	path_attachment: std::path::PathBuf,
	path_sticker: std::path::PathBuf,
	path_config: std::path::PathBuf,
	sqlite_connection: rusqlite::Connection,
	count_sticker: usize,
	count_avatar: usize,
	written_frames: usize,
	force_overwrite: bool,
}

impl SignalOutputRaw {
	/// Creates new output object
	///
	/// `force_write` determines whether existing files will be overwritten.
	pub fn new(path: &std::path::Path, force_overwrite: bool) -> Result<Self, anyhow::Error> {
		info!("Output path: {}", &path.to_string_lossy());

		Self::set_directory(&path, "", force_overwrite)?;

		// determine sqlite path
		let path_sqlite = path.join("signal_backup.db");

		if path_sqlite.exists() {
			if force_overwrite {
				std::fs::remove_file(&path_sqlite).with_context(|| {
					format!(
						"could not delete old database: {}",
						path_sqlite.to_string_lossy()
					)
				})?;
			} else {
				return Err(anyhow!(
					"Backup database already exists and may not be overwritten. Try -f"
				));
			}
		}

		Ok(Self {
			path_avatar: Self::set_directory(&path, "avatar", force_overwrite)?,
			path_attachment: Self::set_directory(&path, "attachment", force_overwrite)?,
			path_sticker: Self::set_directory(&path, "sticker", force_overwrite)?,
			path_config: Self::set_directory(&path, "config", force_overwrite)?,
			sqlite_connection: rusqlite::Connection::open(&path_sqlite).with_context(|| {
				format!(
					"could not open connection to database file: {}.",
					path_sqlite.to_string_lossy()
				)
			})?,
			count_sticker: 0,
			count_avatar: 0,
			// we set read frames to 1 due to the header frame we will never write
			written_frames: 1,
			force_overwrite,
		})
	}

	fn write_to_file(&self, path: &std::path::Path, data: &[u8]) -> Result<(), anyhow::Error> {
		// determine mime type
		let infer = infer::Infer::new();
		let mut path = path.to_path_buf();
		if let Some(x) = infer.get(&data) {
			path.set_extension(x.ext);
		}

		if path.exists() && !self.force_overwrite {
			return Err(anyhow!(
				"File already exists and may not be overwritten: {}",
				path.to_string_lossy()
			));
		}

		let mut buffer = std::fs::File::create(&path).with_context(|| {
			format!("Failed to open attachment file: {}", path.to_string_lossy())
		})?;

		buffer.write_all(data).with_context(|| {
			format!(
				"Failed to write to attachment file: {}",
				path.to_string_lossy()
			)
		})?;

		Ok(())
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

impl crate::output::SignalOutput for SignalOutputRaw {
	fn write_statement(
		&mut self,
		statement: &str,
		parameters: &[rusqlite::types::Value],
	) -> Result<(), anyhow::Error> {
		// In database version 9 signal added full text search and uses TRIGGERs to create the virtual tables. however this breaks when importing the data.
		if statement.starts_with("CREATE TRIGGER")
			|| statement.contains("_fts")
			|| statement.starts_with("CREATE TABLE sqlite_")
		{
			return Ok(());
		}

		debug!("Write statement: {}", &statement);

		let mut stmt = self
			.sqlite_connection
			.prepare_cached(statement)
			.with_context(|| format!("failed to prepare database statement: {}", statement))?;
		stmt.execute(parameters)
			.with_context(|| format!("failed to execute database statement: {}", statement))?;

		self.written_frames += 1;

		Ok(())
	}

	fn write_attachment(
		&mut self,
		data: &[u8],
		attachmend_id: u64,
		row_id: u64,
	) -> Result<(), anyhow::Error> {
		let path = self
			.path_attachment
			.join(format!("{}_{}", attachmend_id, row_id));
		self.write_to_file(&path, &data)?;

		self.written_frames += 1;

		Ok(())
	}

	fn write_sticker(&mut self, data: &[u8], row_id: u64) -> Result<(), anyhow::Error> {
		//let mut path = self.path_sticker.join(format!("{}_{}", row_id, 1));
		//if path.exists() {
		//    path = self.path_sticker.join(format!("{}_{}", row_id, 2));
		//}

		let path = self
			.path_sticker
			.join(format!("{}_{}", row_id, self.count_sticker));
		self.write_to_file(&path, &data)?;

		self.count_sticker += 1;
		self.written_frames += 1;

		Ok(())
	}

	fn write_avatar(&mut self, data: &[u8], name: &str) -> Result<(), anyhow::Error> {
		//let mut path = self.path_sticker.join(format!("{}_{}", row_id, 1));
		//if path.exists() {
		//    path = self.path_sticker.join(format!("{}_{}", row_id, 2));
		//}

		let path = self
			.path_avatar
			.join(format!("{}_{}", name, self.count_avatar));
		self.write_to_file(&path, &data)?;

		self.count_avatar += 1;
		self.written_frames += 1;

		Ok(())
	}

	fn write_preference(
		&mut self,
		pref: &crate::Backups::SharedPreference,
	) -> Result<(), anyhow::Error> {
		let path = self.path_config.join(pref.get_file());
		let mut conf = ini::Ini::load_from_file(&path).unwrap_or_default();
		conf.with_section(None::<String>)
			.set(pref.get_key(), pref.get_value());
		conf.write_to_file(&path).with_context(|| {
			format!(
				"Could not write to preference file: {}",
				path.to_string_lossy()
			)
		})?;

		self.written_frames += 1;

		Ok(())
	}

	fn write_version(&mut self, version: u32) -> Result<(), anyhow::Error> {
		info!("Database Version: {:?}", version);
		self.written_frames += 1;
		Ok(())
	}

	fn get_written_frames(&self) -> usize {
		self.written_frames
	}
}
