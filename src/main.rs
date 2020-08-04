use anyhow::anyhow;
use anyhow::Context;
use log::error;
use log::info;
use std::io::Write;

mod Backups;
mod args;
mod decrypter;
mod frame;
mod input;
mod output_raw;

fn frame_callback(frame_count: usize, seek_position: usize) {
	std::io::stdout()
		.write_all(
			format!(
				"Successfully read {} frames and {} bytes. Info about written bytes is missing.\r",
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

	// input
	let mut reader =
		input::InputFile::new(&config.path_input, &config.password, config.verify_mac)?;

	// channel to parallelize input reading / processing and output writing
	// and to display correct status
	let (frame_tx, frame_rx) = std::sync::mpsc::channel();
	let (status_tx, status_rx) = std::sync::mpsc::channel();

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

			status_tx
				.send((reader.get_count_frame(), reader.get_count_byte()))
				.unwrap();
		}

		Ok(())
	});

	let thread_output = std::thread::spawn(move || -> Result<(), anyhow::Error> {
		for received in frame_rx {
			output.write_frame(received)?;
		}

		Ok(())
	});

	let thread_status = std::thread::spawn(move || {
		for received in status_rx {
			frame_callback(received.0, received.1);
		}

		println!("");
	});

	thread_input.join().unwrap()?;
	thread_output.join().unwrap()?;
	thread_status.join().unwrap();

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
