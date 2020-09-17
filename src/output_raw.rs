use anyhow::anyhow;
use anyhow::Context;
use log::{debug, info};
use std::io::Write;

/// Write raw backup
///
/// This output module writes the backup in a sqlite database and media files in different
/// directories.
pub struct SignalOutputRaw {
	path_output: std::path::PathBuf,
	force_write: bool,
	sqlite_connection: rusqlite::Connection,
	sqlite_in_memory: bool,
	count_attachment: usize,
	count_sticker: usize,
	count_avatar: usize,
	written_frames: usize,
}

impl SignalOutputRaw {
	/// Creates new output object
	///
	/// `force_write` determines whether existing files will be overwritten.
	pub fn new(
		path: &std::path::Path,
		force_write: bool,
		open_db_in_memory: bool,
	) -> Result<Self, anyhow::Error> {
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

		// open database connection
		let path_sqlite = path.join("signal_backup.db");

		if path_sqlite.exists() {
			if force_write {
				std::fs::remove_file(&path_sqlite).with_context(|| {
					format!(
						"Could not delete old database: {}",
						path_sqlite.to_string_lossy()
					)
				})?;
			} else {
				return Err(anyhow!(
					"Backup database already exists: {}. Try -f",
					path_sqlite.to_string_lossy()
				));
			}
		}

		let sqlite_connection = if open_db_in_memory {
			rusqlite::Connection::open_in_memory()
				.with_context(|| format!("could not open connection to in memory database",))?
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

	fn write_to_file(
		&self,
		path_specific: &str,
		filename: &str,
		data: &[u8],
	) -> Result<(), anyhow::Error> {
		// create path to attachment file
		let path = self.path_output.join(path_specific);
		std::fs::create_dir_all(&path)
			.with_context(|| format!("Failed to create path: {}", path.to_string_lossy()))?;

		// add filename and extension to path
		let mut path = path.join(filename);
		let infer = infer::Infer::new();
		if let Some(x) = infer.get(&data) {
			path.set_extension(x.ext);
		}

		if path.exists() && !self.force_write {
			return Err(anyhow!(
				"File does already exist: {}. Try -f",
				path.to_string_lossy()
			));
		}

		// open connection to file
		let mut buffer = std::fs::File::create(&path)
			.with_context(|| format!("Failed to open file: {}", path.to_string_lossy()))?;

		// write to file
		buffer
			.write_all(data)
			.with_context(|| format!("Failed to write to file: {}", path.to_string_lossy()))?;

		Ok(())
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
			self.written_frames += 1;
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
		self.write_to_file(
			"attachment",
			&format!("{}_{}", attachmend_id, row_id),
			&data,
		)?;

		self.count_attachment += 1;
		self.written_frames += 1;

		Ok(())
	}

	fn write_sticker(&mut self, data: &[u8], row_id: u64) -> Result<(), anyhow::Error> {
		//let mut path = self.path_sticker.join(format!("{}_{}", row_id, 1));
		//if path.exists() {
		//    path = self.path_sticker.join(format!("{}_{}", row_id, 2));
		//}

		self.write_to_file(
			"sticker",
			&format!("{}_{}", row_id, self.count_sticker),
			&data,
		)?;

		self.count_sticker += 1;
		self.written_frames += 1;

		Ok(())
	}

	fn write_avatar(&mut self, data: &[u8], name: &str) -> Result<(), anyhow::Error> {
		//let mut path = self.path_sticker.join(format!("{}_{}", row_id, 1));
		//if path.exists() {
		//    path = self.path_sticker.join(format!("{}_{}", row_id, 2));
		//}

		self.write_to_file("avatar", &format!("{}_{}", name, self.count_avatar), &data)?;

		self.count_avatar += 1;
		self.written_frames += 1;

		Ok(())
	}

	fn write_preference(
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

	fn write_version(&mut self, version: u32) -> Result<(), anyhow::Error> {
		info!("Database Version: {:?}", version);
		self.written_frames += 1;
		Ok(())
	}

	fn get_written_frames(&self) -> usize {
		self.written_frames
	}

	fn finish(&mut self) -> Result<(), anyhow::Error> {
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
