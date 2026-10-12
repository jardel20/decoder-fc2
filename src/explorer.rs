//! Presentation-neutral, indexed access to one immutable decoded FC2 source.
use crate::{
    error::{Error, Result},
    export::{
        self,
        works::{ExportMode, Manifest, WorkbookOrder},
    },
    reader,
    work_records::WorkMeasurement,
    workspace::{self, Dataset, Work},
};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub type WorkKey = usize;

/// Stable source-order indexes, independent of potentially duplicated stored IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordKey {
    pub work: WorkKey,
    pub group: usize,
    pub record: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoPoint {
    pub key: RecordKey,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkSummary {
    pub records: usize,
    pub groups: usize,
    pub coordinate_records: usize,
    pub warned_records: usize,
}

/// Wrap in `Arc` when sharing with background workers; fields retain source evidence.
#[derive(Debug)]
pub struct LoadedDataset {
    pub source_path: PathBuf,
    pub source_bytes: Vec<u8>,
    pub dataset: Dataset,
    pub summaries: Vec<WorkSummary>,
    pub records: Vec<Vec<RecordKey>>,
    pub locations: Vec<GeoPoint>,
}

impl LoadedDataset {
    pub fn load(path: &Path) -> Result<Self> {
        Self::from_bytes(path.to_owned(), reader::read_file(path)?)
    }

    pub fn from_bytes(source_path: PathBuf, source_bytes: Vec<u8>) -> Result<Self> {
        if source_bytes.len() > reader::MAX_FILE_SIZE {
            return Err(Error::Format("input exceeds the 64 MiB limit".into()));
        }
        let dataset = workspace::decode(&source_bytes)?;
        let mut summaries = Vec::with_capacity(dataset.works.len());
        let mut records = Vec::with_capacity(dataset.works.len());
        let mut locations = Vec::new();
        for (work_index, work) in dataset.works.iter().enumerate() {
            let mut keys = Vec::with_capacity(work.record_count());
            let mut summary = WorkSummary {
                groups: work.groups.len(),
                ..Default::default()
            };
            for (group_index, group) in work.groups.iter().enumerate() {
                for (record_index, record) in group.records.iter().enumerate() {
                    let key = RecordKey {
                        work: work_index,
                        group: group_index,
                        record: record_index,
                    };
                    keys.push(key);
                    summary.records += 1;
                    summary.warned_records += usize::from(!record.warnings.is_empty());
                    if let (Some(latitude), Some(longitude)) = (record.latitude, record.longitude) {
                        if latitude.is_finite()
                            && longitude.is_finite()
                            && latitude.abs() <= 90.0
                            && longitude.abs() <= 180.0
                        {
                            locations.push(GeoPoint {
                                key,
                                latitude,
                                longitude,
                            });
                            summary.coordinate_records += 1;
                        }
                    }
                }
            }
            summaries.push(summary);
            records.push(keys);
        }
        Ok(Self {
            source_path,
            source_bytes,
            dataset,
            summaries,
            records,
            locations,
        })
    }

    pub fn work(&self, key: WorkKey) -> Option<&Work> {
        self.dataset.works.get(key)
    }

    pub fn record(&self, key: RecordKey) -> Option<&WorkMeasurement> {
        self.work(key.work)?
            .groups
            .get(key.group)?
            .records
            .get(key.record)
    }

    /// Preserve existing XLSX validation and its per-work failure reporting.
    pub fn export(&self, work_keys: &[WorkKey], path: &Path, mode: ExportMode) -> Result<Manifest> {
        if work_keys.is_empty() {
            return Err(Error::Validation("work selection must not be empty".into()));
        }
        let mut seen = BTreeSet::new();
        let numbers = work_keys
            .iter()
            .map(|&key| {
                if !seen.insert(key) {
                    return Err(Error::Validation(
                        "work selection contains a duplicate index".into(),
                    ));
                }
                self.work(key)
                    .map(|work| work.number)
                    .ok_or_else(|| Error::Validation(format!("unknown work index {key}")))
            })
            .collect::<Result<Vec<_>>>()?;
        export::works::export_directory_options(
            &self.source_bytes,
            &self.dataset,
            &self
                .source_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            path,
            Some(&numbers),
            WorkbookOrder::Leaf,
            mode,
        )
    }

    /// Structural audit JSON, explicitly distinct from reference-validated scientific CSV.
    /// Unknown fields, absent values and record evidence remain in the original model.
    pub fn export_json(&self, work_key: WorkKey, path: &Path) -> Result<()> {
        let work = self
            .work(work_key)
            .ok_or_else(|| Error::Validation(format!("unknown work index {work_key}")))?;
        if reader::hash(&self.source_bytes) != self.dataset.source_sha256 {
            return Err(Error::Validation(
                "audit export source hash mismatch".into(),
            ));
        }
        let original = workspace::decode(&self.source_bytes)?;
        let expected = original
            .works
            .get(work_key)
            .ok_or_else(|| Error::Validation("source work is absent".into()))?;
        // A profile may annotate an unchanged work, but must not conceal mutated source fields.
        let source_fields = |value: &Work| -> Result<serde_json::Value> {
            let mut value = serde_json::to_value(value)?;
            if let Some(object) = value.as_object_mut() {
                object.remove("reference_validation");
            }
            Ok(value)
        };
        if source_fields(work)? != source_fields(expected)? {
            return Err(Error::Validation(
                "work audit differs from original source".into(),
            ));
        }
        #[derive(Serialize)]
        struct Audit<'a> {
            schema: &'static str,
            tool_version: &'static str,
            source_name: String,
            source_sha256: &'a str,
            validation: &'static str,
            coordinate_reference: &'static str,
            limitations: &'a [String],
            work: &'a Work,
        }
        let audit = Audit {
            schema: "fc2-work-structural-audit-v1",
            tool_version: env!("CARGO_PKG_VERSION"),
            source_name: self
                .source_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            source_sha256: &self.dataset.source_sha256,
            validation: "structural audit; scientific equivalence is reference-specific",
            coordinate_reference: "finite latitude/longitude in bounds; datum not confirmed",
            limitations: &self.dataset.limitations,
            work,
        };
        let mut bytes = serde_json::to_vec_pretty(&audit)?;
        bytes.push(b'\n');
        export::write_new(path, &bytes)
    }
}
