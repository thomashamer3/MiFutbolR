//! Consultas de los reportes de Estadísticas portados de MiFutbolC
//! (`posiciones.c`, `filtros.c`, `reportes.c` y las opciones de
//! `estadisticas*.c` que faltaban). Las expresiones de agrupación son
//! constantes del código: nunca se interpola texto del usuario.

use chrono::NaiveDate;
use rusqlite::{Connection, params};

use super::error::DbError;

/// PJ/PG/PE/PP/GF/GC de un conjunto de partidos.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totales {
    /// Jugados.
    pub pj: i64,
    /// Ganados.
    pub pg: i64,
    /// Empatados.
    pub pe: i64,
    /// Perdidos.
    pub pp: i64,
    /// Goles a favor.
    pub gf: i64,
    /// Goles en contra.
    pub gc: i64,
}

impl Totales {
    /// Puntos (3 por victoria, 1 por empate).
    #[must_use]
    pub const fn puntos(&self) -> i64 {
        self.pg * 3 + self.pe
    }
}

/// Tabla de posiciones por año (más reciente primero). C sumaba los goles
/// personales como "a favor"; aquí se usa el marcador del equipo cuando está
/// cargado (`goles_equipo`/`goles_rival` >= 0) y si no, lo de C.
pub fn posiciones_por_anio(conn: &Connection) -> Result<Vec<(String, Totales)>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT substr(fecha_hora, 1, 4), COUNT(*),
                SUM(resultado = 1), SUM(resultado = 2), SUM(resultado = 3),
                SUM(CASE WHEN goles_equipo >= 0 THEN goles_equipo ELSE goles END),
                SUM(CASE WHEN goles_rival >= 0 THEN goles_rival ELSE goles_en_contra END)
           FROM partido WHERE resultado > 0
          GROUP BY 1 ORDER BY 1 DESC",
    )?;
    let filas = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                Totales {
                    pj: row.get(1)?,
                    pg: row.get(2)?,
                    pe: row.get(3)?,
                    pp: row.get(4)?,
                    gf: row.get(5)?,
                    gc: row.get(6)?,
                },
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(filas)
}

/// `(clave, partidos, suma, promedio)` de `metrica` agrupado por `clave_sql`,
/// mejor promedio primero.
pub fn agrupado(
    conn: &Connection,
    clave_sql: &str,
    metrica: &str,
) -> Result<Vec<(String, i64, i64, f64)>, DbError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT CAST({clave_sql} AS TEXT), COUNT(*), COALESCE(SUM({metrica}), 0),
                COALESCE(AVG({metrica}), 0.0)
           FROM partido GROUP BY 1 ORDER BY 4 DESC, 1"
    ))?;
    let filas = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(filas)
}

/// Rendimiento promedio de los 5 partidos con cansancio alto (> 7) más
/// recientes (`recientes`) o más antiguos.
pub fn rendimiento_cansado(conn: &Connection, recientes: bool) -> Result<Option<f64>, DbError> {
    let orden = if recientes { "DESC" } else { "ASC" };
    Ok(conn.query_row(
        &format!(
            "SELECT AVG(r) FROM (SELECT rendimiento_general AS r FROM partido
              WHERE cansancio > 7 ORDER BY fecha_hora {orden} LIMIT 5)"
        ),
        [],
        |row| row.get(0),
    )?)
}

/// Partido resumido para listados de reportes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilaPartido {
    /// Id.
    pub id: i64,
    /// Fecha y hora ISO.
    pub fecha_hora: String,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Código de resultado (1 V, 2 E, 3 D).
    pub resultado: i64,
    /// Rendimiento.
    pub rendimiento: i64,
    /// Cancha.
    pub cancha: String,
}

/// Filtros avanzados (`filtros.c`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FiltroPartidos {
    /// Desde (inclusive).
    pub desde: Option<NaiveDate>,
    /// Hasta (inclusive, el día completo).
    pub hasta: Option<NaiveDate>,
    /// Resultado 1/2/3.
    pub resultado: Option<i64>,
    /// Cancha.
    pub cancha_id: Option<i64>,
    /// Solo cansancio alto (> 7).
    pub cansancio_alto: bool,
}

/// Partidos que cumplen el filtro, más recientes primero (máx. 100 como C).
/// C comparaba `fecha_hora <= 'AAAA-MM-DD'` y dejaba afuera ese día.
pub fn filtrar(conn: &Connection, filtro: &FiltroPartidos) -> Result<Vec<FilaPartido>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.fecha_hora, p.goles, p.asistencias, p.resultado,
                p.rendimiento_general, COALESCE(c.nombre, '')
           FROM partido p LEFT JOIN cancha c ON c.id = p.cancha_id
          WHERE (?1 IS NULL OR date(p.fecha_hora) >= ?1)
            AND (?2 IS NULL OR date(p.fecha_hora) <= ?2)
            AND (?3 IS NULL OR p.resultado = ?3)
            AND (?4 IS NULL OR p.cancha_id = ?4)
            AND (?5 = 0 OR p.cansancio > 7)
          ORDER BY p.fecha_hora DESC LIMIT 100",
    )?;
    let fecha = |d: Option<NaiveDate>| d.map(|d| d.format("%Y-%m-%d").to_string());
    let filas = stmt
        .query_map(
            params![
                fecha(filtro.desde),
                fecha(filtro.hasta),
                filtro.resultado,
                filtro.cancha_id,
                filtro.cansancio_alto
            ],
            |row| {
                Ok(FilaPartido {
                    id: row.get(0)?,
                    fecha_hora: row.get(1)?,
                    goles: row.get(2)?,
                    asistencias: row.get(3)?,
                    resultado: row.get(4)?,
                    rendimiento: row.get(5)?,
                    cancha: row.get(6)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(filas)
}

/// Un número de una consulta sin parámetros.
pub fn contar(conn: &Connection, sql: &str) -> Result<i64, DbError> {
    Ok(conn.query_row(sql, [], |row| row.get(0))?)
}

/// Registra un reporte generado.
pub fn registrar_generado(conn: &Connection, nombre: &str, archivo: &str) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO reporte_generado (nombre, archivo) VALUES (?1, ?2)",
        params![nombre, archivo],
    )?;
    Ok(())
}

/// Últimos 20 reportes generados: `(nombre, archivo, fecha)`.
pub fn generados(conn: &Connection) -> Result<Vec<(String, String, String)>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT nombre, archivo, fecha_generacion FROM reporte_generado
          ORDER BY id DESC LIMIT 20",
    )?;
    let filas = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(filas)
}
