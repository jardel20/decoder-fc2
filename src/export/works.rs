//! XLSX packages with complete scientific and raw-data audit companions.
use super::xlsx::{self, WorkMetadata, WorkbookRow, WorkbookSheet};
use crate::{
    downloads::{self, DownloadWork},
    error::{Error, Result},
    parser, reader,
    workspace::{Dataset, Work},
};
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub enum WorkbookOrder {
    Stored,
    Leaf,
}

/// Complete audit package or workbooks alone; both modes validate the original source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportMode {
    Complete,
    Simple,
}

#[derive(Serialize)]
pub struct Manifest {
    pub schema: String,
    pub tool_version: String,
    pub source_name: String,
    pub source_sha256: String,
    pub order: String,
    pub works: Vec<DownloadWork>,
    pub successful_workbooks: usize,
    pub failures: usize,
    pub record_count: usize,
    pub group_count: usize,
    pub limitations: Vec<String>,
}

fn json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn work_source_signature(work: &Work) -> Result<Vec<u8>> {
    let mut value = serde_json::to_value(work)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| Error::Validation("invalid work document".into()))?;
    object.remove("reference_validation");
    object.remove("errors");
    Ok(serde_json::to_vec(&value)?)
}

fn included(number: u16, selection: Option<&[u16]>) -> bool {
    selection.is_none_or(|numbers| numbers.contains(&number))
}

fn validate_selection(dataset: &Dataset, selection: Option<&[u16]>) -> Result<()> {
    if let Some(numbers) = selection {
        if numbers.is_empty() {
            return Err(Error::Validation("work selection must not be empty".into()));
        }
        let mut seen = std::collections::BTreeSet::new();
        for &number in numbers {
            if !seen.insert(number) {
                return Err(Error::Validation(format!(
                    "selected work {number} is repeated"
                )));
            }
            selected_directory(dataset, number)?;
        }
    }
    Ok(())
}

fn verify_active_coverage(
    bytes: &[u8],
    dataset: &Dataset,
    selection: Option<&[u16]>,
) -> Result<()> {
    let original = crate::workspace::decode(bytes)?;
    let selected: Vec<_> = dataset
        .works
        .iter()
        .filter(|w| included(w.number, selection))
        .collect();
    let actual: Vec<_> = original
        .works
        .iter()
        .filter(|w| included(w.number, selection))
        .collect();
    if selected.len() != actual.len()
        || dataset.memory_sha256 != original.memory_sha256
        || json(&dataset.filesystem)? != json(&original.filesystem)?
    {
        return Err(Error::Validation(
            "active work or raw-file inventory was altered or omitted after decoding".into(),
        ));
    }
    for (provided, actual) in selected.iter().zip(actual) {
        if work_source_signature(provided)? != work_source_signature(actual)?
            || actual
                .errors
                .iter()
                .any(|error| !provided.errors.contains(error))
        {
            return Err(Error::Validation(format!(
                "{}: active records, configuration, group coverage or source order differ from the original",
                provided.directory
            )));
        }
    }
    Ok(())
}

fn workbook(dataset: &Dataset, work: &Work, order: WorkbookOrder) -> Result<Vec<u8>> {
    if !work.errors.is_empty() {
        return Err(Error::Validation(work.errors.join("; ")));
    }
    if work.groups.is_empty() {
        return Err(Error::Validation(
            "no measurement groups; workbook cannot represent unknown contents".into(),
        ));
    }
    if work.reference_validation.as_ref().is_some_and(|r| {
        !r.accepted
            || r.input_sha256 != dataset.source_sha256
            || r.expected_records != work.record_count()
            || r.matched_records != work.record_count()
            || !r.active_sector_selection_verified
            || !r.filesystem_checksum_verified
    }) {
        return Err(Error::Validation(
            "inconsistent work reference validation report".into(),
        ));
    }
    let sheets: Vec<_> = work
        .groups
        .iter()
        .map(|group| {
            let mut records: Vec<_> = group.records.iter().collect();
            if matches!(order, WorkbookOrder::Leaf) {
                records.sort_by_key(|a| a.point.to_string());
            }
            WorkbookSheet {
                name: format!("Medidas {}", group.group),
                rows: records
                    .into_iter()
                    .map(|r| WorkbookRow {
                        id: r.id,
                        point: r.point,
                        date: r.date.clone(),
                        time: r.time.clone(),
                        latitude: r.latitude,
                        longitude: r.longitude,
                        chlorophyll_a_tenths: r.chlorophyll_a_tenths,
                        chlorophyll_b_tenths: r.chlorophyll_b_tenths,
                        chlorophyll_total_tenths: r.chlorophyll_total_tenths,
                    })
                    .collect(),
            }
        })
        .collect();
    let label = if work.name.is_some() {
        work.display_name.clone()
    } else {
        format!("{} (nome indisponível)", work.display_name)
    };
    xlsx::render_sheets(
        &WorkMetadata {
            work: label,
            source_sha256: dataset.source_sha256.clone(),
            reference_point: None,
            generated_at: None,
            validation: if work.reference_validation.is_some() {
                "reference_and_active_filesystem"
            } else {
                "active_filesystem_crc_and_configuration; not independently compared to Leaf"
            }
            .into(),
        },
        &sheets,
    )
}

fn basename(work: &Work) -> String {
    let label: String = work
        .display_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{}-{label}.xlsx", work.number)
}

fn selected_directory(dataset: &Dataset, number: u16) -> Result<&str> {
    let mut matches = dataset.works.iter().filter(|work| work.number == number);
    let work = matches
        .next()
        .ok_or_else(|| Error::Validation("selected active work not found".into()))?;
    if matches.next().is_some() {
        return Err(Error::Validation(
            "selected work number is ambiguous".into(),
        ));
    }
    Ok(&work.directory)
}

fn raw_archive_selection(
    bytes: &[u8],
    dataset: &Dataset,
    selection: Option<&[u16]>,
) -> Result<Vec<u8>> {
    if reader::hash(bytes) != dataset.source_sha256 {
        return Err(Error::Validation("raw archive source hash mismatch".into()));
    }
    let image = parser::parse(bytes)?;
    let mut entries = if selection.is_none() {
        vec![
            ("original.FC2".into(), bytes.to_vec()),
            ("memory.bin".into(), image.bytes.clone()),
            ("container.json".into(), json(&image.blocks)?),
        ]
    } else {
        vec![(
            "provenance.json".into(),
            json(&match selection {
                Some([number]) => {
                    serde_json::json!({"source_sha256":dataset.source_sha256,"work":number,"scope":"selected work files only; complete FC2 and memory excluded"})
                }
                _ => {
                    serde_json::json!({"source_sha256":dataset.source_sha256,"works":selection,"scope":"selected work files only; complete FC2 and memory excluded"})
                }
            })?,
        )]
    };
    let prefixes = selection
        .map(|numbers| {
            numbers
                .iter()
                .map(|&number| {
                    selected_directory(dataset, number).map(|directory| format!("{directory}/"))
                })
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?;
    for file in dataset.filesystem.files.iter().filter(|f| {
        prefixes
            .as_ref()
            .is_none_or(|paths| paths.iter().any(|p| f.path.starts_with(p)))
    }) {
        if file.raw_hex.len() != file.size_bytes * 2 {
            return Err(Error::Validation(
                "raw filesystem file length mismatch".into(),
            ));
        }
        let raw = file
            .raw_hex
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                std::str::from_utf8(pair)
                    .ok()
                    .and_then(|s| u8::from_str_radix(s, 16).ok())
                    .ok_or_else(|| Error::Validation("invalid raw file hex".into()))
            })
            .collect::<Result<Vec<_>>>()?;
        if reader::hash(&raw) != file.sha256 {
            return Err(Error::Validation(
                "raw filesystem file hash mismatch".into(),
            ));
        }
        let mut source = Vec::new();
        for segment in &file.segments {
            source.extend_from_slice(reader::slice(bytes, segment.file_offset, segment.length)?);
        }
        if source != raw {
            return Err(Error::Validation(
                "raw filesystem file differs from original source segments".into(),
            ));
        }
        entries.push((format!("filesystem{}", file.path), raw));
    }
    xlsx::archive_files(&entries)
}

pub fn raw_archive(bytes: &[u8], dataset: &Dataset) -> Result<Vec<u8>> {
    raw_archive_selection(bytes, dataset, None)
}

fn audit_json(dataset: &Dataset, selection: Option<&[u16]>) -> Result<Vec<u8>> {
    if let Some(numbers) = selection {
        let mut audit = serde_json::to_value(dataset)?;
        let directories = numbers
            .iter()
            .map(|&number| selected_directory(dataset, number))
            .collect::<Result<Vec<_>>>()?;
        let prefixes: Vec<_> = directories
            .iter()
            .map(|directory| format!("{directory}/"))
            .collect();
        for key in ["works"] {
            if let Some(array) = audit[key].as_array_mut() {
                array.retain(|w| {
                    w["number"]
                        .as_u64()
                        .is_some_and(|n| numbers.iter().any(|&number| n == u64::from(number)))
                });
            }
        }
        if let Some(files) = audit["filesystem"]["files"].as_array_mut() {
            files.retain(|f| {
                f["path"]
                    .as_str()
                    .is_some_and(|p| prefixes.iter().any(|prefix| p.starts_with(prefix)))
            });
        }
        let mut sectors = std::collections::BTreeSet::new();
        if let Some(files) = audit["filesystem"]["files"].as_array() {
            for file in files {
                if let Some(parts) = file["segments"].as_array() {
                    for part in parts {
                        if let (Some(address), Some(length)) =
                            (part["memory_address"].as_u64(), part["length"].as_u64())
                        {
                            if length > 0 {
                                for start in address / 512..=(address + length - 1) / 512 {
                                    sectors.insert(start * 512);
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(dirs) = audit["filesystem"]["directories"].as_array_mut() {
            dirs.retain(|d| {
                d["path"]
                    .as_str()
                    .is_some_and(|path| directories.contains(&path))
            });
            for dir in dirs {
                if let Some(parts) = dir["sectors"].as_array() {
                    for part in parts {
                        if let Some(address) = part["memory_address"].as_u64() {
                            sectors.insert(address);
                        }
                    }
                }
            }
        }
        if let Some(inventory) = audit["filesystem"]["sectors"].as_array_mut() {
            inventory.retain(|s| {
                s["memory_address"]
                    .as_u64()
                    .is_some_and(|a| sectors.contains(&a))
            });
        }
        if let Some(files) = audit["unclassified_files"].as_array_mut() {
            files.retain(|f| {
                f.as_str()
                    .is_some_and(|p| prefixes.iter().any(|prefix| p.starts_with(prefix)))
            });
        }
        audit["scope"] = serde_json::json!(if numbers.len() == 1 {
            "selected work only; filesystem summary counts and input hashes describe the complete source"
        } else {
            "selected works only; filesystem summary counts and input hashes describe the complete source"
        });
        json(&audit)
    } else {
        json(dataset)
    }
}

/// Prepare all files before creating the output directory; never replace an existing directory.
pub fn export_directory(
    bytes: &[u8],
    dataset: &Dataset,
    source_name: &str,
    output: &Path,
    selection: Option<u16>,
    order: WorkbookOrder,
) -> Result<Manifest> {
    let numbers = selection.map(|number| [number]);
    export_directory_options(
        bytes,
        dataset,
        source_name,
        output,
        numbers.as_ref().map(|values| values.as_slice()),
        order,
        ExportMode::Complete,
    )
}

/// Validate and prepare the selected works in source order before creating the directory.
/// `None` selects all works; an explicit selection must be nonempty and contain unique numbers.
/// Simple mode returns the same audit report but writes only successful XLSX workbooks.
pub fn export_directory_options(
    bytes: &[u8],
    dataset: &Dataset,
    source_name: &str,
    output: &Path,
    selection: Option<&[u16]>,
    order: WorkbookOrder,
    mode: ExportMode,
) -> Result<Manifest> {
    if reader::hash(bytes) != dataset.source_sha256 {
        return Err(Error::Validation("work export source hash mismatch".into()));
    }
    validate_selection(dataset, selection)?;
    verify_active_coverage(bytes, dataset, selection)?;
    let selected: Vec<_> = dataset
        .works
        .iter()
        .filter(|w| included(w.number, selection))
        .collect();
    if selected.is_empty() {
        return Err(Error::Validation("selected active work not found".into()));
    }
    let image = parser::parse(bytes)?;
    for work in &selected {
        for record in work.groups.iter().flat_map(|group| &group.records) {
            crate::work_records::verify_source_record(bytes, &image, record)?;
        }
    }
    let mut entries = Vec::new();
    let mut downloads = Vec::new();
    for work in &selected {
        let result = workbook(dataset, work, order);
        let (filename, error) = match result {
            Ok(bytes) => {
                let name = basename(work);
                entries.push((name.clone(), bytes));
                (Some(name), None)
            }
            Err(error) => (None, Some(error.to_string())),
        };
        downloads.push(DownloadWork {
            number: work.number,
            directory: work.directory.clone(),
            name: work.display_name.clone(),
            records: work.record_count(),
            groups: work.groups.len(),
            warnings: work.warning_count(),
            filename,
            error,
        });
    }
    let failures = downloads.iter().filter(|w| w.error.is_some()).count();
    let manifest = Manifest {
        schema: "fc2-downloads-v1".into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        source_name: source_name.into(),
        source_sha256: dataset.source_sha256.clone(),
        order: match order {
            WorkbookOrder::Stored => "active_file_record_order",
            WorkbookOrder::Leaf => "lexicographic_point_then_active_file_record_order",
        }
        .into(),
        successful_workbooks: downloads.len() - failures,
        failures,
        record_count: selected.iter().map(|w| w.record_count()).sum(),
        group_count: selected.iter().map(|w| w.groups.len()).sum(),
        works: downloads,
        limitations: dataset.limitations.clone(),
    };
    if mode == ExportMode::Complete {
        let manifest_bytes = json(&manifest)?;
        let mut bundle_entries = entries.clone();
        bundle_entries.push(("manifest.json".into(), manifest_bytes.clone()));
        entries.push((
            "todos-trabalhos.zip".into(),
            xlsx::archive_files(&bundle_entries)?,
        ));
        entries.push((
            "dados-brutos.zip".into(),
            raw_archive_selection(bytes, dataset, selection)?,
        ));
        entries.push(("dataset.json".into(), audit_json(dataset, selection)?));
        entries.push((
            "index.html".into(),
            downloads::page(source_name, &dataset.source_sha256, &manifest.works)?,
        ));
        entries.push(("manifest.json".into(), manifest_bytes));
    }
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(output)?;
    for (name, bytes) in entries {
        super::write_new(&output.join(name), &bytes)?;
    }
    Ok(manifest)
}
