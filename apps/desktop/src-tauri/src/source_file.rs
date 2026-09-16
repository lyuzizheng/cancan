//! Size policy and bounded reads for files the user hands CanCan.
//!
//! Both acquisition channels — Local Inbox capture and explicit handoff —
//! read a whole source file into memory before hashing, container validation,
//! encryption, parsing, and page rendering. They share this ceiling so the
//! worst case stays one bounded buffer instead of whatever the filesystem
//! happens to hold.

use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};
use zeroize::Zeroizing;

/// Ceiling for one source document. Generous for real statements — scanned
/// exports stay well below it — while refusing a multi-gigabyte file before
/// the first read.
pub(crate) const MAX_SOURCE_FILE_BYTES: u64 = 128 * 1024 * 1024;

pub(crate) fn too_large() -> io::Error {
    io::Error::new(
        io::ErrorKind::FileTooLarge,
        "source file exceeds the supported size",
    )
}

pub(crate) fn read_source_file(path: &Path) -> io::Result<Zeroizing<Vec<u8>>> {
    let mut file = File::open(path)?;
    read_bounded_source_file(&mut file)
}

/// Reads a source file without ever allocating more than
/// [`MAX_SOURCE_FILE_BYTES`], so a file that grows between its size check and
/// this read still cannot exhaust memory.
pub(crate) fn read_bounded_source_file(file: &mut File) -> io::Result<Zeroizing<Vec<u8>>> {
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(MAX_SOURCE_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SOURCE_FILE_BYTES {
        return Err(too_large());
    }
    Ok(bytes)
}
