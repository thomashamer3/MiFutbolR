//! Agregaciones SQL para el módulo Estadísticas (§14).
//!
//! Las columnas de agrupación se eligen de listas cerradas: nunca se interpola
//! texto del usuario en el SQL.

use rusqlite::Connection;

use super::error::DbError;
use crate::domain::models::{
    CanchaStats, CanchasStats, Clima, EstadisticasGenerales, FranjaHoraria, GolesStats, MetaStats,
    PeriodoStats, PromedioGrupo, RendimientoStats, Resultado, Superficie, TechadaEstado,
    TipoCancha,
};

/// Agregados generales con rachas, consistencia y extremos.
pub fn generales(conn: &Connection) -> Result<EstadisticasGenerales, DbError> {
    let (
        total,
        goles,
        asistencias,
        promedio_goles,
        promedio_asistencias,
        promedio_rendimiento,
        victorias,
        empates,
        derrotas,
    ): (i64, i64, i64, f64, f64, f64, i64, i64, i64) = conn.query_row(
        "SELECT COUNT(*),
                    COALESCE(SUM(goles), 0),
                    COALESCE(SUM(asistencias), 0),
                    COALESCE(AVG(goles), 0.0),
                    COALESCE(AVG(asistencias), 0.0),
                    COALESCE(AVG(rendimiento_general), 0.0),
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
                row.get(8)?,
            ))
        },
    )?;

    let porcentaje_victorias = if total > 0 {
        #[allow(clippy::cast_precision_loss)]
        {
            victorias as f64 * 100.0 / total as f64
        }
    } else {
        0.0
    };

    let (racha_actual, mejor_racha) = rachas(conn)?;
    let consistencia: f64 = conn.query_row(
        "SELECT COALESCE(AVG(rendimiento_general * rendimiento_general)
                         - AVG(rendimiento_general) * AVG(rendimiento_general), 0.0)
           FROM partido",
        [],
        |row| row.get(0),
    )?;

    Ok(EstadisticasGenerales {
        total,
        goles,
        asistencias,
        promedio_goles,
        promedio_asistencias,
        promedio_rendimiento,
        victorias,
        empates,
        derrotas,
        porcentaje_victorias,
        racha_actual,
        mejor_racha,
        consistencia: consistencia.max(0.0).sqrt(),
        mejor_partido: extremo(conn, true)?,
        peor_partido: extremo(conn, false)?,
    })
}

/// Serie mensual.
pub fn por_mes(conn: &Connection) -> Result<Vec<PeriodoStats>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT mes_anio, COUNT(*), COALESCE(SUM(goles), 0), COALESCE(SUM(asistencias), 0),
                COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido
          GROUP BY mes_anio
          ORDER BY mes_anio ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PeriodoStats {
                clave: row.get(0)?,
                partidos: row.get(1)?,
                goles: row.get(2)?,
                asistencias: row.get(3)?,
                rendimiento: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Serie anual.
pub fn por_anio(conn: &Connection) -> Result<Vec<PeriodoStats>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT substr(fecha_hora, 1, 4) AS anio, COUNT(*), COALESCE(SUM(goles), 0),
                COALESCE(SUM(asistencias), 0), COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido
          GROUP BY anio
          ORDER BY anio ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PeriodoStats {
                clave: row.get(0)?,
                partidos: row.get(1)?,
                goles: row.get(2)?,
                asistencias: row.get(3)?,
                rendimiento: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Récords simples.
pub fn records(conn: &Connection) -> Result<Vec<(String, String)>, DbError> {
    let mut registros = Vec::new();
    for (etiqueta, sql) in [
        (
            "Más goles en un partido",
            "SELECT COALESCE(MAX(goles), 0) FROM partido",
        ),
        (
            "Más asistencias en un partido",
            "SELECT COALESCE(MAX(asistencias), 0) FROM partido",
        ),
        (
            "Mejor rendimiento",
            "SELECT COALESCE(MAX(rendimiento_general), 0) FROM partido",
        ),
        (
            "Más minutos jugados",
            "SELECT COALESCE(MAX(minutos_jugados), 0) FROM partido",
        ),
        (
            "Más goles en un mes",
            "SELECT COALESCE(MAX(total), 0) FROM (SELECT SUM(goles) AS total FROM partido GROUP BY mes_anio)",
        ),
    ] {
        let valor: i64 = conn.query_row(sql, [], |row| row.get(0))?;
        registros.push((etiqueta.to_string(), valor.to_string()));
    }
    Ok(registros)
}

/// Métricas de rendimiento.
pub fn rendimiento(conn: &Connection) -> Result<RendimientoStats, DbError> {
    let promedio: f64 = conn.query_row(
        "SELECT COALESCE(AVG(rendimiento_general), 0.0) FROM partido",
        [],
        |row| row.get(0),
    )?;

    let por_clima = grupo_etiquetado(
        conn,
        "SELECT clima, COUNT(*), COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido GROUP BY clima ORDER BY clima",
        |codigo| {
            Clima::try_from(codigo).map_or_else(|_| "?".into(), |clima| clima.label().to_string())
        },
    )?;

    let por_dia_semana = grupo_etiquetado(
        conn,
        "SELECT CAST(strftime('%w', fecha_hora) AS INTEGER) AS dia_semana, COUNT(*),
                COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido GROUP BY dia_semana ORDER BY dia_semana",
        dia_semana,
    )?;

    let por_franja = grupo_etiquetado(
        conn,
        "SELECT dia, COUNT(*), COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido GROUP BY dia ORDER BY dia",
        |codigo| {
            FranjaHoraria::try_from(codigo)
                .map_or_else(|_| "?".into(), |franja| franja.label().to_string())
        },
    )?;

    let por_cansancio = grupo_numerico(
        conn,
        "SELECT cansancio, COUNT(*), COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido GROUP BY cansancio ORDER BY cansancio",
    )?;
    let por_animo = grupo_numerico(
        conn,
        "SELECT estado_animo, COUNT(*), COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido GROUP BY estado_animo ORDER BY estado_animo",
    )?;
    let por_intensidad = grupo_numerico(
        conn,
        "SELECT intensidad, COUNT(*), COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido GROUP BY intensidad ORDER BY intensidad",
    )?;
    let por_dolor = grupo_numerico(
        conn,
        "SELECT dolor_fisico, COUNT(*), COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido GROUP BY dolor_fisico ORDER BY dolor_fisico",
    )?;
    let por_arbitraje = grupo_numerico(
        conn,
        "SELECT arbitraje_score, COUNT(*), COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido GROUP BY arbitraje_score ORDER BY arbitraje_score",
    )?;

    let mut stmt = conn.prepare(
        "SELECT CASE
                    WHEN temperatura_c IS NULL THEN 'Sin dato'
                    WHEN temperatura_c <= 5 THEN '≤ 5 °C'
                    WHEN temperatura_c <= 15 THEN '6-15 °C'
                    WHEN temperatura_c <= 25 THEN '16-25 °C'
                    WHEN temperatura_c <= 35 THEN '26-35 °C'
                    ELSE '> 35 °C'
                END AS rango,
                COUNT(*),
                COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido
          GROUP BY rango
          ORDER BY MIN(COALESCE(temperatura_c, 999))",
    )?;
    let por_temperatura = stmt
        .query_map([], |row| {
            Ok(PromedioGrupo {
                clave: row.get(0)?,
                partidos: row.get(1)?,
                promedio: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT rendimiento_general FROM partido ORDER BY fecha_hora DESC, id DESC LIMIT 20",
    )?;
    let mut tendencia = stmt
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    tendencia.reverse();
    let tendencia = tendencia.into_iter().map(|valor| valor as f64).collect();

    Ok(RendimientoStats {
        promedio,
        por_clima,
        por_dia_semana,
        por_franja,
        por_cansancio,
        por_animo,
        por_intensidad,
        por_dolor,
        por_arbitraje,
        por_temperatura,
        tendencia,
    })
}

/// Métricas de canchas.
pub fn canchas(conn: &Connection) -> Result<CanchasStats, DbError> {
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM cancha", [], |row| row.get(0))?;
    let activas: i64 =
        conn.query_row("SELECT COUNT(*) FROM cancha WHERE activa = 1", [], |row| {
            row.get(0)
        })?;
    let partidos: i64 = conn.query_row("SELECT COUNT(*) FROM partido", [], |row| row.get(0))?;

    let mut stmt = conn.prepare(
        "SELECT c.nombre, COUNT(p.id), COALESCE(SUM(p.goles), 0),
                COALESCE(SUM(p.asistencias), 0), COALESCE(AVG(p.rendimiento_general), 0.0)
           FROM cancha c
           LEFT JOIN partido p ON p.cancha_id = c.id
          GROUP BY c.id
          ORDER BY COUNT(p.id) DESC, c.nombre ASC",
    )?;
    let ranking = stmt
        .query_map([], |row| {
            Ok(CanchaStats {
                nombre: row.get(0)?,
                partidos: row.get(1)?,
                goles: row.get(2)?,
                asistencias: row.get(3)?,
                rendimiento: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let por_techada = grupo_etiquetado(
        conn,
        "SELECT c.techada_estado_codigo, COUNT(p.id),
                COALESCE(AVG(p.rendimiento_general), 0.0)
           FROM cancha c LEFT JOIN partido p ON p.cancha_id = c.id
          GROUP BY c.techada_estado_codigo ORDER BY c.techada_estado_codigo",
        |codigo| {
            TechadaEstado::try_from(codigo)
                .map_or_else(|_| "?".into(), |estado| estado.label().to_string())
        },
    )?;

    let por_tipo = grupo_etiquetado(
        conn,
        "SELECT ct.tipo_codigo, COUNT(p.id),
                COALESCE(AVG(p.rendimiento_general), 0.0)
           FROM cancha_tipo ct
           JOIN cancha c ON c.id = ct.cancha_id
           LEFT JOIN partido p ON p.cancha_id = c.id
          GROUP BY ct.tipo_codigo ORDER BY ct.tipo_codigo",
        |codigo| {
            TipoCancha::try_from(codigo)
                .map_or_else(|_| "?".into(), |tipo| tipo.label().to_string())
        },
    )?;

    let por_superficie = grupo_etiquetado(
        conn,
        "SELECT cs.superficie_codigo, COUNT(p.id),
                COALESCE(AVG(p.rendimiento_general), 0.0)
           FROM cancha_superficie cs
           JOIN cancha c ON c.id = cs.cancha_id
           LEFT JOIN partido p ON p.cancha_id = c.id
          GROUP BY cs.superficie_codigo ORDER BY cs.superficie_codigo",
        |codigo| {
            Superficie::try_from(codigo)
                .map_or_else(|_| "?".into(), |superficie| superficie.label().to_string())
        },
    )?;

    let mut stmt = conn.prepare(
        "SELECT CASE
                    WHEN CAST(substr(p.fecha_hora, 12, 2) AS INTEGER) < 12 THEN 'Mañana'
                    WHEN CAST(substr(p.fecha_hora, 12, 2) AS INTEGER) < 18 THEN 'Tarde'
                    ELSE 'Noche'
                END AS franja,
                COUNT(*), COALESCE(AVG(p.rendimiento_general), 0.0)
           FROM partido p
          GROUP BY franja
          ORDER BY CASE franja WHEN 'Mañana' THEN 1 WHEN 'Tarde' THEN 2 ELSE 3 END",
    )?;
    let por_franja = stmt
        .query_map([], |row| {
            Ok(PromedioGrupo {
                clave: row.get(0)?,
                partidos: row.get(1)?,
                promedio: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT c.localidad, COUNT(p.id), COALESCE(AVG(p.rendimiento_general), 0.0)
           FROM cancha c LEFT JOIN partido p ON p.cancha_id = c.id
          WHERE c.localidad <> ''
          GROUP BY c.localidad
          ORDER BY COUNT(p.id) DESC, c.localidad ASC",
    )?;
    let por_localidad = stmt
        .query_map([], |row| {
            Ok(PromedioGrupo {
                clave: row.get(0)?,
                partidos: row.get(1)?,
                promedio: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CanchasStats {
        total,
        activas,
        partidos,
        ranking,
        por_techada,
        por_tipo,
        por_superficie,
        por_franja,
        por_localidad,
    })
}

/// Métricas de goles y asistencias.
pub fn goles(conn: &Connection) -> Result<GolesStats, DbError> {
    let (promedio_goles, promedio_asistencias, efectividad, eficiencia): (f64, f64, f64, f64) =
        conn.query_row(
            "SELECT COALESCE(AVG(goles), 0.0),
                    COALESCE(AVG(asistencias), 0.0),
                    CASE WHEN COUNT(*) = 0 THEN 0.0
                         ELSE CAST(SUM(goles + asistencias) AS REAL) / COUNT(*) END,
                    COALESCE(AVG(CASE WHEN rendimiento_general > 0
                                      THEN CAST(goles AS REAL) / rendimiento_general END), 0.0)
               FROM partido",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

    let por_clima = grupo_promedio_goles(
        conn,
        "SELECT clima, COUNT(*), COALESCE(AVG(goles), 0.0)
           FROM partido GROUP BY clima ORDER BY clima",
        |codigo| {
            Clima::try_from(codigo).map_or_else(|_| "?".into(), |clima| clima.label().to_string())
        },
    )?;
    let por_franja = grupo_promedio_goles(
        conn,
        "SELECT dia, COUNT(*), COALESCE(AVG(goles), 0.0)
           FROM partido GROUP BY dia ORDER BY dia",
        |codigo| {
            FranjaHoraria::try_from(codigo)
                .map_or_else(|_| "?".into(), |franja| franja.label().to_string())
        },
    )?;

    let goles_cansancio_alto: f64 = conn.query_row(
        "SELECT COALESCE(AVG(goles), 0.0) FROM partido WHERE cansancio > 7",
        [],
        |row| row.get(0),
    )?;
    let goles_cansancio_bajo: f64 = conn.query_row(
        "SELECT COALESCE(AVG(goles), 0.0) FROM partido WHERE cansancio <= 7",
        [],
        |row| row.get(0),
    )?;

    let atipicos = atipicos(conn)?;
    let exigentes_bien: i64 = conn.query_row(
        "SELECT COUNT(*) FROM partido WHERE intensidad >= 7 AND rendimiento_general >= 7",
        [],
        |row| row.get(0),
    )?;
    let faciles_mal: i64 = conn.query_row(
        "SELECT COUNT(*) FROM partido WHERE intensidad <= 4 AND rendimiento_general <= 4",
        [],
        |row| row.get(0),
    )?;

    Ok(GolesStats {
        promedio_goles,
        promedio_asistencias,
        por_clima,
        por_franja,
        goles_cansancio_alto,
        goles_cansancio_bajo,
        eficiencia,
        efectividad,
        atipicos,
        exigentes_bien,
        faciles_mal,
    })
}

fn rachas(conn: &Connection) -> Result<(Option<(String, usize)>, usize), DbError> {
    let mut stmt = conn.prepare("SELECT resultado FROM partido ORDER BY fecha_hora ASC, id ASC")?;
    let resultados: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let Some(ultimo) = resultados.last() else {
        return Ok((None, 0));
    };
    let largo = resultados
        .iter()
        .rev()
        .take_while(|valor| *valor == ultimo)
        .count();
    let etiqueta = Resultado::try_from(*ultimo).map_or_else(
        |_| "?".to_string(),
        |resultado| resultado.label().to_string(),
    );
    let mut mejor = 0;
    let mut actual = 0;
    for resultado in &resultados {
        if *resultado == 1 {
            actual += 1;
            mejor = mejor.max(actual);
        } else {
            actual = 0;
        }
    }
    Ok((Some((etiqueta, largo)), mejor))
}

fn extremo(conn: &Connection, mejor: bool) -> Result<Option<String>, DbError> {
    let orden = if mejor { "DESC" } else { "ASC" };
    let sql = format!(
        "SELECT id, fecha_hora, rival_nombre, goles, asistencias, rendimiento_general
           FROM partido
          ORDER BY (goles * 2 + asistencias + rendimiento_general) {orden}, id ASC
          LIMIT 1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;
    match rows.next() {
        Some(Ok((id, fecha, rival, goles, asistencias, rendimiento))) => {
            let rival = if rival.trim().is_empty() {
                "sin rival"
            } else {
                rival.as_str()
            };
            Ok(Some(format!(
                "#{id} · {fecha} · {rival} · {goles} G, {asistencias} A, rend. {rendimiento}"
            )))
        }
        Some(Err(error)) => Err(error.into()),
        None => Ok(None),
    }
}

fn atipicos(conn: &Connection) -> Result<i64, DbError> {
    let mut stmt = conn.prepare("SELECT goles FROM partido")?;
    let valores: Vec<f64> = stmt
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|valor| valor as f64)
        .collect();
    if valores.len() < 2 {
        return Ok(0);
    }
    #[allow(clippy::cast_precision_loss)]
    let media = valores.iter().sum::<f64>() / valores.len() as f64;
    #[allow(clippy::cast_precision_loss)]
    let varianza = valores
        .iter()
        .map(|valor| (valor - media) * (valor - media))
        .sum::<f64>()
        / valores.len() as f64;
    let desvio = varianza.sqrt();
    if desvio == 0.0 {
        return Ok(0);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let atipicos = valores
        .iter()
        .filter(|valor| ((*valor - media) / desvio).abs() > 2.0)
        .count() as i64;
    Ok(atipicos)
}

fn grupo_etiquetado<F>(
    conn: &Connection,
    sql: &str,
    etiqueta: F,
) -> Result<Vec<PromedioGrupo>, DbError>
where
    F: Fn(i64) -> String,
{
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map([], |row| {
            let codigo: i64 = row.get(0)?;
            Ok(PromedioGrupo {
                clave: etiqueta(codigo),
                partidos: row.get(1)?,
                promedio: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn grupo_numerico(conn: &Connection, sql: &str) -> Result<Vec<PromedioGrupo>, DbError> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map([], |row| {
            let valor: i64 = row.get(0)?;
            Ok(PromedioGrupo {
                clave: valor.to_string(),
                partidos: row.get(1)?,
                promedio: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn grupo_promedio_goles<F>(
    conn: &Connection,
    sql: &str,
    etiqueta: F,
) -> Result<Vec<PromedioGrupo>, DbError>
where
    F: Fn(i64) -> String,
{
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map([], |row| {
            let codigo: i64 = row.get(0)?;
            Ok(PromedioGrupo {
                clave: etiqueta(codigo),
                partidos: row.get(1)?,
                promedio: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn dia_semana(codigo: i64) -> String {
    match codigo {
        0 => "Domingo".to_string(),
        1 => "Lunes".to_string(),
        2 => "Martes".to_string(),
        3 => "Miércoles".to_string(),
        4 => "Jueves".to_string(),
        5 => "Viernes".to_string(),
        6 => "Sábado".to_string(),
        _ => "?".to_string(),
    }
}
/// Métricas de meta-análisis (IMP-15).
pub fn meta(conn: &Connection) -> Result<MetaStats, DbError> {
    let mut stmt = conn.prepare("SELECT rendimiento_general FROM partido")?;
    let valores = stmt
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let partidos = valores.len() as i64;
    let promedio = if partidos > 0 {
        valores.iter().map(|valor| *valor as f64).sum::<f64>() / partidos as f64
    } else {
        0.0
    };
    let desvio = if partidos > 0 {
        let varianza = valores
            .iter()
            .map(|valor| {
                let diferencia = *valor as f64 - promedio;
                diferencia * diferencia
            })
            .sum::<f64>()
            / partidos as f64;
        varianza.sqrt()
    } else {
        0.0
    };
    let cv = if promedio.abs() > f64::EPSILON {
        desvio * 100.0 / promedio
    } else {
        0.0
    };
    let outliers = if desvio > f64::EPSILON {
        valores
            .iter()
            .filter(|valor| (**valor as f64 - promedio).abs() > 2.0 * desvio)
            .count() as i64
    } else {
        0
    };
    let minimo = valores.iter().copied().min().unwrap_or(0);
    let maximo = valores.iter().copied().max().unwrap_or(0);

    let promedio_cond = |condicion: &str| -> Result<f64, DbError> {
        let sql = format!(
            "SELECT COALESCE(AVG(rendimiento_general), 0.0) FROM partido WHERE {condicion}"
        );
        Ok(conn.query_row(&sql, [], |row| row.get(0))?)
    };

    let (goles_por_partido, asistencias_por_partido): (f64, f64) = conn.query_row(
        "SELECT COALESCE(AVG(goles), 0.0), COALESCE(AVG(asistencias), 0.0) FROM partido",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let mut stmt = conn.prepare(
        "SELECT c.nombre, AVG(p.rendimiento_general)
           FROM partido p JOIN cancha c ON c.id = p.cancha_id
          GROUP BY c.id
         HAVING COUNT(*) >= 2
          ORDER BY AVG(p.rendimiento_general) DESC",
    )?;
    let contextos = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mejor_contexto = contextos.first().cloned();
    let peor_contexto = contextos.last().cloned();

    let mut stmt = conn.prepare(
        "SELECT rendimiento_general FROM partido ORDER BY fecha_hora DESC, id DESC LIMIT 12",
    )?;
    let mut tendencia = stmt
        .query_map([], |row| row.get::<_, f64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    tendencia.reverse();

    Ok(MetaStats {
        partidos,
        promedio,
        desvio,
        cv,
        minimo,
        maximo,
        outliers,
        cansancio_alto: promedio_cond("cansancio >= 7")?,
        cansancio_bajo: promedio_cond("cansancio <= 3")?,
        animo_alto: promedio_cond("estado_animo >= 7")?,
        animo_bajo: promedio_cond("estado_animo <= 3")?,
        goles_por_partido,
        asistencias_por_partido,
        mejor_contexto,
        peor_contexto,
        tendencia,
    })
}

#[cfg(test)]
mod canchas_extra_tests {
    use super::*;
    use crate::db::Database;
    use rusqlite::params;

    #[test]
    fn canchas_reports_cover_type_surface_and_slot() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute(
                "INSERT INTO cancha (nombre, localidad) VALUES ('El Potrero', 'Lanús')",
                [],
            )
            .unwrap();
        let cancha = db.conn().last_insert_rowid();
        db.conn()
            .execute(
                "INSERT INTO cancha_tipo (cancha_id, tipo_codigo) VALUES (?1, 7)",
                params![cancha],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO cancha_superficie (cancha_id, superficie_codigo) VALUES (?1, 2)",
                params![cancha],
            )
            .unwrap();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Azul')", [])
            .unwrap();
        let camiseta = db.conn().last_insert_rowid();
        db.conn()
            .execute(
                "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, goles, asistencias,
                                      rendimiento_general, resultado)
                 VALUES (?1, ?2, '2026-01-10 21:00:00', 2, 1, 8, 1)",
                params![cancha, camiseta],
            )
            .unwrap();

        let stats = canchas(db.conn()).unwrap();
        assert_eq!(stats.ranking.len(), 1);
        assert_eq!(stats.por_tipo.len(), 1);
        assert_eq!(stats.por_superficie.len(), 1);
        assert_eq!(stats.por_franja.len(), 1);
        assert!(stats.por_franja[0].clave.contains("Noche"));
        assert_eq!(stats.por_localidad[0].clave, "Lanús");
    }
}
