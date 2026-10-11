use crate::{
    blocks::{self, BLOCK_SIZE},
    parser,
    reader::{hash, u32_le},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StringEvidence {
    pub file_offset: usize,
    pub file_offset_hex: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct BlockHypothesis {
    pub size: usize,
    pub complete_blocks: usize,
    pub remainder: usize,
    pub fs2_at_boundary: usize,
}

#[derive(Debug, Serialize)]
pub struct FilesystemEvidence {
    pub validation: String,
    pub active_sectors: Option<usize>,
    pub released_sectors: Option<usize>,
    pub erased_sectors: Option<usize>,
    pub crc_verified_sectors: Option<usize>,
    pub active_files: Option<usize>,
    pub active_work_directories: Option<usize>,
    pub unreferenced_active_sectors: Vec<u16>,
}

#[derive(Debug, Serialize)]
pub struct Analysis {
    pub schema: String,
    pub size_bytes: usize,
    pub sha256: String,
    pub fs2_signature_occurrences: usize,
    pub first_signature_offsets: Vec<usize>,
    pub block_hypotheses: Vec<BlockHypothesis>,
    pub structural_validation: String,
    pub decoded_memory_bytes: Option<usize>,
    pub filesystem: Option<FilesystemEvidence>,
    pub regions: Vec<blocks::Region>,
    pub strings: Vec<StringEvidence>,
    pub strings_truncated: bool,
    pub header_u32_le: Vec<u32>,
    pub header_u32_be: Vec<u32>,
    pub confirmed_fields: Vec<String>,
    pub unknown_fields: Vec<String>,
    pub scientific_export_requires_reference: bool,
}

pub fn analyze(bytes: &[u8]) -> Analysis {
    let hits: Vec<_> = bytes
        .windows(4)
        .enumerate()
        .filter_map(|(i, b)| (b == blocks::MAGIC).then_some(i))
        .collect();
    let hypotheses = [128, 256, 512, 1024, 2048, 4096]
        .into_iter()
        .map(|size| BlockHypothesis {
            size,
            complete_blocks: bytes.len() / size,
            remainder: bytes.len() % size,
            fs2_at_boundary: bytes
                .chunks_exact(size)
                .filter(|b| b[..4] == blocks::MAGIC)
                .count(),
        })
        .collect();
    let parsed = parser::parse(bytes);
    let memory_size = parsed.as_ref().ok().map(|i| i.bytes.len());
    let valid = parsed.is_ok();
    let filesystem = parsed
        .as_ref()
        .ok()
        .map(|image| match crate::filesystem::inspect(image) {
            Ok(snapshot) => FilesystemEvidence {
                validation: format!(
                    "confirmed observed {} variant; active directory traversal and CRC verified",
                    snapshot.layout
                ),
                active_sectors: Some(snapshot.active_sector_count),
                released_sectors: Some(snapshot.released_sector_count),
                erased_sectors: Some(snapshot.erased_sector_count),
                crc_verified_sectors: Some(snapshot.crc_verified_count),
                active_files: Some(snapshot.files.len()),
                active_work_directories: Some(
                    snapshot
                        .directories
                        .iter()
                        .filter(|d| {
                            d.path.strip_prefix("/trabalho").is_some_and(|s| {
                                !s.is_empty()
                                    && s.bytes().all(|b| b.is_ascii_digit())
                                    && s.parse::<u16>().is_ok()
                            })
                        })
                        .count(),
                ),
                unreferenced_active_sectors: snapshot.unreferenced_active_sectors,
            },
            Err(error) => FilesystemEvidence {
                validation: error.to_string(),
                active_sectors: None,
                released_sectors: None,
                erased_sectors: None,
                crc_verified_sectors: None,
                active_files: None,
                active_work_directories: None,
                unreferenced_active_sectors: Vec::new(),
            },
        });
    let validation = match parsed {
        Ok(_) => "confirmed observed FS2 container variant; checksum semantics unknown".into(),
        Err(e) => e.to_string(),
    };
    let mut strings = Vec::new();
    let mut total = 0;
    let mut start = 0;
    for end in 0..=bytes.len() {
        if end < bytes.len() && (32..=126).contains(&bytes[end]) {
            continue;
        }
        if end - start >= 8 {
            total += 1;
            if strings.len() < 128 {
                strings.push(StringEvidence {
                    file_offset: start,
                    file_offset_hex: format!("0x{start:x}"),
                    value: String::from_utf8_lossy(&bytes[start..end.min(start + 160)])
                        .into_owned(),
                });
            }
        }
        start = end.saturating_add(1);
    }
    Analysis {
        schema: "fc2-analysis-v1".into(),
        size_bytes: bytes.len(),
        sha256: hash(bytes),
        fs2_signature_occurrences: hits.len(),
        first_signature_offsets: hits.into_iter().take(32).collect(),
        block_hypotheses: hypotheses,
        structural_validation: validation,
        decoded_memory_bytes: memory_size,
        filesystem,
        regions: if valid {
            blocks::payload_regions(bytes)
        } else {
            Vec::new()
        },
        strings,
        strings_truncated: total > 128,
        header_u32_le: (0..bytes.len().min(32) / 4)
            .filter_map(|i| u32_le(bytes, i * 4).ok())
            .collect(),
        header_u32_be: bytes[..bytes.len().min(32)]
            .chunks_exact(4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            .collect(),
        confirmed_fields: if valid {
            vec![
                "FS2 magic, secondary magic and constant trailer".into(),
                "512-byte blocks; 256-byte payloads".into(),
                "little-endian block numbers, counts and memory addresses".into(),
            ]
        } else {
            vec![]
        },
        unknown_fields: vec![
            "selection semantics for other filesystem variants".into(),
            "firmware and model compatibility".into(),
            "calibration and sensor fields".into(),
            "integrity of firmware and fields outside the observed filesystem CRC coverage".into(),
            "timezone and absent-coordinate semantics".into(),
        ],
        scientific_export_requires_reference: true,
    }
}

pub fn inspect_text(analysis: &Analysis) -> String {
    let filesystem=analysis.filesystem.as_ref().map(|f|format!("Filesystem: {}\nActive works: {:?}; active files: {:?}; sector CRCs checked: {:?}\n",f.validation,f.active_work_directories,f.active_files,f.crc_verified_sectors)).unwrap_or_default();
    format!(
        "Size: {} bytes\nSHA-256: {}\nFS2 signatures: {}\n512-byte segments: {} (remainder {})\nRecognized payload regions: {}\nReconstructed memory: {:?}\nValidation: {}\n{filesystem}Confirmed: {}\nUnknown: {}\nScientific export: active works use structural validation; exact-file reference profiles add external comparison\n",
        analysis.size_bytes,
        analysis.sha256,
        analysis.fs2_signature_occurrences,
        analysis.size_bytes / BLOCK_SIZE,
        analysis.size_bytes % BLOCK_SIZE,
        analysis.regions.len(),
        analysis.decoded_memory_bytes,
        analysis.structural_validation,
        analysis.confirmed_fields.join("; "),
        analysis.unknown_fields.join("; ")
    )
}
