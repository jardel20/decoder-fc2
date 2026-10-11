use fc2_rs::{records::Segment, work_records::decode48};

fn segments() -> Vec<Segment> {
    vec![Segment {
        file_offset: 1024,
        memory_address: 512,
        length: 48,
    }]
}
fn raw(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn synthetic_record(
    id: u32,
    point: u16,
    calendar: [u8; 5],
    year: u16,
    indices: [u16; 3],
    coordinates: [f32; 2],
) -> Vec<u8> {
    let mut bytes = vec![0; 48];
    bytes[..5].copy_from_slice(&calendar);
    for (offset, value) in [
        (24, indices[0]),
        (26, indices[1]),
        (28, indices[2]),
        (30, point),
        (32, year),
    ] {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    bytes[36..40].copy_from_slice(&id.to_le_bytes());
    for (offset, value) in [(40, coordinates[0]), (44, coordinates[1])] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[test]
fn synthetic_record_decodes_fields_and_float_bits() {
    let r = synthetic_record(
        7001,
        3,
        [6, 5, 4, 12, 3],
        24,
        [123, 45, 180],
        [-15.25, -47.75],
    );
    let measurement = decode48(&r, segments()).unwrap();
    assert_eq!(measurement.id, 7001);
    assert_eq!(measurement.point, 3);
    assert_eq!(measurement.date.as_deref(), Some("2024-03-12"));
    assert_eq!(measurement.time.as_deref(), Some("04:05:06"));
    assert_eq!(measurement.chlorophyll_a_index, "12.3");
    assert_eq!(measurement.chlorophyll_b_index, "4.5");
    assert_eq!(measurement.chlorophyll_total_index, "18.0");
    assert_eq!(measurement.latitude, Some(-15.25));
    assert!(measurement.warnings.is_empty());
}

#[test]
fn all_zero_calendar_and_nan_coordinates_are_preserved_without_imputation() {
    let r = synthetic_record(7002, 21, [0; 5], 0, [145, 40, 185], [f32::NAN; 2]);
    let measurement = decode48(&r, segments()).unwrap();
    assert_eq!(measurement.id, 7002);
    assert_eq!(measurement.point, 21);
    assert_eq!(measurement.chlorophyll_total_index, "18.5");
    assert!(measurement.date.is_none() && measurement.time.is_none());
    assert!(measurement.latitude.is_none() && measurement.longitude.is_none());
    assert_eq!(measurement.latitude_bits, 0x7fc00000);
    assert_eq!(measurement.longitude_bits, 0x7fc00000);
    assert_eq!(
        measurement.raw_hex,
        r.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    let json = serde_json::to_value(measurement).unwrap();
    assert!(json["latitude"].is_null());
    assert_eq!(json["raw_calendar"]["year"], 0);
}

#[test]
fn malformed_calendar_and_coordinate_do_not_discard_the_record() {
    let mut r = synthetic_record(
        7001,
        3,
        [6, 5, 4, 12, 3],
        24,
        [123, 45, 180],
        [-15.25, -47.75],
    );
    r[3] = 31;
    r[4] = 2;
    r[40..44].copy_from_slice(&100.0_f32.to_le_bytes());
    let measurement = decode48(&r, segments()).unwrap();
    assert_eq!(measurement.id, 7001);
    assert!(measurement.date.is_none());
    assert!(measurement.latitude.is_none());
    assert_eq!(measurement.latitude_bits, 100.0_f32.to_bits());
    assert_eq!(measurement.warnings.len(), 2);
}

#[test]
fn truncated_records_and_invalid_source_mapping_are_errors() {
    assert!(decode48(&[0; 47], segments()).is_err());
    assert!(decode48(&[0; 48], Vec::new()).is_err());
    let mut mapping = segments();
    mapping[0].file_offset = usize::MAX;
    assert!(decode48(&[0; 48], mapping).is_err());
}

#[test]
fn source_verification_preserves_signed_zero_and_absent_nonfinite_coordinates() {
    for bits in [0x80000000u32, 0x7fc00000] {
        let mut record_bytes = vec![0; 48];
        record_bytes[40..44].copy_from_slice(&bits.to_le_bytes());
        let mut memory = vec![255; 256];
        memory[..48].copy_from_slice(&record_bytes);
        let mut source = vec![0; 512];
        source[..4].copy_from_slice(&fc2_rs::blocks::MAGIC);
        for (offset, value) in [
            (4, fc2_rs::blocks::MAGIC2),
            (16, 256),
            (24, 1),
            (508, fc2_rs::blocks::TRAILER),
        ] {
            source[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        source[32..288].copy_from_slice(&memory);
        let image = fc2_rs::parser::parse(&source).unwrap();
        let mut measurement = decode48(
            &record_bytes,
            vec![Segment {
                file_offset: 32,
                memory_address: 0,
                length: 48,
            }],
        )
        .unwrap();
        fc2_rs::work_records::verify_source_record(&source, &image, &measurement).unwrap();
        measurement.latitude = if bits == 0x80000000 {
            Some(0.0)
        } else {
            Some(f64::NAN)
        };
        assert!(fc2_rs::work_records::verify_source_record(&source, &image, &measurement).is_err());
    }
}
