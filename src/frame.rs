use std::convert::TryInto;

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
                data_length: usize,
                id: u64,
                row: u64,
	},
	Version {
		version: u32,
	},
	End,
	Avatar {
            data_length: usize,
            name: &'a str,
	},
	Sticker {
                data_length: usize,
                row: u64,
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
		};

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
		};

		if frame.has_preference() {
			fields_count += 1;
			ret = Some(Self::Preference {
				preference: frame.get_preference(),
			});
		};

		if frame.has_attachment() {
			fields_count += 1;
			ret = Some(Self::Attachment {
                                data_length: frame.get_attachment().get_length().try_into().unwrap(),
                                id: frame.get_attachment().get_attachmentId(),
                                row: frame.get_attachment().get_rowId(),
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
			ret = Some(Self::Avatar {
                                data_length: frame.get_avatar().get_length().try_into().unwrap(),
                                name: frame.get_avatar().get_name(),
			});
		};

		if frame.has_sticker() {
			fields_count += 1;
			ret = Some(Self::Sticker {
                                data_length: frame.get_sticker().get_length().try_into().unwrap(),
                                row: frame.get_sticker().get_rowId(),
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
}
