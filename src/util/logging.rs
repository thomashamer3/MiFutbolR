//! Log mínimo a archivo (`mifutbol.log`).
//!
//! Se evita una dependencia extra de logging: el volumen esperado es bajo y
//! solo se registran errores y arranques.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static LOG: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();

/// Inicializa el log en la ruta indicada. Si falla, la app sigue funcionando.
pub fn init(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let _ = LOG.set(Mutex::new(Some(file)));
    info("MiFutbolR iniciado");
    Ok(())
}

/// Escribe una línea con timestamp.
pub fn write_line(level: &str, message: &str) {
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{stamp}] [{level}] {message}\n");
    if let Some(lock) = LOG.get()
        && let Ok(mut guard) = lock.lock()
        && let Some(file) = guard.as_mut()
    {
        let _ = file.write_all(line.as_bytes());
        let _ = file.flush();
    }
}

/// Registra información.
pub fn info(message: &str) {
    write_line("INFO", message);
}

/// Registra un error.
pub fn error(message: &str) {
    write_line("ERROR", message);
}
