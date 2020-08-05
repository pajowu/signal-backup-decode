/// Dispaly a progress bar
pub struct Progress {
	bar: indicatif::ProgressBar,
}

impl Progress {
	pub fn new() -> Self {
		let sty = indicatif::ProgressStyle::default_bar()
            .template("Read vs. written frames: [{elapsed_precise}] [{bar:50.cyan/blue}] {pos:>5}/{len:5}")
            .progress_chars("#>-");

		// we set 2 read frames in the beginning because we have 1) a header frame and 2) a version
		// frame we do not count in written frames.
		let bar = indicatif::ProgressBar::new(2);
		bar.set_style(sty);
		//bar.set_position(2);

		Self { bar }
	}

	pub fn set_read_frames(&self, length: u64) {
		self.bar.set_length(length);
	}

	pub fn set_written_frames(&self, length: u64) {
		self.bar.set_position(length);
	}

	pub fn finish(&self) {
		self.bar.finish();
	}
}
