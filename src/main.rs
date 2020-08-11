use anyhow::anyhow;
use anyhow::Context;
use log::error;
use log::info;
use std::convert::TryInto;

mod Backups;
mod args;
mod decrypter;
mod display;
mod frame;
mod input;
mod output_raw;

fn run(config: &args::Config) -> Result<(), anyhow::Error> {
	// output
	let mut output = output_raw::Output::new(&config.path_output, true)?;

	// input
	let mut reader =
		input::InputFile::new(&config.path_input, &config.password, config.verify_mac)?;

	// progress bar
	let progress = std::sync::Arc::new(display::Progress::new(
		reader.get_file_size(),
		reader.get_count_frame().try_into().unwrap(),
	));
	let progress_read = progress.clone();
	let progress_write = progress.clone();

	// channel to parallelize input reading / processing and output writing
	// and to display correct status
	let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel(10);

	let thread_input = std::thread::spawn(move || -> Result<(), anyhow::Error> {
		loop {
			let frame = reader.read_frame()?;
			let mut frame = protobuf::parse_from_bytes::<crate::Backups::BackupFrame>(&frame)
				.with_context(|| format!("Could not parse frame from {:?}", frame))?;
			let mut frame = crate::frame::Frame::new(&mut frame);

			match frame {
				frame::Frame::Version { version } => {
					info!("Database Version: {:?}", version);
				}
				frame::Frame::Attachment { data_length, .. } => {
					frame.set_data(reader.read_data(data_length)?);
					frame_tx.send(frame).unwrap();
				}
				frame::Frame::Avatar { data_length, .. } => {
					frame.set_data(reader.read_data(data_length)?);
					frame_tx.send(frame).unwrap();
				}
				frame::Frame::Sticker { data_length, .. } => {
					frame.set_data(reader.read_data(data_length)?);
					frame_tx.send(frame).unwrap();
				}
				frame::Frame::Header { .. } => return Err(anyhow!("unexpected header found")),
				frame::Frame::End => {
					break;
				}
				_ => {
					frame_tx.send(frame).unwrap();
				}
			};

			progress_read.set_read_frames(reader.get_count_frame().try_into().unwrap());
			progress_read.set_read_bytes(reader.get_count_byte().try_into().unwrap());
		}

		progress_read.finish_bytes();
		Ok(())
	});

	let thread_output = std::thread::spawn(move || -> Result<(), anyhow::Error> {
		for received in frame_rx {
			output.write_frame(received)?;
			progress_write.set_written_frames(output.get_written_frames().try_into().unwrap());
		}

		progress_write.finish_frames();
		Ok(())
	});

	progress.finish_all();
	thread_input.join().unwrap()?;
	thread_output.join().unwrap()?;

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

	if let Err(e) = run(&config) {
		error!("{}.", e);
		std::process::exit(1);
	}
}
