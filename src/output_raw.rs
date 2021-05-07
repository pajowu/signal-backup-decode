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
	buffer_keyvalue: std::io::BufWriter<std::fs::File>,
	force_write: bool,
	sqlite_connection: rusqlite::Connection,
	count_attachment: usize,
	count_sticker: usize,
	count_avatar: usize,
	written_frames: usize,
	created_files: std::boxed::Box<std::collections::HashSet<std::path::PathBuf>>,
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
				.with_context(|| "could not open connection to in memory database".to_string())?
		} else {
			rusqlite::Connection::open(&path_sqlite).with_context(|| {
				format!(
					"could not open connection to database file: {}",
					path_sqlite.to_string_lossy()
				)
			})?
		};

		// open keyvalue textfile
		let path_keyvalue = path.join("keyvalue");

		if path_keyvalue.exists() {
			if force_write {
				std::fs::remove_file(&path_keyvalue).with_context(|| {
					format!(
						"Could not delete old keyvalue file: {}",
						path_keyvalue.to_string_lossy()
					)
				})?;
			} else {
				return Err(anyhow!(
					"Backup keyvalue file already exists: {}. Try -f",
					path_keyvalue.to_string_lossy()
				));
			}
		}

		let fd_keyvalue = std::fs::File::create(&path_keyvalue).with_context(|| {
			format!(
				"Could not create keyvalue file: {}",
				path_keyvalue.to_string_lossy()
			)
		})?;
		let buffer_keyvalue = std::io::BufWriter::new(fd_keyvalue);

		// return self
		Ok(Self {
			path_output: path.to_path_buf(),
			buffer_keyvalue,
			force_write,
			sqlite_connection,
			count_attachment: 0,
			count_sticker: 0,
			count_avatar: 0,
			// we set read frames to 1 due to the header frame we will never write
			written_frames: 1,
			created_files: std::boxed::Box::new(std::collections::HashSet::new()),
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
			path.set_extension(x.extension());
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
		attachment_id: u64,
		row_id: u64,
	) -> Result<(), anyhow::Error> {
		self.write_to_file(
			"attachment",
			&format!("{}_{}", attachment_id, row_id),
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

	fn write_avatar(&mut self, data: &[u8], _name: &str) -> Result<(), anyhow::Error> {
		// avatar has never a name
		self.write_to_file("avatar", &format!("{}", self.count_avatar), &data)?;

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
		if path.exists() && !self.force_write && !self.created_files.contains(&path) {
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

		self.created_files.insert(path);
		self.written_frames += 1;

		Ok(())
	}

	fn write_version(&mut self, version: u32) -> Result<(), anyhow::Error> {
		info!("Database Version: {:?}", version);
		self.written_frames += 1;
		Ok(())
	}

	fn write_keyvalue(
		&mut self,
		key: &str,
		value: &crate::frame::KeyValueContent,
	) -> Result<(), anyhow::Error> {
		self.buffer_keyvalue
			.write(format!("{} = {:?}\n", key, value).as_bytes())
			.context("Could not write to keyvalue file")?;

		self.written_frames += 1;
		Ok(())
	}

	fn get_written_frames(&self) -> usize {
		self.written_frames
	}

	fn finish(&mut self) -> Result<(), anyhow::Error> {
		let path_sqlite = self.path_output.join("signal_backup.db");

		// if path already exists we have directly written to database and don't need to flush the
		// db to a file.
		if path_sqlite.exists() {
			return Ok(());
		}

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
