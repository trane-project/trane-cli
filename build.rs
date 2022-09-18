use std::{env, path};

use built::Options;

fn main() {
    let mut opts = Options::default();
    opts.set_dependencies(true);
    let manifest_location = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dst = path::Path::new(&env::var("OUT_DIR").unwrap()).join("built.rs");
    built::write_built_file_with_opts(&opts, manifest_location.as_ref(), &dst)
        .expect("Failed to acquire build-time information");
}
