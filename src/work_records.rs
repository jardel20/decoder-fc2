//! Decoding of structurally selected 48-byte measurement records.
//!
//! File and work selection belong to the filesystem layer. These fields were
//! compared with the 120 reference measurements; absent and anomalous values
//! retain their raw representation rather than being imputed.
use crate::{
    error::{Error, Result},
    parser::Image,
    reader::{hash, hex},
    records::{Segment, decimal_tenths},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawCalendar {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkMeasurement {
    pub id: u32,
    pub point: u16,
    pub date: Option<String>,
    pub time: Option<String>,
    pub timezone: Option<String>,
    pub raw_calendar: RawCalendar,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub latitude_bits: u32,
    pub longitude_bits: u32,
    pub chlorophyll_a_tenths: u16,
    pub chlorophyll_b_tenths: u16,
    pub chlorophyll_total_tenths: u16,
    pub chlorophyll_a_index: String,
    pub chlorophyll_b_index: String,
    pub chlorophyll_total_index: String,
    /// Bytes 5 through 23: their scientific meaning is not established.
    pub unknown_prefix_hex: String,
    /// Offset 34; interpretation and unit are not established.
    pub unknown_34: u16,
    pub raw_hex: String,
    pub raw_sha256: String,
    pub segments: Vec<Segment>,
    pub warnings: Vec<String>,
}

fn word(raw: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([raw[offset], raw[offset + 1]])
}
fn dword(raw: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        raw[offset],
        raw[offset + 1],
        raw[offset + 2],
        raw[offset + 3],
    ])
}

fn coordinate(bits: u32, bound: f64, label: &str, warnings: &mut Vec<String>) -> Option<f64> {
    let value = f64::from(f32::from_bits(bits));
    if !value.is_finite() {
        warnings.push(format!("{label}_nonfinite_raw_bits_preserved"));
        None
    } else if value.abs() > bound {
        warnings.push(format!("{label}_out_of_range_raw_bits_preserved"));
        None
    } else {
        Some(value)
    }
}

/// Interpret one record selected by a validated file chain, preserving its origin.
/// Calendar year base 2000 is the observed interpretation, not universal proof.
pub fn decode48(raw: &[u8], segments: Vec<Segment>) -> Result<WorkMeasurement> {
    if raw.len() != 48 {
        return Err(Error::Format(format!(
            "measurement requires 48 bytes, received {}",
            raw.len()
        )));
    }
    let mut total = 0usize;
    for segment in &segments {
        if segment.length == 0
            || segment.file_offset.checked_add(segment.length).is_none()
            || segment.memory_address.checked_add(segment.length).is_none()
        {
            return Err(Error::Format("invalid measurement source segment".into()));
        }
        total = total
            .checked_add(segment.length)
            .ok_or_else(|| Error::Format("measurement segment length overflow".into()))?;
    }
    if total != 48 {
        return Err(Error::Format(
            "measurement source segments must cover exactly 48 bytes".into(),
        ));
    }
    let mut warnings = Vec::new();
    let raw_calendar = RawCalendar {
        second: raw[0],
        minute: raw[1],
        hour: raw[2],
        day: raw[3],
        month: raw[4],
        year: word(raw, 32),
    };
    let calendar_empty = raw[..5].iter().all(|b| *b == 0) && raw_calendar.year == 0;
    let (date, time) = if calendar_empty {
        warnings.push("calendar_all_zero_raw_values_preserved".into());
        (None, None)
    } else {
        let year = 2000 + u32::from(raw_calendar.year);
        let days = match raw_calendar.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        };
        let date = if raw_calendar.year <= 99 && raw_calendar.day > 0 && raw_calendar.day <= days {
            Some(format!(
                "{year:04}-{:02}-{:02}",
                raw_calendar.month, raw_calendar.day
            ))
        } else {
            warnings.push("calendar_invalid_raw_values_preserved".into());
            None
        };
        let time =
            if raw_calendar.hour <= 23 && raw_calendar.minute <= 59 && raw_calendar.second <= 59 {
                Some(format!(
                    "{:02}:{:02}:{:02}",
                    raw_calendar.hour, raw_calendar.minute, raw_calendar.second
                ))
            } else {
                warnings.push("time_invalid_raw_values_preserved".into());
                None
            };
        (date, time)
    };
    let latitude_bits = dword(raw, 40);
    let longitude_bits = dword(raw, 44);
    let latitude = coordinate(latitude_bits, 90.0, "latitude", &mut warnings);
    let longitude = coordinate(longitude_bits, 180.0, "longitude", &mut warnings);
    let chlorophyll_a_tenths = word(raw, 24);
    let chlorophyll_b_tenths = word(raw, 26);
    let chlorophyll_total_tenths = word(raw, 28);
    Ok(WorkMeasurement {
        id: dword(raw, 36),
        point: word(raw, 30),
        date,
        time,
        timezone: None,
        raw_calendar,
        latitude,
        longitude,
        latitude_bits,
        longitude_bits,
        chlorophyll_a_tenths,
        chlorophyll_b_tenths,
        chlorophyll_total_tenths,
        chlorophyll_a_index: decimal_tenths(chlorophyll_a_tenths),
        chlorophyll_b_index: decimal_tenths(chlorophyll_b_tenths),
        chlorophyll_total_index: decimal_tenths(chlorophyll_total_tenths),
        unknown_prefix_hex: hex(&raw[5..24]),
        unknown_34: word(raw, 34),
        raw_hex: hex(raw),
        raw_sha256: hash(raw),
        segments,
        warnings,
    })
}

/// Require every exported field to be reproducible from its original source bytes.
/// This also checks the memory/physical mapping supplied by the container parser.
pub fn verify_source_record(bytes: &[u8], image: &Image, record: &WorkMeasurement) -> Result<()> {
    if record.latitude.is_some_and(|value| !value.is_finite())
        || record.longitude.is_some_and(|value| !value.is_finite())
    {
        return Err(Error::Validation(
            "nonfinite coordinates must retain their absent interpreted representation".into(),
        ));
    }
    let mut raw = Vec::with_capacity(48);
    for segment in &record.segments {
        if segment.length == 0 || segment.length > 48 || raw.len() + segment.length > 48 {
            return Err(Error::Validation(
                "invalid measurement provenance length".into(),
            ));
        }
        let original = crate::reader::slice(bytes, segment.file_offset, segment.length)?;
        for (index, byte) in original.iter().enumerate() {
            let address = segment
                .memory_address
                .checked_add(index)
                .ok_or_else(|| Error::Validation("measurement memory address overflow".into()))?;
            let physical = segment
                .file_offset
                .checked_add(index)
                .ok_or_else(|| Error::Validation("measurement physical offset overflow".into()))?;
            if image.source_offset(address)? != physical || image.bytes[address] != *byte {
                return Err(Error::Validation(
                    "measurement source segments disagree with container mapping".into(),
                ));
            }
        }
        raw.extend_from_slice(original);
    }
    let decoded = decode48(&raw, record.segments.clone())?;
    // Comparing serialized bytes also distinguishes +0.0 from -0.0; JSON Value
    // number equality would consider those numeric values equal.
    if serde_json::to_vec(record)? != serde_json::to_vec(&decoded)? {
        return Err(Error::Validation(format!(
            "measurement {} differs from original source bytes",
            record.id
        )));
    }
    Ok(())
}
