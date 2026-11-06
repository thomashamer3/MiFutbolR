//! Consultas para el Resumen Compartible (§17.3).

use rusqlite::{Connection, OptionalExtension};

use super::error::DbError;
use crate::domain::dates;
use crate::domain::models::{Clima, RachaTipo, Resultado};
use crate::domain::resumen::{
    DatosResumen, Favoritos, MejorPartido, PerfilJuego, Racha, Resultados, UltimoPartido,
};

/// Ensambla los datos del resumen (sin logros, que agrega el servicio).
pub fn datos(conn: &Connection) -> Result<DatosResumen, DbError> {
    let (
        total_partidos,
        total_goles,
        total_asistencias,
        promedio_goles,
        promedio_asistencias,
        victorias,
        empates,
        derrotas,
    ): (i64, i64, i64, f64, f64, i64, i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(goles), 0), COALESCE(SUM(asistencias), 0),
                COALESCE(AVG(goles), 0.0), COALESCE(AVG(asistencias), 0.0),
                COALESCE(SUM(CASE WHEN resultado = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 2 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 3 THEN 1 ELSE 0 END), 0)
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
                row.get(7)?,
            ))
        },
    )?;

    let mejor = conn
        .query_row(
            "SELECT rendimiento_general, fecha_hora FROM partido
              ORDER BY rendimiento_general DESC, goles DESC, asistencias DESC LIMIT 1",
            [],
            |row| {
                let fecha_texto: String = row.get(1)?;
                Ok(MejorPartido {
                    rendimiento: row.get(0)?,
                    fecha: dates::parse_datetime(&fecha_texto).unwrap_or_else(dates::now_local),
                })
            },
        )
        .optional()?
        .unwrap_or_else(|| MejorPartido {
            rendimiento: 0,
            fecha: dates::now_local(),
        });

    let racha = racha(conn)?;
    let favoritos = Favoritos {
        camiseta: top_texto(
            conn,
            "SELECT c.nombre FROM partido p JOIN camiseta c ON c.id = p.camiseta_id
              GROUP BY c.id ORDER BY COUNT(*) DESC, c.nombre ASC LIMIT 1",
        )?
        .unwrap_or_default(),
        cancha: top_texto(
            conn,
            "SELECT c.nombre FROM partido p JOIN cancha c ON c.id = p.cancha_id
              GROUP BY c.id ORDER BY COUNT(*) DESC, c.nombre ASC LIMIT 1",
        )?
        .unwrap_or_default(),
        rival: top_texto(
            conn,
            "SELECT rival_nombre FROM partido WHERE rival_nombre <> ''
              GROUP BY rival_nombre ORDER BY COUNT(*) DESC, rival_nombre ASC LIMIT 1",
        )?
        .unwrap_or_default(),
        posicion: top_texto(
            conn,
            "SELECT posicion_jugada FROM partido WHERE posicion_jugada <> ''
              GROUP BY posicion_jugada ORDER BY COUNT(*) DESC, posicion_jugada ASC LIMIT 1",
        )?
        .unwrap_or_default(),
    };

    let ultimo = conn
        .query_row(
            "SELECT fecha_hora, rival_nombre, goles, asistencias, resultado
               FROM partido ORDER BY fecha_hora DESC, id DESC LIMIT 1",
            [],
            |row| {
                let fecha_texto: String = row.get(0)?;
                Ok(UltimoPartido {
                    fecha: dates::parse_datetime(&fecha_texto).unwrap_or_else(dates::now_local),
                    rival: row.get(1)?,
                    goles: row.get(2)?,
                    asistencias: row.get(3)?,
                    resultado: Resultado::try_from(row.get::<_, i64>(4)?)
                        .unwrap_or(Resultado::NoAplica),
                })
            },
        )
        .optional()?
        .unwrap_or_else(|| UltimoPartido {
            fecha: dates::now_local(),
            rival: String::new(),
            goles: 0,
            asistencias: 0,
            resultado: Resultado::NoAplica,
        });

    let clima_codigo: Option<i64> = conn
        .query_row(
            "SELECT clima FROM partido GROUP BY clima ORDER BY COUNT(*) DESC, clima ASC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let clima = clima_codigo
        .and_then(|codigo| Clima::try_from(codigo).ok())
        .map_or_else(String::new, |clima| clima.label().to_string());

    let torneo = top_texto(
        conn,
        "SELECT t.nombre FROM partido_torneo pt JOIN torneo t ON t.id = pt.torneo_id
          GROUP BY pt.torneo_id ORDER BY COUNT(*) DESC, t.nombre ASC LIMIT 1",
    )?
    .unwrap_or_default();

    let lesiones: i64 = conn.query_row("SELECT COUNT(*) FROM lesion", [], |row| row.get(0))?;

    let jugador = conn
        .query_row(
            "SELECT nombre FROM carrera_identidad WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .unwrap_or_default();

    Ok(DatosResumen {
        jugador: if jugador.trim().is_empty() {
            "Jugador".to_string()
        } else {
            jugador
        },
        total_partidos,
        total_goles,
        total_asistencias,
        promedio_goles,
        promedio_asistencias,
        fecha_gen: dates::now_local(),
        logros_total: 0,
        mejor,
        racha,
        favoritos,
        resultados: Resultados {
            victorias,
            empates,
            derrotas,
        },
        ultimo,
        perfil: PerfilJuego {
            clima,
            torneo,
            lesiones: lesiones.to_string(),
        },
    })
}

/// Cantidad de hitos registrados.
pub fn hitos_count(conn: &Connection) -> Result<i64, DbError> {
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM carrera_partido_hito", [], |row| {
        row.get(0)
    })?;
    Ok(total)
}

fn racha(conn: &Connection) -> Result<Racha, DbError> {
    let mut stmt =
        conn.prepare("SELECT resultado FROM partido ORDER BY fecha_hora DESC, id DESC LIMIT 10")?;
    let resultados: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let Some(ultimo) = resultados.first().copied() else {
        return Ok(Racha {
            tipo: RachaTipo::Neutral,
            descripcion: "Sin partidos registrados".to_string(),
        });
    };
    let largo = resultados
        .iter()
        .take_while(|valor| **valor == ultimo)
        .count();
    match ultimo {
        1 if largo >= 3 => Ok(Racha {
            tipo: RachaTipo::Positiva,
            descripcion: format!("{largo} victorias consecutivas"),
        }),
        3 if largo >= 3 => Ok(Racha {
            tipo: RachaTipo::Negativa,
            descripcion: format!("{largo} derrotas consecutivas"),
        }),
        _ => Ok(Racha {
            tipo: RachaTipo::Neutral,
            descripcion: "Sin racha dominante".to_string(),
        }),
    }
}

fn top_texto(conn: &Connection, sql: &str) -> Result<Option<String>, DbError> {
    let valor = conn
        .query_row(sql, [], |row| row.get::<_, String>(0))
        .optional()?;
    Ok(valor)
}
