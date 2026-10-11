//! Active work discovery, independent from reference-driven byte searches.
use crate::{
    error::{Error, Result},
    filesystem::{self, FilesystemDirectory, FilesystemFile, FilesystemSector},
    parser::{self, Image},
    reader::{hash, hex},
    records::Segment,
    work_records::{self, WorkMeasurement},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
pub struct RawFile {
    pub path: String,
    pub size_bytes: usize,
    pub sha256: String,
    pub raw_hex: String,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Serialize)]
pub struct WorkGroup {
    pub source_file: String,
    pub group: u16,
    pub source_sha256: String,
    pub expected_records: Option<usize>,
    pub records: Vec<WorkMeasurement>,
}

#[derive(Debug, Serialize)]
pub struct Work {
    pub directory: String,
    pub number: u16,
    pub name: Option<String>,
    pub display_name: String,
    pub name_source: String,
    pub configuration_file: Option<String>,
    pub expected_groups: Option<usize>,
    pub groups: Vec<WorkGroup>,
    pub reference_validation: Option<crate::validator::ValidationReport>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
impl Work {
    pub fn record_count(&self) -> usize {
        self.groups.iter().map(|g| g.records.len()).sum()
    }
    /// Points are identified within their group, rather than across the work.
    pub fn point_count(&self) -> usize {
        self.groups
            .iter()
            .map(|g| {
                g.records
                    .iter()
                    .map(|r| r.point)
                    .collect::<BTreeSet<_>>()
                    .len()
            })
            .sum()
    }
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
            + self
                .groups
                .iter()
                .flat_map(|g| &g.records)
                .filter(|r| !r.warnings.is_empty())
                .count()
    }
}

#[derive(Debug, Serialize)]
pub struct FilesystemAudit {
    pub layout: String,
    pub active_sector_count: usize,
    pub released_sector_count: usize,
    pub erased_sector_count: usize,
    pub crc_verified_count: usize,
    pub unreferenced_active_sectors: Vec<u16>,
    pub directories: Vec<FilesystemDirectory>,
    pub sectors: Vec<FilesystemSector>,
    pub files: Vec<RawFile>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Dataset {
    pub schema: String,
    pub tool_version: String,
    pub source_sha256: String,
    pub source_size_bytes: usize,
    pub memory_sha256: String,
    pub filesystem: FilesystemAudit,
    pub works: Vec<Work>,
    pub unclassified_files: Vec<String>,
    pub validation: String,
    pub limitations: Vec<String>,
}

/// Preserve physical discontinuities, including both container and filesystem boundaries.
pub fn source_segments(image: &Image, addresses: &[usize]) -> Result<Vec<Segment>> {
    let mut segments: Vec<Segment> = Vec::new();
    for &address in addresses {
        let source = image.source_offset(address)?;
        if let Some(last) = segments.last_mut().filter(|s| {
            s.file_offset.checked_add(s.length) == Some(source)
                && s.memory_address.checked_add(s.length) == Some(address)
        }) {
            last.length += 1;
        } else {
            segments.push(Segment {
                file_offset: source,
                memory_address: address,
                length: 1,
            });
        }
    }
    Ok(segments)
}

fn work_number(path: &str) -> Option<u16> {
    let suffix = path.strip_prefix("/trabalho")?;
    if suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    suffix.parse().ok()
}

fn extract_work(
    image: &Image,
    directory: &str,
    number: u16,
    files: &[FilesystemFile],
) -> Result<Work> {
    let prefix = format!("{directory}/");
    let config = files
        .iter()
        .find(|f| f.path == format!("{prefix}config_measures"));
    let mut work = Work {
        directory: directory.into(),
        number,
        name: None,
        display_name: directory.trim_start_matches('/').into(),
        name_source: "directory_fallback".into(),
        configuration_file: config.map(|f| f.path.clone()),
        expected_groups: None,
        groups: Vec::new(),
        reference_validation: None,
        errors: Vec::new(),
        warnings: Vec::new(),
    };
    if let Some(config) = config {
        if config.bytes.len() != 336 {
            work.errors
                .push("unsupported configuration length; expected observed 336-byte layout".into());
        } else {
            let bytes = &config.bytes;
            let end = bytes[..24].iter().position(|b| *b == 0).unwrap_or(24);
            match std::str::from_utf8(&bytes[..end]) {
                Ok(name) if !name.is_empty() && !name.chars().any(char::is_control) => {
                    work.name = Some(name.into());
                    work.display_name = name.into();
                    work.name_source = "configuration_bytes_0_23".into();
                }
                _ => work
                    .warnings
                    .push("configuration name unavailable; directory label retained".into()),
            }
            let stored_number = u16::from_le_bytes([bytes[28], bytes[29]]);
            if stored_number != number {
                work.errors.push(format!("configuration work number {stored_number} differs from directory number {number}"));
            }
            work.expected_groups = Some(usize::from(bytes[27]));
            if bytes[27] > 74 {
                work.errors.push(
                    "configuration group count exceeds the supported counter-table boundary".into(),
                );
            }
        }
    } else {
        work.warnings
            .push("configuration missing: name and independent record counts are unknown".into());
    }
    let mut groups: Vec<_> = files
        .iter()
        .filter_map(|file| {
            let basename = file.path.strip_prefix(&prefix)?;
            if basename.is_empty() || !basename.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            Some((basename.parse::<u16>(), file))
        })
        .collect();
    groups.sort_by_key(|(group, _)| group.clone().unwrap_or(u16::MAX));
    let mut group_numbers = BTreeSet::new();
    for (number, file) in groups {
        let group = match number {
            Ok(n) if n > 0 && group_numbers.insert(n) => n,
            _ => {
                work.errors
                    .push(format!("{}: invalid or duplicate group number", file.path));
                continue;
            }
        };
        if work
            .expected_groups
            .is_some_and(|expected| usize::from(group) > expected)
        {
            work.errors.push(format!(
                "{}: group number exceeds configured group count",
                file.path
            ));
        }
        if file.bytes.len() % 48 != 0 {
            work.errors
                .push(format!("{}: incomplete 48-byte measurement", file.path));
            continue;
        }
        let expected_records = config.filter(|c| c.bytes.len() == 336).and_then(|c| {
            let p = 40 + 2 * (usize::from(group) - 1);
            if p + 2 > 188 {
                return None;
            }
            c.bytes
                .get(p..p + 2)
                .map(|b| usize::from(u16::from_le_bytes([b[0], b[1]])))
        });
        if config.is_some_and(|c| c.bytes.len() == 336) && expected_records.is_none() {
            work.errors.push(format!(
                "{}: group counter lies outside known configuration",
                file.path
            ));
        }
        if let Some(expected) = expected_records {
            if expected != file.bytes.len() / 48 {
                work.errors.push(format!(
                    "{}: configuration expects {expected} records, file holds {}",
                    file.path,
                    file.bytes.len() / 48
                ));
            }
        }
        let mut records = Vec::new();
        for (raw, addresses) in file
            .bytes
            .chunks_exact(48)
            .zip(file.addresses.chunks_exact(48))
        {
            records.push(work_records::decode48(
                raw,
                source_segments(image, addresses)?,
            )?);
        }
        work.groups.push(WorkGroup {
            source_file: file.path.clone(),
            group,
            source_sha256: hash(&file.bytes),
            expected_records,
            records,
        });
    }
    if let Some(expected) = work.expected_groups {
        if expected != work.groups.len() {
            work.errors.push(format!(
                "configuration expects {expected} groups, active directory contains {}",
                work.groups.len()
            ));
        }
    }
    if work.groups.is_empty() {
        work.warnings
            .push("no numeric measurement file in this active directory".into());
    }
    for file in files.iter().filter(|f| f.path.starts_with(&prefix)) {
        if work.configuration_file.as_ref() != Some(&file.path)
            && !work.groups.iter().any(|g| g.source_file == file.path)
        {
            work.warnings.push(format!(
                "unclassified active file {}; raw bytes retained",
                file.path
            ));
        }
    }
    Ok(work)
}

pub fn decode(bytes: &[u8]) -> Result<Dataset> {
    let image = parser::parse(bytes)?;
    let snapshot = filesystem::inspect(&image)?;
    let mut works = Vec::new();
    let mut classified = BTreeSet::new();
    for directory in &snapshot.directories {
        if let Some(number) = work_number(&directory.path) {
            let work = extract_work(&image, &directory.path, number, &snapshot.files)?;
            if let Some(path) = &work.configuration_file {
                classified.insert(path.clone());
            }
            for group in &work.groups {
                classified.insert(group.source_file.clone());
            }
            works.push(work);
        }
    }
    works.sort_by_key(|w| w.number);
    let mut numbers: BTreeMap<u16, Vec<usize>> = BTreeMap::new();
    for (index, work) in works.iter().enumerate() {
        numbers.entry(work.number).or_default().push(index);
    }
    for (number, indices) in numbers.iter().filter(|(_, indices)| indices.len() > 1) {
        for &index in indices {
            works[index].errors.push(format!(
                "ambiguous directory spelling: duplicate work number {number}"
            ));
        }
    }
    let mut locations: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (index, work) in works.iter().enumerate() {
        for record in work.groups.iter().flat_map(|g| &g.records) {
            locations.entry(record.id).or_default().push(index);
        }
    }
    for (id, indices) in locations.iter().filter(|(_, v)| v.len() > 1) {
        for index in indices.iter().copied().collect::<BTreeSet<_>>() {
            works[index].errors.push(format!(
                "duplicate active measurement ID {id}; no copies silently selected"
            ));
        }
    }
    let unclassified_files = snapshot
        .files
        .iter()
        .filter(|f| !classified.contains(&f.path))
        .map(|f| f.path.clone())
        .collect();
    let files = snapshot
        .files
        .iter()
        .map(|f| {
            Ok(RawFile {
                path: f.path.clone(),
                size_bytes: f.bytes.len(),
                sha256: hash(&f.bytes),
                raw_hex: hex(&f.bytes),
                segments: source_segments(&image, &f.addresses)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if works.is_empty() {
        return Err(Error::Format(
            "no active work directories in the supported filesystem".into(),
        ));
    }
    Ok(Dataset {schema:"fc2-active-works-v1".into(),tool_version:env!("CARGO_PKG_VERSION").into(),source_sha256:hash(bytes),source_size_bytes:bytes.len(),memory_sha256:hash(&image.bytes),filesystem:FilesystemAudit{layout:snapshot.layout,active_sector_count:snapshot.active_sector_count,released_sector_count:snapshot.released_sector_count,erased_sector_count:snapshot.erased_sector_count,crc_verified_count:snapshot.crc_verified_count,unreferenced_active_sectors:snapshot.unreferenced_active_sectors,directories:snapshot.directories,sectors:snapshot.sectors,files,limitations:snapshot.limitations},works,unclassified_files,validation:"active_filesystem_crc_and_configuration; reference comparison remains work-specific".into(),limitations:vec!["Scientific field layout was compared with a private reference; other works are structurally decoded without independent Leaf equivalence.".into(),"Calendar interpretation has been externally validated only for a limited reference set.".into(),"Zero calendar and nonfinite coordinates become null; raw values and bits remain available.".into(),"Unknown sensor/calibration fields, directory timestamp epochs and most device configuration semantics remain uninterpreted.".into(),"Firmware and reserved memory are preserved by raw export, not semantically decoded.".into()]})
}

/// Cross-check the active-file selection, rather than only finding matching byte patterns.
pub fn compare_reference(
    bytes: &[u8],
    dataset: &mut Dataset,
    profile: &crate::validator::Profile,
) -> Result<()> {
    for work in &mut dataset.works {
        work.reference_validation = None;
    }
    if dataset.source_sha256 != hash(bytes) {
        return Err(Error::Validation(
            "dataset belongs to different source bytes".into(),
        ));
    }
    let (report, _) = crate::validator::decode(bytes, profile)?;
    let matches: Vec<_> = dataset
        .works
        .iter_mut()
        .filter(|w| w.name.as_deref() == Some(profile.reference.work.as_str()))
        .collect();
    if matches.len() != 1 {
        return Err(Error::Validation(
            "reference work name is absent or ambiguous in active directories".into(),
        ));
    }
    let work = matches
        .into_iter()
        .next()
        .ok_or_else(|| Error::Validation("reference work not found".into()))?;
    if !work.errors.is_empty() || work.record_count() != profile.reference.rows.len() {
        return Err(Error::Validation(
            "active work is inconsistent or record count differs from reference selection".into(),
        ));
    }
    let records: BTreeMap<_, _> = work
        .groups
        .iter()
        .flat_map(|g| &g.records)
        .map(|r| (r.id, r))
        .collect();
    for expected in &profile.reference.rows {
        let actual = records.get(&expected.id).ok_or_else(|| {
            Error::Validation(format!(
                "reference ID {} absent from active work",
                expected.id
            ))
        })?;
        let fields = crate::records::ReferenceRow {
            id: actual.id,
            point: actual.point,
            date: actual.date.clone().unwrap_or_default(),
            time: actual.time.clone().unwrap_or_default(),
            latitude: actual.latitude.unwrap_or(f64::NAN),
            longitude: actual.longitude.unwrap_or(f64::NAN),
            chlorophyll_a_tenths: actual.chlorophyll_a_tenths,
            chlorophyll_b_tenths: actual.chlorophyll_b_tenths,
            chlorophyll_total_tenths: actual.chlorophyll_total_tenths,
        };
        let differences = crate::decoder::discrepancies(expected, &fields);
        if !differences.is_empty() {
            return Err(Error::Validation(format!(
                "active reference ID {} differs in {}",
                expected.id,
                differences.join(", ")
            )));
        }
    }
    let mut report = report;
    report.active_sector_selection_verified = true;
    report.filesystem_checksum_verified = true;
    work.reference_validation = Some(report);
    Ok(())
}
