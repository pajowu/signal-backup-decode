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
	let mut output = output_raw::Output::new(&config.path_output_main, true)?;

	// input
	let mut reader =
		input::InputFile::new(&config.path_input, &config.password, config.verify_mac)?;

	// progress bar
	let progress = display::Progress::new(
		reader.get_file_size(),
		reader.get_count_frame().try_into().unwrap(),
	);
	let progress_read = progress.clone();
	let progress_write = progress.clone();

	// channel to parallelize input reading / processing and output writing
	// and to display correct status
	let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel(10);

	let thread_input = std::thread::spawn(move || {
		while let Some(frame) = reader.next() {
			match frame {
				Ok(x) => {
					// if we cannot send a frame, probably an error has occured in the
					// output thread. Thus, just shut down the input thread. We will print
					// the error in the output thread.
					if let Err(_) = frame_tx.send(x) {
						break;
					}

					// forward progress bar if everything is ok
					progress_read.set_read_frames(reader.get_count_frame().try_into().unwrap());
					progress_read.set_read_bytes(reader.get_count_byte().try_into().unwrap());
				}
				Err(e) => {
					error!("{}.", e);
					break;
				}
			}
		}

		progress_read.bar_bytes.finish_at_current_pos();
	});

	let thread_output = std::thread::spawn(move || {
		for received in frame_rx {
			match output.write_frame(received) {
				Ok(_) => progress_write
					.set_written_frames(output.get_written_frames().try_into().unwrap()),
				Err(e) => {
					error!("{}.", e);
					break;
				}
			}
		}

		progress_write.bar_frames.finish_at_current_pos();
	});

	progress.finish_all();
	thread_input.join().unwrap();
	thread_output.join().unwrap();

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
