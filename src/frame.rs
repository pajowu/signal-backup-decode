use std::convert::TryInto;
use anyhow::Context;

/// Frame
pub enum Frame {
	Header {
		salt: Vec<u8>,
		iv: Vec<u8>,
	},
	Statement {
		statement: String,
		parameter: Vec<rusqlite::types::Value>,
	},
	Preference {
		preference: crate::Backups::SharedPreference,
	},
	Attachment {
		data_length: usize,
		id: u64,
		row: u64,
		data: Option<Vec<u8>>,
	},
	Version {
		version: u32,
	},
	End,
	Avatar {
		data_length: usize,
		name: String,
		data: Option<Vec<u8>>,
	},
	Sticker {
		data_length: usize,
		row: u64,
		data: Option<Vec<u8>>,
	},
}

impl Frame {
	pub fn new(frame: &mut crate::Backups::BackupFrame) -> Self {
		let mut fields_count = 0;
		let mut ret: Option<Self> = None;

		if frame.has_header() {
			fields_count += 1;
			let mut header = frame.take_header();
			ret = Some(Self::Header {
				salt: header.take_salt(),
				iv: header.take_iv(),
			});
		};

		if frame.has_statement() {
			fields_count += 1;
			let mut statement = frame.take_statement();
			ret = Some(Self::Statement {
				statement: statement.take_statement(),
				parameter: {
					let mut params: Vec<rusqlite::types::Value> = Vec::new();
					for param in statement.take_parameters().iter_mut() {
						if param.has_stringParamter() {
							params.push(param.take_stringParamter().into());
						} else if param.has_integerParameter() {
							params.push((param.get_integerParameter() as i64).into());
						} else if param.has_doubleParameter() {
							params.push(param.get_doubleParameter().into());
						} else if param.has_blobParameter() {
							params.push(param.take_blobParameter().into());
						} else if param.has_nullparameter() {
							params.push(rusqlite::types::Null.into());
						} else {
							panic!("Parameter type not known {:?}", param);
						}
					}
					params
				},
			});
		};

		if frame.has_preference() {
			fields_count += 1;
			ret = Some(Self::Preference {
				preference: frame.take_preference(),
			});
		};

		if frame.has_attachment() {
			fields_count += 1;
			let attachment = frame.take_attachment();
			ret = Some(Self::Attachment {
				data_length: attachment.get_length().try_into().unwrap(),
				id: attachment.get_attachmentId(),
				row: attachment.get_rowId(),
				data: None,
			});
		};

		if frame.has_version() {
			fields_count += 1;
			ret = Some(Self::Version {
				version: frame.get_version().get_version(),
			});
		};

		if frame.has_end() {
			fields_count += 1;
			ret = Some(Self::End);
		};

		if frame.has_avatar() {
			fields_count += 1;
			let mut avatar = frame.take_avatar();
			ret = Some(Self::Avatar {
				data_length: avatar.get_length().try_into().unwrap(),
				name: avatar.take_name(),
				data: None,
			});
		};

		if frame.has_sticker() {
			fields_count += 1;
			let sticker = frame.take_sticker();
			ret = Some(Self::Sticker {
				data_length: sticker.get_length().try_into().unwrap(),
				row: sticker.get_rowId(),
				data: None,
			});
		};

		if fields_count != 1 {
			panic!(
				"Frame with an unsupported number of fields found, please report to author: {:?}",
				frame
			);
		};

		ret.unwrap()
	}

	pub fn set_data(&mut self, data_add: Vec<u8>) {
		match self {
			Frame::Attachment { ref mut data, .. } => *data = Some(data_add),
			Frame::Avatar { ref mut data, .. } => *data = Some(data_add),
			Frame::Sticker { ref mut data, .. } => *data = Some(data_add),
			_ => panic!("Cannot set data on variant without data field."),
		}
	}
}

impl std::fmt::Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Header { salt, iv } => {
                write!(f, "Salt: {:02X?} (length: {}), IV: {:02X?} (length: {})", salt, salt.len(), iv, iv.len())
            }
            _ => Ok(())
        }
    }
}

impl std::convert::TryFrom<Vec<u8>> for Frame {
    type Error = anyhow::Error;

    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
        let mut frame = protobuf::parse_from_bytes::<crate::Backups::BackupFrame>(&data)
				.with_context(|| format!("Could not parse frame from {:02X?}", &data))?;
        Ok(Self::new(&mut frame))
    }
}
