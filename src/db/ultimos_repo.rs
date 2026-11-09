//! Consultas del submenú "Último" (IMP-16).

use chrono::NaiveDateTime;
use rusqlite::{Connection, OptionalExtension};

use super::error::DbError;
use crate::domain::dates;
use crate::domain::inicio::PartidoBreve;
use crate::domain::models::{RachaTipo, Resultado};
use crate::domain::ultimos::UltimoItem;

fn parse_fecha(texto: &str) -> Result<NaiveDateTime, DbError> {
    dates::parse_datetime(texto)
        .ok_or_else(|| DbError::InvalidData(format!("fecha inválida: {texto}")))
}

fn ultimo(conn: &Connection, condicion: &str) -> Result<Option<PartidoBreve>, DbError> {
    let sql = format!(
        "SELECT id, fecha_hora, rival_nombre, goles, asistencias, resultado
           FROM partido
          WHERE {condicion}
          ORDER BY fecha_hora DESC, id DESC
          LIMIT 1"
    );
    let fila = conn
        .query_row(&sql, [], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .optional()?;
    fila.map(|(id, fecha, rival, goles, asistencias, resultado)| {
        Ok(PartidoBreve {
            id,
            fecha_hora: parse_fecha(&fecha)?,
            rival,
            goles,
            asistencias,
            resultado: Resultado::try_from(resultado).unwrap_or(Resultado::NoAplica),
        })
    })
    .transpose()
}

/// Calcula los cinco "últimos".
pub fn listar(conn: &Connection) -> Result<Vec<UltimoItem>, DbError> {
    let ahora = chrono::Local::now().naive_local();
    let casos: [(&'static str, &'static str); 5] = [
        ("Última victoria", "resultado = 1"),
        ("Última derrota", "resultado = 3"),
        ("Último empate", "resultado = 2"),
        ("Último gol", "goles > 0"),
        ("Última asistencia", "asistencias > 0"),
    ];
    let mut items = Vec::with_capacity(casos.len());
    for (etiqueta, condicion) in casos {
        let partido = ultimo(conn, condicion)?;
        let hace = partido.as_ref().map_or_else(
            || "sin registros".to_string(),
            |partido| dates::tiempo_transcurrido(partido.fecha_hora, ahora),
        );
        items.push(UltimoItem {
            etiqueta,
            partido,
            hace,
        });
    }
    Ok(items)
}

/// Racha actual reutilizada por la pantalla.
pub fn racha(conn: &Connection) -> Result<RachaTipo, DbError> {
    let mut stmt =
        conn.prepare("SELECT resultado FROM partido ORDER BY fecha_hora DESC, id DESC LIMIT 20")?;
    let mut rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    if let Some(valor) = rows.next() {
        return Ok(
            match Resultado::try_from(valor?).unwrap_or(Resultado::NoAplica) {
                Resultado::Victoria => RachaTipo::Positiva,
                Resultado::Derrota => RachaTipo::Negativa,
                _ => RachaTipo::Neutral,
            },
        );
    }
    Ok(RachaTipo::Neutral)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use rusqlite::params;

    #[test]
    fn finds_last_items() {
        let db = Database::open_in_memory().unwrap();
        for (resultado, goles, asistencias) in [(1, 2, 1), (3, 0, 0), (2, 1, 0)] {
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
                    "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, resultado, goles, asistencias)
                     VALUES (?1, ?2, datetime('now','localtime','-1 day'), ?3, ?4, ?5)",
                    params![cancha, camiseta, resultado, goles, asistencias],
                )
                .unwrap();
        }
        let items = listar(db.conn()).unwrap();
        assert_eq!(items.len(), 5);
        assert!(items[0].partido.is_some());
        assert!(items[1].partido.is_some());
        assert!(items[3].partido.is_some());
    }
}
