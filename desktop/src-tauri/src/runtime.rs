use crate::{ExportResult, SourceSummary};
use fc2_rs::explorer::LoadedDataset;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Manager, State};

type Session = Mutex<Option<Arc<LoadedDataset>>>;
fn current(state: &Session) -> Result<Arc<LoadedDataset>, String> {
    state
        .lock()
        .map_err(|_| "Sessão indisponível.".to_owned())?
        .clone()
        .ok_or("Abra um arquivo FC2 primeiro.".into())
}

#[tauri::command]
async fn open_source(
    state: State<'_, Session>,
    path: Option<String>,
) -> Result<Option<SourceSummary>, String> {
    let path = if let Some(path) = path {
        PathBuf::from(path)
    } else {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_title("Abrir arquivo FC2")
            .add_filter("Falker FC2", &["FC2", "fc2"])
            .pick_file()
            .await
        else {
            return Ok(None);
        };
        file.path().to_owned()
    };
    let loaded = tauri::async_runtime::spawn_blocking(move || LoadedDataset::load(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("Não foi possível ler o arquivo: {e}"))?;
    let result = crate::summary(&loaded);
    *state
        .lock()
        .map_err(|_| "Sessão indisponível.".to_owned())? = Some(Arc::new(loaded));
    Ok(Some(result))
}

#[tauri::command]
async fn get_work(
    state: State<'_, Session>,
    source: String,
    key: usize,
) -> Result<serde_json::Value, String> {
    let loaded = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || crate::work_detail(&loaded, &source, key))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_locations(
    state: State<'_, Session>,
    source: String,
) -> Result<Vec<crate::Location>, String> {
    let loaded = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || crate::locations(&loaded, &source))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_tile(z: u8, x: u32, y: u32) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || crate::tiles::fetch(z, x, y))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn offline_plan(bounds: crate::tiles::Bounds) -> Result<crate::tiles::Plan, String> {
    crate::tiles::plan(
        bounds,
        crate::tiles::MAX_OFFLINE_ZOOM,
        crate::tiles::MAX_OFFLINE_TILES,
        &crate::tiles::cache_root(),
    )
}

#[tauri::command]
async fn prefetch_tile(z: u8, x: u32, y: u32) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || crate::tiles::ensure(z, x, y))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn export_selection(
    state: State<'_, Session>,
    source: String,
    keys: Vec<usize>,
    mode: String,
) -> Result<Option<ExportResult>, String> {
    let loaded = current(&state)?;
    crate::check_source(&loaded, &source)?;
    if keys.is_empty() || keys.iter().any(|&k| loaded.work(k).is_none()) {
        return Err("Selecione trabalhos válidos antes de exportar.".into());
    }
    if mode == "json" && keys.len() != 1 {
        return Err("JSON de auditoria requer um único trabalho.".into());
    }
    if !["simple", "complete", "json"].contains(&mode.as_str()) {
        return Err("Formato de exportação desconhecido.".into());
    }
    let path = if mode == "json" {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_title("Salvar auditoria JSON")
            .set_file_name(format!(
                "{}-auditoria.json",
                loaded.dataset.works[keys[0]].number
            ))
            .add_filter("JSON", &["json"])
            .save_file()
            .await
        else {
            return Ok(None);
        };
        file.path().to_owned()
    } else {
        let Some(folder) = rfd::AsyncFileDialog::new()
            .set_title("Escolha a pasta de destino")
            .pick_folder()
            .await
        else {
            return Ok(None);
        };
        let stem = loaded
            .source_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        fc2_rs::paths::new_output(folder.path(), &format!("{stem}-trabalhos"))
    };
    tauri::async_runtime::spawn_blocking(move || {
        crate::export(loaded, &source, &keys, &mode, &path)
    })
    .await
    .map_err(|e| e.to_string())?
    .map(Some)
}

#[tauri::command]
fn open_tui(path: Option<String>) -> Result<(), String> {
    crate::terminal::launch(path.as_deref().map(std::path::Path::new))
}

#[tauri::command]
fn initial_path() -> Option<String> {
    std::env::args_os()
        .nth(1)
        .map(|p| p.to_string_lossy().into_owned())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Session::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_source,
            get_work,
            get_locations,
            get_tile,
            offline_plan,
            prefetch_tile,
            export_selection,
            initial_path,
            open_tui
        ])
        .run(tauri::generate_context!())
        .expect("Falha ao iniciar Leaf Explorer");
}
