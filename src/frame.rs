use anyhow::anyhow;
use anyhow::Context;
use std::convert::TryInto;

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
	KeyValue {
		key: String,
		value: KeyValueContent,
	},
}

#[derive(Debug)]
pub enum KeyValueContent {
	Blob(Vec<u8>),
	Bool(bool),
	Float(f32),
	Int(i64),
	String(String),
}

impl Frame {
	/// Creates a new frame from protobuf
	pub fn new(frame: &mut crate::Backups::BackupFrame) -> Result<Self, anyhow::Error> {
		// The field count and return value are necessary to check against unknown protobuf field
		// types. Unknown field types might not be detected, thus `field_count` stays at zero.
		// Sometimes they result in reading two different known field types, thus `field_count`
		// gets increased to two.
		//
		// See: https://github.com/pajowu/signal-backup-decode/pull/43 for a discussion on this.
		let mut field_count = 0;
		let mut ret = Self::End;

		if frame.has_header() {
			// increase field count
			field_count += 1;

			// get header
			let mut header = frame.take_header();

			// return header
			ret = Self::Header {
				salt: header.take_salt(),
				iv: header.take_iv(),
			};
		}

		if frame.has_statement() {
			// increase field count
			field_count += 1;

			// build statement
			let mut statement = frame.take_statement();
			ret = Self::Statement {
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
			};
		}

		if frame.has_preference() {
			// increase field count
			field_count += 1;

			// return preference
			ret = Self::Preference {
				preference: frame.take_preference(),
			};
		}

		if frame.has_attachment() {
			// increase field count
			field_count += 1;

			// get attachment
			let attachment = frame.take_attachment();

			// return attachment
			ret = Self::Attachment {
				data_length: attachment.get_length().try_into().unwrap(),
				id: attachment.get_attachmentId(),
				row: attachment.get_rowId(),
				data: None,
			};
		}

		if frame.has_version() {
			// increase field count
			field_count += 1;

			// return version
			ret = Self::Version {
				version: frame.get_version().get_version(),
			}
		}

		if frame.has_end() {
			// increase field count
			field_count += 1;

			ret = Self::End;
		}

		if frame.has_avatar() {
			// increase field count
			field_count += 1;

			// take avatar
			let mut avatar = frame.take_avatar();

			// return avatar
			ret = Self::Avatar {
				data_length: avatar.get_length().try_into().unwrap(),
				name: avatar.take_name(),
				data: None,
			};
		}

		if frame.has_sticker() {
			// increase field count
			field_count += 1;

			// take sticker
			let sticker = frame.take_sticker();

			// return sticker
			ret = Self::Sticker {
				data_length: sticker.get_length().try_into().unwrap(),
				row: sticker.get_rowId(),
				data: None,
			};
		}

		if frame.has_keyValue() {
			// increase field count
			field_count += 1;

			// get keyvalue
			let mut keyvalue = frame.take_keyValue();
			let value = if keyvalue.has_blobValue() {
				KeyValueContent::Blob(keyvalue.take_blobValue())
			} else if keyvalue.has_booleanValue() {
				KeyValueContent::Bool(keyvalue.get_booleanValue())
			} else if keyvalue.has_floatValue() {
				KeyValueContent::Float(keyvalue.get_floatValue())
			} else if keyvalue.has_integerValue() {
				KeyValueContent::Int(keyvalue.get_integerValue().into())
			} else if keyvalue.has_longValue() {
				KeyValueContent::Int(keyvalue.get_longValue())
			} else if keyvalue.has_stringValue() {
				KeyValueContent::String(keyvalue.take_stringValue())
			} else {
				unreachable!()
			};

			// return keyvalue
			ret = Self::KeyValue {
				key: keyvalue.take_key(),
				value,
			};
		}

		if field_count != 1 {
			Err(anyhow!(
				"Frame with an unsupported field found, please report to author: {:?}",
				frame
			))
		} else {
			Ok(ret)
		}
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
			Self::Header { salt, iv } => write!(
				f,
				"Header Frame (salt: {:02X?} (length: {}), iv: {:02X?} (length: {}))",
				salt,
				salt.len(),
				iv,
				iv.len()
			),
			Self::Sticker { data_length, .. } => write!(f, "Sticker (size: {})", data_length),
			Self::Attachment { data_length, .. } => write!(f, "Attachment (size: {})", data_length),
			Self::Avatar { data_length, .. } => write!(f, "Avatar (size: {})", data_length),
			Self::Preference { .. } => write!(f, "Preference"),
			Self::Statement { .. } => write!(f, "Statement"),
			Self::Version { version } => write!(f, "Version ({})", version),
			Self::End => write!(f, "End"),
			Self::KeyValue { key, value } => write!(f, "KeyValue: {} = {:?}", key, value),
		}
	}
}

impl std::convert::TryFrom<Vec<u8>> for Frame {
	type Error = anyhow::Error;

	fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
		let mut frame = protobuf::Message::parse_from_bytes(&data)
			.with_context(|| format!("Could not parse frame from {:02X?}", &data))?;
		Self::new(&mut frame)
	}
}
