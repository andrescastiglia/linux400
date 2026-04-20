use std::path::PathBuf;

fn main() {
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ENV");
    println!("cargo:rerun-if-changed=src/bdb_shim.c");

    if target_env == "musl" {
        return;
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let local_db_include = manifest_dir
        .parent()
        .unwrap()
        .join("scratch/libdb/extracted53/usr/include");

    let mut build = cc::Build::new();
    build.file("src/bdb_shim.c");
    if local_db_include.exists() {
        build.include(local_db_include);
    } else {
        build.include("/usr/include");
    }
    build.compile("l400_bdb_shim");

    println!("cargo:rustc-link-lib=db-5.3");
}
