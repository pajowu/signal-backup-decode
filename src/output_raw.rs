use anyhow::anyhow;
use anyhow::Context;
use log::info;
use std::io::Write;

/// Write raw backup
///
/// This output module writes the backup in a sqlite database and media files in different
/// directories.
pub struct Output {
	path_output: std::path::PathBuf,
	force_write: bool,
	sqlite_connection: rusqlite::Connection,
	sqlite_in_memory: bool,
	count_attachment: usize,
	count_sticker: usize,
	count_avatar: usize,
	written_frames: usize,
}

impl Output {
	/// Creates new output object
	///
	/// `force_write` determines whether existing files will be overwritten.
	pub fn new(
		path: &std::path::Path,
		force_write: bool,
		open_db_in_memory: bool,
	) -> Result<Self, anyhow::Error> {
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

		// open database connection
		let path_sqlite = path.join("signal_backup.db");

		if path_sqlite.exists() && !force_write {
			if force_write {
				std::fs::remove_file(&path_sqlite).with_context(|| {
					format!(
						"Could not delete old database: {}",
						path_sqlite.to_string_lossy()
					)
				})?;
			} else {
				return Err(anyhow!(
					"Sqlite database already exists: {}. Try -f",
					path_sqlite.to_string_lossy()
				));
			}
		}

		let sqlite_connection = if open_db_in_memory {
			rusqlite::Connection::open_in_memory()
				.with_context(|| "could not open connection to in memory database".to_string(),)?
		} else {
			rusqlite::Connection::open(&path_sqlite).with_context(|| {
				format!(
					"could not open connection to database file: {}",
					path_sqlite.to_string_lossy()
				)
			})?
		};

		Ok(Self {
			path_output: path.to_path_buf(),
			force_write,
			sqlite_connection,
			sqlite_in_memory: open_db_in_memory,
			count_attachment: 0,
			count_sticker: 0,
			count_avatar: 0,
			// we set read frames to 1 due to the header frame we will never write
			written_frames: 1,
		})
	}

	pub fn write_statement(
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

		let mut stmt = self
			.sqlite_connection
			.prepare_cached(statement)
			.with_context(|| format!("failed to prepare database statement: {}", statement))?;
		stmt.execute(parameters)
			.with_context(|| format!("failed to execute database statement: {}", statement))?;

		self.written_frames += 1;

		Ok(())
	}

	pub fn write_attachment(
		&mut self,
		data: &[u8],
		attachmend_id: u64,
		row_id: u64,
	) -> Result<(), anyhow::Error> {
		// create path to attachment file
		let path = self.path_output.join("attachment");
		std::fs::create_dir_all(&path)
			.with_context(|| format!("Failed to create path: {}", path.to_string_lossy()))?;

		// open connection to file
		let path = path.join(format!("{}_{}", attachmend_id, row_id));
		if path.exists() && !self.force_write {
			return Err(anyhow!(
				"Attachment file does already exist: {}. Try -f",
				path.to_string_lossy()
			));
		}

		let mut buffer = std::fs::File::create(&path).with_context(|| {
			format!("Failed to open attachment file: {}", path.to_string_lossy())
		})?;

		// write to file
		buffer.write_all(data).with_context(|| {
			format!(
				"Failed to write to attachment file: {}",
				path.to_string_lossy()
			)
		})?;

		self.count_attachment += 1;
		self.written_frames += 1;

		Ok(())
	}

	pub fn write_sticker(&mut self, data: &[u8], row_id: u64) -> Result<(), anyhow::Error> {
		//let mut path = self.path_sticker.join(format!("{}_{}", row_id, 1));
		//if path.exists() {
		//    path = self.path_sticker.join(format!("{}_{}", row_id, 2));
		//}

		// create path to attachment file
		let path = self.path_output.join("sticker");
		std::fs::create_dir_all(&path)
			.with_context(|| format!("Failed to create path: {}", path.to_string_lossy()))?;

		// open connection to file
		let path = path.join(format!("{}_{}", row_id, self.count_sticker));
		if path.exists() && !self.force_write {
			return Err(anyhow!(
				"Sticker file does already exist: {}. Try -f",
				path.to_string_lossy()
			));
		}

		let mut buffer = std::fs::File::create(&path)
			.with_context(|| format!("Failed to open sticker file: {}", path.to_string_lossy()))?;

		// write to file
		buffer.write_all(data).with_context(|| {
			format!(
				"Failed to write to attachment file: {}",
				path.to_string_lossy()
			)
		})?;

		self.count_sticker += 1;
		self.written_frames += 1;

		Ok(())
	}

	pub fn write_avatar(&mut self, data: &[u8], name: &str) -> Result<(), anyhow::Error> {
		//let mut path = self.path_sticker.join(format!("{}_{}", row_id, 1));
		//if path.exists() {
		//    path = self.path_sticker.join(format!("{}_{}", row_id, 2));
		//}

		// create path to attachment file
		let path = self.path_output.join("avatar");
		std::fs::create_dir_all(&path)
			.with_context(|| format!("Failed to create path: {}", path.to_string_lossy()))?;

		// open connection to file
		let path = path.join(format!("{}_{}", name, self.count_avatar));
		if path.exists() && !self.force_write {
			return Err(anyhow!(
				"Avatar file does already exist: {}. Try -f",
				path.to_string_lossy()
			));
		}

		let mut buffer = std::fs::File::create(&path)
			.with_context(|| format!("Failed to open avatar file: {}", path.to_string_lossy()))?;

		// write to file
		buffer.write_all(data).with_context(|| {
			format!(
				"Failed to write to attachment file: {}",
				path.to_string_lossy()
			)
		})?;

		self.count_avatar += 1;
		self.written_frames += 1;

		Ok(())
	}

	pub fn write_preference(
		&mut self,
		pref: &crate::Backups::SharedPreference,
	) -> Result<(), anyhow::Error> {
		// create path to attachment file
		let path = self.path_output.join("preference");
		std::fs::create_dir_all(&path)
			.with_context(|| format!("Failed to create path: {}", path.to_string_lossy()))?;

		// open connection to file
		let path = path.join(pref.get_file());
		if path.exists() && !self.force_write {
			return Err(anyhow!(
				"Config file does already exist: {}. Try -f",
				path.to_string_lossy()
			));
		}

		// write to file
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

	pub fn write_version(&mut self, version: u32) -> Result<(), anyhow::Error> {
		info!("Database Version: {:?}", version);
		self.written_frames += 1;
		Ok(())
	}

	pub fn write_frame(&mut self, frame: crate::frame::Frame) -> Result<(), anyhow::Error> {
		match frame {
			crate::frame::Frame::Statement {
				statement,
				parameter,
			} => self.write_statement(&statement, &parameter),
			crate::frame::Frame::Preference { preference } => self.write_preference(&preference),
			crate::frame::Frame::Attachment { id, row, data, .. } => {
				self.write_attachment(data.as_ref().unwrap(), id, row)
			}
			crate::frame::Frame::Avatar { name, data, .. } => {
				self.write_avatar(data.as_ref().unwrap(), &name)
			}
			crate::frame::Frame::Sticker { row, data, .. } => {
				self.write_sticker(data.as_ref().unwrap(), row)
			}
			crate::frame::Frame::Version { version } => self.write_version(version),
			_ => Err(anyhow!("unexpected frame found")),
		}
	}

	pub fn get_written_frames(&self) -> usize {
		self.written_frames
	}

	pub fn finish(&mut self) -> Result<(), anyhow::Error> {
		if !self.sqlite_in_memory {
			return Ok(());
		}

		let path_sqlite = self.path_output.join("signal_backup.db");

		self.sqlite_connection
			.execute(
				&format!("VACUUM INTO \"{}\";", path_sqlite.to_string_lossy()),
				rusqlite::NO_PARAMS,
			)
			.with_context(|| {
				format!(
					"Failed to copy in memory database to file: {}",
					path_sqlite.to_string_lossy()
				)
			})?;

		Ok(())
	}
}
