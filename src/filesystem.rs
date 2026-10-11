//! Read-only support for the observed aligned-header SMARTFS variant.
//! Sector selection uses committed/not-released flags, never record plausibility.
use crate::{
    error::{Error, Result},
    parser::Image,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const PARTITION_ADDRESS: usize = 0x80000;
pub const SECTOR_SIZE: usize = 512;
pub const DATA_OFFSET: usize = 25;
pub const DATA_CAPACITY: usize = 487;

#[derive(Clone, Debug, Serialize)]
pub struct FilesystemSector {
    pub logical: u16,
    pub memory_address: usize,
    pub source_offset: usize,
    pub status: u8,
    pub sequence: u8,
    pub crc: u8,
    pub crc_verified: bool,
    pub kind: u8,
    pub next: u16,
    pub used: u16,
}

#[derive(Clone, Debug, Serialize)]
pub struct DirectoryEntry {
    pub name: String,
    pub flags: u16,
    pub first_sector: u16,
    pub timestamp_raw: u32,
    pub memory_address: usize,
    pub source_offset: usize,
    pub active: bool,
    pub directory: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct FilesystemDirectory {
    pub path: String,
    pub first_sector: u16,
    pub sectors: Vec<FilesystemSector>,
    pub entries: Vec<DirectoryEntry>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FilesystemFile {
    pub path: String,
    pub first_sector: u16,
    pub bytes: Vec<u8>,
    /// One reconstructed-memory address for every byte of `bytes`.
    pub addresses: Vec<usize>,
    pub sectors: Vec<FilesystemSector>,
}

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub layout: String,
    pub partition_address: usize,
    pub partition_length: usize,
    pub name_size: usize,
    pub active_sector_count: usize,
    pub released_sector_count: usize,
    pub erased_sector_count: usize,
    pub crc_verified_count: usize,
    pub files: Vec<FilesystemFile>,
    pub directories: Vec<FilesystemDirectory>,
    pub sectors: Vec<FilesystemSector>,
    pub unreferenced_active_sectors: Vec<u16>,
    pub limitations: Vec<String>,
}

fn word(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}

/// Observed CRC-8: reflected polynomial 0xb2, initial 0xff, xor-out 0xff.
/// The customized firmware covers the body, then the first three header bytes,
/// then status. Header offsets 16..19 (logical and sequence) are not covered.
/// Released status is normalized to its original committed/not-released state.
pub fn sector_crc(bytes: &[u8]) -> Result<u8> {
    if bytes.len() != SECTOR_SIZE {
        return Err(Error::Format(
            "CRC requires one complete 512-byte sector".into(),
        ));
    }
    let status = bytes[0] | 0x40;
    let suffix = [status, bytes[1], bytes[2], status];
    let mut crc = 0xffu8;
    for byte in bytes[20..].iter().chain(suffix.iter()) {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xb2
            } else {
                crc >> 1
            };
        }
    }
    Ok(!crc)
}

struct Walker<'a> {
    image: &'a Image,
    active: BTreeMap<u16, FilesystemSector>,
    claimed: BTreeSet<u16>,
    files: Vec<FilesystemFile>,
    directories: Vec<FilesystemDirectory>,
}
impl Walker<'_> {
    fn chain(&mut self, first: u16) -> Result<Vec<FilesystemSector>> {
        let mut chain = Vec::new();
        let mut next = first;
        while next != u16::MAX {
            if !self.claimed.insert(next) {
                return Err(Error::Format(format!(
                    "cycle or cross-linked active logical sector {next}"
                )));
            }
            let sector =
                self.active.get(&next).cloned().ok_or_else(|| {
                    Error::Format(format!("missing active logical sector {next}"))
                })?;
            next = sector.next;
            chain.push(sector);
        }
        Ok(chain)
    }
    fn file(&mut self, first: u16, path: String) -> Result<()> {
        let sectors = self.chain(first)?;
        let mut bytes = Vec::new();
        let mut addresses = Vec::new();
        for sector in &sectors {
            if ![0, 2, 255].contains(&sector.kind) {
                return Err(Error::Format(format!(
                    "{path}: unsupported file sector type {} at logical {}",
                    sector.kind, sector.logical
                )));
            }
            let used = usize::from(sector.used);
            if used > DATA_CAPACITY {
                return Err(Error::Format(format!(
                    "{path}: used length {used} exceeds 487 at logical {}",
                    sector.logical
                )));
            }
            let start = sector.memory_address + DATA_OFFSET;
            bytes.extend_from_slice(&self.image.bytes[start..start + used]);
            addresses.extend(start..start + used);
        }
        self.files.push(FilesystemFile {
            path,
            first_sector: first,
            bytes,
            addresses,
            sectors,
        });
        Ok(())
    }
    fn directory(&mut self, first: u16, path: String, depth: usize) -> Result<()> {
        if depth > 32 {
            return Err(Error::Format("directory nesting exceeds 32".into()));
        }
        let sectors = self.chain(first)?;
        if sectors.first().is_none_or(|s| s.kind != 1) {
            return Err(Error::Format(format!(
                "{path}: unsupported initial directory sector type"
            )));
        }
        let mut entries = Vec::new();
        let mut names = BTreeSet::new();
        for sector in &sectors {
            if ![1, 255].contains(&sector.kind) {
                return Err(Error::Format(format!(
                    "{path}: unsupported directory continuation type {}",
                    sector.kind
                )));
            }
            let start = sector.memory_address;
            let bytes = &self.image.bytes[start..start + 512];
            for offset in (DATA_OFFSET..=512 - 24).step_by(24) {
                let flags = word(bytes, offset);
                if flags & 0x8000 != 0 {
                    continue;
                }
                let raw_name = &bytes[offset + 8..offset + 24];
                let end = raw_name.iter().position(|x| *x == 0).unwrap_or(16);
                let name = std::str::from_utf8(&raw_name[..end])
                    .map_err(|_| Error::Format("unsupported non-UTF8 directory name".into()))?
                    .to_owned();
                if name.is_empty()
                    || name.contains('/')
                    || name.contains('\\')
                    || name == "."
                    || name == ".."
                    || name.chars().any(char::is_control)
                {
                    return Err(Error::Format(
                        "unsafe or empty filesystem entry name".into(),
                    ));
                }
                let active = flags & 0x4000 != 0;
                if active && flags & 0x1000 == 0 {
                    return Err(Error::Format(format!(
                        "{path}/{name}: active entry has deletion-in-progress flag"
                    )));
                }
                if active && !names.insert(name.clone()) {
                    return Err(Error::Format(format!(
                        "duplicate active name {path}/{name}"
                    )));
                }
                entries.push(DirectoryEntry {
                    name,
                    flags,
                    first_sector: word(bytes, offset + 2),
                    timestamp_raw: u32::from_le_bytes(
                        bytes[offset + 4..offset + 8]
                            .try_into()
                            .map_err(|_| Error::Format("invalid timestamp bytes".into()))?,
                    ),
                    memory_address: start + offset,
                    source_offset: self.image.source_offset(start + offset)?,
                    active,
                    directory: flags & 0x2000 != 0,
                });
            }
        }
        for entry in entries.iter().filter(|e| e.active) {
            let child = format!("{path}/{}", entry.name);
            if entry.directory {
                self.directory(entry.first_sector, child, depth + 1)?;
            } else {
                self.file(entry.first_sector, child)?;
            }
        }
        self.directories.push(FilesystemDirectory {
            path,
            first_sector: first,
            sectors,
            entries,
        });
        Ok(())
    }
}

/// Require the observed 512 KiB partition and aligned 20-byte SMART header.
/// Other variants produce an unsupported-format error instead of a guess.
pub fn inspect(image: &Image) -> Result<Snapshot> {
    if image.bytes.len() != 0x100000 {
        return Err(Error::Format(
            "SMARTFS support currently requires the observed 1 MiB reconstructed image".into(),
        ));
    }
    let mut active = BTreeMap::new();
    let mut sectors = Vec::new();
    let mut released = 0;
    let mut erased = 0;
    for (number, bytes) in image.bytes[PARTITION_ADDRESS..]
        .chunks_exact(SECTOR_SIZE)
        .enumerate()
    {
        let address = PARTITION_ADDRESS + number * SECTOR_SIZE;
        if bytes.iter().all(|x| *x == 255) {
            erased += 1;
            continue;
        }
        if ![0x05, 0x45].contains(&bytes[0]) || bytes[1..16].iter().any(|x| *x != 255) {
            return Err(Error::Format(format!(
                "unsupported SMART header at memory 0x{address:x}"
            )));
        }
        if sector_crc(bytes)? != bytes[19] {
            return Err(Error::Format(format!(
                "SMART sector CRC mismatch at memory 0x{address:x}"
            )));
        }
        let sector = FilesystemSector {
            logical: word(bytes, 16),
            memory_address: address,
            source_offset: image.source_offset(address)?,
            status: bytes[0],
            sequence: bytes[18],
            crc: bytes[19],
            crc_verified: true,
            kind: bytes[20],
            next: word(bytes, 21),
            used: word(bytes, 23),
        };
        if sector.status == 0x45 {
            if sector.logical == u16::MAX || active.insert(sector.logical, sector.clone()).is_some()
            {
                return Err(Error::Format(format!(
                    "duplicate or invalid active logical sector {}",
                    sector.logical
                )));
            }
        } else {
            released += 1;
        }
        sectors.push(sector);
    }
    let format = active
        .get(&0)
        .ok_or_else(|| Error::Format("missing active SMART format sector zero".into()))?;
    let header = &image.bytes[format.memory_address + 20..format.memory_address + 27];
    if header != [b'S', b'M', b'R', b'T', 1, 16, 0] {
        return Err(Error::Format(
            "unsupported SMART format signature, version or name length".into(),
        ));
    }
    let active_count = active.len();
    let mut walker = Walker {
        image,
        active,
        claimed: BTreeSet::from([0]),
        files: Vec::new(),
        directories: Vec::new(),
    };
    walker.directory(3, String::new(), 0)?;
    walker.files.sort_by(|a, b| a.path.cmp(&b.path));
    walker.directories.sort_by(|a, b| a.path.cmp(&b.path));
    let unreferenced = walker
        .active
        .keys()
        .filter(|id| !walker.claimed.contains(id))
        .copied()
        .collect();
    Ok(Snapshot{layout:"aligned20-smartfs-crc8-v1".into(),partition_address:PARTITION_ADDRESS,partition_length:0x80000,name_size:16,active_sector_count:active_count,released_sector_count:released,erased_sector_count:erased,crc_verified_count:sectors.len(),files:walker.files,directories:walker.directories,sectors,unreferenced_active_sectors:unreferenced,limitations:vec!["Observed partition geometry only; other firmware layouts are unsupported.".into(),"CRC verifies payload, but the customized layout does not protect logical number or sequence.".into(),"Directory timestamps are preserved as raw u32; device clock and epoch are not validated.".into(),"Released sectors are preserved in the sector inventory, never exported as active file contents.".into()]})
}
