//! Preserve native cancellation while exposing backend errors instead of silently cancelling.
use std::{cell::RefCell, path::PathBuf, sync::Once};
thread_local! { static ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) }; }
struct DialogLogger;
static LOGGER: DialogLogger = DialogLogger;
static INIT: Once = Once::new();
impl log::Log for DialogLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn && metadata.target().starts_with("rfd")
    }
    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let message = record.args().to_string();
        eprintln!("Diálogo nativo: {message}");
        if record.level() == log::Level::Error {
            ERRORS.with(|errors| errors.borrow_mut().push(message));
        }
    }
    fn flush(&self) {}
}
pub fn choose(operation: impl FnOnce() -> Option<PathBuf>) -> Result<Option<PathBuf>, String> {
    INIT.call_once(|| {
        if log::set_logger(&LOGGER).is_ok() {
            log::set_max_level(log::LevelFilter::Warn);
        }
    });
    ERRORS.with(|errors| errors.borrow_mut().clear());
    let selected = operation();
    let errors = ERRORS.with(|errors| std::mem::take(&mut *errors.borrow_mut()));
    if selected.is_none() && !errors.is_empty() {
        let error = errors.join("; ");
        return Err(format!(
            "O seletor de arquivos do sistema não pôde abrir: {error}. No Linux, confira o serviço XDG Desktop Portal e seu backend de seleção de arquivos."
        ));
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_is_not_a_backend_failure() {
        assert!(choose(|| None).unwrap().is_none());
    }
    #[test]
    fn backend_failure_is_reported_and_does_not_leak_into_next_dialog() {
        let failed = choose(|| {
            log::error!(target: "rfd::test", "erro inicial do portal");
            log::error!(target: "rfd::test", "backend indisponível");
            None
        });
        let error = failed.unwrap_err();
        assert!(error.contains("backend indisponível"));
        assert!(error.contains("erro inicial do portal"));
        assert!(choose(|| None).unwrap().is_none());
    }
}
