use anyhow::anyhow;

/// Trait that defines common ouptut functions
pub trait SignalOutput: Send {
	fn write_statement(
		&mut self,
		statement: &str,
		parameters: &[rusqlite::types::Value],
	) -> Result<(), anyhow::Error>;

	fn write_attachment(
		&mut self,
		data: &[u8],
		attachmend_id: u64,
		row_id: u64,
	) -> Result<(), anyhow::Error>;

	fn write_sticker(&mut self, data: &[u8], row_id: u64) -> Result<(), anyhow::Error>;

	fn write_avatar(&mut self, data: &[u8], name: &str) -> Result<(), anyhow::Error>;

	fn write_preference(
		&mut self,
		pref: &crate::Backups::SharedPreference,
	) -> Result<(), anyhow::Error>;

	fn write_version(&mut self, version: u32) -> Result<(), anyhow::Error>;

	fn get_written_frames(&self) -> usize;

	fn write_frame(&mut self, frame: crate::frame::Frame) -> Result<(), anyhow::Error> {
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
}

/// Defined output types
pub enum SignalOutputType {
	None,
	Raw,
	Csv,
}
