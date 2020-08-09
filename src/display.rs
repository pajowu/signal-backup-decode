/// Dispaly a progress bar
pub struct Progress {
	bar_multi: indicatif::MultiProgress,
	bar_bytes: indicatif::ProgressBar,
	bar_frames: indicatif::ProgressBar,
}

impl Progress {
	pub fn new(bytes_to_read: u64, frames_to_read: u64) -> Self {
		let sty_bytes = indicatif::ProgressStyle::default_bar()
                    .template("             Bytes read: [{elapsed_precise}] [{bar:50.blue/blue}] {bytes}/{total_bytes}")
                    .progress_chars("#>-");
		let sty_frames = indicatif::ProgressStyle::default_bar()
                    .template("Read vs. written frames: [{elapsed_precise}] [{bar:50.cyan/cyan}] {pos:>5}/{len:5}")
                    .progress_chars("#>-");

		// we set 2 read frames in the beginning because we have 1) a header frame and 2) a version
		// frame we do not count in written frames.
		let multi = indicatif::MultiProgress::new();
		let bar_bytes = multi.add(indicatif::ProgressBar::new(bytes_to_read));
		bar_bytes.set_style(sty_bytes);
		let bar_frames = multi.add(indicatif::ProgressBar::new(frames_to_read));
		bar_frames.set_style(sty_frames);

		Self {
			bar_multi: multi,
			bar_bytes,
			bar_frames,
		}
	}

	pub fn set_read_frames(&self, length: u64) {
		self.bar_frames.set_length(length);
	}

	pub fn set_written_frames(&self, length: u64) {
		self.bar_frames.set_position(length);
	}

	pub fn set_read_bytes(&self, length: u64) {
		self.bar_bytes.set_position(length);
	}

	pub fn finish_frames(&self) {
		self.bar_frames.finish();
	}

	pub fn finish_bytes(&self) {
		self.bar_bytes.finish();
	}

	pub fn finish_all(&self) {
		self.bar_multi.join().unwrap();
	}
}
