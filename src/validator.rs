use crate::{
    decoder::{self, LAYOUT},
    error::{Error, Result},
    parser,
    reader::hash,
    records::*,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Serialize, Deserialize)]
pub struct RowValidation {
    pub id: u32,
    pub matched_occurrences: usize,
    pub raw_variants: usize,
    pub candidate_mismatches: Vec<Vec<String>>,
    pub accepted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationReport {
    pub input_sha256: String,
    pub source_reference_sha256: String,
    pub work: String,
    pub expected_records: usize,
    pub matched_records: usize,
    pub matched_occurrences: usize,
    pub accepted: bool,
    pub coordinate_tolerance: f64,
    pub active_sector_selection_verified: bool,
    pub filesystem_checksum_verified: bool,
    pub rows: Vec<RowValidation>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: String,
    pub layout: String,
    pub tool_version: String,
    pub reference: Reference,
    pub validation: ValidationReport,
}

fn sha_valid(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn validate(
    bytes: &[u8],
    reference: &Reference,
) -> Result<(ValidationReport, Vec<Measurement>)> {
    let input_hash = hash(bytes);
    if reference.schema != "fc2-reference-v1"
        || !reference.confirmed_same_source
        || reference.source_fc2_sha256 != input_hash
        || !sha_valid(&reference.source_reference_sha256)
        || reference.work.is_empty()
        || reference.rows.is_empty()
        || reference.rows.len() > 10_000
    {
        return Err(Error::Validation(
            "reference schema, provenance, file hash or row count invalid".into(),
        ));
    }
    let ids: BTreeSet<_> = reference.rows.iter().map(|r| r.id).collect();
    if ids.len() != reference.rows.len() {
        return Err(Error::Validation(
            "duplicate IDs in reference selection".into(),
        ));
    }
    if reference
        .rows
        .iter()
        .any(|r| !r.latitude.is_finite() || !r.longitude.is_finite())
    {
        return Err(Error::Validation(
            "reference coordinates must be finite".into(),
        ));
    }
    let image = parser::parse(bytes)?;
    let mut candidates = decoder::candidates(&image, &ids)?;
    let mut reports = Vec::new();
    let mut measurements = Vec::new();
    let mut total_occurrences = 0;
    for (index, expected) in reference.rows.iter().enumerate() {
        let found = candidates.remove(&expected.id).unwrap_or_default();
        let (matching, mismatches): (Vec<_>, Vec<_>) = found
            .into_iter()
            .partition(|c| decoder::discrepancies(expected, &c.fields).is_empty());
        let raw_variants = matching
            .iter()
            .map(|c| c.occurrence.raw_hex.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let accepted = !matching.is_empty() && raw_variants == 1;
        total_occurrences += matching.len();
        reports.push(RowValidation {
            id: expected.id,
            matched_occurrences: matching.len(),
            raw_variants,
            accepted,
            candidate_mismatches: mismatches
                .iter()
                .map(|c| decoder::discrepancies(expected, &c.fields))
                .collect(),
        });
        if accepted {
            let fields = &matching[0].fields;
            let bits = matching[0].bits;
            measurements.push(Measurement {
                id: fields.id,
                work: reference.work.clone(),
                point: fields.point,
                date: fields.date.clone(),
                time: fields.time.clone(),
                timezone: None,
                latitude: fields.latitude,
                longitude: fields.longitude,
                latitude_bits: bits[0],
                longitude_bits: bits[1],
                chlorophyll_a_tenths: fields.chlorophyll_a_tenths,
                chlorophyll_b_tenths: fields.chlorophyll_b_tenths,
                chlorophyll_total_tenths: fields.chlorophyll_total_tenths,
                reference_row_index: index,
                chlorophyll_a_index: decimal_tenths(fields.chlorophyll_a_tenths),
                chlorophyll_b_index: decimal_tenths(fields.chlorophyll_b_tenths),
                chlorophyll_total_index: decimal_tenths(fields.chlorophyll_total_tenths),
                validation: "reference_scoped".into(),
                occurrences: matching.into_iter().map(|c| c.occurrence).collect(),
            });
        }
    }
    let accepted = measurements.len() == reference.rows.len();
    let report = ValidationReport {
        input_sha256: input_hash,
        source_reference_sha256: reference.source_reference_sha256.clone(),
        work: reference.work.clone(),
        expected_records: reference.rows.len(),
        matched_records: measurements.len(),
        matched_occurrences: total_occurrences,
        accepted,
        coordinate_tolerance: decoder::COORDINATE_TOLERANCE,
        active_sector_selection_verified: false,
        filesystem_checksum_verified: false,
        rows: reports,
    };
    Ok((report, measurements))
}

pub fn decode(bytes: &[u8], profile: &Profile) -> Result<(ValidationReport, Vec<Measurement>)> {
    if profile.schema != "fc2-validation-profile-v1"
        || profile.layout != LAYOUT
        || !profile.validation.accepted
        || profile.validation.input_sha256 != hash(bytes)
        || profile.validation.source_reference_sha256 != profile.reference.source_reference_sha256
    {
        return Err(Error::Validation(
            "profile rejected, incompatible, or belongs to a different input".into(),
        ));
    }
    let (report, measurements) = validate(bytes, &profile.reference)?;
    if !report.accepted {
        return Err(Error::Validation(format!(
            "only {}/{} reference records match; no scientific output written",
            report.matched_records, report.expected_records
        )));
    }
    Ok((report, measurements))
}
