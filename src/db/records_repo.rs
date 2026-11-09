//! Cálculo de los 16 récords y rankings (IMP-05).

use rusqlite::{Connection, OptionalExtension};

use super::error::DbError;
use crate::domain::dates;
use crate::domain::records::RecordItem;

fn fecha_corta(texto: &str) -> String {
    dates::parse_datetime(texto).map_or_else(
        || texto.to_string(),
        |fecha| fecha.format("%d/%m/%Y").to_string(),
    )
}

fn partido_record(
    conn: &Connection,
    titulo: &str,
    orden: &str,
) -> Result<Option<RecordItem>, DbError> {
    let sql = format!(
        "SELECT p.id, p.rival_nombre, p.fecha_hora, p.goles, p.asistencias,
                p.rendimiento_general, c.nombre
           FROM partido p JOIN cancha c ON c.id = p.cancha_id
          ORDER BY {orden}
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
                row.get::<_, String>(6)?,
            ))
        })
        .optional()?;
    Ok(fila.map(
        |(id, rival, fecha, goles, asistencias, rendimiento, cancha)| RecordItem {
            titulo: titulo.to_string(),
            detalle: format!(
                "vs {} · {} · {} G · {} A · rend. {} · {}",
                if rival.trim().is_empty() {
                    "rival"
                } else {
                    &rival
                },
                fecha_corta(&fecha),
                goles,
                asistencias,
                rendimiento,
                cancha
            ),
            partido_id: Some(id),
        },
    ))
}

fn combo(conn: &Connection, titulo: &str, orden: &str) -> Result<Option<RecordItem>, DbError> {
    let sql = format!(
        "SELECT ca.nombre, c.nombre, AVG(p.rendimiento_general), COUNT(*)
           FROM partido p
           JOIN cancha ca ON ca.id = p.cancha_id
           JOIN camiseta c ON c.id = p.camiseta_id
          GROUP BY p.cancha_id, p.camiseta_id
         HAVING COUNT(*) >= 2
          ORDER BY {orden}
          LIMIT 1"
    );
    let fila = conn
        .query_row(&sql, [], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .optional()?;
    Ok(
        fila.map(|(cancha, camiseta, promedio, partidos)| RecordItem {
            titulo: titulo.to_string(),
            detalle: format!("{cancha} + {camiseta} · rend. {promedio:.2} en {partidos} PJ"),
            partido_id: None,
        }),
    )
}

fn temporada(conn: &Connection, titulo: &str, orden: &str) -> Result<Option<RecordItem>, DbError> {
    let sql = format!(
        "SELECT substr(fecha_hora, 1, 4), COUNT(*), SUM(goles), SUM(asistencias),
                AVG(rendimiento_general)
           FROM partido
          GROUP BY substr(fecha_hora, 1, 4)
         HAVING COUNT(*) >= 3
          ORDER BY {orden}
          LIMIT 1"
    );
    let fila = conn
        .query_row(&sql, [], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, f64>(4)?,
            ))
        })
        .optional()?;
    Ok(fila.map(
        |(anio, partidos, goles, asistencias, promedio)| RecordItem {
            titulo: titulo.to_string(),
            detalle: format!(
                "{anio} · {partidos} PJ · {} G · {} A · rend. {promedio:.2}",
                goles, asistencias
            ),
            partido_id: None,
        },
    ))
}

fn contar(conn: &Connection, titulo: &str, condicion: &str) -> Result<RecordItem, DbError> {
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM partido WHERE {condicion}"),
        [],
        |row| row.get(0),
    )?;
    Ok(RecordItem {
        titulo: titulo.to_string(),
        detalle: format!("{total} partido(s)"),
        partido_id: None,
    })
}

struct PartidoFila {
    fecha: String,
    resultado: i64,
    goles: i64,
}

fn filas_partidos(conn: &Connection) -> Result<Vec<PartidoFila>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT fecha_hora, resultado, goles
           FROM partido ORDER BY fecha_hora ASC, id ASC",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok(PartidoFila {
            fecha: row.get(0)?,
            resultado: row.get(1)?,
            goles: row.get(2)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

fn racha_maxima(
    filas: &[PartidoFila],
    condicion: impl Fn(&PartidoFila) -> bool,
) -> Option<(usize, &PartidoFila, &PartidoFila)> {
    let mut mejor = 0;
    let mut actual = 0;
    let mut inicio = 0;
    let mut mejor_rango = None;
    for (indice, fila) in filas.iter().enumerate() {
        if condicion(fila) {
            if actual == 0 {
                inicio = indice;
            }
            actual += 1;
            if actual > mejor {
                mejor = actual;
                mejor_rango = Some((inicio, indice));
            }
        } else {
            actual = 0;
        }
    }
    mejor_rango.map(|(inicio, fin)| (mejor, &filas[inicio], &filas[fin]))
}

fn detalle_racha(
    titulo: &str,
    filas: &[PartidoFila],
    condicion: impl Fn(&PartidoFila) -> bool,
) -> Option<RecordItem> {
    let (cantidad, inicio, fin) = racha_maxima(filas, condicion)?;
    Some(RecordItem {
        titulo: titulo.to_string(),
        detalle: format!(
            "{cantidad} partidos · del {} al {}",
            fecha_corta(&inicio.fecha),
            fecha_corta(&fin.fecha)
        ),
        partido_id: None,
    })
}

fn top_partidos(conn: &Connection) -> Result<RecordItem, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, rival_nombre, fecha_hora, rendimiento_general
           FROM partido ORDER BY rendimiento_general DESC, fecha_hora DESC LIMIT 5",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    let mut partes = Vec::new();
    for (indice, fila) in filas.enumerate() {
        let (_, rival, fecha, rendimiento) = fila?;
        partes.push(format!(
            "{}. {} ({}) · {}",
            indice + 1,
            if rival.trim().is_empty() {
                "Partido"
            } else {
                &rival
            },
            fecha_corta(&fecha),
            rendimiento
        ));
    }
    Ok(RecordItem {
        titulo: "Top 5 partidos por rendimiento".to_string(),
        detalle: if partes.is_empty() {
            "Sin partidos".to_string()
        } else {
            partes.join("  ·  ")
        },
        partido_id: None,
    })
}

fn ranking_camisetas(conn: &Connection) -> Result<RecordItem, DbError> {
    let mut stmt = conn.prepare(
        "SELECT c.nombre, COUNT(*), SUM(CASE WHEN p.resultado = 1 THEN 1 ELSE 0 END)
           FROM partido p JOIN camiseta c ON c.id = p.camiseta_id
          GROUP BY c.id
          ORDER BY (SUM(CASE WHEN p.resultado = 1 THEN 1 ELSE 0 END) * 1.0 / COUNT(*)) DESC,
                   COUNT(*) DESC
          LIMIT 5",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    let mut partes = Vec::new();
    for fila in filas {
        let (nombre, partidos, victorias) = fila?;
        let winrate = if partidos > 0 {
            victorias as f64 * 100.0 / partidos as f64
        } else {
            0.0
        };
        partes.push(format!("{nombre} {winrate:.0}% ({victorias}/{partidos})"));
    }
    Ok(RecordItem {
        titulo: "Ranking de camisetas".to_string(),
        detalle: if partes.is_empty() {
            "Sin camisetas usadas".to_string()
        } else {
            partes.join("  ·  ")
        },
        partido_id: None,
    })
}

/// Calcula los 16 reportes.
pub fn todos(conn: &Connection) -> Result<Vec<RecordItem>, DbError> {
    let mut items = Vec::new();
    items.push(
        partido_record(
            conn,
            "Récord de goles en un partido",
            "p.goles DESC, p.fecha_hora DESC",
        )?
        .unwrap_or_else(|| RecordItem {
            titulo: "Récord de goles en un partido".to_string(),
            detalle: "Sin partidos".to_string(),
            partido_id: None,
        }),
    );
    items.push(
        partido_record(
            conn,
            "Récord de asistencias en un partido",
            "p.asistencias DESC, p.fecha_hora DESC",
        )?
        .unwrap_or_else(|| RecordItem {
            titulo: "Récord de asistencias en un partido".to_string(),
            detalle: "Sin partidos".to_string(),
            partido_id: None,
        }),
    );
    items.push(
        partido_record(
            conn,
            "Mejor partido por rendimiento",
            "p.rendimiento_general DESC, p.fecha_hora DESC",
        )?
        .unwrap_or_else(|| RecordItem {
            titulo: "Mejor partido por rendimiento".to_string(),
            detalle: "Sin partidos".to_string(),
            partido_id: None,
        }),
    );
    items.push(
        partido_record(
            conn,
            "Peor partido por rendimiento",
            "p.rendimiento_general ASC, p.fecha_hora DESC",
        )?
        .unwrap_or_else(|| RecordItem {
            titulo: "Peor partido por rendimiento".to_string(),
            detalle: "Sin partidos".to_string(),
            partido_id: None,
        }),
    );

    items.push(
        combo(
            conn,
            "Mejor combinación cancha + camiseta",
            "AVG(p.rendimiento_general) DESC",
        )?
        .unwrap_or_else(|| RecordItem {
            titulo: "Mejor combinación cancha + camiseta".to_string(),
            detalle: "Hacen falta al menos 2 partidos por combinación".to_string(),
            partido_id: None,
        }),
    );
    items.push(
        combo(
            conn,
            "Peor combinación cancha + camiseta",
            "AVG(p.rendimiento_general) ASC",
        )?
        .unwrap_or_else(|| RecordItem {
            titulo: "Peor combinación cancha + camiseta".to_string(),
            detalle: "Hacen falta al menos 2 partidos por combinación".to_string(),
            partido_id: None,
        }),
    );

    items.push(
        temporada(conn, "Mejor temporada", "AVG(rendimiento_general) DESC")?.unwrap_or_else(|| {
            RecordItem {
                titulo: "Mejor temporada".to_string(),
                detalle: "Hacen falta al menos 3 partidos en un año".to_string(),
                partido_id: None,
            }
        }),
    );
    items.push(
        temporada(conn, "Peor temporada", "AVG(rendimiento_general) ASC")?.unwrap_or_else(|| {
            RecordItem {
                titulo: "Peor temporada".to_string(),
                detalle: "Hacen falta al menos 3 partidos en un año".to_string(),
                partido_id: None,
            }
        }),
    );

    items.push(contar(conn, "Partidos sin goles", "goles = 0")?);
    items.push(contar(conn, "Partidos sin asistencias", "asistencias = 0")?);

    let filas = filas_partidos(conn)?;
    items.push(
        detalle_racha("Mejor racha de victorias", &filas, |fila| {
            fila.resultado == 1
        })
        .unwrap_or_else(|| RecordItem {
            titulo: "Mejor racha de victorias".to_string(),
            detalle: "Sin victorias".to_string(),
            partido_id: None,
        }),
    );
    items.push(
        detalle_racha("Peor racha de derrotas", &filas, |fila| fila.resultado == 3).unwrap_or_else(
            || RecordItem {
                titulo: "Peor racha de derrotas".to_string(),
                detalle: "Sin derrotas".to_string(),
                partido_id: None,
            },
        ),
    );
    items.push(
        detalle_racha("Mejor racha sin perder", &filas, |fila| fila.resultado != 3).unwrap_or_else(
            || RecordItem {
                titulo: "Mejor racha sin perder".to_string(),
                detalle: "Sin partidos".to_string(),
                partido_id: None,
            },
        ),
    );
    items.push(
        detalle_racha("Partidos consecutivos anotando", &filas, |fila| {
            fila.goles > 0
        })
        .unwrap_or_else(|| RecordItem {
            titulo: "Partidos consecutivos anotando".to_string(),
            detalle: "Sin goles".to_string(),
            partido_id: None,
        }),
    );
    items.push(top_partidos(conn)?);
    items.push(ranking_camisetas(conn)?);

    Ok(items)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use rusqlite::params;

    #[test]
    fn computes_sixteen_records() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO cancha (nombre) VALUES ('El Potrero')", [])
            .unwrap();
        let cancha = db.conn().last_insert_rowid();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Azul')", [])
            .unwrap();
        let camiseta = db.conn().last_insert_rowid();
        for (fecha, goles, asistencias, rendimiento, resultado) in [
            ("2026-01-10 10:00:00", 2, 1, 7, 1),
            ("2026-01-17 10:00:00", 4, 2, 9, 1),
            ("2026-01-24 10:00:00", 0, 0, 3, 3),
            ("2026-02-01 10:00:00", 1, 3, 8, 2),
        ] {
            db.conn()
                .execute(
                    "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, goles, asistencias,
                                          rendimiento_general, resultado)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        cancha,
                        camiseta,
                        fecha,
                        goles,
                        asistencias,
                        rendimiento,
                        resultado
                    ],
                )
                .unwrap();
        }

        let items = todos(db.conn()).unwrap();
        assert_eq!(items.len(), 16);
        let goles = items
            .iter()
            .find(|item| item.titulo.contains("Récord de goles"))
            .unwrap();
        assert!(goles.detalle.contains("4 G"), "detalle: {}", goles.detalle);
        assert!(goles.partido_id.is_some());
        let victorias = items
            .iter()
            .find(|item| item.titulo.contains("racha de victorias"))
            .unwrap();
        assert!(victorias.detalle.contains("2 partidos"));
        assert!(
            items
                .iter()
                .any(|item| item.titulo.contains("Ranking de camisetas"))
        );
    }
}
