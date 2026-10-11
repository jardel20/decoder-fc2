//! Candidate reconstruction, never a general selector of active measurements.
use crate::{
    error::{Error, Result},
    parser::Image,
    reader::{hash, hex},
    records::*,
};
use std::collections::{BTreeMap, BTreeSet};

pub const LAYOUT: &str = "fs2-256-linked-candidates-v1";
pub const COORDINATE_TOLERANCE: f64 = 0.0000005;

#[derive(Clone, Debug)]
struct Sector {
    start: usize,
    logical: u16,
    next: u16,
    used: usize,
}

#[derive(Debug)]
pub struct Candidate {
    pub fields: ReferenceRow,
    pub occurrence: Occurrence,
    pub bits: [u32; 2],
}

fn word(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}
fn dword(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

pub fn fields(raw: &[u8]) -> Result<(ReferenceRow, [u32; 2])> {
    if raw.len() != 48 {
        return Err(Error::Format(
            "candidate must contain exactly 48 bytes".into(),
        ));
    }
    let year = 2000 + u32::from(word(raw, 32));
    let (month, day) = (raw[4], raw[3]);
    let days = match month {
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
    let bits = [dword(raw, 40), dword(raw, 44)];
    let (lat, lon) = (
        f64::from(f32::from_bits(bits[0])),
        f64::from(f32::from_bits(bits[1])),
    );
    if year > 2099
        || day == 0
        || day > days
        || raw[0] > 59
        || raw[1] > 59
        || raw[2] > 23
        || !lat.is_finite()
        || !lon.is_finite()
        || !(-90.0..=90.0).contains(&lat)
        || !(-180.0..=180.0).contains(&lon)
    {
        return Err(Error::Format(
            "candidate has invalid calendar, time or coordinates".into(),
        ));
    }
    Ok((
        ReferenceRow {
            id: dword(raw, 36),
            point: word(raw, 30),
            date: format!("{year:04}-{month:02}-{day:02}"),
            time: format!("{:02}:{:02}:{:02}", raw[2], raw[1], raw[0]),
            latitude: lat,
            longitude: lon,
            chlorophyll_a_tenths: word(raw, 24),
            chlorophyll_b_tenths: word(raw, 26),
            chlorophyll_total_tenths: word(raw, 28),
        },
        bits,
    ))
}

pub fn discrepancies(expected: &ReferenceRow, actual: &ReferenceRow) -> Vec<String> {
    let mut differences = Vec::new();
    macro_rules! exact { ($($field:ident),*) => { $(if expected.$field!=actual.$field {
        differences.push(stringify!($field).into()); })* }; }
    exact!(
        id,
        point,
        date,
        time,
        chlorophyll_a_tenths,
        chlorophyll_b_tenths,
        chlorophyll_total_tenths
    );
    for (name, a, b) in [
        ("latitude", expected.latitude, actual.latitude),
        ("longitude", expected.longitude, actual.longitude),
    ] {
        if !a.is_finite() || !b.is_finite() || (a - b).abs() > COORDINATE_TOLERANCE {
            differences.push(name.into());
        }
    }
    differences
}

fn candidate(image: &Image, addresses: Vec<usize>) -> Result<Candidate> {
    if addresses.len() != 48 {
        return Err(Error::Format("incomplete linked candidate".into()));
    }
    let mut bytes = Vec::with_capacity(48);
    let mut segments: Vec<Segment> = Vec::new();
    for address in addresses {
        let byte = image
            .bytes
            .get(address)
            .ok_or_else(|| Error::Format("unmapped candidate byte".into()))?;
        bytes.push(*byte);
        let file = image.source_offset(address)?;
        if let Some(last) = segments
            .last_mut()
            .filter(|s| s.file_offset + s.length == file && s.memory_address + s.length == address)
        {
            last.length += 1;
        } else {
            segments.push(Segment {
                file_offset: file,
                memory_address: address,
                length: 1,
            });
        }
    }
    let (fields, bits) = fields(&bytes)?;
    Ok(Candidate {
        fields,
        bits,
        occurrence: Occurrence {
            segments,
            raw_hex: hex(&bytes),
            raw_sha256: hash(&bytes),
        },
    })
}

/// Enumerate physical candidates and linked fragments for requested IDs.
/// Status bytes and links are observed patterns; active-sector selection is unproved.
pub fn candidates(image: &Image, ids: &BTreeSet<u32>) -> Result<BTreeMap<u32, Vec<Candidate>>> {
    let mut positions: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    let mut total = 0;
    for (pos, b) in image.bytes.windows(4).enumerate() {
        let id = dword(b, 0);
        if ids.contains(&id) {
            total += 1;
            if total > 50_000 {
                return Err(Error::Validation(
                    "too many matching ID byte patterns".into(),
                ));
            }
            positions.entry(id).or_default().push(pos);
        }
    }
    let mut sectors = BTreeMap::new();
    let mut by_logical: BTreeMap<u16, Vec<usize>> = BTreeMap::new();
    let mut by_next: BTreeMap<u16, Vec<usize>> = BTreeMap::new();
    for (i, s) in image.bytes.chunks_exact(512).enumerate() {
        let used = usize::from(word(s, 23));
        if ![0x05, 0x45].contains(&s[0]) || used > 487 || s[1..16].iter().any(|b| *b != 255) {
            continue;
        }
        let sector = Sector {
            start: i * 512,
            logical: word(s, 16),
            next: word(s, 21),
            used,
        };
        by_logical
            .entry(sector.logical)
            .or_default()
            .push(sector.start);
        if sector.next != u16::MAX {
            by_next.entry(sector.next).or_default().push(sector.start);
        }
        sectors.insert(sector.start, sector);
    }
    let mut output = BTreeMap::new();
    let mut combinations = 0;
    for (id, hits) in positions {
        let mut found = Vec::new();
        for p in hits {
            let Some(sec) = sectors.get(&(p / 512 * 512)) else {
                continue;
            };
            let data_start = sec.start + 25;
            let data_end = data_start + sec.used;
            if p < data_start || p + 4 > data_end || p < 36 {
                continue;
            }
            let start = p - 36;
            let end = p + 12;
            let mut choices = Vec::new();
            if start >= data_start && end <= data_end {
                choices.push((start..end).collect());
            } else if start < data_start && end <= data_end {
                let needed = data_start - start;
                for prev in by_next
                    .get(&sec.logical)
                    .into_iter()
                    .flatten()
                    .filter_map(|s| sectors.get(s))
                {
                    if prev.used < needed {
                        continue;
                    }
                    let tail = prev.start + 25 + prev.used - needed;
                    choices.push((tail..tail + needed).chain(data_start..end).collect());
                }
            } else if start >= data_start && end > data_end && sec.next != u16::MAX {
                let needed = end - data_end;
                for next in by_logical
                    .get(&sec.next)
                    .into_iter()
                    .flatten()
                    .filter_map(|s| sectors.get(s))
                {
                    if next.used < needed {
                        continue;
                    }
                    choices.push(
                        (start..data_end)
                            .chain(next.start + 25..next.start + 25 + needed)
                            .collect(),
                    );
                }
            }
            for addresses in choices {
                combinations += 1;
                if combinations > 100_000 {
                    return Err(Error::Validation(
                        "too many linked candidate combinations".into(),
                    ));
                }
                if let Ok(c) = candidate(image, addresses) {
                    if c.fields.id == id {
                        found.push(c);
                    }
                }
            }
        }
        found.sort_by_key(|c| {
            c.occurrence
                .segments
                .iter()
                .map(|s| s.file_offset)
                .collect::<Vec<_>>()
        });
        found.dedup_by(|a, b| a.occurrence.segments == b.occurrence.segments);
        output.insert(id, found);
    }
    Ok(output)
}
