//! Repositorio del módulo Carrera (§16).

use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::dates;
use crate::domain::models::{
    CarreraHall, CarreraIdentidad, CarreraPerfil, CarreraResumen, HallPartido, HitoRow,
    TimelineEvento,
};

/// Carga la identidad (o valores por defecto).
pub fn identidad(conn: &Connection) -> Result<CarreraIdentidad, DbError> {
    let identidad = conn
        .query_row(
            "SELECT nombre, edad, pie_habil, posiciones, altura_cm, peso_kg, estilo,
                    dorsal_favorito, objetivos, historia
               FROM carrera_identidad WHERE id = 1",
            [],
            |row| {
                Ok(CarreraIdentidad {
                    nombre: row.get(0)?,
                    edad: row.get(1)?,
                    pie_habil: row.get(2)?,
                    posiciones: row.get(3)?,
                    altura_cm: row.get(4)?,
                    peso_kg: row.get(5)?,
                    estilo: row.get(6)?,
                    dorsal_favorito: row.get(7)?,
                    objetivos: row.get(8)?,
                    historia: row.get(9)?,
                })
            },
        )
        .optional()?;
    Ok(identidad.unwrap_or_default())
}

/// Guarda la identidad única.
pub fn guardar_identidad(conn: &Connection, identidad: &CarreraIdentidad) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO carrera_identidad
            (id, nombre, edad, pie_habil, posiciones, altura_cm, peso_kg, estilo,
             dorsal_favorito, objetivos, historia, updated_at)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, datetime('now','localtime'))
         ON CONFLICT(id) DO UPDATE SET
            nombre = excluded.nombre,
            edad = excluded.edad,
            pie_habil = excluded.pie_habil,
            posiciones = excluded.posiciones,
            altura_cm = excluded.altura_cm,
            peso_kg = excluded.peso_kg,
            estilo = excluded.estilo,
            dorsal_favorito = excluded.dorsal_favorito,
            objetivos = excluded.objetivos,
            historia = excluded.historia,
            updated_at = excluded.updated_at",
        params![
            identidad.nombre,
            identidad.edad,
            identidad.pie_habil,
            identidad.posiciones,
            identidad.altura_cm,
            identidad.peso_kg,
            identidad.estilo,
            identidad.dorsal_favorito,
            identidad.objetivos,
            identidad.historia,
        ],
    )?;
    Ok(())
}

/// Resumen base (sin badges, que se calculan en el servicio).
pub fn resumen(conn: &Connection) -> Result<CarreraResumen, DbError> {
    let (partidos, goles, asistencias, victorias, empates, derrotas, rendimiento_promedio): (
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        f64,
    ) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(goles), 0), COALESCE(SUM(asistencias), 0),
                COALESCE(SUM(CASE WHEN resultado = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 2 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 3 THEN 1 ELSE 0 END), 0),
                COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido",
        [],
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

    let mejor_anio: Option<String> = conn
        .query_row(
            "SELECT substr(fecha_hora, 1, 4) AS anio
               FROM partido
              GROUP BY anio
              ORDER BY SUM(goles + asistencias) DESC, COUNT(*) DESC
              LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;

    let titulos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM equipo_historial WHERE posicion_final = 1",
        [],
        |row| row.get(0),
    )?;
    let podios: i64 = conn.query_row(
        "SELECT COUNT(*) FROM equipo_historial WHERE posicion_final BETWEEN 1 AND 3",
        [],
        |row| row.get(0),
    )?;
    let temporadas: i64 = conn.query_row("SELECT COUNT(*) FROM temporada", [], |row| row.get(0))?;

    Ok(CarreraResumen {
        partidos,
        goles,
        asistencias,
        victorias,
        empates,
        derrotas,
        rendimiento_promedio,
        mejor_anio,
        mejor_racha: mejor_racha(conn)?,
        titulos,
        podios,
        temporadas,
        badges: Vec::new(),
    })
}

/// Métricas auxiliares para badges.
pub fn metricas_badges(conn: &Connection) -> Result<(i64, i64, i64, i64), DbError> {
    let hat_tricks: i64 =
        conn.query_row("SELECT COUNT(*) FROM partido WHERE goles >= 3", [], |row| {
            row.get(0)
        })?;
    let rendimiento_perfecto: i64 = conn.query_row(
        "SELECT COUNT(*) FROM partido WHERE rendimiento_general = 10",
        [],
        |row| row.get(0),
    )?;
    let recuperaciones: i64 = conn.query_row(
        "SELECT COUNT(*) FROM lesion WHERE estado = 'RECUPERADO'",
        [],
        |row| row.get(0),
    )?;
    let racha5: i64 = if mejor_racha(conn)? >= 5 { 1 } else { 0 };
    Ok((hat_tricks, rendimiento_perfecto, recuperaciones, racha5))
}

/// Lista de hitos con datos del partido.
pub fn hitos(conn: &Connection) -> Result<Vec<HitoRow>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT h.id, h.partido_id, p.fecha_hora, p.rival_nombre, h.tipo_hito, h.nota
           FROM carrera_partido_hito h
           JOIN partido p ON p.id = h.partido_id
          ORDER BY p.fecha_hora DESC, h.id DESC",
    )?;
    let rows = stmt
        .query_map([], map_hito)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Obtiene un hito.
pub fn hito(conn: &Connection, id: i64) -> Result<HitoRow, DbError> {
    conn.query_row(
        "SELECT h.id, h.partido_id, p.fecha_hora, p.rival_nombre, h.tipo_hito, h.nota
           FROM carrera_partido_hito h
           JOIN partido p ON p.id = h.partido_id
          WHERE h.id = ?1",
        [id],
        map_hito,
    )
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
            entidad: "Hito",
            id,
        },
        other => other.into(),
    })
}

/// `true` si el partido ya tiene un hito.
pub fn existe_hito_partido(
    conn: &Connection,
    partido_id: i64,
    exclude_id: Option<i64>,
) -> Result<bool, DbError> {
    let exclude = exclude_id.unwrap_or(0);
    let exists: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM carrera_partido_hito
             WHERE partido_id = ?1 AND (?2 = 0 OR id <> ?2)
        )",
        params![partido_id, exclude],
        |row| row.get(0),
    )?;
    Ok(exists)
}

/// Inserta un hito.
pub fn insertar_hito(
    conn: &Connection,
    partido_id: i64,
    tipo_hito: &str,
    nota: &str,
) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO carrera_partido_hito (partido_id, tipo_hito, nota)
         VALUES (?1, ?2, ?3)",
        params![partido_id, tipo_hito, nota],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza un hito.
pub fn actualizar_hito(
    conn: &Connection,
    id: i64,
    partido_id: i64,
    tipo_hito: &str,
    nota: &str,
) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE carrera_partido_hito SET partido_id = ?1, tipo_hito = ?2, nota = ?3 WHERE id = ?4",
        params![partido_id, tipo_hito, nota, id],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Hito",
            id,
        });
    }
    Ok(())
}

/// Elimina un hito.
pub fn eliminar_hito(conn: &Connection, id: i64) -> Result<(), DbError> {
    let deleted = conn.execute("DELETE FROM carrera_partido_hito WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(DbError::NotFound {
            entidad: "Hito",
            id,
        });
    }
    Ok(())
}

/// Línea de tiempo mezclada (partidos recientes, hitos y lesiones).
pub fn timeline(conn: &Connection) -> Result<Vec<TimelineEvento>, DbError> {
    let mut eventos = Vec::new();

    let mut stmt = conn.prepare(
        "SELECT fecha_hora, rival_nombre, goles, asistencias
           FROM partido ORDER BY fecha_hora DESC LIMIT 60",
    )?;
    let partidos = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (fecha, rival, goles, asistencias) in partidos {
        if let Some(fecha) = dates::parse_datetime(&fecha) {
            let rival = if rival.trim().is_empty() {
                "sin rival"
            } else {
                rival.as_str()
            };
            eventos.push(TimelineEvento {
                fecha,
                categoria: "Partido".to_string(),
                descripcion: format!("{rival} · {goles} G, {asistencias} A"),
            });
        }
    }

    let mut stmt = conn.prepare(
        "SELECT p.fecha_hora, h.tipo_hito, h.nota
           FROM carrera_partido_hito h JOIN partido p ON p.id = h.partido_id",
    )?;
    let hitos = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (fecha, tipo, nota) in hitos {
        if let Some(fecha) = dates::parse_datetime(&fecha) {
            eventos.push(TimelineEvento {
                fecha,
                categoria: "Hito".to_string(),
                descripcion: if nota.trim().is_empty() {
                    tipo
                } else {
                    format!("{tipo} · {nota}")
                },
            });
        }
    }

    let mut stmt = conn.prepare("SELECT fecha, tipo, descripcion FROM lesion")?;
    let lesiones = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (fecha, tipo, descripcion) in lesiones {
        if let Some(fecha) = dates::parse_datetime(&fecha) {
            eventos.push(TimelineEvento {
                fecha,
                categoria: "Lesión".to_string(),
                descripcion: format!("{tipo} · {descripcion}"),
            });
        }
    }

    eventos.sort_by_key(|evento| std::cmp::Reverse(evento.fecha));
    Ok(eventos)
}

/// Hall of Fame: top 5 partidos y rachas.
pub fn hall_of_fame(conn: &Connection) -> Result<CarreraHall, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, fecha_hora, rival_nombre, goles, asistencias, rendimiento_general
           FROM partido
          ORDER BY (goles * 2 + asistencias + rendimiento_general) DESC, id ASC
          LIMIT 5",
    )?;
    let partidos = stmt
        .query_map([], |row| {
            let fecha_texto: String = row.get(1)?;
            let fecha = dates::parse_datetime(&fecha_texto).ok_or_else(|| {
                rusqlite::Error::InvalidColumnType(
                    1,
                    "fecha_hora".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?;
            Ok(HallPartido {
                id: row.get(0)?,
                fecha,
                rival: row.get(2)?,
                goles: row.get(3)?,
                asistencias: row.get(4)?,
                rendimiento: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare("SELECT resultado FROM partido ORDER BY fecha_hora ASC, id ASC")?;
    let resultados: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let racha_actual = resultados.last().map(|ultimo| {
        let largo = resultados
            .iter()
            .rev()
            .take_while(|valor| *valor == ultimo)
            .count();
        let etiqueta = match ultimo {
            1 => "Victoria",
            2 => "Empate",
            3 => "Derrota",
            _ => "Sin resultado",
        };
        (etiqueta.to_string(), largo)
    });

    Ok(CarreraHall {
        partidos,
        mejor_racha: mejor_racha(conn)?,
        racha_actual,
    })
}

/// Perfil dinámico: últimos 5 vs anteriores 5.
pub fn perfil(conn: &Connection) -> Result<CarreraPerfil, DbError> {
    let mut stmt = conn.prepare(
        "SELECT rendimiento_general, goles FROM partido
          ORDER BY fecha_hora DESC, id DESC LIMIT 10",
    )?;
    let filas = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    let recientes: Vec<(i64, i64)> = filas.iter().take(5).copied().collect();
    let previos: Vec<(i64, i64)> = filas.iter().skip(5).take(5).copied().collect();
    let promedio = |valores: &[(i64, i64)]| -> f64 {
        if valores.is_empty() {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        {
            valores
                .iter()
                .map(|(rendimiento, _)| *rendimiento as f64)
                .sum::<f64>()
                / valores.len() as f64
        }
    };
    let delta = promedio(&recientes) - promedio(&previos);
    let goles_recientes: i64 = recientes.iter().map(|(_, goles)| goles).sum();
    let goles_previos: i64 = previos.iter().map(|(_, goles)| goles).sum();
    let etiqueta = if recientes.is_empty() {
        "Sin datos"
    } else if delta > 0.75 {
        "En racha"
    } else if delta < -0.75 {
        "Bajón"
    } else {
        "Estable"
    };
    Ok(CarreraPerfil {
        etiqueta: etiqueta.to_string(),
        delta_rendimiento: delta,
        goles_recientes,
        goles_previos,
    })
}

/// Partidos de la misma fecha (día-mes) en años anteriores.
pub fn retro(conn: &Connection) -> Result<Vec<HallPartido>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, fecha_hora, rival_nombre, goles, asistencias, rendimiento_general
           FROM partido
          WHERE substr(fecha_hora, 6, 5) = substr(date('now'), 6, 5)
            AND substr(fecha_hora, 1, 4) < substr(date('now'), 1, 4)
          ORDER BY fecha_hora DESC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            let fecha_texto: String = row.get(1)?;
            let fecha = dates::parse_datetime(&fecha_texto).ok_or_else(|| {
                rusqlite::Error::InvalidColumnType(
                    1,
                    "fecha_hora".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?;
            Ok(HallPartido {
                id: row.get(0)?,
                fecha,
                rival: row.get(2)?,
                goles: row.get(3)?,
                asistencias: row.get(4)?,
                rendimiento: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn mejor_racha(conn: &Connection) -> Result<usize, DbError> {
    let mut stmt = conn.prepare("SELECT resultado FROM partido ORDER BY fecha_hora ASC, id ASC")?;
    let resultados: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut mejor = 0;
    let mut actual = 0;
    for resultado in resultados {
        if resultado == 1 {
            actual += 1;
            mejor = mejor.max(actual);
        } else {
            actual = 0;
        }
    }
    Ok(mejor)
}

fn map_hito(row: &rusqlite::Row<'_>) -> rusqlite::Result<HitoRow> {
    let fecha_texto: String = row.get(2)?;
    let fecha = dates::parse_datetime(&fecha_texto).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(2, "fecha".to_string(), rusqlite::types::Type::Text)
    })?;
    Ok(HitoRow {
        id: row.get(0)?,
        partido_id: row.get(1)?,
        fecha,
        rival: row.get(3)?,
        tipo_hito: row.get(4)?,
        nota: row.get(5)?,
    })
}
