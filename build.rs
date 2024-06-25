use std::path::PathBuf;
use std::env;

use deno_runtime::ops::bootstrap::SnapshotOptions;
use deno_runtime::deno_core;

fn create_startup_snapshot(snapshot_path: PathBuf) {

    let snapshot_options = SnapshotOptions {
        deno_version: env!("CARGO_PKG_VERSION").to_string(),
        ts_version: mtsc::version().to_string(),
        v8_version: deno_core::v8_version(),
        target: std::env::var("TARGET").unwrap(),
    };

    deno_runtime::snapshot::create_runtime_snapshot(
        snapshot_path,
        snapshot_options,
        vec![]
    );
}   

fn main() {
    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
    println!("cargo:rustc-env=PROFILE={}", env::var("PROFILE").unwrap());  

    create_startup_snapshot(PathBuf::from(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("STARTUP_SNAPSHOT.bin")
    ));
}