//! Repositorio de camisetas (§8.3). SQL preparado y mapeo fila → struct.

use rusqlite::{Connection, Row, params};

use super::error::DbError;
use crate::domain::Cents;
use crate::domain::dates;
use crate::domain::models::{
    Camiseta, CamisetaDependencias, CamisetaFilter, CamisetaRow, EstadoFisico, FichaCamiseta,
};

/// Columnas de `camiseta` en el orden que espera [`map_camiseta`].
const COLUMNS: &str = "id, nombre, color_principal, color_secundario, marca, modelo, temporada,
                       estado_fisico, fecha_compra, costo_centavos, observaciones, proveedor,
                       fue_regalo, regalo_de, imagen_ruta, activa, sorteada";

/// Lista camisetas con agregados de partidos.
pub fn list(conn: &Connection, filter: &CamisetaFilter) -> Result<Vec<CamisetaRow>, DbError> {
    // El ORDER BY se elige de una lista cerrada; nunca se interpola input.
    let sql = format!(
        "SELECT c.id, c.nombre, c.activa, c.imagen_ruta,
                COUNT(p.id) AS partidos,
                COALESCE(SUM(p.goles), 0) AS goles,
                COALESCE(SUM(p.asistencias), 0) AS asistencias,
                COALESCE(SUM(CASE WHEN p.resultado = 1 THEN 1 ELSE 0 END), 0) AS victorias,
                COALESCE(SUM(CASE WHEN p.resultado = 2 THEN 1 ELSE 0 END), 0) AS empates,
                COALESCE(SUM(CASE WHEN p.resultado = 3 THEN 1 ELSE 0 END), 0) AS derrotas
           FROM camiseta c
           LEFT JOIN partido p ON p.camiseta_id = c.id
          WHERE (?1 = 0 OR c.activa = CASE ?1 WHEN 1 THEN 1 ELSE 0 END)
            AND (?2 = '' OR c.nombre LIKE '%' || ?2 || '%' COLLATE NOCASE)
          GROUP BY c.id
          ORDER BY {}",
        filter.orden.sql_order()
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(
            params![filter.estado.sql_code(), filter.texto.trim()],
            |row| {
                Ok(CamisetaRow {
                    id: row.get(0)?,
                    nombre: row.get(1)?,
                    activa: row.get::<_, i64>(2)? != 0,
                    imagen_ruta: row.get(3)?,
                    partidos: row.get(4)?,
                    goles: row.get(5)?,
                    asistencias: row.get(6)?,
                    victorias: row.get(7)?,
                    empates: row.get(8)?,
                    derrotas: row.get(9)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Obtiene una camiseta por id.
pub fn get(conn: &Connection, id: i64) -> Result<Camiseta, DbError> {
    let sql = format!("SELECT {COLUMNS} FROM camiseta WHERE id = ?1");
    conn.query_row(&sql, [id], map_camiseta)
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                entidad: "Camiseta",
                id,
            },
            other => other.into(),
        })
}

/// Inserta una camiseta y devuelve su id.
pub fn insert(conn: &Connection, camiseta: &Camiseta) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO camiseta
            (nombre, color_principal, color_secundario, marca, modelo, temporada,
             estado_fisico, fecha_compra, costo_centavos, observaciones, proveedor,
             fue_regalo, regalo_de, imagen_ruta, activa, sorteada, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                 datetime('now','localtime'))",
        params![
            camiseta.nombre,
            camiseta.color_principal,
            camiseta.color_secundario,
            camiseta.marca,
            camiseta.modelo,
            camiseta.temporada,
            camiseta.estado_fisico.as_str(),
            camiseta
                .fecha_compra
                .map(dates::to_iso_date)
                .unwrap_or_default(),
            camiseta.costo.0,
            camiseta.observaciones,
            camiseta.proveedor,
            i64::from(camiseta.fue_regalo),
            camiseta.regalo_de,
            camiseta.imagen_ruta,
            i64::from(camiseta.activa),
            i64::from(camiseta.sorteada),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza todos los campos editables de una camiseta.
pub fn update(conn: &Connection, camiseta: &Camiseta) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE camiseta
            SET nombre = ?1, color_principal = ?2, color_secundario = ?3, marca = ?4,
                modelo = ?5, temporada = ?6, estado_fisico = ?7, fecha_compra = ?8,
                costo_centavos = ?9, observaciones = ?10, proveedor = ?11,
                fue_regalo = ?12, regalo_de = ?13, imagen_ruta = ?14, activa = ?15,
                sorteada = ?16, updated_at = datetime('now','localtime')
          WHERE id = ?17",
        params![
            camiseta.nombre,
            camiseta.color_principal,
            camiseta.color_secundario,
            camiseta.marca,
            camiseta.modelo,
            camiseta.temporada,
            camiseta.estado_fisico.as_str(),
            camiseta
                .fecha_compra
                .map(dates::to_iso_date)
                .unwrap_or_default(),
            camiseta.costo.0,
            camiseta.observaciones,
            camiseta.proveedor,
            i64::from(camiseta.fue_regalo),
            camiseta.regalo_de,
            camiseta.imagen_ruta,
            i64::from(camiseta.activa),
            i64::from(camiseta.sorteada),
            camiseta.id,
        ],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Camiseta",
            id: camiseta.id,
        });
    }
    Ok(())
}

/// Activa o desactiva una camiseta.
pub fn set_active(conn: &Connection, id: i64, active: bool) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE camiseta SET activa = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
        params![i64::from(active), id],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Camiseta",
            id,
        });
    }
    Ok(())
}

/// Elimina físicamente una camiseta (el servicio decide si corresponde).
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    let deleted = conn.execute("DELETE FROM camiseta WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(DbError::NotFound {
            entidad: "Camiseta",
            id,
        });
    }
    Ok(())
}

/// Cuenta las dependencias que impiden borrar una camiseta.
pub fn dependencias(conn: &Connection, id: i64) -> Result<CamisetaDependencias, DbError> {
    let partidos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM partido WHERE camiseta_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    let lesiones: i64 = conn.query_row(
        "SELECT COUNT(*) FROM lesion WHERE camiseta_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    Ok(CamisetaDependencias { partidos, lesiones })
}

/// Ficha con estadísticas derivadas.
pub fn ficha(conn: &Connection, id: i64) -> Result<FichaCamiseta, DbError> {
    conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM partido WHERE camiseta_id = ?1),
            (SELECT COALESCE(SUM(goles), 0) FROM partido WHERE camiseta_id = ?1),
            (SELECT COALESCE(SUM(asistencias), 0) FROM partido WHERE camiseta_id = ?1),
            (SELECT COUNT(*) FROM lesion WHERE camiseta_id = ?1)",
        [id],
        |row| {
            Ok(FichaCamiseta {
                partidos: row.get(0)?,
                goles: row.get(1)?,
                asistencias: row.get(2)?,
                lesiones: row.get(3)?,
            })
        },
    )
    .map_err(Into::into)
}

/// Comprueba si ya existe una camiseta con ese nombre (sin distinguir mayúsculas).
pub fn existe_nombre(
    conn: &Connection,
    nombre: &str,
    exclude_id: Option<i64>,
) -> Result<bool, DbError> {
    let exclude = exclude_id.unwrap_or(0);
    let exists: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM camiseta
             WHERE lower(nombre) = lower(?1)
               AND (?2 = 0 OR id <> ?2)
        )",
        params![nombre.trim(), exclude],
        |row| row.get(0),
    )?;
    Ok(exists)
}

/// Camisetas activas pendientes de sorteo.
pub fn draw_pending(conn: &Connection) -> Result<Vec<Camiseta>, DbError> {
    let sql =
        format!("SELECT {COLUMNS} FROM camiseta WHERE activa = 1 AND sorteada = 0 ORDER BY id");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], map_camiseta)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Marca una camiseta como sorteada.
pub fn mark_drawn(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE camiseta SET sorteada = 1, updated_at = datetime('now','localtime') WHERE id = ?1",
        [id],
    )?;
    Ok(())
}

/// Reinicia el ciclo de sorteo de todas las camisetas activas.
pub fn reset_draw(conn: &Connection) -> Result<usize, DbError> {
    let updated = conn.execute(
        "UPDATE camiseta SET sorteada = 0, updated_at = datetime('now','localtime') WHERE activa = 1",
        [],
    )?;
    Ok(updated)
}

fn map_camiseta(row: &Row<'_>) -> rusqlite::Result<Camiseta> {
    let estado_texto: String = row.get(7)?;
    let fecha_texto: String = row.get(8)?;
    Ok(Camiseta {
        id: row.get(0)?,
        nombre: row.get(1)?,
        color_principal: row.get(2)?,
        color_secundario: row.get(3)?,
        marca: row.get(4)?,
        modelo: row.get(5)?,
        temporada: row.get(6)?,
        estado_fisico: EstadoFisico::from_str(&estado_texto).unwrap_or(EstadoFisico::Otra),
        fecha_compra: dates::parse_date(&fecha_texto),
        costo: Cents(row.get(9)?),
        observaciones: row.get(10)?,
        proveedor: row.get(11)?,
        fue_regalo: row.get::<_, i64>(12)? != 0,
        regalo_de: row.get(13)?,
        imagen_ruta: row.get(14)?,
        activa: row.get::<_, i64>(15)? != 0,
        sorteada: row.get::<_, i64>(16)? != 0,
    })
}
