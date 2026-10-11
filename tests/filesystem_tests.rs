mod common;
use fc2_rs::{
    filesystem::{self, PARTITION_ADDRESS},
    parser,
};

fn sector(memory: &mut [u8], number: usize, logical: u16, kind: u8, next: u16, used: u16) {
    let start = PARTITION_ADDRESS + number * 512;
    let b = &mut memory[start..start + 512];
    b.fill(255);
    b[0] = 0x45;
    b[16..18].copy_from_slice(&logical.to_le_bytes());
    b[18] = 1;
    b[20] = kind;
    b[21..23].copy_from_slice(&next.to_le_bytes());
    b[23..25].copy_from_slice(&used.to_le_bytes());
}
fn finish(memory: &mut [u8]) {
    for b in memory[PARTITION_ADDRESS..].chunks_exact_mut(512) {
        if [5, 0x45].contains(&b[0]) {
            b[19] = filesystem::sector_crc(b).unwrap();
        }
    }
}
fn fixture() -> Vec<u8> {
    let mut m = vec![255; 0x100000];
    sector(&mut m, 0, 0, b'S', 65535, 65535);
    m[PARTITION_ADDRESS + 20..PARTITION_ADDRESS + 27].copy_from_slice(b"SMRT\x01\x10\x00");
    sector(&mut m, 1, 3, 1, 65535, 65535);
    let p = PARTITION_ADDRESS + 512 + 25;
    m[p..p + 2].copy_from_slice(&0x5fb6u16.to_le_bytes());
    m[p + 2..p + 4].copy_from_slice(&12u16.to_le_bytes());
    m[p + 4..p + 8].copy_from_slice(&42u32.to_le_bytes());
    m[p + 8..p + 24].fill(0);
    m[p + 8..p + 12].copy_from_slice(b"data");
    sector(&mut m, 2, 12, 0, 13, 487);
    m[PARTITION_ADDRESS + 2 * 512 + 25..PARTITION_ADDRESS + 3 * 512].fill(0x22);
    sector(&mut m, 3, 13, 0, 65535, 48);
    m[PARTITION_ADDRESS + 3 * 512 + 25..PARTITION_ADDRESS + 3 * 512 + 73]
        .copy_from_slice(&common::raw_record());
    // A released historical copy must not create a duplicate logical sector.
    sector(&mut m, 4, 12, 0, 65535, 1);
    m[PARTITION_ADDRESS + 4 * 512] = 5;
    finish(&mut m);
    m
}
fn inspect(memory: &[u8]) -> filesystem::Snapshot {
    filesystem::inspect(&parser::parse(&common::wrap(memory)).unwrap()).unwrap()
}
#[test]
fn active_files_follow_links_and_keep_every_source_byte() {
    let m = fixture();
    let s = inspect(&m);
    assert_eq!(s.active_sector_count, 4);
    assert_eq!(s.released_sector_count, 1);
    assert_eq!(s.crc_verified_count, 5);
    assert!(s.unreferenced_active_sectors.is_empty());
    let f = &s.files[0];
    assert_eq!(f.path, "/data");
    assert_eq!(f.bytes.len(), 535);
    assert_eq!(
        f.sectors.iter().map(|s| s.logical).collect::<Vec<_>>(),
        vec![12, 13]
    );
    for (i, address) in f.addresses.iter().enumerate() {
        assert_eq!(m[*address], f.bytes[i]);
    }
    assert_eq!(&f.bytes[487..], &common::raw_record());
}
#[test]
fn crc_damage_duplicate_live_sectors_cycles_and_missing_links_are_rejected() {
    for kind in 0..4 {
        let mut m = fixture();
        match kind {
            0 => m[PARTITION_ADDRESS + 2 * 512 + 100] ^= 1,
            1 => m[PARTITION_ADDRESS + 4 * 512] = 0x45,
            2 => m[PARTITION_ADDRESS + 3 * 512 + 21..PARTITION_ADDRESS + 3 * 512 + 23]
                .copy_from_slice(&12u16.to_le_bytes()),
            _ => m[PARTITION_ADDRESS + 3 * 512 + 21..PARTITION_ADDRESS + 3 * 512 + 23]
                .copy_from_slice(&99u16.to_le_bytes()),
        }
        if kind != 0 {
            finish(&mut m);
        }
        assert!(filesystem::inspect(&parser::parse(&common::wrap(&m)).unwrap()).is_err());
    }
}
#[test]
fn crc_does_not_pretend_to_protect_custom_header_logical_or_sequence() {
    let mut b = [255u8; 512];
    b[0] = 0x45;
    let crc = filesystem::sector_crc(&b).unwrap();
    b[16] = 12;
    b[18] = 7;
    assert_eq!(filesystem::sector_crc(&b).unwrap(), crc);
    b[0] = 5;
    assert_eq!(filesystem::sector_crc(&b).unwrap(), crc);
    b[20] ^= 1;
    assert_ne!(filesystem::sector_crc(&b).unwrap(), crc);
    assert!(filesystem::sector_crc(&[]).is_err());
}
#[test]
fn unsupported_geometry_and_bad_names_never_panic() {
    let small = parser::parse(&common::wrap(&[255; 512])).unwrap();
    assert!(filesystem::inspect(&small).is_err());
    for name in [b"../x", b"a\\bc", b"a/bc"] {
        let mut m = fixture();
        let p = PARTITION_ADDRESS + 512 + 25 + 8;
        m[p..p + 16].fill(0);
        m[p..p + 4].copy_from_slice(name);
        finish(&mut m);
        assert!(filesystem::inspect(&parser::parse(&common::wrap(&m)).unwrap()).is_err());
    }
}

#[test]
fn directory_file_type_confusion_and_pending_deletion_are_rejected() {
    for kind in 0..3 {
        let mut memory = fixture();
        match kind {
            0 => memory[PARTITION_ADDRESS + 2 * 512 + 20] = 1,
            1 => memory[PARTITION_ADDRESS + 512 + 20] = 2,
            _ => {
                let p = PARTITION_ADDRESS + 512 + 25;
                memory[p..p + 2].copy_from_slice(&0x4fb6u16.to_le_bytes());
            }
        }
        finish(&mut memory);
        assert!(filesystem::inspect(&parser::parse(&common::wrap(&memory)).unwrap()).is_err());
    }
}

#[test]
fn empty_files_inactive_entries_timestamps_and_unused_tail_remain_explicit() {
    let mut memory = fixture();
    let p = PARTITION_ADDRESS + 512 + 25;
    memory[p + 4..p + 8].copy_from_slice(&u32::MAX.to_le_bytes());
    let tail = PARTITION_ADDRESS + 3 * 512 + 73;
    memory[tail..tail + 48].copy_from_slice(&common::raw_record());
    finish(&mut memory);
    let snapshot = inspect(&memory);
    assert_eq!(snapshot.files[0].bytes.len(), 535);
    assert_eq!(snapshot.directories[0].entries[0].timestamp_raw, u32::MAX);
    // A valid-looking record past `used` is never appended to a file.
    assert!(!snapshot.files[0].addresses.contains(&tail));
    // Empty files have no invented zero-filled content.
    memory[p + 2..p + 4].copy_from_slice(&u16::MAX.to_le_bytes());
    finish(&mut memory);
    let snapshot = inspect(&memory);
    assert!(snapshot.files[0].bytes.is_empty());
    assert_eq!(snapshot.unreferenced_active_sectors, vec![12, 13]);
    // Inactive entries retain metadata and do not traverse their links.
    memory[p..p + 2].copy_from_slice(&0x1fb6u16.to_le_bytes());
    finish(&mut memory);
    let snapshot = inspect(&memory);
    assert!(snapshot.files.is_empty());
    assert!(!snapshot.directories[0].entries[0].active);
}

#[test]
fn valid_crc_mutations_of_one_mib_image_remain_panic_free() {
    let original = fixture();
    let mut image = parser::parse(&common::wrap(&original)).unwrap();
    let mut offsets = Vec::new();
    for sector in 0..5 {
        for offset in [
            0, 1, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 32, 33, 34, 48, 49, 511,
        ] {
            offsets.push(PARTITION_ADDRESS + sector * 512 + offset);
        }
    }
    for offset in offsets {
        for value in [0, 1, 5, 0x45, 255] {
            image.bytes.clone_from(&original);
            image.bytes[offset] = value;
            // Recompute CRC to exercise structural checks beyond CRC rejection.
            finish(&mut image.bytes);
            let result = std::panic::catch_unwind(|| filesystem::inspect(&image));
            assert!(
                result.is_ok(),
                "panic at memory {offset:x}, value {value:x}"
            );
        }
    }
}
