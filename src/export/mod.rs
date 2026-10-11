pub mod csv;
pub mod json;
pub mod works;
pub mod xlsx;

use crate::{
    error::{Error, Result},
    records::Measurement,
    validator::ValidationReport,
};
use serde::Serialize;
use std::{io::Write, path::Path};

#[derive(Serialize)]
pub struct Export {
    pub schema: String,
    pub tool_version: String,
    pub source_name: String,
    pub source_sha256: String,
    pub reference_sha256: String,
    pub order: String,
    pub scope: String,
    pub limitations: Vec<String>,
    pub validation: ValidationReport,
    pub records: Vec<Measurement>,
}

pub fn ensure_validated(document: &Export) -> Result<()> {
    let report = &document.validation;
    if !report.accepted
        || document.records.is_empty()
        || document.records.len() != report.expected_records
        || report.matched_records != report.expected_records
        || document.source_sha256 != report.input_sha256
        || document.records.iter().any(|r| {
            r.occurrences.is_empty()
                || r.validation != "reference_scoped"
                || r.occurrences.iter().any(|o| {
                    o.raw_hex.len() != 96
                        || o.segments.iter().map(|s| s.length).sum::<usize>() != 48
                })
        })
    {
        return Err(Error::Validation(
            "export requires a complete accepted reference selection and raw provenance".into(),
        ));
    }
    Ok(())
}

pub fn document(
    source_name: String,
    report: ValidationReport,
    mut records: Vec<Measurement>,
    reference_order: bool,
) -> Export {
    if reference_order {
        records.sort_by_key(|r| r.reference_row_index);
    } else {
        records.sort_by_key(|r| {
            r.occurrences
                .first()
                .and_then(|o| o.segments.first())
                .map(|s| s.file_offset)
        });
    }
    Export {schema:"fc2-measurements-v1".into(),tool_version:env!("CARGO_PKG_VERSION").into(),source_name,
        source_sha256:report.input_sha256.clone(),reference_sha256:report.source_reference_sha256.clone(),
        order:if reference_order {"reference_row"} else {"first_matching_source_offset"}.into(),
        scope:"reference selection for this exact input hash only".into(),
        limitations:vec!["active-sector selection and filesystem checksums are unverified".into(),
            "work label is provided by the reference, not independently selected from the filesystem".into(),
            "other works, deleted records and calibration are not decoded".into(),
            "timestamps retain device wall time; timezone is unknown".into()],validation:report,records}
}

/// Publish a complete file atomically without replacing any existing path.
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist_noclobber(path).map_err(|e| e.error)?;
    Ok(())
}
