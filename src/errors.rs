error_chain! {
	foreign_links {
		Io(::std::io::Error);
		OpenSSL(::openssl::error::ErrorStack);
		Sqlite(::sqlite::Error);
	}
	errors {
		MacVerificationError(s: Vec<u8>, i: Vec<u8>) {
			description("mac verification failed")
			display("mac verification failed. should be {:?}, is {:?}", s.as_slice(), i.as_slice())
		}
	}
}
