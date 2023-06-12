//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs};

use tgnt::layer::Layer;

fn main() {
    memory();
    serialize_layout("./layers.ron", "./src/layers.pc");
}

fn memory() {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("../memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=../memory.x");

    // --nmagic turns off page alignment of sections (which saves flash space)
    println!("cargo:rustc-link-arg-bins=--nmagic");

    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tlink-rp.x");
    //println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
}

fn serialize_layout(ron_path: &str, postcard_path: &str) {
    println!("cargo:rerun-if-changed={ron_path}");

    let layers = fs::read_to_string(ron_path).expect("Failed to read .ron");
    let layers: Vec<Layer> = ron::from_str(&layers).expect("Failed to deserialize .ron");

    let serialized = postcard::to_stdvec(&layers).expect("Failed to serialize layers");

    File::create(postcard_path)
        .expect("Failed to create .pc")
        .write_all(&serialized)
        .expect("Failed to write .pc");
}
