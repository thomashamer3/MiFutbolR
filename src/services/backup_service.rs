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

/// Cuenta dueña de un backup por el sufijo `_u<id>.db` (`None` = invitado / sin cuenta).
#[must_use]
pub fn propietario(nombre: &str) -> Option<i64> {
    let base = nombre.strip_suffix(".db")?;
    let (_, id) = base.rsplit_once("_u")?;
    (!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
        .then(|| id.parse().ok())
        .flatten()
}

/// Sufijo con el que una cuenta firma sus backups (`""` sin cuenta).
#[must_use]
pub fn sufijo_cuenta(usuario: Option<i64>) -> String {
    usuario.map_or_else(String::new, |id| format!("_u{id}"))
}

/// Backups `backup_*.db` de `dir` que pertenecen a `usuario` (por el sufijo
/// `_u<id>`; sin cuenta, solo los que no llevan sufijo), el más reciente primero.
/// `Exportaciones/` es compartida entre perfiles: sin este filtro una cuenta
/// vacía podía restaurar la base de otra.
#[must_use]
pub fn listar(dir: &Path, usuario: Option<i64>) -> Vec<BackupInfo> {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut backups: Vec<BackupInfo> = entradas
        .filter_map(std::result::Result::ok)
        .map(|entrada| entrada.path())
        .filter(|ruta| es_backup(ruta))
        .filter(|ruta| {
            ruta.file_name()
                .is_some_and(|n| propietario(&n.to_string_lossy()) == usuario)
        })
        .map(|ruta| {
            let meta = std::fs::metadata(&ruta).ok();
            let fecha = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| jiff::ZonedDateTime::try_from(t).ok())
                .map(|t| t.strftime("%d/%m/%Y %H:%M").to_string())
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

/// Días distintos que conserva el backup de inicio (`BACKUP_HISTORIAL_MAX` de `db.c`).
pub const DIAS_BACKUP_INICIO: usize = 7;

/// Backup al iniciar (`backup_al_iniciar` de `db.c`): si la base tiene datos,
/// la copia a `dir/backup_<AAAAMMDD_HHMMSS>_<etiqueta>.db`. Como C, la copia de
/// hoy reemplaza a la anterior del mismo día y quedan los últimos 7 días en que
/// se abrió la app. Base sin datos: no hace nada, así no pisa un backup bueno.
pub fn backup_inicio(
    db: &crate::db::Database,
    dir: &Path,
    etiqueta: &str,
    ahora: jiff::civil::DateTime,
) -> Result<Option<PathBuf>, String> {
    if !crate::db::tiene_datos_usuario(db.conn()) {
        return Ok(None);
    }
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let sufijo = format!("_{etiqueta}.db");
    let destino = dir.join(format!(
        "backup_{}{sufijo}",
        ahora.strftime("%Y%m%d_%H%M%S")
    ));
    if !destino.exists() {
        db.backup(&destino).map_err(|error| error.to_string())?;
    }
    // Se borra recién después de copiar: si la copia falla, quedan las anteriores.
    let mut nombres: Vec<String> = std::fs::read_dir(dir)
        .map_err(|error| error.to_string())?
        .filter_map(std::result::Result::ok)
        .map(|entrada| entrada.file_name().to_string_lossy().into_owned())
        .filter(|nombre| nombre.starts_with("backup_") && nombre.ends_with(&sufijo))
        .collect();
    nombres.sort_by(|a, b| b.cmp(a));
    let mut dias: Vec<&str> = Vec::new();
    for nombre in &nombres {
        let dia = nombre.get(7..15).unwrap_or("");
        if dias.contains(&dia) || dias.len() >= DIAS_BACKUP_INICIO {
            let _ = std::fs::remove_file(dir.join(nombre));
        } else {
            dias.push(dia);
        }
    }
    Ok(Some(destino))
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

        let lista = listar(dir.path(), None);
        assert_eq!(lista.len(), 2);
        assert_eq!(lista[0].nombre, "backup_20260201_100000.db");
        assert_eq!(lista[0].partidos, Some(1));
        assert_eq!(lista[1].partidos, Some(0));
        assert!(lista[0].bytes > 0 && !lista[0].fecha.is_empty());

        assert!(eliminar(&dir.path().join("otro.db")).is_err());
        eliminar(&lista[1].ruta).unwrap();
        assert_eq!(listar(dir.path(), None).len(), 1);
        assert_eq!(tamano(2048), "2 KB");
        assert_eq!(tamano(3 * 1024 * 1024), "3.0 MB");
    }

    fn fecha(dia: u32, hora: i8) -> jiff::civil::DateTime {
        crate::domain::dates::fecha(2026, 3, dia)
            .unwrap()
            .at(hora, 0, 0, 0)
    }

    #[test]
    fn backup_inicio_omite_base_vacia_y_conserva_siete_dias() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open_in_memory().unwrap();
        assert_eq!(
            backup_inicio(&db, dir.path(), "inicio", fecha(1, 10)).unwrap(),
            None
        );
        db.conn()
            .execute("INSERT INTO cancha (nombre) VALUES ('C')", [])
            .unwrap();
        // Otro perfil y un backup manual no se tocan.
        std::fs::write(dir.path().join("backup_20260101_000000_inicio_u2.db"), b"x").unwrap();
        std::fs::write(dir.path().join("backup_20260101_000000.db"), b"x").unwrap();
        for dia in 1..=9 {
            backup_inicio(&db, dir.path(), "inicio", fecha(dia, 10)).unwrap();
        }
        // Segundo arranque del día 9: reemplaza al de las 10.
        let ultimo = backup_inicio(&db, dir.path(), "inicio", fecha(9, 18))
            .unwrap()
            .unwrap();
        let nombres: Vec<String> = listar(dir.path(), None)
            .into_iter()
            .map(|b| b.nombre)
            .collect();
        let propios: Vec<&String> = nombres
            .iter()
            .filter(|n| n.ends_with("_inicio.db"))
            .collect();
        assert_eq!(propios.len(), DIAS_BACKUP_INICIO, "{nombres:?}");
        assert_eq!(propios[0], "backup_20260309_180000_inicio.db");
        assert_eq!(propios[6], "backup_20260303_100000_inicio.db");
        assert!(ultimo.exists());
        assert!(
            dir.path()
                .join("backup_20260101_000000_inicio_u2.db")
                .exists()
        );
        assert!(nombres.contains(&"backup_20260101_000000.db".to_string()));
        // El restaurar al inicio (B2) toma el más reciente: el de inicio de hoy.
        assert_eq!(nombres[0], "backup_20260309_180000_inicio.db");
    }

    #[test]
    fn cada_cuenta_ve_solo_sus_backups() {
        let dir = tempfile::tempdir().unwrap();
        for nombre in [
            "backup_20260101_000000.db",
            "backup_20260102_000000_inicio_u1.db",
            "backup_20260103_000000_u2.db",
            "backup_20260104_000000_inicio_u12.db",
        ] {
            std::fs::write(dir.path().join(nombre), b"x").unwrap();
        }
        let nombres = |usuario| -> Vec<String> {
            listar(dir.path(), usuario)
                .into_iter()
                .map(|b| b.nombre)
                .collect()
        };
        assert_eq!(nombres(None), ["backup_20260101_000000.db"]);
        assert_eq!(nombres(Some(1)), ["backup_20260102_000000_inicio_u1.db"]);
        assert_eq!(nombres(Some(2)), ["backup_20260103_000000_u2.db"]);
        assert_eq!(nombres(Some(12)), ["backup_20260104_000000_inicio_u12.db"]);
        assert!(nombres(Some(3)).is_empty());
        assert_eq!(propietario("backup_x_uX.db"), None);
        assert_eq!(sufijo_cuenta(Some(7)), "_u7");
    }
}
