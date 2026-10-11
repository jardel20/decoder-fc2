#![allow(dead_code)]
use fc2_rs::{
    blocks::*,
    reader::hash,
    records::{Reference, ReferenceRow},
};

pub fn wrap(memory: &[u8]) -> Vec<u8> {
    assert_eq!(memory.len() % 256, 0);
    let count = memory.len() / 256;
    let mut out = vec![0; count * 512];
    for i in 0..count {
        let block = &mut out[i * 512..i * 512 + 512];
        block[..4].copy_from_slice(&MAGIC);
        for (offset, value) in [
            (4, MAGIC2),
            (12, (i * 256) as u32),
            (16, 256),
            (20, i as u32),
            (24, count as u32),
            (508, TRAILER),
        ] {
            block[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        block[32..288].copy_from_slice(&memory[i * 256..i * 256 + 256]);
    }
    out
}

pub fn raw_record() -> [u8; 48] {
    let mut b = [0; 48];
    b[..5].copy_from_slice(&[1, 2, 3, 4, 5]);
    for (offset, value) in [
        (24, 123_u16),
        (26, 45),
        (28, 169),
        (30, 1),
        (32, 26),
        (34, 650),
    ] {
        b[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    b[36..40].copy_from_slice(&42_u32.to_le_bytes());
    b[40..44].copy_from_slice(&(-12.25_f32).to_le_bytes());
    b[44..48].copy_from_slice(&34.5_f32.to_le_bytes());
    b
}

pub fn sector(memory: &mut [u8], start: usize, logical: u16, next: u16, used: u16) {
    memory[start..start + 512].fill(255);
    memory[start] = 5;
    memory[start + 16..start + 18].copy_from_slice(&logical.to_le_bytes());
    memory[start + 21..start + 23].copy_from_slice(&next.to_le_bytes());
    memory[start + 23..start + 25].copy_from_slice(&used.to_le_bytes());
}

pub fn fixture() -> (Vec<u8>, Reference) {
    let mut m = vec![255; 512];
    sector(&mut m, 0, 1, 65535, 48);
    m[25..73].copy_from_slice(&raw_record());
    let bytes = wrap(&m);
    let reference = reference(&bytes);
    (bytes, reference)
}

pub fn reference(bytes: &[u8]) -> Reference {
    Reference {
        schema: "fc2-reference-v1".into(),
        source_fc2_sha256: hash(bytes),
        source_reference_sha256: hash(b"synthetic reference"),
        source_reference_name: "synthetic.json".into(),
        work: "SYNTHETIC".into(),
        confirmed_same_source: true,
        rows: vec![ReferenceRow {
            id: 42,
            point: 1,
            date: "2024-03-12".into(),
            time: "03:02:01".into(),
            latitude: -12.25,
            longitude: 34.5,
            chlorophyll_a_tenths: 123,
            chlorophyll_b_tenths: 45,
            chlorophyll_total_tenths: 169,
        }],
    }
}
