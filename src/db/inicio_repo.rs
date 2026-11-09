//! Consultas del panel "Hoy" (IMP-01).

use chrono::NaiveDateTime;
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::error::DbError;
use crate::domain::dates;
use crate::domain::inicio::{MesResumen, PartidoBreve, RachaInfo};
use crate::domain::models::{RachaTipo, Resultado};

fn parse_fecha(texto: &str) -> Result<NaiveDateTime, DbError> {
    dates::parse_datetime(texto)
        .ok_or_else(|| DbError::InvalidData(format!("fecha inválida: {texto}")))
}

fn map_breve(row: &Row<'_>) -> rusqlite::Result<(i64, String, String, i64, i64, i64)> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn breve(
    (id, fecha, rival, goles, asistencias, resultado): (i64, String, String, i64, i64, i64),
) -> Result<PartidoBreve, DbError> {
    Ok(PartidoBreve {
        id,
        fecha_hora: parse_fecha(&fecha)?,
        rival,
        goles,
        asistencias,
        resultado: Resultado::try_from(resultado).unwrap_or(Resultado::NoAplica),
    })
}

/// Próximo partido (fecha futura).
pub fn proximo_partido(conn: &Connection) -> Result<Option<PartidoBreve>, DbError> {
    let row = conn
        .query_row(
            "SELECT id, fecha_hora, rival_nombre, goles, asistencias, resultado
               FROM partido
              WHERE datetime(fecha_hora) >= datetime('now', 'localtime')
              ORDER BY fecha_hora ASC, id ASC
              LIMIT 1",
            [],
            map_breve,
        )
        .optional()?;
    row.map(breve).transpose()
}

/// Último partido jugado.
pub fn ultimo_partido(conn: &Connection) -> Result<Option<PartidoBreve>, DbError> {
    let row = conn
        .query_row(
            "SELECT id, fecha_hora, rival_nombre, goles, asistencias, resultado
               FROM partido
              ORDER BY fecha_hora DESC, id DESC
              LIMIT 1",
            [],
            map_breve,
        )
        .optional()?;
    row.map(breve).transpose()
}

/// Racha actual de resultados.
pub fn racha(conn: &Connection) -> Result<RachaInfo, DbError> {
    let mut stmt =
        conn.prepare("SELECT resultado FROM partido ORDER BY fecha_hora DESC, id DESC LIMIT 50")?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    let mut tipo = RachaTipo::Neutral;
    let mut cantidad = 0;
    let mut definida = false;

    for valor in rows {
        let resultado = Resultado::try_from(valor?).unwrap_or(Resultado::NoAplica);
        if resultado == Resultado::NoAplica {
            continue;
        }
        if !definida {
            tipo = match resultado {
                Resultado::Victoria => RachaTipo::Positiva,
                Resultado::Derrota => RachaTipo::Negativa,
                _ => RachaTipo::Neutral,
            };
            cantidad = 1;
            definida = true;
            continue;
        }
        let mismo = matches!(
            (tipo, resultado),
            (RachaTipo::Positiva, Resultado::Victoria)
                | (RachaTipo::Negativa, Resultado::Derrota)
                | (RachaTipo::Neutral, Resultado::Empate)
        );
        if !mismo {
            break;
        }
        cantidad += 1;
    }

    Ok(RachaInfo { tipo, cantidad })
}

/// Totales del mes indicado (`YYYY-MM`).
pub fn mes(conn: &Connection, mes: &str) -> Result<MesResumen, DbError> {
    let (partidos, goles, asistencias, victorias, empates, derrotas, promedio) = conn.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(goles), 0),
                COALESCE(SUM(asistencias), 0),
                COALESCE(SUM(CASE WHEN resultado = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 2 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 3 THEN 1 ELSE 0 END), 0),
                COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido
          WHERE substr(fecha_hora, 1, 7) = ?1",
        params![mes],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    )?;
    Ok(MesResumen {
        mes: mes.to_string(),
        partidos,
        goles,
        asistencias,
        victorias,
        empates,
        derrotas,
        rendimiento_promedio: promedio,
    })
}

/// Lesiones que siguen activas.
pub fn lesiones_activas(conn: &Connection) -> Result<i64, DbError> {
    let total = conn.query_row(
        "SELECT COUNT(*) FROM lesion
          WHERE estado IN ('ACTIVA', 'EN TRATAMIENTO', 'RECAIDA')",
        [],
        |row| row.get(0),
    )?;
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use chrono::Duration;

    fn base(conn: &Connection) -> (i64, i64) {
        conn.execute("INSERT INTO cancha (nombre) VALUES ('Cancha test')", [])
            .unwrap();
        let cancha = conn.last_insert_rowid();
        conn.execute("INSERT INTO camiseta (nombre) VALUES ('Camiseta test')", [])
            .unwrap();
        let camiseta = conn.last_insert_rowid();
        (cancha, camiseta)
    }

    fn partido(
        conn: &Connection,
        cancha: i64,
        camiseta: i64,
        fecha: chrono::NaiveDateTime,
        resultado: i64,
        goles: i64,
    ) {
        conn.execute(
            "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, resultado, goles,
                                  asistencias, rendimiento_general, estado_animo)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, 7, 8)",
            params![
                cancha,
                camiseta,
                fecha.format("%Y-%m-%d %H:%M:%S").to_string(),
                resultado,
                goles
            ],
        )
        .unwrap();
    }

    #[test]
    fn dashboard_data_with_matches() {
        let db = Database::open_in_memory().unwrap();
        let (cancha, camiseta) = base(db.conn());
        let base = chrono::NaiveDate::from_ymd_opt(2026, 1, 10)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        partido(db.conn(), cancha, camiseta, base, 1, 2);
        partido(db.conn(), cancha, camiseta, base + Duration::days(5), 1, 1);
        partido(db.conn(), cancha, camiseta, base + Duration::days(10), 3, 0);

        let ultimo = ultimo_partido(db.conn()).unwrap().unwrap();
        assert_eq!(ultimo.resultado, Resultado::Derrota);
        assert!(proximo_partido(db.conn()).unwrap().is_none());

        let racha = racha(db.conn()).unwrap();
        assert_eq!(racha.tipo, RachaTipo::Negativa);
        assert_eq!(racha.cantidad, 1);

        let mes = mes(db.conn(), "2026-01").unwrap();
        assert_eq!(mes.partidos, 3);
        assert_eq!(mes.goles, 3);
        assert_eq!(mes.victorias, 2);
    }

    #[test]
    fn next_match_and_active_injuries() {
        let db = Database::open_in_memory().unwrap();
        let (cancha, camiseta) = base(db.conn());
        let ahora = chrono::Local::now().naive_local();
        partido(db.conn(), cancha, camiseta, ahora + Duration::days(2), 0, 0);
        db.conn()
            .execute(
                "INSERT INTO lesion (jugador, tipo, descripcion, fecha, camiseta_id, estado)
                 VALUES ('Ana', 'Esguince', 'Tobillo', '2026-01-01', ?1, 'ACTIVA')",
                params![camiseta],
            )
            .unwrap();
        assert!(proximo_partido(db.conn()).unwrap().is_some());
        assert_eq!(lesiones_activas(db.conn()).unwrap(), 1);
    }
}
