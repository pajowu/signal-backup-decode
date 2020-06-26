use anyhow::anyhow;
use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use crypto::mac::Mac;
use log::error;
use log::info;
use openssl::hash::{Hasher, MessageDigest};
use openssl::symm;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufReader;
use std::io::{Read, Write};

mod Backups;
mod args;
mod decrypter;
mod frame;
mod input;
mod output_raw;

struct CipherData {
    hmac: crypto::hmac::Hmac<crypto::sha2::Sha256>,
    cipher_key: [u8; 32],
    counter: Vec<u8>,
}

fn read_frame<T: Read>(
    r: &mut T,
    cipher_data: &mut Option<CipherData>,
    verify_mac: bool,
    decrypter: &mut Option<decrypter::Decrypter>,
) -> Result<(usize, Vec<u8>), anyhow::Error> {
    let len = r.read_u32::<BigEndian>()?.try_into()?;
    let mut frame_content = vec![0u8; len as usize];
    r.read_exact(&mut frame_content)?;

    if let Some(decrypter) = decrypter {
        decrypter.decrypt(&mut frame_content)?;
        Ok((len, frame_content))
    } else {
        Ok((len, frame_content))
    }
}
fn decrypt(key: &[u8; 32], counter: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
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
fn generate_keys(key: &[u8], salt: &[u8]) -> Result<([u8; 32], [u8; 32]), anyhow::Error> {
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

fn read_attachment<R: Read>(
    reader: &mut R,
    cipher_data: &mut CipherData,
    length: usize,
    verify_mac: bool,
) -> Result<(std::vec::Vec<u8>, usize), anyhow::Error> {
    let mut decrypter = symm::Crypter::new(
        symm::Cipher::aes_256_ctr(),
        symm::Mode::Decrypt,
        &cipher_data.cipher_key,
        Some(&&cipher_data.counter),
    )?;
    let block_size = symm::Cipher::aes_256_ctr().block_size();
    let mut plaintext: std::vec::Vec<u8> = vec![0; 8192 + block_size];
    let mut plaintext_total: std::vec::Vec<u8> = std::vec::Vec::new();

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
        // writer.write_all(&plaintext[..count])?;
        plaintext_total.extend_from_slice(&plaintext[..count]);
    }

    let mut mac = [0u8; 10];
    reader.read_exact(&mut mac)?;
    if verify_mac {
        let hmac_result = cipher_data.hmac.result();
        let calculated_mac = &hmac_result.code()[..10];
        cipher_data.hmac.reset();
        if !crypto::util::fixed_time_eq(calculated_mac, &mac) {
            return Err(anyhow!(
                "MacVerificationError, {:?}, {:?}.",
                calculated_mac.to_vec(),
                mac.to_vec()
            ));
        }
    }
    increase_counter(&mut cipher_data.counter, None);
    Ok((plaintext_total, length))
}

fn decode_backup<R: Read>(
    mut reader: R,
    config: &args::Config,
    output: &mut output_raw::Output,
    callback: fn(usize, usize, usize),
) -> Result<usize, anyhow::Error> {
    let mut decrypter: Option<decrypter::Decrypter> = None;

    let mut cipher_data: Option<CipherData> = None;
    let verify_mac = config.no_verify_mac;

    let mut frame_count = 0;
    let mut seek_position = 0;

    loop {
        let (consumed_bytes, frame_content) =
            read_frame(&mut reader, &mut cipher_data, verify_mac, &mut decrypter)?;
        seek_position += consumed_bytes;
        let frame = protobuf::parse_from_bytes::<Backups::BackupFrame>(&frame_content)
            .unwrap_or_else(|_| panic!("Could not parse frame from {:?}", frame_content));
        let frame = frame::Frame::new(&frame);

        match frame {
            frame::Frame::Header { salt, iv } => {
                let (cipher_key, mac_key) =
                    generate_keys(&config.password, salt).expect("Error generating keys");
                decrypter = Some(decrypter::Decrypter::new(
                    &config.password,
                    salt,
                    iv,
                    config.no_verify_mac,
                ));
                cipher_data = Some(CipherData {
                    hmac: crypto::hmac::Hmac::new(crypto::sha2::Sha256::new(), &mac_key),
                    cipher_key,
                    counter: iv.to_vec(),
                })
            }
            frame::Frame::Version { version } => {
                println!("Database Version: {:?}", version);
            }
            frame::Frame::Attachment { attachment } => {
                if let Some(ref mut c) = cipher_data {
                    let (data, read_bytes) = read_attachment(
                        &mut reader,
                        c,
                        attachment.get_length().try_into()?,
                        verify_mac,
                    )?;
                    seek_position += read_bytes;
                    output.write_attachment(
                        &data,
                        attachment.get_attachmentId(),
                        attachment.get_rowId(),
                    )?;
                } else {
                    panic!("Attachment found before header, exiting");
                }
            }
            frame::Frame::Avatar { avatar } => {
                if let Some(ref mut c) = cipher_data {
                    let (data, read_bytes) = read_attachment(
                        &mut reader,
                        c,
                        avatar.get_length().try_into()?,
                        verify_mac,
                    )?;
                    seek_position += read_bytes;
                    output.write_avatar(&data, avatar.get_name())?;
                } else {
                    panic!("Attachment/Avatar found before header, exiting");
                }
            }
            frame::Frame::Sticker { sticker } => {
                if let Some(ref mut c) = cipher_data {
                    let (data, read_bytes) = read_attachment(
                        &mut reader,
                        c,
                        sticker.get_length().try_into()?,
                        verify_mac,
                    )?;
                    seek_position += read_bytes;
                    output.write_sticker(&data, sticker.get_rowId())?;
                } else {
                    panic!("Attachment/Sticker found before header, exiting");
                }
            }
            frame::Frame::Statement {
                statement,
                parameter,
            } => {
                output.write_statement(statement, parameter)?;
            }
            frame::Frame::Preference { preference } => {
                output.write_preference(preference)?;
            }
            frame::Frame::End => {
                break;
            }
        };

        // TODO this has to checked elsewhere
        //else if cipher_data.is_none() {
        //    panic!("Read non-header frame before header frame");
        //}
        frame_count += 1;
        callback(frame_count, output.get_attachment_count(), seek_position);
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

fn run(config: &args::Config) -> Result<(), anyhow::Error> {
    // input file
    let file = File::open(&config.path_input).expect("Backup file could not be opened");
    let mut reader = BufReader::new(file);

    // output
    let mut output = output_raw::Output::new(&config.path_output_main, true)?;

    decode_backup(&mut reader, config, &mut output, frame_callback)?;

    println!();
    Ok(())
}

fn main() {
    // build config structure
    let config = args::Config::new().unwrap_or_else(|e| {
        eprintln!("Problem parsing arguments: {}.", e);
        std::process::exit(1);
    });

    simplelog::TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
    )
    .unwrap();

    // measuring runtime and run program
    let now = std::time::Instant::now();

    if let Err(e) = run(&config) {
        error!("{}.", e);
        std::process::exit(1);
    }

    info! {"Runtime duration: {} seconds", now.elapsed().as_secs()};
}
