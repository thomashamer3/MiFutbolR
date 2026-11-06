//! Generación y exportación del Resumen Compartible (§17).

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::resumen_repo;
use crate::domain::resumen::{DatosResumen, ResumenFormato};
use crate::services::carrera_service;

/// Error del servicio.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
    /// Carrera.
    #[error(transparent)]
    Carrera(#[from] carrera_service::ServiceError),
    /// No hay partidos.
    #[error("Sin datos suficientes; registra partidos para generar el resumen.")]
    SinDatos,
    /// Error de escritura.
    #[error("No se pudo escribir el resumen: {0}")]
    Io(String),
}

/// Genera los datos del resumen.
pub fn generar(conn: &Connection) -> Result<DatosResumen, ServiceError> {
    let mut datos = resumen_repo::datos(conn)?;
    if datos.total_partidos == 0 {
        return Err(ServiceError::SinDatos);
    }
    let carrera = carrera_service::resumen(conn)?;
    let badges = carrera.badges.iter().filter(|badge| badge.obtenido).count() as i64;
    datos.logros_total = resumen_repo::hitos_count(conn)? + badges;
    Ok(datos)
}

/// Renderiza según el formato.
#[must_use]
pub fn render(datos: &DatosResumen, formato: ResumenFormato) -> String {
    match formato {
        ResumenFormato::Markdown => datos.to_markdown(),
        ResumenFormato::Html => datos.to_html(),
        ResumenFormato::Texto => datos.to_text(),
    }
}

/// Genera y escribe el archivo en `dir`; devuelve la ruta.
pub fn exportar(
    conn: &Connection,
    formato: ResumenFormato,
    dir: &Path,
) -> Result<PathBuf, ServiceError> {
    let datos = generar(conn)?;
    let contenido = render(&datos, formato);
    std::fs::create_dir_all(dir).map_err(|error| ServiceError::Io(error.to_string()))?;
    let ruta = dir.join(datos.nombre_archivo(formato));
    std::fs::write(&ruta, contenido).map_err(|error| ServiceError::Io(error.to_string()))?;
    Ok(ruta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn preparar() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO cancha (nombre) VALUES ('Sur')", [])
            .unwrap();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
            .unwrap();
        for (fecha, goles, resultado, rendimiento) in [
            ("2026-03-01 20:00:00", 3, 1, 9),
            ("2026-03-08 20:00:00", 1, 1, 7),
            ("2026-03-15 20:00:00", 0, 3, 5),
        ] {
            db.conn()
                .execute(
                    "INSERT INTO partido
                        (cancha_id, camiseta_id, fecha_hora, mes_anio, goles, asistencias,
                         resultado, rendimiento_general, clima, dia, tipo_partido)
                     VALUES (1, 1, ?1, ?2, ?3, 1, ?4, ?5, 1, 2, 1)",
                    rusqlite::params![fecha, &fecha[..7], goles, resultado, rendimiento],
                )
                .unwrap();
        }
        db
    }

    #[test]
    fn generates_and_exports() {
        let db = preparar();
        let datos = generar(db.conn()).unwrap();
        assert_eq!(datos.total_partidos, 3);
        assert_eq!(datos.total_goles, 4);
        assert_eq!(datos.mejor.rendimiento, 9);
        assert!(
            datos.racha.descripcion.contains("derrotas")
                || datos.racha.descripcion.contains("Sin racha")
        );

        let dir = tempfile::tempdir().unwrap();
        let ruta = exportar(db.conn(), ResumenFormato::Html, dir.path()).unwrap();
        assert!(ruta.exists());
        let contenido = std::fs::read_to_string(&ruta).unwrap();
        assert!(contenido.contains("<!doctype html>"));
        assert!(contenido.contains("Sur"));
    }

    #[test]
    fn empty_database_is_rejected() {
        let db = Database::open_in_memory().unwrap();
        assert!(matches!(generar(db.conn()), Err(ServiceError::SinDatos)));
    }

    #[test]
    fn all_formats_render() {
        let db = preparar();
        let datos = generar(db.conn()).unwrap();
        assert!(render(&datos, ResumenFormato::Markdown).contains("# Resumen"));
        assert!(render(&datos, ResumenFormato::Texto).contains("RESUMEN DE CARRERA"));
        assert!(render(&datos, ResumenFormato::Html).contains("<html"));
    }
}
