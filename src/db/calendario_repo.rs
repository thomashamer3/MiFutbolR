//! Eventos del calendario (IMP-02).

use chrono::NaiveDate;
use rusqlite::{Connection, params};

use super::error::DbError;
use crate::domain::calendario::{EventoCalendario, EventoTipo};
use crate::domain::{Cents, dates};

fn fecha_de(texto: &str) -> Option<NaiveDate> {
    if texto.len() >= 10 {
        dates::parse_date(&texto[..10])
    } else {
        dates::parse_date(texto)
    }
}

/// Eventos (partidos y finanzas) de un mes.
pub fn eventos_mes(
    conn: &Connection,
    year: i32,
    month: u32,
) -> Result<Vec<EventoCalendario>, DbError> {
    let mes = format!("{year:04}-{month:02}");
    let mut eventos = Vec::new();

    let mut stmt = conn.prepare(
        "SELECT id, fecha_hora, rival_nombre, goles, asistencias, resultado
           FROM partido
          WHERE substr(fecha_hora, 1, 7) = ?1",
    )?;
    let partidos = stmt.query_map(params![mes], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;
    for partido in partidos {
        let (id, fecha_hora, rival, goles, asistencias, resultado) = partido?;
        let Some(fecha) = fecha_de(&fecha_hora) else {
            continue;
        };
        let resultado = match resultado {
            1 => "Victoria",
            2 => "Empate",
            3 => "Derrota",
            _ => "Sin resultado",
        };
        let titulo = if rival.trim().is_empty() {
            "Partido".to_string()
        } else {
            format!("vs {rival}")
        };
        eventos.push(EventoCalendario {
            fecha,
            tipo: EventoTipo::Partido,
            titulo,
            detalle: format!("{goles} goles · {asistencias} asist. · {resultado}"),
            referencia: Some(id),
        });
    }

    let mut stmt = conn.prepare(
        "SELECT id, fecha, tipo, descripcion, monto_centavos
           FROM financiamiento
          WHERE substr(fecha, 1, 7) = ?1",
    )?;
    let movimientos = stmt.query_map(params![mes], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;
    for movimiento in movimientos {
        let (id, fecha, tipo, descripcion, monto) = movimiento?;
        let Some(fecha) = fecha_de(&fecha) else {
            continue;
        };
        let signo = if tipo == 0 { "Ingreso" } else { "Gasto" };
        eventos.push(EventoCalendario {
            fecha,
            tipo: EventoTipo::Financiamiento,
            titulo: format!("{signo}: {descripcion}"),
            detalle: Cents(monto).display_ars(),
            referencia: Some(id),
        });
    }

    let mut stmt = conn.prepare(
        "SELECT id, fecha, hora, titulo, tematica
           FROM recordatorio
          WHERE substr(fecha, 1, 7) = ?1 AND completado = 0",
    )?;
    let recordatorios = stmt.query_map(params![mes], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;
    for recordatorio in recordatorios {
        let (id, fecha, hora, titulo, tematica) = recordatorio?;
        let Some(fecha) = fecha_de(&fecha) else {
            continue;
        };
        eventos.push(EventoCalendario {
            fecha,
            tipo: EventoTipo::Recordatorio,
            titulo: titulo.clone(),
            detalle: if hora.is_empty() {
                tematica
            } else {
                format!("{hora} · {tematica}")
            },
            referencia: Some(id),
        });
    }

    eventos.sort_by_key(|evento| evento.fecha);
    Ok(eventos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn events_grouped_by_month() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO cancha (nombre) VALUES ('C')", [])
            .unwrap();
        let cancha = db.conn().last_insert_rowid();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('K')", [])
            .unwrap();
        let camiseta = db.conn().last_insert_rowid();
        db.conn()
            .execute(
                "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, resultado, goles)
                 VALUES (?1, ?2, '2026-03-15 21:00:00', 1, 3)",
                params![cancha, camiseta],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO financiamiento (fecha, tipo, categoria, descripcion, monto_centavos)
                 VALUES ('2026-03-20', 1, 0, 'Cancha', 150000)",
                [],
            )
            .unwrap();

        let eventos = eventos_mes(db.conn(), 2026, 3).unwrap();
        assert_eq!(eventos.len(), 2);
        assert_eq!(eventos[0].tipo, EventoTipo::Partido);
        assert_eq!(eventos[1].tipo, EventoTipo::Financiamiento);
        assert!(eventos_mes(db.conn(), 2026, 4).unwrap().is_empty());
    }
}
