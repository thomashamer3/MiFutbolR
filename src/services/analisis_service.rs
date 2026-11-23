//! Análisis básico, evolución mensual y química (port de `analisis.c`).

use rusqlite::Connection;

use crate::db::{DbError, analisis_repo, quimica_repo};
use crate::domain::analisis::{self, AnalisisBasico, Evolucion};
use crate::domain::quimica::MejorDupla;

/// Análisis básico; `None` sin partidos.
pub fn basico(conn: &Connection) -> Result<Option<AnalisisBasico>, DbError> {
    Ok(analisis::analisis_basico(&analisis_repo::serie(conn)?))
}

/// Evolución temporal; `None` sin partidos.
pub fn evolucion(conn: &Connection) -> Result<Option<Evolucion>, DbError> {
    Ok(analisis::evolucion(&analisis_repo::serie(conn)?))
}

/// Mejor combinación de jugadores según los equipos asignados a partidos.
pub fn mejor_combinacion(conn: &Connection) -> Result<Option<MejorDupla>, DbError> {
    quimica_repo::mejor_combinacion(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn partido(db: &Database, fecha: &str, goles: i64, resultado: i64) {
        db.conn()
            .execute(
                "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, goles, resultado,
                                      rendimiento_general)
                 VALUES (1, 1, ?1, ?2, ?3, 7)",
                rusqlite::params![fecha, goles, resultado],
            )
            .unwrap();
    }

    #[test]
    fn basico_y_evolucion_desde_la_base() {
        let db = Database::open_in_memory().unwrap();
        assert!(basico(db.conn()).unwrap().is_none());
        assert!(evolucion(db.conn()).unwrap().is_none());
        db.conn()
            .execute_batch(
                "INSERT INTO cancha (nombre) VALUES ('C');
                 INSERT INTO camiseta (nombre) VALUES ('K');",
            )
            .unwrap();
        partido(&db, "2026-02-01 20:00:00", 2, 1);
        partido(&db, "2026-01-01 20:00:00", 0, 3);
        let b = basico(db.conn()).unwrap().unwrap();
        // El más reciente primero aunque se insertó antes.
        assert_eq!(b.ultimos[0].fecha_hora, "2026-02-01 20:00:00");
        assert_eq!((b.mejor_racha_victorias, b.peor_racha_derrotas), (1, 1));
        let e = evolucion(db.conn()).unwrap().unwrap();
        assert_eq!(e.meses.len(), 2);
        assert_eq!(e.meses[0].mes, 2);
    }

    #[test]
    fn mejor_combinacion_por_winrate() {
        let db = Database::open_in_memory().unwrap();
        assert!(mejor_combinacion(db.conn()).unwrap().is_none());
        db.conn()
            .execute_batch(
                "INSERT INTO cancha (nombre) VALUES ('C');
                 INSERT INTO camiseta (nombre) VALUES ('K');",
            )
            .unwrap();
        // Partido 1 ganado con Ana+Beto; partido 2 perdido con Ana+Caro.
        partido(&db, "2026-01-01 20:00:00", 1, 1);
        partido(&db, "2026-01-08 20:00:00", 0, 3);
        db.conn()
            .execute_batch(
                "INSERT INTO equipo (nombre, partido_id) VALUES ('E1', 1), ('E2', 2);
                 INSERT INTO jugador (equipo_id, nombre, numero, posicion) VALUES
                    (1, 'Ana', 1, 1), (1, 'Beto', 2, 2), (2, 'Ana', 1, 1), (2, 'Caro', 3, 3);",
            )
            .unwrap();
        let mejor = mejor_combinacion(db.conn()).unwrap().unwrap();
        assert_eq!(
            (mejor.jugador_a.as_str(), mejor.jugador_b.as_str()),
            ("Ana", "Beto")
        );
        assert_eq!((mejor.victorias, mejor.derrotas, mejor.empates), (1, 0, 0));
        assert!((mejor.winrate() - 100.0).abs() < f64::EPSILON);
    }
}
