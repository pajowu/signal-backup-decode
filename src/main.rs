use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use log::error;
use log::info;
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

fn read_frame<T: Read>(
    r: &mut T,
    decrypter: &mut Option<decrypter::Decrypter>,
) -> Result<(usize, Vec<u8>), anyhow::Error> {
    let len = r.read_u32::<BigEndian>()?.try_into()?;

    if let Some(decrypter) = decrypter {
        let mut frame_content = vec![0u8; len as usize - 10];
        let mut frame_hmac = vec![0u8; 10];
        r.read_exact(&mut frame_content)?;
        r.read_exact(&mut frame_hmac)?;
        decrypter.decrypt(&mut frame_content);
        decrypter.verify_mac(&frame_hmac)?;
        decrypter.increase_iv();
        Ok((len, frame_content.to_vec()))
    } else {
        let mut frame_content = vec![0u8; len as usize];
        r.read_exact(&mut frame_content)?;
        Ok((len, frame_content))
    }
}

fn read_attachment<R: Read>(
    reader: &mut R,
    length: usize,
    decrypter: &mut Option<decrypter::Decrypter>,
) -> Result<(std::vec::Vec<u8>, usize), anyhow::Error> {
    let mut bytes_left = length as usize;
    let mut attachment_data = Vec::new();
    let mut attachment_hmac = [0u8; 10];

    if let Some(decrypter) = decrypter {
        decrypter.mac_update_with_iv();
    }

    while bytes_left > 0 {
        let mut buffer = vec![0u8; std::cmp::min(bytes_left, 8192)];
        reader.read_exact(&mut buffer)?;
        bytes_left -= buffer.len();
        if let Some(decrypter) = decrypter {
            decrypter.decrypt(&mut buffer);
        }
        attachment_data.append(&mut buffer);
    }

    reader.read_exact(&mut attachment_hmac)?;
    if let Some(decrypter) = decrypter {
        decrypter.verify_mac(&attachment_hmac)?;
        decrypter.increase_iv();
    }

    Ok((attachment_data, length))
}

fn decode_backup<R: Read>(
    mut reader: R,
    config: &args::Config,
    output: &mut output_raw::Output,
    callback: fn(usize, usize, usize),
) -> Result<usize, anyhow::Error> {
    let mut decrypter: Option<decrypter::Decrypter> = None;

    let mut frame_count = 0;
    let mut seek_position = 0;

    loop {
        let (consumed_bytes, frame_content) = read_frame(&mut reader, &mut decrypter)?;
        seek_position += consumed_bytes;
        let frame = protobuf::parse_from_bytes::<Backups::BackupFrame>(&frame_content)
            .unwrap_or_else(|_| panic!("Could not parse frame from {:?}", frame_content));
        let frame = frame::Frame::new(&frame);

        match frame {
            frame::Frame::Header { salt, iv } => {
                decrypter = Some(decrypter::Decrypter::new(
                    &config.password,
                    salt,
                    iv,
                    config.no_verify_mac,
                ));
            }
            frame::Frame::Version { version } => {
                println!("Database Version: {:?}", version);
            }
            frame::Frame::Attachment { attachment } => {
                if decrypter.is_some() {
                    let (data, read_bytes) = read_attachment(
                        &mut reader,
                        attachment.get_length().try_into()?,
                        &mut decrypter,
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
                if decrypter.is_some() {
                    let (data, read_bytes) = read_attachment(
                        &mut reader,
                        avatar.get_length().try_into()?,
                        &mut decrypter,
                    )?;
                    seek_position += read_bytes;
                    output.write_avatar(&data, avatar.get_name())?;
                } else {
                    panic!("Attachment/Avatar found before header, exiting");
                }
            }
            frame::Frame::Sticker { sticker } => {
                if decrypter.is_some() {
                    let (data, read_bytes) = read_attachment(
                        &mut reader,
                        sticker.get_length().try_into()?,
                        &mut decrypter,
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
