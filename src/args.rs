// imports
use anyhow::Context;
use anyhow::anyhow;
use clap::{crate_authors, crate_description, crate_name, crate_version};
use std::io::BufRead;

/// Config struct
///
/// Stores all global variables
#[derive(Debug)]
pub struct Config {
	pub path_input: std::path::PathBuf,
	pub path_output: std::path::PathBuf,
	pub password: Vec<u8>,
	pub verify_mac: bool,
	pub log_level: log::LevelFilter,
}

impl Config {
	/// Create new config object
	pub fn new() -> Result<Self, anyhow::Error> {
		// TODO add check argument
		// TODO add verbosity argument
		let matches = clap::App::new(crate_name!())
			.version(crate_version!())
			.about(crate_description!())
			.author(crate_authors!())
			.arg(
				clap::Arg::with_name("input-file")
					.help("Sets the input file to use")
					.takes_value(true)
					.value_name("INPUT")
					.required(true)
					.index(1),
			)
			.arg(
				clap::Arg::with_name("output-path")
					.help("Directory to save output to. If not given, input file directory is used")
					.long("output-path")
					.short("o")
					.takes_value(true)
					.value_name("FOLDER"),
			)
			.arg(
				clap::Arg::with_name("log-level")
					.help("Verbosity level, either DEBUG, INFO, WARN, or ERROR")
					.long("log-level")
					.short("l")
					.takes_value(true)
					.value_name("LEVEL"),
			)
			.arg(
				clap::Arg::with_name("no-verify-mac")
					.help("Do not verify the HMAC of each frame in the backup")
					.long("no-verify-mac"),
			)
			.arg(
				clap::Arg::with_name("password-string")
					.help("Backup password (30 digits, with or without spaces)")
					.long("password")
					.takes_value(true)
					.value_name("PASSWORD")
					.short("p"),
			)
			.arg(
				clap::Arg::with_name("password-file")
					.help("File to read the backup password from")
					.long("password-file")
					.short("f")
					.takes_value(true)
					.value_name("FILE"),
			)
			.arg(
				clap::Arg::with_name("password-command")
					.help("Read backup password from stdout from COMMAND")
					.long("password-command")
					.takes_value(true)
					.value_name("COMMAND"),
			)
			.group(
				clap::ArgGroup::with_name("password")
					.args(&["password-string", "password-file", "password-command"])
					.required(true)
					.multiple(false),
			)
			.get_matches();

                // input file handling
		let input_file = std::path::PathBuf::from(matches.value_of("input-file").unwrap());

                // output path handling
		// TODO add force / overwrite CLI argument instead of default overwriting?
		let output_path = std::path::PathBuf::from(matches.value_of("output-path").unwrap_or({
			input_file
				.file_stem()
				.unwrap()
				.to_str()
				.context("output-path is not given and path to input file could not be read.")?
		}));

                // password handling
		let mut password = {
			if matches.is_present("password-string") {
				String::from(matches.value_of("password-string").unwrap())
			} else if matches.is_present("password-file") {
				let password_file = std::io::BufReader::new(
					std::fs::File::open(matches.value_of("password-file").unwrap())
						.context("Unable to open password file")?,
				);
				password_file
					.lines()
					.next()
					.context("Password file is empty")?
					.context("Unable to read from password file")?
			} else if matches.is_present("password-command") {
				let shell = std::env::var("SHELL").context("Could not determine current shell")?;
				String::from_utf8(
					std::process::Command::new(shell)
						.arg("-c")
						.arg(matches.value_of("password-command").unwrap())
						.output()
						.context("Failed to execute password command")?
						.stdout,
				)
				.context("Password command returned invalid characters")?
			} else {
				unreachable!()
			}
		};
		password.retain(|c| c >= '0' && c <= '9');
		let password = password.as_bytes().to_vec();

                // verbosity handling
                let log_level = if let Some(x) = matches.value_of("log-level") {
                    match x.to_lowercase().as_str() {
                        "debug" => log::LevelFilter::Debug,
                        "info" => log::LevelFilter::Info,
                        "warn" => log::LevelFilter::Warn,
                        "error" => log::LevelFilter::Error,
                        _ => return Err(anyhow!("Unknown log level given")),
                    }
                } else {
                    log::LevelFilter::Info
                };

		Ok(Self {
			path_input: input_file,
			path_output: output_path,
			password,
			verify_mac: !matches.is_present("no_verify_mac"),
			log_level,
		})
	}
}
