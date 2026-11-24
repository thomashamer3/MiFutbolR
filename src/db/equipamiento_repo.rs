//! Persistencia del historial de equipamiento.

use rusqlite::{Connection, Row, params};

use super::error::DbError;
use crate::domain::equipamiento::Equipamiento;
use crate::domain::money::Cents;

const COLUMNAS: &str = "id, tipo, marca, modelo, fecha_compra, precio_centavos, partidos_usados,
                        estado_fisico, rating, notas, activo";

fn mapear(row: &Row<'_>) -> rusqlite::Result<Equipamiento> {
    Ok(Equipamiento {
        id: row.get(0)?,
        tipo: row.get(1)?,
        marca: row.get(2)?,
        modelo: row.get(3)?,
        fecha_compra: row.get(4)?,
        precio: Cents::from(row.get::<_, i64>(5)?),
        partidos_usados: row.get(6)?,
        estado_fisico: row.get(7)?,
        rating: row.get(8)?,
        notas: row.get(9)?,
        activo: row.get::<_, i64>(10)? != 0,
    })
}

/// Lista (activos primero, como el listado de C por id).
pub fn list(conn: &Connection) -> Result<Vec<Equipamiento>, DbError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNAS} FROM equipamiento_historial ORDER BY activo DESC, id"
    ))?;
    let filas = stmt.query_map([], mapear)?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Inserta un ítem.
pub fn insert(conn: &Connection, e: &Equipamiento) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO equipamiento_historial
            (tipo, marca, modelo, fecha_compra, precio_centavos, partidos_usados,
             estado_fisico, rating, notas, activo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            e.tipo,
            e.marca,
            e.modelo,
            e.fecha_compra,
            i64::from(e.precio),
            e.partidos_usados,
            e.estado_fisico,
            e.rating,
            e.notas,
            i64::from(e.activo)
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza un ítem.
pub fn update(conn: &Connection, e: &Equipamiento) -> Result<(), DbError> {
    let cambiadas = conn.execute(
        "UPDATE equipamiento_historial
            SET tipo = ?1, marca = ?2, modelo = ?3, fecha_compra = ?4, precio_centavos = ?5,
                partidos_usados = ?6, estado_fisico = ?7, rating = ?8, notas = ?9, activo = ?10
          WHERE id = ?11",
        params![
            e.tipo,
            e.marca,
            e.modelo,
            e.fecha_compra,
            i64::from(e.precio),
            e.partidos_usados,
            e.estado_fisico,
            e.rating,
            e.notas,
            i64::from(e.activo),
            e.id
        ],
    )?;
    if cambiadas == 0 {
        return Err(DbError::NotFound {
            entidad: "equipamiento",
            id: e.id,
        });
    }
    Ok(())
}

/// Elimina un ítem.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM equipamiento_historial WHERE id = ?1", [id])?;
    Ok(())
}
