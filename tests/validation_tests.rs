mod common;
use fc2_rs::{
    decoder, export, reader,
    validator::{self, Profile},
};

#[test]
fn exact_reference_fields_and_independent_total() {
    let (bytes, reference) = common::fixture();
    let (report, records) = validator::validate(&bytes, &reference).unwrap();
    assert!(report.accepted);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].chlorophyll_total_index, "16.9");
    assert_eq!(records[0].chlorophyll_a_index, "12.3");
    assert_eq!(records[0].chlorophyll_b_index, "4.5");
    assert!(records[0].timezone.is_none());
}
#[test]
fn reference_hash_duplicates_and_wrong_fields_block_validation() {
    let (bytes, mut reference) = common::fixture();
    reference.rows[0].chlorophyll_a_tenths += 1;
    assert!(!validator::validate(&bytes, &reference).unwrap().0.accepted);
    reference.rows.push(reference.rows[0].clone());
    assert!(validator::validate(&bytes, &reference).is_err());
    let (_, mut reference) = common::fixture();
    reference.source_fc2_sha256 = reader::hash(b"other");
    assert!(validator::validate(&bytes, &reference).is_err());
}
#[test]
fn coordinate_tolerance_is_limited_and_nonfinite_is_rejected() {
    let (bytes, mut reference) = common::fixture();
    reference.rows[0].latitude += 0.0000004;
    assert!(validator::validate(&bytes, &reference).unwrap().0.accepted);
    reference.rows[0].latitude += 0.0000002;
    assert!(!validator::validate(&bytes, &reference).unwrap().0.accepted);
    let mut raw = common::raw_record();
    raw[40..44].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(decoder::fields(&raw).is_err());
    raw = common::raw_record();
    raw[3] = 31;
    raw[4] = 2;
    assert!(decoder::fields(&raw).is_err());
}
#[test]
fn records_cross_payload_pages_and_linked_sectors() {
    for start in [248_usize, 470, 484] {
        let mut memory = vec![255; 1024];
        common::sector(&mut memory, 0, 1, 2, 487);
        common::sector(&mut memory, 512, 2, 65535, 48);
        let raw = common::raw_record();
        let in_first = (512 - start).min(48);
        memory[start..start + in_first].copy_from_slice(&raw[..in_first]);
        if in_first < 48 {
            memory[537..537 + 48 - in_first].copy_from_slice(&raw[in_first..]);
        }
        let bytes = common::wrap(&memory);
        let reference = common::reference(&bytes);
        let (report, records) = validator::validate(&bytes, &reference).unwrap();
        assert!(report.accepted, "start={start}");
        assert!(records[0].occurrences[0].segments.len() > 1);
        let recovered: Vec<_> = records[0].occurrences[0]
            .segments
            .iter()
            .flat_map(|s| {
                bytes[s.file_offset..s.file_offset + s.length]
                    .iter()
                    .copied()
            })
            .collect();
        assert_eq!(recovered, raw);
        assert_eq!(records[0].occurrences[0].raw_sha256, reader::hash(&raw));
    }
}
#[test]
fn duplicate_copies_retained_and_unknown_raw_conflicts_block_profile() {
    let mut memory = vec![255; 1024];
    for s in [0, 512] {
        common::sector(&mut memory, s, 1, 65535, 48);
        memory[s + 25..s + 73].copy_from_slice(&common::raw_record());
    }
    let bytes = common::wrap(&memory);
    let reference = common::reference(&bytes);
    let (report, records) = validator::validate(&bytes, &reference).unwrap();
    assert!(report.accepted);
    assert_eq!(records[0].occurrences.len(), 2);
    memory[512 + 25 + 6] = 1;
    let bytes = common::wrap(&memory);
    let reference = common::reference(&bytes);
    let report = validator::validate(&bytes, &reference).unwrap().0;
    assert!(!report.accepted);
    assert_eq!(report.rows[0].raw_variants, 2);
}
#[test]
fn profile_is_revalidated_and_exports_are_reproducible() {
    let (bytes, reference) = common::fixture();
    let (validation, records) = validator::validate(&bytes, &reference).unwrap();
    let profile = Profile {
        schema: "fc2-validation-profile-v1".into(),
        layout: decoder::LAYOUT.into(),
        tool_version: "test".into(),
        reference,
        validation,
    };
    let (report, _) = validator::decode(&bytes, &profile).unwrap();
    let doc = export::document("original.FC2".into(), report, records, true);
    let json = export::json::encode(&doc).unwrap();
    assert_eq!(json, export::json::encode(&doc).unwrap());
    let decoded: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(decoded["records"][0]["chlorophyll_total_index"], "16.9");
    let csv = String::from_utf8(export::csv::encode(&doc).unwrap()).unwrap();
    assert!(csv.contains(",12.3,4.5,16.9,"));
    assert_eq!(csv.lines().count(), 2);
    let mut wrong = bytes.clone();
    wrong[100] ^= 1;
    assert!(validator::decode(&wrong, &profile).is_err());
}
#[test]
fn complete_writes_never_overwrite_existing_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.FC2");
    std::fs::write(&path, b"original").unwrap();
    assert!(export::write_new(&path, b"new").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    let output = dir.path().join("out.json");
    export::write_new(&output, b"complete").unwrap();
    assert_eq!(std::fs::read(output).unwrap(), b"complete");
}

#[test]
fn rejected_reports_cannot_be_exported_from_the_library() {
    let (bytes, mut reference) = common::fixture();
    reference.rows[0].id = 999;
    let (report, records) = validator::validate(&bytes, &reference).unwrap();
    let doc = export::document("input.FC2".into(), report, records, false);
    assert!(export::json::encode(&doc).is_err());
    assert!(export::csv::encode(&doc).is_err());
}
