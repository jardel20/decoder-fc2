mod common;
use fc2_rs::{analysis, parser};

#[test]
fn empty_truncated_and_incorrect_signatures_are_rejected() {
    assert!(parser::parse(&[]).is_err());
    let (bytes, _) = common::fixture();
    for length in [1, 511, 513, bytes.len() - 1] {
        assert!(parser::parse(&bytes[..length]).is_err());
    }
    for offset in [0, 4, 508] {
        let mut bad = bytes.clone();
        bad[offset] ^= 1;
        assert!(parser::parse(&bad).is_err());
    }
}
#[test]
fn size_count_duplicates_overlap_and_padding_are_checked() {
    let (bytes, _) = common::fixture();
    for (offset, value) in [(8, 1_u32), (12, 256), (16, 255), (20, 2), (24, 4), (28, 1)] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(parser::parse(&bad).is_err());
    }
    let mut duplicate = bytes.clone();
    let first = duplicate[..512].to_vec();
    duplicate[512..].copy_from_slice(&first);
    assert!(parser::parse(&duplicate).is_err());
    let mut padding = bytes.clone();
    padding[288] = 1;
    assert!(parser::parse(&padding).is_err());
}
#[test]
fn physical_block_order_and_mapping_are_preserved() {
    let (bytes, _) = common::fixture();
    let swapped = [&bytes[512..], &bytes[..512]].concat();
    let a = parser::parse(&bytes).unwrap();
    let b = parser::parse(&swapped).unwrap();
    assert_eq!(a.bytes, b.bytes);
    assert_eq!(b.source_offset(25).unwrap(), 512 + 32 + 25);
    assert!(b.source_offset(512).is_err());
}
#[test]
fn arbitrary_and_mutated_bytes_never_panic() {
    let mut seed = 0x12345678_u32;
    for len in [0, 1, 4, 31, 256, 511, 512, 513, 1024, 2048] {
        for _ in 0..32 {
            let data: Vec<_> = (0..len)
                .map(|_| {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    seed as u8
                })
                .collect();
            assert!(
                std::panic::catch_unwind(|| {
                    let _ = parser::parse(&data);
                    let _ = analysis::analyze(&data);
                })
                .is_ok()
            );
        }
    }
    let (valid, _) = common::fixture();
    for i in 0..valid.len() {
        let mut mutated = valid.clone();
        mutated[i] ^= 255;
        assert!(std::panic::catch_unwind(|| parser::parse(&mutated)).is_ok());
    }
}
