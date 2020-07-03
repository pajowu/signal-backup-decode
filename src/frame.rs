/// Frame
pub enum Frame<'a> {
	Header {
		salt: &'a [u8],
		iv: &'a [u8],
	},
	Statement {
		statement: &'a str,
		parameter: Vec<rusqlite::types::ToSqlOutput<'a>>,
	},
	Preference {
		preference: &'a crate::Backups::SharedPreference,
	},
	Attachment {
		attachment: &'a crate::Backups::Attachment,
	},
	Version {
		version: u32,
	},
	End,
	Avatar {
		avatar: &'a crate::Backups::Avatar,
	},
	Sticker {
		sticker: &'a crate::Backups::Sticker,
	},
}

impl<'a> Frame<'a> {
	pub fn new(frame: &'a crate::Backups::BackupFrame) -> Self {
		let mut fields_count = 0;
		let mut ret: Option<Self> = None;

		if frame.has_header() {
			fields_count += 1;
			ret = Some(Self::Header {
				salt: frame.get_header().get_salt(),
				iv: frame.get_header().get_iv(),
			});
		}

		if frame.has_statement() {
			fields_count += 1;
			ret = Some(Self::Statement {
				statement: frame.get_statement().get_statement(),
				parameter: {
					let mut params: Vec<rusqlite::types::ToSqlOutput<'a>> = Vec::new();
					for param in frame.get_statement().get_parameters().iter() {
						if param.has_stringParamter() {
							params.push(param.get_stringParamter().into());
						} else if param.has_integerParameter() {
							params.push((param.get_integerParameter() as i64).into());
						} else if param.has_doubleParameter() {
							params.push(param.get_doubleParameter().into());
						} else if param.has_blobParameter() {
							params.push(param.get_blobParameter().into());
						} else if param.has_nullparameter() {
							params.push(rusqlite::types::Null.into());
						} else {
							panic!("Parameter type not known {:?}", param);
						}
					}
					params
				},
			});
		}

		if frame.has_preference() {
			fields_count += 1;
			ret = Some(Self::Preference {
				preference: frame.get_preference(),
			});
		}

		if frame.has_attachment() {
			fields_count += 1;
			ret = Some(Self::Attachment {
				attachment: frame.get_attachment(),
			});
		}

		if frame.has_version() {
			fields_count += 1;
			ret = Some(Self::Version {
				version: frame.get_version().get_version(),
			});
		}

		if frame.has_end() {
			fields_count += 1;
			ret = Some(Self::End);
		}

		if frame.has_avatar() {
			fields_count += 1;
			ret = Some(Self::Avatar {
				avatar: frame.get_avatar(),
			});
		}

		if frame.has_sticker() {
			fields_count += 1;
			ret = Some(Self::Sticker {
				sticker: frame.get_sticker(),
			});
		}

		if fields_count != 1 {
			panic!(
				"Frame with an unsupported number of fields found, please report to author: {:?}",
				frame
			);
		}

		ret.unwrap()
	}
}
