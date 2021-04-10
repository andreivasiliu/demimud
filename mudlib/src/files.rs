//! Abstraction for reading files.
//!
//! This is used to emulate reading files on WASM in a browser, while using the
//! real filesystem otherwise.

use std::{borrow::Cow, fs::File, path::Path};

pub trait Files {
    /// Read a file's contents into a `Vec<u8>`
    fn read_file_raw(&self, path: &str) -> Result<Vec<u8>, std::io::Error>;

    /// Ignore UTF8 errors and fix new lines to match telnet's \r\n
    fn read_file(&self, path: &str) -> Result<String, std::io::Error> {
        let bytes = self.read_file_raw(path)?;
        let string = String::from_utf8_lossy(&bytes);
        Ok(fix_newlines(&string).into_owned())
    }
}

pub(crate) struct RealFiles;

impl Files for RealFiles {
    fn read_file_raw(&self, path: &str) -> Result<Vec<u8>, std::io::Error> {
        use std::io::Read;

        let mut bytes = Vec::new();
        let mut file = File::open(Path::new(path))?;
        file.read_to_end(&mut bytes)?;

        Ok(bytes)
    }
}

fn valid_newlines(text: &str) -> bool {
    let mut last_was_r = false;

    for c in text.chars() {
        match (c, last_was_r) {
            // Invalid: Two \r in a row
            ('\r', true) => return false,
            // Valid: starting with \r
            ('\r', false) => last_was_r = true,
            // Valid: \n following \r
            ('\n', true) => last_was_r = false,
            // Invalid: \n not following \r
            ('\n', false) => return false,
            // Invalid: \r not followed by \n
            (_, true) => return false,
            // Valid: anything else
            (_, false) => (),
        };
    }

    // Invalid: Ending in '\r'
    if last_was_r {
        return false;
    }

    true
}

/// Turn all types of newlines into "\r\n"
pub fn fix_newlines(text: &str) -> Cow<'_, str> {
    if valid_newlines(text) {
        return Cow::Borrowed(text);
    }

    let sanitized_text = text.replace("\r", "");

    Cow::Owned(sanitized_text.replace("\n", "\r\n"))
}
