use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

pub const MAX_FILE_SIZE: usize = 64 * 1024 * 1024;

pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_SIZE as u64 {
        return Err(Error::Format(
            "requires a regular file of at most 64 MiB".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_SIZE as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_FILE_SIZE {
        return Err(Error::Format("input grew beyond the 64 MiB limit".into()));
    }
    Ok(bytes)
}

pub fn slice(bytes: &[u8], offset: usize, length: usize) -> Result<&[u8]> {
    offset
        .checked_add(length)
        .and_then(|end| bytes.get(offset..end))
        .ok_or(Error::Bounds {
            offset,
            length,
            size: bytes.len(),
        })
}

pub fn u32_le(bytes: &[u8], offset: usize) -> Result<u32> {
    let b = slice(bytes, offset, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

pub fn hexdump(bytes: &[u8], offset: usize, length: usize) -> Result<String> {
    use std::fmt::Write;
    let selected = slice(bytes, offset, length)?;
    let mut output = String::new();
    for (index, row) in selected.chunks(16).enumerate() {
        let _ = write!(output, "{:08x}  ", offset + index * 16);
        for column in 0..16 {
            if let Some(byte) = row.get(column) {
                let _ = write!(output, "{byte:02x} ");
            } else {
                output.push_str("   ");
            }
            if column == 7 {
                output.push(' ');
            }
        }
        output.push_str(" | ");
        for byte in row {
            output.push(if (32..=126).contains(byte) {
                char::from(*byte)
            } else {
                '.'
            });
        }
        output.push('\n');
    }
    Ok(output)
}
