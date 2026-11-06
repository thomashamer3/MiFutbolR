//! Orquestación de las estadísticas (§14).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::estadisticas_repo;
use crate::domain::models::Estadisticas;

/// Error del servicio.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
}

/// Carga todas las estadísticas en un solo paquete.
pub fn cargar(conn: &Connection) -> Result<Estadisticas, ServiceError> {
    Ok(Estadisticas {
        generales: estadisticas_repo::generales(conn)?,
        por_mes: estadisticas_repo::por_mes(conn)?,
        por_anio: estadisticas_repo::por_anio(conn)?,
        records: estadisticas_repo::records(conn)?,
        rendimiento: estadisticas_repo::rendimiento(conn)?,
        canchas: estadisticas_repo::canchas(conn)?,
        goles: estadisticas_repo::goles(conn)?,
    })
}

/// `true` si hay al menos un partido cargado.
pub fn hay_datos(conn: &Connection) -> Result<bool, ServiceError> {
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM partido", [], |row| row.get(0))
        .map_err(DbError::from)?;
    Ok(total > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn preparar() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute(
                "INSERT INTO cancha (nombre, localidad) VALUES ('Sur', 'Quilmes')",
                [],
            )
            .unwrap();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
            .unwrap();
        for (fecha, goles, asistencias, resultado, rendimiento) in [
            ("2026-03-10 20:00:00", 2, 1, 1, 7),
            ("2026-03-20 21:00:00", 1, 0, 3, 5),
            ("2026-04-05 10:00:00", 3, 2, 1, 9),
        ] {
            db.conn()
                .execute(
                    "INSERT INTO partido
                        (cancha_id, camiseta_id, fecha_hora, mes_anio, goles, asistencias,
                         resultado, rendimiento_general, clima, dia, tipo_partido)
                     VALUES (1, 1, ?1, ?2, ?3, ?4, ?5, ?6, 1, 2, 1)",
                    rusqlite::params![
                        fecha,
                        &fecha[..7],
                        goles,
                        asistencias,
                        resultado,
                        rendimiento
                    ],
                )
                .unwrap();
        }
        db
    }

    #[test]
    fn carga_totales_y_series() {
        let db = preparar();
        let stats = cargar(db.conn()).unwrap();
        assert_eq!(stats.generales.total, 3);
        assert_eq!(stats.generales.goles, 6);
        assert_eq!(stats.generales.asistencias, 3);
        assert_eq!(stats.generales.victorias, 2);
        assert_eq!(stats.generales.derrotas, 1);
        assert_eq!(stats.por_mes.len(), 2);
        assert_eq!(stats.por_anio.len(), 1);
        assert_eq!(stats.records.len(), 5);
        assert!(stats.generales.mejor_partido.is_some());
    }

    #[test]
    fn carga_rendimiento_canchas_y_goles() {
        let db = preparar();
        let stats = cargar(db.conn()).unwrap();
        assert_eq!(stats.rendimiento.por_clima.len(), 1);
        assert_eq!(stats.rendimiento.por_dia_semana.len(), 3);
        assert_eq!(stats.rendimiento.por_franja.len(), 1);
        assert_eq!(stats.rendimiento.tendencia.len(), 3);
        assert_eq!(stats.canchas.total, 1);
        assert_eq!(stats.canchas.partidos, 3);
        assert_eq!(stats.canchas.ranking[0].partidos, 3);
        assert!((stats.goles.promedio_goles - 2.0).abs() < f64::EPSILON);
        assert!((stats.goles.efectividad - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_database_reports_no_data() {
        let db = Database::open_in_memory().unwrap();
        assert!(!hay_datos(db.conn()).unwrap());
        let stats = cargar(db.conn()).unwrap();
        assert_eq!(stats.generales.total, 0);
        assert!(stats.por_mes.is_empty());
    }
}
