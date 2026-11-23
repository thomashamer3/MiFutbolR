//! Administración de backups `backup_*.db` (port de `backup.c`: listar, restaurar, eliminar).

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

/// Backup encontrado en la carpeta de exportaciones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupInfo {
    /// Ruta completa.
    pub ruta: PathBuf,
    /// Nombre del archivo.
    pub nombre: String,
    /// Fecha de modificación `DD/MM/YYYY HH:MM`.
    pub fecha: String,
    /// Tamaño en bytes.
    pub bytes: u64,
    /// Partidos que contiene (`None` si no se pudo abrir).
    pub partidos: Option<i64>,
}

fn es_backup(ruta: &Path) -> bool {
    ruta.extension().is_some_and(|ext| ext == "db")
        && ruta
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("backup_"))
}

/// Backups `backup_*.db` de `dir`, el más reciente primero (el nombre lleva la fecha).
#[must_use]
pub fn listar(dir: &Path) -> Vec<BackupInfo> {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut backups: Vec<BackupInfo> = entradas
        .filter_map(std::result::Result::ok)
        .map(|entrada| entrada.path())
        .filter(|ruta| es_backup(ruta))
        .map(|ruta| {
            let meta = std::fs::metadata(&ruta).ok();
            let fecha = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .format("%d/%m/%Y %H:%M")
                        .to_string()
                })
                .unwrap_or_default();
            let partidos = Connection::open_with_flags(&ruta, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .and_then(|c| c.query_row("SELECT COUNT(*) FROM partido", [], |row| row.get(0)))
                .ok();
            BackupInfo {
                nombre: ruta
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                fecha,
                bytes: meta.map_or(0, |m| m.len()),
                partidos,
                ruta,
            }
        })
        .collect();
    backups.sort_by(|a, b| b.nombre.cmp(&a.nombre));
    backups
}

/// Borra un backup; rechaza rutas que no sean `backup_*.db`.
pub fn eliminar(ruta: &Path) -> std::io::Result<()> {
    if !es_backup(ruta) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Solo se pueden eliminar archivos backup_*.db",
        ));
    }
    std::fs::remove_file(ruta)
}

/// "1.5 MB", "320 KB".
#[must_use]
pub fn tamano(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn lista_ordena_y_elimina() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open_in_memory().unwrap();
        db.backup(&dir.path().join("backup_20260101_100000.db"))
            .unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO cancha (nombre) VALUES ('C');
                 INSERT INTO camiseta (nombre) VALUES ('K');
                 INSERT INTO partido (cancha_id, camiseta_id, fecha_hora)
                     VALUES (1, 1, '2026-01-01 20:00:00');",
            )
            .unwrap();
        db.backup(&dir.path().join("backup_20260201_100000.db"))
            .unwrap();
        std::fs::write(dir.path().join("otro.db"), b"x").unwrap();

        let lista = listar(dir.path());
        assert_eq!(lista.len(), 2);
        assert_eq!(lista[0].nombre, "backup_20260201_100000.db");
        assert_eq!(lista[0].partidos, Some(1));
        assert_eq!(lista[1].partidos, Some(0));
        assert!(lista[0].bytes > 0 && !lista[0].fecha.is_empty());

        assert!(eliminar(&dir.path().join("otro.db")).is_err());
        eliminar(&lista[1].ruta).unwrap();
        assert_eq!(listar(dir.path()).len(), 1);
        assert_eq!(tamano(2048), "2 KB");
        assert_eq!(tamano(3 * 1024 * 1024), "3.0 MB");
    }
}
