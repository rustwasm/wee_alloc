extern crate globwalk;

use std::env::{self, VarError};
use std::fs::File;
use std::io::Write;
use std::path::Path;

const DEFAULT_STATIC_ARRAY_BACKEND_SIZE_BYTES: u32 = 1024 * 1024 * 32;
const WEE_ALLOC_STATIC_ARRAY_BACKEND_BYTES: &'static str = "WEE_ALLOC_STATIC_ARRAY_BACKEND_BYTES";

fn main() {
    create_static_array_backend_size_bytes_file();
    export_rerun_rules();
}

fn create_static_array_backend_size_bytes_file() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR environment variable not provided");
    let dest_path = Path::new(&out_dir).join("wee_alloc_static_array_backend_size_bytes.txt");
    let size: u32 = match env::var(WEE_ALLOC_STATIC_ARRAY_BACKEND_BYTES) {
        Ok(s) => s.parse().expect("Could not interpret WEE_ALLOC_STATIC_ARRAY_BACKEND_BYTES as a 32 bit unsigned integer"),
        Err(ve) => match ve {
            VarError::NotPresent => { DEFAULT_STATIC_ARRAY_BACKEND_SIZE_BYTES },
            VarError::NotUnicode(_) => { panic!("Could not interpret WEE_ALLOC_STATIC_ARRAY_BACKEND_BYTES as a string representing a 32 bit unsigned integer")},
        },
    };
    println!("Setting the static_array_backend size to {} bytes", size);
    let mut f = File::create(&dest_path)
        .expect("Could not create file to store wee_alloc static_array_backend size metadata.");
    write!(f, "{}", size)
        .expect("Could not write to wee_alloc static_array_backend size metadata file");
    f.flush()
        .expect("Could not flush write to wee_alloc static_array_backend size metadata file");
}
fn export_rerun_rules() {
    println!(
        "cargo:rerun-if-env-changed={}",
        WEE_ALLOC_STATIC_ARRAY_BACKEND_BYTES
    );
    for entry_result in
        globwalk::glob("*.{toml,rs}").expect("Could not create a valid rust-file-finding glob")
    {
        let file = entry_result.expect("Failed to read file information.");
        println!("cargo:rerun-if-changed={}", file.path().display());
    }
}
