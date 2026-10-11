use crate::{
    blocks::*,
    error::{Error, Result},
    reader::{slice, u32_le},
};
use std::collections::BTreeMap;

/// Contiguous memory reconstructed from the observed FS2 variant, with source mapping.
#[derive(Debug)]
pub struct Image {
    pub bytes: Vec<u8>,
    pub blocks: Vec<Block>,
    page_sources: BTreeMap<usize, usize>,
}

impl Image {
    pub fn source_offset(&self, address: usize) -> Result<usize> {
        slice(&self.bytes, address, 1)?;
        let base = address / PAYLOAD_SIZE * PAYLOAD_SIZE;
        self.page_sources
            .get(&base)
            .map(|file| file + PAYLOAD_OFFSET + address % PAYLOAD_SIZE)
            .ok_or_else(|| Error::Format(format!("unmapped address 0x{address:x}")))
    }
}

pub fn parse(bytes: &[u8]) -> Result<Image> {
    if bytes.is_empty() || bytes.len() % BLOCK_SIZE != 0 {
        return Err(Error::Format(
            "empty input or incomplete 512-byte block".into(),
        ));
    }
    let count = bytes.len() / BLOCK_SIZE;
    if count > 32768 {
        return Err(Error::Format(
            "reconstructed image exceeds 8 MiB limit".into(),
        ));
    }
    let mut pages = BTreeMap::new();
    let mut blocks = Vec::with_capacity(count);
    for (i, block) in bytes.chunks_exact(BLOCK_SIZE).enumerate() {
        let offset = i * BLOCK_SIZE;
        if block[..4] != MAGIC || u32_le(block, 4)? != MAGIC2 || u32_le(block, 508)? != TRAILER {
            return Err(Error::Format(format!(
                "block at 0x{offset:x}: incorrect magic or trailer"
            )));
        }
        let address = u32_le(block, 12)? as usize;
        let number = u32_le(block, 20)? as usize;
        if u32_le(block, 8)? != 0 || u32_le(block, 28)? != 0 || u32_le(block, 16)? != 256 {
            return Err(Error::Format(format!(
                "block at 0x{offset:x}: unsupported flags, family or payload size"
            )));
        }
        if u32_le(block, 24)? as usize != count || number >= count || address != number * 256 {
            return Err(Error::Format(format!(
                "block at 0x{offset:x}: inconsistent count, number or address"
            )));
        }
        if block[288..508].iter().any(|b| *b != 0) {
            return Err(Error::Format(format!(
                "block at 0x{offset:x}: unsupported nonzero padding"
            )));
        }
        if pages.insert(address, offset).is_some() {
            return Err(Error::Format(format!(
                "duplicate target address 0x{address:x}"
            )));
        }
        blocks.push(Block {
            file_offset: offset,
            target_address: address,
            number,
            total_blocks: count,
        });
    }
    let mut memory = Vec::with_capacity(count * 256);
    for offset in pages.values() {
        memory.extend_from_slice(slice(bytes, offset + 32, 256)?);
    }
    Ok(Image {
        bytes: memory,
        blocks,
        page_sources: pages,
    })
}
