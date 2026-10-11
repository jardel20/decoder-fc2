//! Deterministic OOXML export matching the inspected Leaf table layout.
//! No proprietary workbook or experimental data is embedded in this module.
use super::{Export, ensure_validated};
use crate::{
    error::{Error, Result},
    records::decimal_tenths,
};
use std::fmt::Write;

#[derive(Clone, Debug)]
pub struct WorkMetadata {
    pub work: String,
    pub source_sha256: String,
    pub reference_point: Option<u16>,
    /// Optional caller-supplied export timestamp. Not a device measurement date.
    pub generated_at: Option<String>,
    pub validation: String,
}

#[derive(Clone, Debug)]
pub struct WorkbookRow {
    pub id: u32,
    pub point: u16,
    pub date: Option<String>,
    pub time: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub chlorophyll_a_tenths: u16,
    pub chlorophyll_b_tenths: u16,
    pub chlorophyll_total_tenths: u16,
}

#[derive(Clone, Debug)]
pub struct WorkbookSheet {
    pub name: String,
    pub rows: Vec<WorkbookRow>,
}

pub fn render(document: &Export) -> Result<Vec<u8>> {
    ensure_validated(document)?;
    let first = document
        .records
        .first()
        .ok_or_else(|| Error::Validation("empty XLSX selection".into()))?;
    if document
        .records
        .iter()
        .any(|r| r.work != first.work || r.validation != first.validation)
    {
        return Err(Error::Validation(
            "XLSX selection contains multiple works or validation states".into(),
        ));
    }
    let rows: Vec<_> = document
        .records
        .iter()
        .map(|r| WorkbookRow {
            id: r.id,
            point: r.point,
            date: Some(r.date.clone()),
            time: Some(r.time.clone()),
            latitude: Some(r.latitude),
            longitude: Some(r.longitude),
            chlorophyll_a_tenths: r.chlorophyll_a_tenths,
            chlorophyll_b_tenths: r.chlorophyll_b_tenths,
            chlorophyll_total_tenths: r.chlorophyll_total_tenths,
        })
        .collect();
    render_work(
        &WorkMetadata {
            work: first.work.clone(),
            source_sha256: document.source_sha256.clone(),
            reference_point: None,
            generated_at: None,
            validation: first.validation.clone(),
        },
        &rows,
    )
}

/// Export one accepted work; the caller owns filesystem/reference validation.
/// Preserve caller record order. Missing date/time/GPS cells remain absent.
pub fn render_work(metadata: &WorkMetadata, records: &[WorkbookRow]) -> Result<Vec<u8>> {
    archive_files(&work_parts(metadata, records)?)
}

/// Keep distinct measurement files on separate worksheets, preserving point IDs.
pub fn render_sheets(metadata: &WorkMetadata, sheets: &[WorkbookSheet]) -> Result<Vec<u8>> {
    if sheets.is_empty() || sheets.len() > 1000 {
        return Err(Error::Validation(
            "XLSX requires 1..1000 measurement sheets".into(),
        ));
    }
    let mut names = std::collections::HashSet::new();
    let mut entries = work_parts(metadata, &sheets[0].rows)?;
    let mut workbook = format!(
        "{XML}<workbook xmlns=\"{MAIN}\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets>"
    );
    let mut relationships = format!(
        "{XML}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">"
    );
    let mut content = CONTENT_TYPES.replace("</Types>", "");
    for (index, sheet) in sheets.iter().enumerate() {
        let name = if sheets.len() == 1 {
            "Worksheet"
        } else {
            &sheet.name
        };
        if name.is_empty()
            || name.chars().count() > 31
            || name.contains(['[', ']', ':', '*', '?', '/', '\\'])
            || name.starts_with('\'')
            || name.ends_with('\'')
            || !names.insert(name.to_lowercase())
        {
            return Err(Error::Validation(
                "invalid or duplicate XLSX worksheet name".into(),
            ));
        }
        let id = index + 1;
        write!(
            workbook,
            "<sheet name=\"{}\" sheetId=\"{id}\" r:id=\"rId{id}\"/>",
            escape(name)?
        )
        .unwrap();
        write!(relationships,"<Relationship Id=\"rId{id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{id}.xml\"/>").unwrap();
        if index > 0 {
            let sheet_xml = work_parts(metadata, &sheet.rows)?
                .into_iter()
                .find(|(name, _)| name == "xl/worksheets/sheet1.xml")
                .unwrap()
                .1;
            entries.push((format!("xl/worksheets/sheet{id}.xml"), sheet_xml));
            write!(content,"<Override PartName=\"/xl/worksheets/sheet{id}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>").unwrap();
        }
    }
    workbook.push_str("</sheets></workbook>");
    content.push_str("</Types>");
    write!(relationships,"<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>",sheets.len()+1).unwrap();
    for (name, bytes) in &mut entries {
        match name.as_str() {
            "xl/workbook.xml" => *bytes = workbook.as_bytes().to_vec(),
            "xl/_rels/workbook.xml.rels" => *bytes = relationships.as_bytes().to_vec(),
            "[Content_Types].xml" => *bytes = content.as_bytes().to_vec(),
            _ => {}
        }
    }
    archive_files(&entries)
}

fn work_parts(metadata: &WorkMetadata, records: &[WorkbookRow]) -> Result<Vec<(String, Vec<u8>)>> {
    if metadata.work.is_empty()
        || metadata.source_sha256.len() != 64
        || !metadata
            .source_sha256
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
        || metadata.validation.is_empty()
        || records.len() > 1_048_568
        || records.iter().any(|r| {
            r.latitude.is_some_and(|v| !v.is_finite())
                || r.longitude.is_some_and(|v| !v.is_finite())
        })
    {
        return Err(Error::Validation(
            "XLSX requires one accepted work and finite known coordinates".into(),
        ));
    }
    let mut sheet = format!(
        "{XML}<worksheet xmlns=\"{MAIN}\"><dimension ref=\"A1:J{}\"/><sheetViews><sheetView workbookViewId=\"0\" showGridLines=\"1\"/></sheetViews><sheetFormatPr defaultRowHeight=\"14.4\"/><cols><col min=\"1\" max=\"1\" width=\"29\" customWidth=\"1\"/><col min=\"2\" max=\"2\" width=\"15\" customWidth=\"1\"/><col min=\"3\" max=\"3\" width=\"10\" customWidth=\"1\"/><col min=\"4\" max=\"4\" width=\"13\" customWidth=\"1\"/><col min=\"5\" max=\"5\" width=\"14\" customWidth=\"1\"/><col min=\"6\" max=\"9\" width=\"16\" customWidth=\"1\"/><col min=\"10\" max=\"10\" width=\"20\" customWidth=\"1\"/></cols><sheetData>",
        records.len() + 8
    );
    for (row, label, value) in [
        (1, "FC2-RS", None),
        (2, "Arquivo .XLSX gerado com", None),
        (
            3,
            "Programa:",
            Some(format!("FC2-RS {}", env!("CARGO_PKG_VERSION"))),
        ),
        (
            4,
            "Este arquivo foi gerado em",
            metadata.generated_at.clone(),
        ),
        (6, "Trabalho:", Some(metadata.work.clone())),
    ] {
        write!(sheet, "<row r=\"{row}\">").unwrap();
        text(&mut sheet, &format!("A{row}"), label)?;
        if let Some(value) = value {
            text(&mut sheet, &format!("B{row}"), &value)?;
        }
        sheet.push_str("</row>");
    }
    sheet.push_str("<row r=\"7\">");
    text(&mut sheet, "A7", "Ponto de Referência:")?;
    if let Some(value) = metadata.reference_point {
        number(&mut sheet, "B7", 0, &value.to_string());
    }
    sheet.push_str("</row><row r=\"8\">");
    for (column, label) in ['B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J']
        .into_iter()
        .zip([
            "Medição",
            "Ponto",
            "Hora",
            "Data",
            "Latitude",
            "Longitude",
            "Clorofila A",
            "Clorofila B",
            "Clorofila Total",
        ])
    {
        text(&mut sheet, &format!("{column}8"), label)?;
    }
    sheet.push_str("</row>");
    for (index, r) in records.iter().enumerate() {
        let row = index + 9;
        write!(sheet, "<row r=\"{row}\">").unwrap();
        number(&mut sheet, &format!("B{row}"), 0, &r.id.to_string());
        number(&mut sheet, &format!("C{row}"), 0, &r.point.to_string());
        if let Some(time) = &r.time {
            text(&mut sheet, &format!("D{row}"), time)?;
        }
        if let Some(date) = &r.date {
            let parts: Vec<_> = date.split('-').collect();
            if parts.len() != 3
                || parts[0].len() != 4
                || parts[1].len() != 2
                || parts[2].len() != 2
                || parts.iter().any(|v| !v.bytes().all(|b| b.is_ascii_digit()))
            {
                return Err(Error::Validation(
                    "XLSX requires an ISO measurement date".into(),
                ));
            }
            text(
                &mut sheet,
                &format!("E{row}"),
                &format!("{}-{}-{}", parts[2], parts[1], parts[0]),
            )?;
        }
        if let Some(value) = r.latitude {
            number(&mut sheet, &format!("F{row}"), 1, &value.to_string());
        }
        if let Some(value) = r.longitude {
            number(&mut sheet, &format!("G{row}"), 1, &value.to_string());
        }
        for (column, value) in [
            ('H', r.chlorophyll_a_tenths),
            ('I', r.chlorophyll_b_tenths),
            ('J', r.chlorophyll_total_tenths),
        ] {
            number(
                &mut sheet,
                &format!("{column}{row}"),
                2,
                &decimal_tenths(value),
            );
        }
        sheet.push_str("</row>");
    }
    sheet.push_str("</sheetData></worksheet>");
    let mut custom = format!(
        "{XML}<Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/custom-properties\" xmlns:vt=\"http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes\">"
    );
    for (i, (name, value)) in [
        ("SourceSHA256", metadata.source_sha256.as_str()),
        ("Work", metadata.work.as_str()),
        ("Validation", metadata.validation.as_str()),
        (
            "CoordinatePrecision",
            "Full decoded value retained; display uses six decimal places",
        ),
        ("Timezone", "Unknown; device wall time retained"),
    ]
    .into_iter()
    .enumerate()
    {
        write!(custom,"<property fmtid=\"{{D5CDD505-2E9C-101B-9397-08002B2CF9AE}}\" pid=\"{}\" name=\"{name}\"><vt:lpwstr>{}</vt:lpwstr></property>",i+2,escape(value)?).unwrap();
    }
    custom.push_str("</Properties>");
    let entries = [
        ("[Content_Types].xml", CONTENT_TYPES.into()),
        ("_rels/.rels", RELATIONSHIPS.into()),
        ("xl/workbook.xml", WORKBOOK.into()),
        ("xl/_rels/workbook.xml.rels", WORKBOOK_RELS.into()),
        ("xl/styles.xml", STYLES.into()),
        ("xl/worksheets/sheet1.xml", sheet),
        ("docProps/custom.xml", custom),
    ]
    .map(|(name, value): (&str, String)| (name.to_string(), value.into_bytes()));
    Ok(entries.into())
}

const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
fn escape(value: &str) -> Result<String> {
    if value.chars().any(|c| {
        !(c == '\t'
            || c == '\n'
            || c == '\r'
            || (' '..='\u{d7ff}').contains(&c)
            || ('\u{e000}'..='\u{fffd}').contains(&c)
            || ('\u{10000}'..='\u{10ffff}').contains(&c))
    }) {
        return Err(Error::Validation(
            "text contains an XML-incompatible control character".into(),
        ));
    }
    Ok(value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;"))
}
fn text(out: &mut String, address: &str, value: &str) -> Result<()> {
    write!(
        out,
        "<c r=\"{address}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
        escape(value)?
    )
    .unwrap();
    Ok(())
}
fn number(out: &mut String, address: &str, style: usize, value: &str) {
    write!(out, "<c r=\"{address}\" s=\"{style}\"><v>{value}</v></c>").unwrap();
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    !crc
}
fn u16le(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn u32le(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn zip32(value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| Error::Validation("XLSX exceeds supported ZIP32 size".into()))
}
/// Stored entries use deterministic 1980-01-01 timestamps and UTF-8 filenames.
pub fn archive_files(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut directory = Vec::new();
    let mut names = std::collections::HashSet::new();
    for (name, value) in entries {
        if name.is_empty()
            || name.starts_with('/')
            || name.contains('\\')
            || name.contains(':')
            || name
                .split('/')
                .any(|p| p == ".." || p == "." || p.is_empty())
            || name.chars().any(char::is_control)
            || !names.insert(name.to_lowercase())
        {
            return Err(Error::Validation("unsafe ZIP entry path".into()));
        }
        let name_len = u16::try_from(name.len())
            .map_err(|_| Error::Validation("ZIP filename too long".into()))?;
        let size = zip32(value.len())?;
        let offset = zip32(out.len())?;
        let crc = crc32(value);
        u32le(&mut out, 0x04034b50);
        for v in [20, 0x0800, 0, 0, 33] {
            u16le(&mut out, v);
        }
        u32le(&mut out, crc);
        u32le(&mut out, size);
        u32le(&mut out, size);
        u16le(&mut out, name_len);
        u16le(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(value);
        u32le(&mut directory, 0x02014b50);
        for v in [20, 20, 0x0800, 0, 0, 33] {
            u16le(&mut directory, v);
        }
        u32le(&mut directory, crc);
        u32le(&mut directory, size);
        u32le(&mut directory, size);
        for v in [name_len, 0, 0, 0, 0] {
            u16le(&mut directory, v);
        }
        u32le(&mut directory, 0);
        u32le(&mut directory, offset);
        directory.extend_from_slice(name.as_bytes());
    }
    let offset = zip32(out.len())?;
    let size = zip32(directory.len())?;
    let count = u16::try_from(entries.len())
        .map_err(|_| Error::Validation("too many ZIP entries".into()))?;
    out.extend_from_slice(&directory);
    u32le(&mut out, 0x06054b50);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, count);
    u16le(&mut out, count);
    u32le(&mut out, size);
    u32le(&mut out, offset);
    u16le(&mut out, 0);
    zip32(out.len())?;
    Ok(out)
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/><Override PartName="/docProps/custom.xml" ContentType="application/vnd.openxmlformats-officedocument.custom-properties+xml"/></Types>"#;
const RELATIONSHIPS: &str = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties" Target="docProps/custom.xml"/></Relationships>"#;
const WORKBOOK: &str = r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Worksheet" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
const WORKBOOK_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#;
const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="2"><numFmt numFmtId="164" formatCode="0.000000"/><numFmt numFmtId="165" formatCode="0.0"/></numFmts><fonts count="1"><font><sz val="11"/><color rgb="FF000000"/><name val="Calibri"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="3"><xf xfId="0" numFmtId="0" fontId="0" fillId="0" borderId="0"/><xf xfId="0" numFmtId="164" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/><xf xfId="0" numFmtId="165" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"#;
