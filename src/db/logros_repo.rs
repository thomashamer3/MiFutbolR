//! Valores y desbloqueos de logros (IMP-04).

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::models::Resultado;

/// Calcula todas las métricas que usan los logros.
pub fn valores(
    conn: &Connection,
    camiseta_id: Option<i64>,
) -> Result<HashMap<&'static str, i64>, DbError> {
    let (
        partidos,
        goles,
        asistencias,
        victorias,
        empates,
        derrotas,
        contribuciones,
        rendimiento_total,
        animo_total,
        canchas,
        hat_tricks,
        poker,
        perfectos,
        animo_perfecto,
        goles_victorias,
        asistencias_victorias,
    ) = conn.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(goles), 0),
                COALESCE(SUM(asistencias), 0),
                COALESCE(SUM(CASE WHEN resultado = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 2 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 3 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(goles + asistencias), 0),
                COALESCE(SUM(rendimiento_general), 0),
                COALESCE(SUM(estado_animo), 0),
                COALESCE(COUNT(DISTINCT cancha_id), 0),
                COALESCE(SUM(CASE WHEN goles >= 3 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN asistencias >= 4 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN rendimiento_general = 10 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN estado_animo = 10 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 1 THEN goles ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 1 THEN asistencias ELSE 0 END), 0)
           FROM partido
          WHERE (?1 IS NULL OR camiseta_id = ?1)",
        params![camiseta_id],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
                row.get::<_, i64>(12)?,
                row.get::<_, i64>(13)?,
                row.get::<_, i64>(14)?,
                row.get::<_, i64>(15)?,
            ))
        },
    )?;

    let ultimo = conn
        .query_row(
            "SELECT goles, asistencias FROM partido
              WHERE (?1 IS NULL OR camiseta_id = ?1)
              ORDER BY fecha_hora DESC, id DESC LIMIT 1",
            params![camiseta_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?
        .unwrap_or((0, 0));

    let (racha_victorias, racha_derrotas) = rachas(conn, camiseta_id)?;

    let mut mapa = HashMap::new();
    mapa.insert("goles", goles);
    mapa.insert("asistencias", asistencias);
    mapa.insert("partidos", partidos);
    mapa.insert("victorias", victorias);
    mapa.insert("empates", empates);
    mapa.insert("derrotas", derrotas);
    mapa.insert("contribuciones", contribuciones);
    mapa.insert("rendimiento_total", rendimiento_total);
    mapa.insert("animo_total", animo_total);
    mapa.insert("canchas", canchas);
    mapa.insert("hat_tricks", hat_tricks);
    mapa.insert("poker", poker);
    mapa.insert("rendimiento_perfecto", perfectos);
    mapa.insert("animo_perfecto", animo_perfecto);
    mapa.insert("goles_victorias", goles_victorias);
    mapa.insert("asistencias_victorias", asistencias_victorias);
    mapa.insert("racha_victorias", racha_victorias);
    mapa.insert("racha_derrotas", racha_derrotas);
    mapa.insert("ultimo_goles", ultimo.0);
    mapa.insert("ultimo_asistencias", ultimo.1);
    Ok(mapa)
}

fn rachas(conn: &Connection, camiseta_id: Option<i64>) -> Result<(i64, i64), DbError> {
    let mut stmt = conn.prepare(
        "SELECT resultado FROM partido
          WHERE (?1 IS NULL OR camiseta_id = ?1)
          ORDER BY fecha_hora ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![camiseta_id], |row| row.get::<_, i64>(0))?;
    let mut mejor_victorias = 0;
    let mut mejor_derrotas = 0;
    let mut actual_victorias = 0;
    let mut actual_derrotas = 0;
    for valor in rows {
        match Resultado::try_from(valor?).unwrap_or(Resultado::NoAplica) {
            Resultado::Victoria => {
                actual_victorias += 1;
                actual_derrotas = 0;
            }
            Resultado::Derrota => {
                actual_derrotas += 1;
                actual_victorias = 0;
            }
            _ => {
                actual_victorias = 0;
                actual_derrotas = 0;
            }
        }
        mejor_victorias = mejor_victorias.max(actual_victorias);
        mejor_derrotas = mejor_derrotas.max(actual_derrotas);
    }
    Ok((mejor_victorias, mejor_derrotas))
}

/// Ids de logros ya desbloqueados.
pub fn desbloqueados(conn: &Connection) -> Result<HashSet<String>, DbError> {
    let mut stmt = conn.prepare("SELECT logro_id FROM logro_desbloqueado")?;
    let filas = stmt.query_map([], |row| row.get::<_, String>(0))?;
    Ok(filas.collect::<Result<HashSet<_>, _>>()?)
}

/// Registra los logros nuevos y devuelve cuántos se insertaron.
pub fn registrar(conn: &Connection, ids: &[String]) -> Result<usize, DbError> {
    let mut stmt =
        conn.prepare("INSERT OR IGNORE INTO logro_desbloqueado (logro_id) VALUES (?1)")?;
    let mut nuevos = 0;
    for id in ids {
        nuevos += stmt.execute(params![id])?;
    }
    Ok(nuevos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn insertar_partido(conn: &Connection, goles: i64, hat_trick: bool) {
        conn.execute("INSERT INTO cancha (nombre) VALUES ('C')", [])
            .unwrap();
        let cancha = conn.last_insert_rowid();
        conn.execute("INSERT INTO camiseta (nombre) VALUES ('K')", [])
            .unwrap();
        let camiseta = conn.last_insert_rowid();
        let goles_final = if hat_trick { 3 } else { goles };
        conn.execute(
            "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, resultado, goles,
                                  asistencias, rendimiento_general, estado_animo)
             VALUES (?1, ?2, '2026-01-10 10:00:00', 1, ?3, 2, 10, 10)",
            params![cancha, camiseta, goles_final],
        )
        .unwrap();
    }

    #[test]
    fn values_and_unlock_registration() {
        let db = Database::open_in_memory().unwrap();
        insertar_partido(db.conn(), 1, false);
        insertar_partido(db.conn(), 0, true);
        let valores = valores(db.conn(), None).unwrap();
        assert_eq!(valores.get("partidos"), Some(&2));
        assert_eq!(valores.get("hat_tricks"), Some(&1));
        assert_eq!(valores.get("rendimiento_perfecto"), Some(&2));
        assert_eq!(valores.get("victorias"), Some(&2));

        assert!(desbloqueados(db.conn()).unwrap().is_empty());
        let nuevos = registrar(db.conn(), &["goles_1".to_string()]).unwrap();
        assert_eq!(nuevos, 1);
        assert!(desbloqueados(db.conn()).unwrap().contains("goles_1"));
        assert_eq!(registrar(db.conn(), &["goles_1".to_string()]).unwrap(), 0);
    }
}
