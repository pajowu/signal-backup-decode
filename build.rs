#[cfg(feature = "rebuild-protobuf")]
extern crate protoc_rust;

#[cfg(feature = "rebuild-protobuf")]
fn main() {
	protoc_rust::Codegen::new()
		.out_dir("src")
		.inputs(&["proto/Backups.proto"])
		.include("proto")
		.run()
		.expect("Running protoc failed.");
}

#[cfg(not(feature = "rebuild-protobuf"))]
fn main() {}
