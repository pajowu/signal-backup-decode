extern crate protoc_rust;

fn main() {
	protoc_rust::run(protoc_rust::Args {
        out_dir: "src",
        input: &["proto/Backups.proto"],
        includes: &["proto"],
    }).expect("protoc");
}
