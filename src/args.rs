use anyhow::Result;

/// Config struct
///
/// Stores all global variables
#[derive(Debug)]
pub struct Config {
	pub path_input: std::path::PathBuf,
	pub path_output_main: std::path::PathBuf,
	pub path_output_avatar: std::path::PathBuf,
	pub path_output_attachment: std::path::PathBuf,
	pub path_output_sticker: std::path::PathBuf,
	pub path_output_config: std::path::PathBuf,
	pub path_output_sqlite: std::path::PathBuf,
	pub password: Vec<u8>,
	pub no_verify_mac: bool,
	pub no_tmp_sqlite: bool,
	pub log_level: log::LevelFilter,
}

impl Config {
	/// Create new config object
	pub fn new() -> Result<Self> {
		// imports
		use std::io::BufRead;

		// logic
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
                            (@arg sqlite_file: --("sqlite-path") +takes_value "File to store the sqlite database in [default: output_path/signal_backup.db]")
                            (@arg attachment_path: --("attachment-path") default_value[attachments] "Directory to save attachments to")
                            (@arg avatar_path: --("avatar-path") default_value[avatars] "Directory to save avatar images to")
                            (@arg sticker_path: --("sticker-path") default_value[stickers] "Directory to save sticker images to")
                            (@arg config_path: --("config-path") default_value[config] "Directory to save config files to")
                        )
                        (@arg no_tmp_sqlite: --("no-tmp-sqlite") "Do not use a temporary file for the sqlite database")
                        (@arg no_verify_mac: --("no-verify-mac") "Do not verify the HMAC of each frame in the backup")
                        (@arg INPUT: * "Sets the input file to use")
                ).get_matches();

		let input_file = std::path::Path::new(matches.value_of("INPUT").unwrap());

		let output_path =
			std::path::Path::new(matches.value_of("output_path").unwrap_or_else(|| {
				input_file
					.file_stem()
					.unwrap()
					.to_str()
					.expect("output_path not given and could not be automatically determined")
			}));
		if !output_path.exists() {
			std::fs::create_dir(&output_path).unwrap_or_else(|_| {
				panic!("{} could not be created", output_path.to_string_lossy())
			});
		} else if !output_path.is_dir() {
			panic!(
				"{} exists and is not a directory",
				output_path.to_string_lossy()
			);
		}

		let attachment_folder =
			Config::get_directory(output_path, matches.value_of("attachment_path").unwrap());
		let avatar_folder =
			Config::get_directory(output_path, matches.value_of("avatar_path").unwrap());
		let sticker_folder =
			Config::get_directory(output_path, matches.value_of("sticker_path").unwrap());
		let config_folder =
			Config::get_directory(output_path, matches.value_of("config_path").unwrap());

		let sqlite_path = match matches.value_of("sqlite_file") {
			Some(s) => std::path::Path::new(&s).to_path_buf(),
			None => output_path.join("signal_backup.db"),
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
			path_input: input_file.to_path_buf(),
			path_output_main: output_path.to_path_buf(),
			path_output_avatar: avatar_folder,
			path_output_attachment: attachment_folder,
			path_output_sticker: sticker_folder,
			path_output_config: config_folder,
			path_output_sqlite: sqlite_path,
			password,
			no_verify_mac: !matches.is_present("no_verify_mac"),
			no_tmp_sqlite: matches.is_present("no_tmp_sqlite"),
			log_level: log::LevelFilter::Info,
		})
	}

	fn get_directory(base: &std::path::Path, name: &str) -> std::path::PathBuf {
		let folder = base.join(name);
		if !folder.exists() {
			std::fs::create_dir(&folder)
				.unwrap_or_else(|_| panic!("{} could not be created", folder.to_string_lossy()));
		} else if !folder.is_dir() {
			panic!("{} exists and is not a directory", folder.to_string_lossy());
		}
		folder
	}
}
