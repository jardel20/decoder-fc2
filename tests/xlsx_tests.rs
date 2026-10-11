mod common;
use fc2_rs::{
    export::{
        self,
        xlsx::{self, WorkMetadata, WorkbookRow},
    },
    validator,
};
use std::collections::BTreeMap;

// An independent, minimal stored-ZIP reader used only on generated test files.
fn unpack(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut entries = BTreeMap::new();
    let mut position = 0;
    while &bytes[position..position + 4] == b"PK\x03\x04" {
        let u16_at =
            |offset| u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap()) as usize;
        let u32_at =
            |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        assert_eq!(u16_at(position + 8), 0);
        let size = u32_at(position + 18);
        assert_eq!(size, u32_at(position + 22));
        let name_length = u16_at(position + 26);
        let data_start = position + 30 + name_length + u16_at(position + 28);
        let name =
            String::from_utf8(bytes[position + 30..position + 30 + name_length].to_vec()).unwrap();
        let data = bytes[data_start..data_start + size].to_vec();
        let mut crc = u32::MAX;
        for &b in &data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
            }
        }
        assert_eq!(!crc, u32_at(position + 14) as u32);
        assert!(entries.insert(name, data).is_none());
        position = data_start + size;
    }
    assert_eq!(&bytes[position..position + 4], b"PK\x01\x02");
    assert_eq!(&bytes[bytes.len() - 22..bytes.len() - 18], b"PK\x05\x06");
    entries
}
fn xml(parts: &BTreeMap<String, Vec<u8>>, name: &str) -> String {
    String::from_utf8(parts[name].clone()).unwrap()
}
fn metadata() -> WorkMetadata {
    WorkMetadata {
        work: "TRABALHO A&B".into(),
        source_sha256: "a".repeat(64),
        reference_point: None,
        generated_at: None,
        validation: "filesystem_selected".into(),
    }
}
fn row() -> WorkbookRow {
    WorkbookRow {
        id: 7,
        point: 10,
        date: Some("2024-03-12".into()),
        time: Some("04:05:06".into()),
        latitude: Some(-15.25),
        longitude: Some(-47.75),
        chlorophyll_a_tenths: 212,
        chlorophyll_b_tenths: 76,
        chlorophyll_total_tenths: 289,
    }
}

#[test]
fn xlsx_has_example_layout_and_preserves_complete_coordinates_and_independent_total() {
    let bytes = xlsx::render_work(&metadata(), &[row()]).unwrap();
    assert_eq!(bytes, xlsx::render_work(&metadata(), &[row()]).unwrap());
    let parts = unpack(&bytes);
    assert_eq!(parts.len(), 7);
    let sheet = xml(&parts, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("TRABALHO A&amp;B"));
    assert!(sheet.contains("r=\"B8\""));
    assert!(sheet.contains("Clorofila Total"));
    assert!(sheet.contains("r=\"F9\" s=\"1\"><v>-15.25</v>"));
    assert!(sheet.contains("r=\"J9\" s=\"2\"><v>28.9</v>"));
    assert!(sheet.contains("12-03-2024"));
    assert!(!sheet.contains("<f>"));
    assert!(!sheet.contains("Falker Leaf"));
    assert!(!sheet.contains("r=\"B7\""));
    assert!(xml(&parts, "xl/styles.xml").contains("formatCode=\"0.000000\""));
    assert!(xml(&parts, "docProps/custom.xml").contains(&"a".repeat(64)));
}
#[test]
fn missing_fields_are_blank_and_real_zero_is_preserved() {
    let mut row = row();
    row.date = None;
    row.time = None;
    row.latitude = None;
    row.longitude = Some(0.0);
    let parts = unpack(&xlsx::render_work(&metadata(), &[row]).unwrap());
    let sheet = xml(&parts, "xl/worksheets/sheet1.xml");
    for cell in ["D9", "E9", "F9"] {
        assert!(!sheet.contains(&format!("r=\"{cell}\"")));
    }
    assert!(sheet.contains("r=\"G9\" s=\"1\"><v>0</v>"));
    let parts = unpack(&xlsx::render_work(&metadata(), &[]).unwrap());
    assert!(xml(&parts, "xl/worksheets/sheet1.xml").contains("dimension ref=\"A1:J8\""));
}
#[test]
fn bad_coordinates_xml_and_zip_paths_are_rejected() {
    let mut invalid = row();
    invalid.latitude = Some(f64::NAN);
    assert!(xlsx::render_work(&metadata(), &[invalid]).is_err());
    let mut metadata = metadata();
    metadata.work.push('\0');
    assert!(xlsx::render_work(&metadata, &[row()]).is_err());
    for name in [
        "../data.xlsx",
        "/data.xlsx",
        "data\\file.xlsx",
        "a//b.xlsx",
        "C:/data.xlsx",
        "a/./data.xlsx",
    ] {
        assert!(xlsx::archive_files(&[(name.into(), vec![])]).is_err());
    }
    assert!(xlsx::archive_files(&[("a.xlsx".into(), vec![]), ("a.xlsx".into(), vec![])]).is_err());
    let parts = unpack(&xlsx::archive_files(&[("ALPHA.xlsx".into(), vec![0, 1, 2])]).unwrap());
    assert_eq!(parts["ALPHA.xlsx"], vec![0, 1, 2]);
}
#[test]
fn reference_export_reuses_scientific_validation_gate() {
    let (bytes, reference) = common::fixture();
    let (report, records) = validator::validate(&bytes, &reference).unwrap();
    let mut document = export::document("synthetic.FC2".into(), report, records, true);
    assert!(xlsx::render(&document).is_ok());
    document.validation.accepted = false;
    assert!(xlsx::render(&document).is_err());
}

#[test]
fn separate_measurement_groups_keep_point_ids_without_merging() {
    let mut second = row();
    second.id = 8;
    second.point = 10;
    let sheets = [
        xlsx::WorkbookSheet {
            name: "Medidas 1".into(),
            rows: vec![row()],
        },
        xlsx::WorkbookSheet {
            name: "Medidas 2".into(),
            rows: vec![second],
        },
    ];
    let bytes = xlsx::render_sheets(&metadata(), &sheets).unwrap();
    let parts = unpack(&bytes);
    let workbook = xml(&parts, "xl/workbook.xml");
    assert!(workbook.contains("name=\"Medidas 1\""));
    assert!(workbook.contains("name=\"Medidas 2\""));
    assert!(xml(&parts, "xl/worksheets/sheet1.xml").contains("r=\"B9\" s=\"0\"><v>7</v>"));
    assert!(xml(&parts, "xl/worksheets/sheet2.xml").contains("r=\"B9\" s=\"0\"><v>8</v>"));
    assert!(xml(&parts, "xl/_rels/workbook.xml.rels").contains("Target=\"worksheets/sheet2.xml\""));
    assert!(xml(&parts, "[Content_Types].xml").contains("/xl/worksheets/sheet2.xml"));
    let parts = unpack(&xlsx::render_sheets(&metadata(), &sheets[..1]).unwrap());
    assert!(xml(&parts, "xl/workbook.xml").contains("name=\"Worksheet\""));
    assert!(xlsx::render_sheets(&metadata(), &[]).is_err());
    let mut duplicate = sheets.clone();
    duplicate[1].name = "medidas 1".into();
    assert!(xlsx::render_sheets(&metadata(), &duplicate).is_err());
}
