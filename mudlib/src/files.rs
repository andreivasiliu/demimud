//! Abstraction for reading files.
//!
//! This is used to emulate reading files on WASM in a browser, while using the
//! real filesystem otherwise.

use std::{fs::File, path::Path};

pub trait Files {
    fn read_file(&self, path: &str) -> Result<String, std::io::Error>;
}

pub(crate) struct RealFiles;

impl Files for RealFiles {
    fn read_file(&self, path: &str) -> Result<String, std::io::Error> {
        lossy_read_to_string(path)
    }
}

/// Like std::fs::read_to_string, but ignores UTF8 errors
fn lossy_read_to_string(path: &str) -> Result<String, std::io::Error> {
    use std::io::Read;

    let mut bytes = Vec::new();
    let mut file = File::open(Path::new(path))?;
    file.read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
