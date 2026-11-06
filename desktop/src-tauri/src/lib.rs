//! Presentation models retain source identity and defer all decoding/export to fc2-rs.
use fc2_rs::{explorer::LoadedDataset, export::works::ExportMode, workspace::Work};
use serde::Serialize;
use std::{path::Path, sync::Arc};

#[derive(Serialize)]
pub struct WorkSummary {
    pub key: usize,
    pub number: u16,
    pub name: String,
    pub records: usize,
    pub groups: usize,
    pub points: usize,
    pub coordinates: usize,
    pub warnings: usize,
    pub errors: Vec<String>,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
}

#[derive(Serialize)]
pub struct SourceSummary {
    pub name: String,
    pub sha256: String,
    pub size: usize,
    pub records: usize,
    pub groups: usize,
    pub coordinates: usize,
    pub limitations: Vec<String>,
    pub works: Vec<WorkSummary>,
}

pub fn summary(loaded: &LoadedDataset) -> SourceSummary {
    SourceSummary {
        name: loaded
            .source_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        sha256: loaded.dataset.source_sha256.clone(),
        size: loaded.source_bytes.len(),
        records: loaded.summaries.iter().map(|s| s.records).sum(),
        groups: loaded.summaries.iter().map(|s| s.groups).sum(),
        coordinates: loaded.locations.len(),
        limitations: loaded.dataset.limitations.clone(),
        works: loaded
            .dataset
            .works
            .iter()
            .enumerate()
            .map(|(key, work)| {
                let mut dates = work
                    .groups
                    .iter()
                    .flat_map(|g| &g.records)
                    .filter_map(|r| r.date.as_ref())
                    .collect::<Vec<_>>();
                dates.sort();
                WorkSummary {
                    key,
                    number: work.number,
                    name: work.display_name.clone(),
                    records: work.record_count(),
                    groups: work.groups.len(),
                    points: work.point_count(),
                    coordinates: loaded.summaries[key].coordinate_records,
                    warnings: work.warning_count(),
                    errors: work.errors.clone(),
                    first_date: dates.first().map(|s| (*s).clone()),
                    last_date: dates.last().map(|s| (*s).clone()),
                }
            })
            .collect(),
    }
}

pub fn work_detail(
    loaded: &LoadedDataset,
    source: &str,
    key: usize,
) -> Result<serde_json::Value, String> {
    check_source(loaded, source)?;
    let work: &Work = loaded.work(key).ok_or("Trabalho não encontrado.")?;
    serde_json::to_value(work).map_err(|e| e.to_string())
}

fn check_source(loaded: &LoadedDataset, source: &str) -> Result<(), String> {
    if loaded.dataset.source_sha256 != source {
        return Err("O arquivo aberto mudou. Revise a seleção antes de continuar.".into());
    }
    Ok(())
}

#[derive(Serialize)]
pub struct ExportResult {
    pub path: String,
    pub workbooks: usize,
    pub failures: usize,
    pub errors: Vec<String>,
    pub records: usize,
}

pub fn export(
    loaded: Arc<LoadedDataset>,
    source: &str,
    keys: &[usize],
    mode: &str,
    path: &Path,
) -> Result<ExportResult, String> {
    check_source(&loaded, source)?;
    if mode == "json" {
        if keys.len() != 1 {
            return Err("Selecione um trabalho para exportar JSON de auditoria.".into());
        }
        loaded
            .export_json(keys[0], path)
            .map_err(|e| e.to_string())?;
        return Ok(ExportResult {
            path: path.display().to_string(),
            workbooks: 0,
            failures: 0,
            errors: vec![],
            records: loaded.work(keys[0]).map_or(0, |w| w.record_count()),
        });
    }
    let mode = match mode {
        "simple" => ExportMode::Simple,
        "complete" => ExportMode::Complete,
        _ => return Err("Formato de exportação desconhecido.".into()),
    };
    let report = loaded.export(keys, path, mode).map_err(|e| e.to_string())?;
    Ok(ExportResult {
        path: path.display().to_string(),
        workbooks: report.successful_workbooks,
        failures: report.failures,
        records: report.record_count,
        errors: report
            .works
            .iter()
            .filter_map(|w| w.error.as_ref().map(|e| format!("{}: {e}", w.number)))
            .collect(),
    })
}

#[cfg(feature = "desktop")]
pub mod runtime;
