use serde::Serialize;

pub const BLOCK_SIZE: usize = 512;
pub const PAYLOAD_SIZE: usize = 256;
pub const PAYLOAD_OFFSET: usize = 32;
pub const MAGIC: [u8; 4] = *b"FS2\n";
pub const MAGIC2: u32 = 0x9e5d5157;
pub const TRAILER: u32 = 0x0ab16f30;

#[derive(Clone, Debug, Serialize)]
pub struct Block {
    pub file_offset: usize,
    pub target_address: usize,
    pub number: usize,
    pub total_blocks: usize,
}

#[derive(Debug, Serialize)]
pub struct Region {
    pub first_block: usize,
    pub last_block: usize,
    pub file_offset: usize,
    pub file_offset_hex: String,
    pub payload_kind: String,
}

pub fn payload_regions(bytes: &[u8]) -> Vec<Region> {
    let mut result: Vec<Region> = Vec::new();
    for (i, block) in bytes.chunks_exact(BLOCK_SIZE).enumerate() {
        let payload = &block[32..288];
        let kind = if payload.iter().all(|b| *b == 0xff) {
            "constant_ff"
        } else if payload.iter().all(|b| *b == 0) {
            "constant_zero"
        } else {
            "mixed"
        };
        if let Some(last) = result.last_mut().filter(|r| r.payload_kind == kind) {
            last.last_block = i;
        } else {
            result.push(Region {
                first_block: i,
                last_block: i,
                file_offset: i * 512,
                file_offset_hex: format!("0x{:x}", i * 512),
                payload_kind: kind.into(),
            });
        }
    }
    result
}
