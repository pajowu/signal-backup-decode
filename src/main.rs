extern crate byteorder;
extern crate crypto;
extern crate hex;
extern crate openssl;
extern crate protobuf;
#[macro_use]
extern crate error_chain;
extern crate ini;
extern crate sqlite;
#[macro_use]
extern crate clap;
extern crate tempfile;

use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use crypto::mac::Mac;
use openssl::hash::{Hasher, MessageDigest};
use openssl::symm;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::{Read, Write};
use std::iter::Iterator;

mod Backups;
mod errors;
use crate::errors::*;
use std::path::Path;

struct CipherData {
	hmac: crypto::hmac::Hmac<crypto::sha2::Sha256>,
	cipher_key: [u8; 32],
	counter: Vec<u8>,
}

fn read_frame<T: Read>(
	r: &mut T,
	cipher_data: &mut Option<CipherData>,
	verify_mac: bool,
) -> Result<(usize, Vec<u8>)> {
	let len = r.read_u32::<BigEndian>()?.try_into()?;
	let mut frame_content = vec![0u8; len as usize];
	r.read_exact(&mut frame_content)?;
	match *cipher_data {
		None => Ok((len, frame_content)),
		Some(ref mut cipher_data) => {
			let frame_data = &frame_content[..frame_content.len() - 10];
			if verify_mac {
				let frame_mac = &frame_content[frame_content.len() - 10..];
				cipher_data.hmac.input(&frame_data);
				let hmac_result = cipher_data.hmac.result();
				let calculated_mac = &hmac_result.code()[..10];
				cipher_data.hmac.reset();
				if !crypto::util::fixed_time_eq(calculated_mac, frame_mac) {
					return Err(ErrorKind::MacVerificationError(
						calculated_mac.to_vec(),
						frame_mac.to_vec(),
					)
					.into());
				}
			}
			let plaintext = decrypt(&cipher_data.cipher_key, &cipher_data.counter, frame_data)?;
			increase_counter(&mut cipher_data.counter, None);
			Ok((len, plaintext))
		}
	}
}
fn decrypt(key: &[u8; 32], counter: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
	let mut decrypter = symm::Crypter::new(
		symm::Cipher::aes_256_ctr(),
		symm::Mode::Decrypt,
		key,
		Some(&counter),
	)?;
	let block_size = symm::Cipher::aes_256_ctr().block_size();
	let mut plaintext = vec![0; ciphertext.len() + block_size];
	let mut count = decrypter.update(&ciphertext, &mut plaintext)?;
	count += decrypter.finalize(&mut plaintext[count..])?;
	plaintext.truncate(count);
	Ok(plaintext)
}
fn increase_counter(counter: &mut Vec<u8>, start: Option<usize>) {
	let mut i = start.unwrap_or(3);
	loop {
		if counter[i] < 255 {
			counter[i] += 1;
			break;
		} else {
			counter[i] = 0;
			i -= 1
		}
	}
}
fn generate_keys(key: &[u8], salt: &[u8]) -> Result<([u8; 32], [u8; 32])> {
	let mut digest = Hasher::new(MessageDigest::sha512())?;
	digest.update(salt)?;
	let mut hash = key.to_vec();
	for _ in 0..250000 {
		digest.update(&hash)?;
		digest.update(key)?;
		hash = digest.finish()?.to_vec();
	}
	let backup_key = &hash[..32];
	Ok(derive_secrets(backup_key, b"Backup Export", 64))
}
fn derive_secrets(key: &[u8], info: &[u8], length: usize) -> ([u8; 32], [u8; 32]) {
	let mut prk = [0u8; 32];
	crypto::hkdf::hkdf_extract(crypto::sha2::Sha256::new(), &[0u8; 32], key, &mut prk);
	let mut sec = vec![0u8; length];
	crypto::hkdf::hkdf_expand(crypto::sha2::Sha256::new(), &prk, info, &mut sec);
	let mut sec1: [u8; 32] = Default::default();
	let mut sec2: [u8; 32] = Default::default();
	sec1.copy_from_slice(&sec[..32]);
	sec2.copy_from_slice(&sec[32..]);
	(sec1, sec2)
}

fn read_attachment<R: Read, W: Write>(
	reader: &mut R,
	writer: &mut W,
	cipher_data: &mut CipherData,
	length: usize,
	verify_mac: bool,
) -> Result<usize> {
	let mut decrypter = symm::Crypter::new(
		symm::Cipher::aes_256_ctr(),
		symm::Mode::Decrypt,
		&cipher_data.cipher_key,
		Some(&&cipher_data.counter),
	)?;
	let block_size = symm::Cipher::aes_256_ctr().block_size();
	let mut plaintext = vec![0; 8192 + block_size];

	cipher_data.hmac.input(&cipher_data.counter);

	let mut bytes_left = length as usize;
	while bytes_left > 0 {
		let mut buffer = vec![0u8; std::cmp::min(bytes_left, 8192)];
		reader.read_exact(&mut buffer)?;
		bytes_left -= buffer.len();
		if verify_mac {
			cipher_data.hmac.input(&buffer);
		}
		let mut count = decrypter.update(&buffer, &mut plaintext)?;
		count += decrypter.finalize(&mut plaintext[count..])?;
		writer.write_all(&plaintext[..count])?;
	}

	let mut mac = [0u8; 10];
	reader.read_exact(&mut mac)?;
	if verify_mac {
		let hmac_result = cipher_data.hmac.result();
		let calculated_mac = &hmac_result.code()[..10];
		cipher_data.hmac.reset();
		if !crypto::util::fixed_time_eq(calculated_mac, &mac) {
			return Err(
				ErrorKind::MacVerificationError(calculated_mac.to_vec(), mac.to_vec()).into(),
			);
		}
	}
	increase_counter(&mut cipher_data.counter, None);
	Ok(length)
}

fn decode_backup<R: Read>(
	mut reader: R,
	password: &[u8],
	attachment_folder: &Path,
	avatar_folder: &Path,
	sticker_folder: &Path,
	config_folder: &Path,
	connection: &sqlite::Connection,
	callback: fn(usize, usize, usize),
	verify_mac: bool,
) -> Result<usize> {
	let mut cipher_data: Option<CipherData> = None;

	let mut frame_count = 0;
	let mut attachment_count = 0;
	let mut seek_position = 0;

	loop {
		let (consumed_bytes, frame_content) =
			read_frame(&mut reader, &mut cipher_data, verify_mac)?;
		seek_position += consumed_bytes;
		let frame = protobuf::parse_from_bytes::<Backups::BackupFrame>(&frame_content)
			.unwrap_or_else(|_| panic!("Could not parse frame from {:?}", frame_content));

		let frame_fields = [
			frame.has_header(),
			frame.has_statement(),
			frame.has_preference(),
			frame.has_attachment(),
			frame.has_version(),
			frame.has_end(),
			frame.has_avatar(),
			frame.has_sticker(),
		];
		if frame_fields.iter().filter(|x| **x).count() != 1 {
			panic!(
				"Frame with an unsupported number of fields found, please report to author: {:?}",
				frame
			);
		}
		if frame.has_header() {
			let (cipher_key, mac_key) = generate_keys(&password, frame.get_header().get_salt())
				.expect("Error generating keys");
			cipher_data = Some(CipherData {
				hmac: crypto::hmac::Hmac::new(crypto::sha2::Sha256::new(), &mac_key),
				cipher_key,
				counter: frame.get_header().get_iv().to_vec(),
			})
		} else if cipher_data.is_none() {
			panic!("Read non-header frame before header frame");
		} else if frame.has_version() {
			println!("Database Version: {:?}", frame.get_version().get_version());
		} else if frame.has_attachment() {
			let a = frame.get_attachment();
			let attachment_path =
				attachment_folder.join(format!("{}_{}", a.get_attachmentId(), a.get_rowId()));
			let mut buffer = File::create(&attachment_path).unwrap_or_else(|_| {
				panic!(
					"Failed to open attachment file: {}",
					attachment_path.to_string_lossy()
				)
			});
			if let Some(ref mut c) = cipher_data {
				seek_position += read_attachment(
					&mut reader,
					&mut buffer,
					c,
					a.get_length().try_into()?,
					verify_mac,
				)?;
			} else {
				panic!("Attachment found before header, exiting");
			}
			attachment_count += 1;
		} else if frame.has_avatar() {
			let a = frame.get_avatar();
			let mut i = 0;
			let mut path = avatar_folder.join(format!("{}_{}", a.get_name(), i));
			if path.exists() {
				i += 1;
				path = avatar_folder.join(format!("{}_{}", a.get_name(), i));
			}
			let mut buffer = File::create(&path)
				.unwrap_or_else(|_| panic!("Failed to open file: {}", path.to_string_lossy()));
			if let Some(ref mut c) = cipher_data {
				seek_position += read_attachment(
					&mut reader,
					&mut buffer,
					c,
					a.get_length().try_into()?,
					verify_mac,
				)?;
			} else {
				panic!("Attachment/Avatar found before header, exiting");
			}
			attachment_count += 1;
		} else if frame.has_sticker() {
			let a = frame.get_sticker();
			let mut i = 0;
			let mut path = sticker_folder.join(format!("{}_{}", a.get_rowId(), i));
			if path.exists() {
				i += 1;
				path = sticker_folder.join(format!("{}_{}", a.get_rowId(), i));
			}
			let mut buffer = File::create(&path)
				.unwrap_or_else(|_| panic!("Failed to open file: {}", path.to_string_lossy()));
			if let Some(ref mut c) = cipher_data {
				seek_position += read_attachment(
					&mut reader,
					&mut buffer,
					c,
					a.get_length().try_into()?,
					verify_mac,
				)?;
			} else {
				panic!("Attachment/Sticker found before header, exiting");
			}
			attachment_count += 1;
		} else if frame.has_statement() {
			let statement = frame.get_statement().get_statement();
			// In database version 9 signal added full text search and uses TRIGGERs to create the virtual tables. however this breaks when importing the data.
			if statement.starts_with("CREATE TRIGGER")
				|| statement.contains("_fts")
				|| statement.starts_with("CREATE TABLE sqlite_")
			{
				continue;
			}

			let mut statement = connection
				.prepare(frame.get_statement().get_statement())
				.unwrap_or_else(|_| {
					panic!(
						"Failed to prepare statement: {}",
						frame.get_statement().get_statement()
					)
				});

			for (i, param) in frame.get_statement().get_parameters().iter().enumerate() {
				if param.has_stringParamter() {
					statement
						.bind(i + 1, param.get_stringParamter())
						.unwrap_or_else(|_| {
							panic!(
								"Error binding string parameter: {}",
								param.get_stringParamter()
							)
						});
				} else if param.has_integerParameter() {
					statement
						.bind(i + 1, param.get_integerParameter() as i64)
						.unwrap_or_else(|_| {
							panic!(
								"Error binding integer parameter: {}",
								param.get_integerParameter()
							)
						});
				} else if param.has_doubleParameter() {
					statement
						.bind(i + 1, param.get_doubleParameter())
						.unwrap_or_else(|_| {
							panic!(
								"Error binding double parameter: {}",
								param.get_doubleParameter()
							)
						});
				} else if param.has_blobParameter() {
					statement
						.bind(i + 1, param.get_blobParameter())
						.unwrap_or_else(|_| {
							panic!(
								"Error binding blob parameter: {:?}",
								param.get_blobParameter()
							)
						});
				} else if param.has_nullparameter() {
					statement
						.bind(i + 1, ())
						.expect("Error binding null parameter");
				} else {
					panic!("Parameter type not known {:?}", param);
				}
			}

			// Run until statement is completed
			loop {
				let s = statement.next();
				match s {
					Ok(sqlite::State::Row) => continue,
					Err(e) => return Err(e.into()),
					Ok(sqlite::State::Done) => break,
				}
			}
		} else if frame.has_preference() {
			let pref = frame.get_preference();
			let config_file = config_folder.join(pref.get_file());
			let mut conf = ini::Ini::load_from_file(&config_file).unwrap_or_default();
			conf.with_section(None::<String>)
				.set(pref.get_key(), pref.get_value());
			conf.write_to_file(&config_file)?;
		} else if frame.has_end() {
			break;
		} else {
			panic!("Unsupported Frame: {:?}", frame);
		}
		frame_count += 1;
		callback(frame_count, attachment_count, seek_position);
	}
	Ok(frame_count)
}

fn frame_callback(frame_count: usize, attachment_count: usize, seek_position: usize) {
	std::io::stdout()
		.write_all(
			format!(
				"Successfully exported {} frames, {} attachments, {} bytes into file\r",
				frame_count, attachment_count, seek_position
			)
			.as_bytes(),
		)
		.expect("Error writing status to stdout");
	std::io::stdout().flush().expect("Error flushing stdout");
}

fn get_directory(base: &Path, name: &str) -> std::path::PathBuf {
	let folder = base.join(name);
	if !folder.exists() {
		std::fs::create_dir(&folder)
			.unwrap_or_else(|_| panic!("{} could not be created", folder.to_string_lossy()));
	} else if !folder.is_dir() {
		panic!("{} exists and is not a directory", folder.to_string_lossy());
	}
	folder
}

fn main() -> Result<()> {
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

	let input_file = Path::new(matches.value_of("INPUT").unwrap());

	let output_path = Path::new(matches.value_of("output_path").unwrap_or_else(|| {
		input_file
			.file_stem()
			.unwrap()
			.to_str()
			.expect("output_path not given and could not be automatically determined")
	}));
	if !output_path.exists() {
		std::fs::create_dir(&output_path)
			.unwrap_or_else(|_| panic!("{} could not be created", output_path.to_string_lossy()));
	} else if !output_path.is_dir() {
		panic!(
			"{} exists and is not a directory",
			output_path.to_string_lossy()
		);
	}

	let attachment_folder =
		get_directory(output_path, matches.value_of("attachment_path").unwrap());
	let avatar_folder = get_directory(output_path, matches.value_of("avatar_path").unwrap());
	let sticker_folder = get_directory(output_path, matches.value_of("sticker_path").unwrap());
	let config_folder = get_directory(output_path, matches.value_of("config_path").unwrap());

	let sqlite_path = match matches.value_of("sqlite_file") {
		Some(s) => Path::new(&s).to_path_buf(),
		None => output_path.join("signal_backup.db"),
	};

	let mut password = match matches.value_of("password_string") {
		Some(p) => String::from(p),
		None => {
			let password_file = BufReader::new(
				File::open(matches.value_of("password_file").unwrap())
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

	let file = File::open(input_file).expect("Backup file could not be opened");
	let mut reader = BufReader::new(file);

	let mut tmpdir: Option<tempfile::TempDir> = None;

	let connection = if matches.is_present("no_tmp_sqlite") {
		sqlite::open(&sqlite_path)
			.unwrap_or_else(|_| panic!("Could not open database file: {:?}", sqlite_path))
	} else {
		let t = tempfile::tempdir()
			.expect("Failed to create tmpdir. Hint: Try running with --no-tmp-sqlite");
		let sqlite_path = t.path().join("signal_backup.sqlite");
		tmpdir = Some(t);
		sqlite::open(&sqlite_path)
			.unwrap_or_else(|_| panic!("Could not open database file: {:?}", sqlite_path))
	};

	decode_backup(
		&mut reader,
		&password,
		&attachment_folder,
		&avatar_folder,
		&sticker_folder,
		&config_folder,
		&connection,
		frame_callback,
		!matches.is_present("no_verify_mac"),
	)
	.unwrap();
	if tmpdir.is_some() {
		let t = tmpdir.unwrap();
		let sqlite_tmp_path = t.path().join("signal_backup.sqlite");
		match std::fs::rename(&sqlite_tmp_path, &sqlite_path) {
			Ok(_) => {
				println!("Moved sqlite to {}", &sqlite_path.to_string_lossy());
			}
			Err(e) => {
				println!(
					"{}, Could not move {} to {}, trying copy",
					e,
					&sqlite_tmp_path.to_string_lossy(),
					&sqlite_path.to_string_lossy()
				);
				std::fs::copy(&sqlite_tmp_path, &sqlite_path)?;
				std::fs::remove_file(&sqlite_tmp_path)?;
				println!(
					"Copy successful, sqlite at {}",
					&sqlite_path.to_string_lossy()
				);
			}
		}
		t.close().unwrap();
	}
	println!();
	Ok(())
}
