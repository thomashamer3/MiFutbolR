//! Capa de acceso a datos: apertura de SQLite, PRAGMAs y migraciones.
//!
//! La UI nunca ejecuta SQL directamente (§4.1): los repositorios viven aquí y
//! los servicios orquestan transacciones.

pub mod amigo_repo;
pub mod botin_repo;
pub mod busqueda_repo;
pub mod calendario_repo;
pub mod camiseta_repo;
pub mod cancha_repo;
pub mod carrera_repo;
pub mod comparador_repo;
pub mod equipo_repo;
pub mod error;
pub mod estadisticas_repo;
pub mod financiamiento_repo;
pub mod helpers;
pub mod inicio_repo;
pub mod lesion_repo;
pub mod logros_repo;
pub mod media_repo;
pub mod notificacion_repo;
pub mod partido_repo;
pub mod progresion_repo;
pub mod quimica_repo;
pub mod recordatorio_repo;
pub mod records_repo;
pub mod resumen_repo;
pub mod settings_repo;
pub mod temporada_repo;
pub mod tienda_repo;
pub mod torneo_repo;
pub mod ultimos_repo;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

pub use error::DbError;

use crate::domain::models::AppSettings;

/// Migraciones embebidas en el binario (§20.8). El orden es el orden de
/// aplicación y cada archivo es idempotente.
const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/001_init.sql"),
    include_str!("../../migrations/002_indexes.sql"),
    include_str!("../../migrations/003_seed.sql"),
    include_str!("../../migrations/004_ola0.sql"),
    include_str!("../../migrations/005_undo.sql"),
    include_str!("../../migrations/006_ola1.sql"),
    include_str!("../../migrations/007_ola1_temporadas.sql"),
    include_str!("../../migrations/008_ola2.sql"),
    include_str!("../../migrations/009_ola2_recordatorios.sql"),
    include_str!("../../migrations/010_ola2_quimica.sql"),
];

/// Versión de esquema que espera esta compilación.
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// Conexión SQLite administrada por la aplicación.
pub struct Database {
    conn: Connection,
    path: Option<PathBuf>,
}

impl Database {
    /// Abre (o crea) la base de datos en `path` y aplica migraciones.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }

        let existed = path.exists();
        let conn = Connection::open(path)?;
        configure(&conn)?;

        if existed {
            // Backup best-effort antes de migrar (§20.8). Si falla, se sigue:
            // la migración es idempotente y el usuario puede exportar desde Ajustes.
            let version = schema_version(&conn).unwrap_or(0);
            if version > 0 && version < SCHEMA_VERSION {
                let _ = vacuum_into(&conn, &backup_path(path));
            }
        }

        migrate(&conn)?;
        Ok(Self {
            conn,
            path: Some(path.to_path_buf()),
        })
    }

    /// Base de datos en memoria para tests.
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        configure(&conn)?;
        migrate(&conn)?;
        Ok(Self { conn, path: None })
    }

    /// Conexión subyacente (solo lectura).
    #[must_use]
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Conexión subyacente mutable (transacciones).
    #[must_use]
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Ruta del archivo, si no es en memoria.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Carga las preferencias persistidas.
    pub fn settings(&self) -> Result<AppSettings, DbError> {
        settings_repo::load(&self.conn)
    }

    /// Guarda las preferencias.
    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), DbError> {
        settings_repo::save(&self.conn, settings)
    }

    /// Copia consistente de la base usando `VACUUM INTO`.
    pub fn backup(&self, destination: &Path) -> Result<(), DbError> {
        vacuum_into(&self.conn, destination)
    }

    /// Versión actual del esquema (`PRAGMA user_version`).
    pub fn version(&self) -> Result<i64, DbError> {
        schema_version(&self.conn)
    }

    /// Restaura el contenido desde un archivo de backup sobre la conexión viva.
    pub fn restore_from(&mut self, source: &Path) -> Result<(), DbError> {
        let origen =
            Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let backup = rusqlite::backup::Backup::new(&origen, &mut self.conn)?;
        backup.run_to_completion(100, Duration::from_millis(0), None)?;
        Ok(())
    }

    /// Ejecuta las comprobaciones de integridad (§18.5) y devuelve un resumen.
    pub fn integrity(&self) -> Result<String, DbError> {
        let mut stmt = self.conn.prepare("PRAGMA integrity_check")?;
        let resultados = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let integridad = if resultados.iter().all(|linea| linea == "ok") {
            "ok".to_string()
        } else {
            resultados.join("; ")
        };

        let mut stmt = self.conn.prepare("PRAGMA foreign_key_check")?;
        let fk_errores = stmt
            .query_map([], |_| Ok(()))?
            .collect::<Result<Vec<_>, _>>()?
            .len();

        let mut conteos = Vec::new();
        for tabla in [
            "camiseta",
            "cancha",
            "botin",
            "equipo",
            "jugador",
            "partido",
            "lesion",
            "financiamiento",
            "presupuesto_mensual",
            "carrera_partido_hito",
        ] {
            let sql = format!("SELECT COUNT(*) FROM {tabla}");
            let total: i64 = self.conn.query_row(&sql, [], |row| row.get(0))?;
            conteos.push(format!("{tabla}={total}"));
        }

        Ok(format!(
            "Integridad: {integridad}. Claves foráneas inválidas: {fk_errores}. Registros: {}",
            conteos.join(", ")
        ))
    }
}

/// PRAGMAs de apertura (§6.1, Performance.md §2).
fn configure(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;",
    )?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}

/// Aplica las migraciones pendientes según `PRAGMA user_version`.
pub fn migrate(conn: &Connection) -> Result<(), DbError> {
    let version = schema_version(conn)?;
    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let target = index as i64 + 1;
        if version < target {
            conn.execute_batch(sql)
                .map_err(|error| DbError::Migration(format!("migración {target}: {error}")))?;
            conn.pragma_update(None, "user_version", target)?;
        }
    }
    Ok(())
}

/// Lee `PRAGMA user_version`.
pub fn schema_version(conn: &Connection) -> Result<i64, DbError> {
    let version = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version)
}

fn vacuum_into(conn: &Connection, destination: &Path) -> Result<(), DbError> {
    if destination.exists() {
        std::fs::remove_file(destination)?;
    }
    if let Some(parent) = destination.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    // `VACUUM INTO` genera una copia consistente sin bloquear lectores.
    conn.execute("VACUUM INTO ?1", [destination.to_string_lossy().as_ref()])?;
    Ok(())
}

fn backup_path(path: &Path) -> PathBuf {
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let name = path.file_name().map_or_else(
        || "mifutbol.db".to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    path.with_file_name(format!("{name}.bak_{stamp}.db"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.version().unwrap(), SCHEMA_VERSION);
        // Reaplicar no debe fallar ni duplicar seeds.
        migrate(db.conn()).unwrap();
        let settings = db.settings().unwrap();
        assert_eq!(settings.theme, crate::domain::models::Theme::Light);
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM custom_menus", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 11);
    }

    #[test]
    fn schema_has_all_spec_tables() {
        let db = Database::open_in_memory().unwrap();
        let mut stmt = db
            .conn()
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for expected in [
            "camiseta",
            "cancha",
            "botin",
            "equipo",
            "jugador",
            "partido",
            "lesion",
            "financiamiento",
            "presupuesto_mensual",
            "carrera_identidad",
            "carrera_partido_hito",
            "entrenamiento_plan",
            "progresion_jugador",
            "torneo",
            "temporada",
            "settings",
            "usuario",
            "custom_menus",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "falta tabla {expected}"
            );
        }
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let db = Database::open_in_memory().unwrap();
        let result = db.conn().execute(
            "INSERT INTO jugador (equipo_id, nombre, numero, posicion) VALUES (999, 'X', 1, 0)",
            [],
        );
        assert!(result.is_err());
    }
}
