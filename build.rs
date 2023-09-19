use std::{env, path};

fn main() {
    let manifest_var = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_location = path::Path::new(&manifest_var);
    let dst = path::Path::new(&env::var("OUT_DIR").unwrap()).join("built.rs");
    built::write_built_file_with_opts(Some(manifest_location), &dst)
        .expect("Failed to acquire build-time information");
}
