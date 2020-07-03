// imports
use anyhow::Context;
use clap::{clap_app, crate_authors, crate_description, crate_name, crate_version};
use std::io::BufRead;

/// Config struct
///
/// Stores all global variables
#[derive(Debug)]
pub struct Config {
	pub path_input: std::path::PathBuf,
	pub path_output_main: std::path::PathBuf,
	pub password: Vec<u8>,
	pub verify_mac: bool,
	pub log_level: log::LevelFilter,
}

impl Config {
	/// Create new config object
	pub fn new() -> Result<Self, anyhow::Error> {
		// TODO move here to another style? maybe it's easier to read
		let matches = clap_app!(myapp =>
                    (name: crate_name!())
                    (version: crate_version!())
                    (author: crate_authors!())
                    (about: crate_description!())
                    (@group password =>
                            (@attributes +required !multiple)
                            (@arg password_string: -p --("password") [PASSWORD] "Backup password (30 digits, with or without spaces)")
                            (@arg password_file: -f --("password_file") [FILE] "File to read the Backup password from")
                        )
                        (@group output_options =>
                            (@attributes !required +multiple)
                            (@arg output_path: -o --("output-path") [FOLDER] "Directory to save output to")
                        )
                        (@arg no_verify_mac: --("no-verify-mac") "Do not verify the HMAC of each frame in the backup")
                        (@arg INPUT: * "Sets the input file to use")).get_matches();

		let input_file = std::path::PathBuf::from(matches.value_of("INPUT").unwrap());

		// TODO add force / overwrite CLI argument instead of default overwriting?
		let output_path = if let Some(path) = matches.value_of("output_path") {
			std::path::PathBuf::from(path)
		} else {
			let path =
				input_file.file_stem().unwrap().to_str().context(
					"output_path is not given and path to input file could not be read.",
				)?;
			std::path::PathBuf::from(path)
		};

		let mut password = match matches.value_of("password_string") {
			Some(p) => String::from(p),
			None => {
				let password_file = std::io::BufReader::new(
					std::fs::File::open(matches.value_of("password_file").unwrap())
						.expect("Unable to open password file"),
				);
				password_file
					.lines()
					.next()
					.expect("Password file is empty")
					.expect("Unable to read from password file")
			}
		};

		password.retain(|c| c >= '0' && c <= '9');

		let password = password.as_bytes().to_vec();

		Ok(Self {
			path_input: input_file,
			path_output_main: output_path.clone(),
			password,
			verify_mac: !matches.is_present("no_verify_mac"),
			log_level: log::LevelFilter::Info,
		})
	}
}
