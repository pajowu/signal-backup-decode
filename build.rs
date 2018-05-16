#[cfg(feature = "rebuild-protobuf")]
extern crate protoc_rust;
#[cfg(feature = "rebuild-protobuf")]
fn main() {
	protoc_rust::run(protoc_rust::Args {
        out_dir: "src",
        input: &["proto/Backups.proto"],
        includes: &["proto"],
        customize: protoc_rust::Customize {
	      ..Default::default()
	    },
    }).expect("protoc");
}

#[cfg(not(feature = "rebuild-protobuf"))]
fn main() {}
