use anyhow::anyhow;
use anyhow::Context;
use log::error;
use log::info;
use std::convert::TryInto;
use std::io::Write;

mod Backups;
mod args;
mod decrypter;
mod frame;
mod input;
mod output_raw;

fn decode_backup(
    config: &args::Config,
    output: &mut output_raw::Output,
    callback: fn(usize, usize),
) -> Result<(), anyhow::Error> {
    let mut reader =
        input::InputFile::new(&config.path_input, &config.password, config.no_verify_mac)?;

    loop {
        let frame = reader.read_frame()?;
        let frame = protobuf::parse_from_bytes::<crate::Backups::BackupFrame>(&frame)
            .with_context(|| format!("Could not parse frame from {:?}", frame))?;
        let frame = crate::frame::Frame::new(&frame);

        match frame {
            frame::Frame::Version { version } => {
                println!("Database Version: {:?}", version);
            }
            frame::Frame::Attachment { attachment } => {
                let data = reader.read_data(attachment.get_length().try_into()?)?;
                output.write_attachment(
                    &data,
                    attachment.get_attachmentId(),
                    attachment.get_rowId(),
                )?;
            }
            frame::Frame::Avatar { avatar } => {
                let data = reader.read_data(avatar.get_length().try_into()?)?;
                output.write_avatar(&data, avatar.get_name())?;
            }
            frame::Frame::Sticker { sticker } => {
                let data = reader.read_data(sticker.get_length().try_into()?)?;
                output.write_sticker(&data, sticker.get_rowId())?;
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
            _ => return Err(anyhow!("unexpected header found")),
        };

        callback(reader.get_count_frame(), reader.get_count_byte());
    }

    Ok(())
}

fn frame_callback(frame_count: usize, seek_position: usize) {
    std::io::stdout()
        .write_all(
            format!(
                "Successfully read {} frames, {} bytes into file\r",
                frame_count, seek_position
            )
            .as_bytes(),
        )
        .expect("Error writing status to stdout");
    std::io::stdout().flush().expect("Error flushing stdout");
}

fn run(config: &args::Config) -> Result<(), anyhow::Error> {
    // output
    let mut output = output_raw::Output::new(&config.path_output_main, true)?;

    decode_backup(config, &mut output, frame_callback)?;

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
