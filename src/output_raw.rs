use anyhow::anyhow;
use anyhow::Context;
use std::io::Write;

/// Write raw backup
///
/// This output module writes the backup in a sqlite database and media files in different
/// directories.
pub struct Output {
    path_avatar: std::path::PathBuf,
    path_attachment: std::path::PathBuf,
    path_sticker: std::path::PathBuf,
    path_config: std::path::PathBuf,
    sqlite_connection: rusqlite::Connection,
    count_attachment: usize,
    count_sticker: usize,
    count_avatar: usize,
}

impl Output {
    /// Creates new output object
    ///
    /// `force_write` determines whether existing files will be overwritten.
    pub fn new(path: &std::path::Path, force_write: bool) -> Result<Self, anyhow::Error> {
        // TODO this is transitional only
        let path = path.join("ng");
        // check output path
        if !force_write && path.exists() {
            return Err(anyhow!(
                "{} already exists and should not be overwritten",
                path.to_string_lossy()
            ));
        }

        if path.exists() && !path.is_dir() {
            return Err(anyhow!(
                "{} exists and is not a directory",
                path.to_string_lossy()
            ));
        }

        if !path.exists() {
            std::fs::create_dir(&path)
                .with_context(|| format!("{} could not be created", path.to_string_lossy()))?;
        }

        // determine sqlite path
        let path_sqlite = path.join("signal_backup.db");

        if path_sqlite.exists() {
            std::fs::remove_file(&path_sqlite).with_context(|| {
                format!(
                    "could not delete old database: {}",
                    path_sqlite.to_string_lossy()
                )
            })?;
        }

        Ok(Self {
            path_avatar: Output::set_directory(&path, "avatar")?,
            path_attachment: Output::set_directory(&path, "attachment")?,
            path_sticker: Output::set_directory(&path, "sticker")?,
            path_config: Output::set_directory(&path, "config")?,
            sqlite_connection: rusqlite::Connection::open(&path_sqlite).with_context(|| {
                format!(
                    "could not open connection to database file: {}.",
                    path_sqlite.to_string_lossy()
                )
            })?,
            count_attachment: 0,
            count_sticker: 0,
            count_avatar: 0,
        })
    }

    pub fn write_statement(
        &self,
        statement: &str,
        parameters: Vec<rusqlite::types::ToSqlOutput>,
    ) -> Result<(), anyhow::Error> {
        // In database version 9 signal added full text search and uses TRIGGERs to create the virtual tables. however this breaks when importing the data.
        if statement.starts_with("CREATE TRIGGER")
            || statement.contains("_fts")
            || statement.starts_with("CREATE TABLE sqlite_")
        {
            return Ok(());
        }

        self.sqlite_connection
            .execute(statement, parameters)
            .with_context(|| {
                format!(
                    "failed to write this statement into database: {}",
                    statement
                )
            })?;

        Ok(())
    }

    pub fn write_attachment(
        &mut self,
        data: &[u8],
        attachmend_id: u64,
        row_id: u64,
    ) -> Result<(), anyhow::Error> {
        let path = self
            .path_attachment
            .join(format!("{}_{}", attachmend_id, row_id));
        let mut buffer = std::fs::File::create(&path).with_context(|| {
            format!("Failed to open attachment file: {}", path.to_string_lossy())
        })?;

        buffer.write_all(data).with_context(|| {
            format!(
                "Failed to write to attachment file: {}",
                path.to_string_lossy()
            )
        })?;

        self.count_attachment += 1;

        Ok(())
    }

    pub fn write_sticker(&mut self, data: &[u8], row_id: u64) -> Result<(), anyhow::Error> {
        //let mut path = self.path_sticker.join(format!("{}_{}", row_id, 1));
        //if path.exists() {
        //    path = self.path_sticker.join(format!("{}_{}", row_id, 2));
        //}

        let path = self
            .path_sticker
            .join(format!("{}_{}", row_id, self.count_sticker));
        let mut buffer = std::fs::File::create(&path).with_context(|| {
            format!("Failed to open attachment file: {}", path.to_string_lossy())
        })?;

        buffer.write_all(data).with_context(|| {
            format!(
                "Failed to write to attachment file: {}",
                path.to_string_lossy()
            )
        })?;

        self.count_attachment += 1;
        self.count_sticker += 1;

        Ok(())
    }

    pub fn write_avatar(&mut self, data: &[u8], name: &str) -> Result<(), anyhow::Error> {
        //let mut path = self.path_sticker.join(format!("{}_{}", row_id, 1));
        //if path.exists() {
        //    path = self.path_sticker.join(format!("{}_{}", row_id, 2));
        //}

        let path = self
            .path_avatar
            .join(format!("{}_{}", name, self.count_avatar));
        let mut buffer = std::fs::File::create(&path).with_context(|| {
            format!("Failed to open attachment file: {}", path.to_string_lossy())
        })?;

        buffer.write_all(data).with_context(|| {
            format!(
                "Failed to write to attachment file: {}",
                path.to_string_lossy()
            )
        })?;

        self.count_attachment += 1;
        self.count_avatar += 1;

        Ok(())
    }

    pub fn write_preference(
        &self,
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

        Ok(())
    }

    fn set_directory(
        base: &std::path::Path,
        name: &str,
    ) -> Result<std::path::PathBuf, anyhow::Error> {
        let folder = base.join(name);

        if !folder.exists() {
            std::fs::create_dir(&folder)
                .with_context(|| format!("{} could not be created.", folder.to_string_lossy()))?;
        } else if !folder.is_dir() {
            Err(anyhow!(
                "{} exists and is not a directory.",
                folder.to_string_lossy()
            ))?;
        }

        Ok(folder)
    }
}
