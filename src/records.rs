use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceRow {
    pub id: u32,
    pub point: u16,
    pub date: String,
    pub time: String,
    pub latitude: f64,
    pub longitude: f64,
    pub chlorophyll_a_tenths: u16,
    pub chlorophyll_b_tenths: u16,
    pub chlorophyll_total_tenths: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub schema: String,
    pub source_fc2_sha256: String,
    pub source_reference_sha256: String,
    pub source_reference_name: String,
    pub work: String,
    pub confirmed_same_source: bool,
    pub rows: Vec<ReferenceRow>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Segment {
    pub file_offset: usize,
    pub memory_address: usize,
    pub length: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Occurrence {
    pub segments: Vec<Segment>,
    pub raw_hex: String,
    pub raw_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Measurement {
    pub id: u32,
    pub work: String,
    pub point: u16,
    pub date: String,
    pub time: String,
    pub timezone: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub latitude_bits: u32,
    pub longitude_bits: u32,
    pub chlorophyll_a_tenths: u16,
    pub chlorophyll_b_tenths: u16,
    pub chlorophyll_total_tenths: u16,
    pub reference_row_index: usize,
    pub chlorophyll_a_index: String,
    pub chlorophyll_b_index: String,
    pub chlorophyll_total_index: String,
    pub validation: String,
    pub occurrences: Vec<Occurrence>,
}

pub fn decimal_tenths(value: u16) -> String {
    format!("{}.{:01}", value / 10, value % 10)
}
