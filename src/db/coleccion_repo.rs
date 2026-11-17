//! Persistencia de colecciones e inventario.

use rusqlite::{Connection, params};

use super::error::DbError;
use crate::domain::colecciones::{Coleccion, InventarioItem};

const SELECT_ITEM: &str = "SELECT i.id, i.tipo, COALESCE(i.nombre, ''),
        COALESCE(NULLIF(i.nombre, ''), c.nombre, '(Camiseta eliminada)'),
        i.estado, COALESCE(i.valor, 0), COALESCE(i.fecha_compra, ''), i.camiseta_id,
        (SELECT COUNT(*) FROM coleccion_inventario ci WHERE ci.inventario_id = i.id)
   FROM inventario_item i
   LEFT JOIN camiseta c ON c.id = i.camiseta_id";

fn map_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<InventarioItem> {
    Ok(InventarioItem {
        id: row.get(0)?,
        tipo: row.get(1)?,
        nombre: row.get(2)?,
        nombre_visible: row.get(3)?,
        estado: row.get(4)?,
        valor: row.get(5)?,
        fecha_compra: row.get(6)?,
        camiseta_id: row.get(7)?,
        colecciones: row.get(8)?,
    })
}

/// Inventario completo.
pub fn list_items(conn: &Connection) -> Result<Vec<InventarioItem>, DbError> {
    let mut stmt = conn.prepare(&format!("{SELECT_ITEM} ORDER BY i.tipo, i.id"))?;
    let filas = stmt.query_map([], map_item)?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Ítems de una colección.
pub fn items_de(conn: &Connection, coleccion_id: i64) -> Result<Vec<InventarioItem>, DbError> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT_ITEM}
          JOIN coleccion_inventario ci2 ON ci2.inventario_id = i.id AND ci2.coleccion_id = ?1
         ORDER BY i.tipo, i.id"
    ))?;
    let filas = stmt.query_map(params![coleccion_id], map_item)?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Inserta o actualiza (`item.id == 0` inserta).
pub fn save_item(conn: &Connection, item: &InventarioItem) -> Result<i64, DbError> {
    if item.id == 0 {
        conn.execute(
            "INSERT INTO inventario_item (tipo, nombre, estado, valor, fecha_compra, camiseta_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                item.tipo,
                item.nombre,
                item.estado,
                item.valor,
                item.fecha_compra,
                item.camiseta_id
            ],
        )?;
        return Ok(conn.last_insert_rowid());
    }
    conn.execute(
        "UPDATE inventario_item SET tipo = ?1, nombre = ?2, estado = ?3, valor = ?4,
                fecha_compra = ?5, camiseta_id = ?6
          WHERE id = ?7",
        params![
            item.tipo,
            item.nombre,
            item.estado,
            item.valor,
            item.fecha_compra,
            item.camiseta_id,
            item.id
        ],
    )?;
    Ok(item.id)
}

/// Borra un ítem (sus asociaciones caen en cascada).
pub fn delete_item(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM inventario_item WHERE id = ?1", params![id])?;
    Ok(())
}

/// Crea un ítem por cada camiseta que todavía no está en el inventario.
pub fn sincronizar_camisetas(conn: &Connection) -> Result<usize, DbError> {
    Ok(conn.execute(
        "INSERT INTO inventario_item (tipo, nombre, estado, valor, fecha_compra, camiseta_id)
         SELECT 1, '', 0, 0, '', c.id
           FROM camiseta c
           LEFT JOIN inventario_item i ON i.camiseta_id = c.id
          WHERE i.camiseta_id IS NULL",
        [],
    )?)
}

/// Colecciones con su cantidad de ítems.
pub fn list_colecciones(conn: &Connection) -> Result<Vec<Coleccion>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.nombre, COALESCE(c.descripcion, ''),
                (SELECT COUNT(*) FROM coleccion_inventario ci WHERE ci.coleccion_id = c.id)
           FROM coleccion c ORDER BY c.id",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok(Coleccion {
            id: row.get(0)?,
            nombre: row.get(1)?,
            descripcion: row.get(2)?,
            items: row.get(3)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Inserta o actualiza una colección (`id == 0` inserta).
pub fn save_coleccion(
    conn: &Connection,
    id: i64,
    nombre: &str,
    descripcion: &str,
) -> Result<i64, DbError> {
    if id == 0 {
        conn.execute(
            "INSERT INTO coleccion (nombre, descripcion) VALUES (?1, ?2)",
            params![nombre, descripcion],
        )?;
        return Ok(conn.last_insert_rowid());
    }
    conn.execute(
        "UPDATE coleccion SET nombre = ?1, descripcion = ?2 WHERE id = ?3",
        params![nombre, descripcion, id],
    )?;
    Ok(id)
}

/// Borra una colección (los ítems quedan en el inventario).
pub fn delete_coleccion(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM coleccion WHERE id = ?1", params![id])?;
    Ok(())
}

/// Agrega un ítem a una colección; `false` si ya estaba.
pub fn agregar(conn: &Connection, coleccion_id: i64, item_id: i64) -> Result<bool, DbError> {
    Ok(conn.execute(
        "INSERT OR IGNORE INTO coleccion_inventario (coleccion_id, inventario_id) VALUES (?1, ?2)",
        params![coleccion_id, item_id],
    )? > 0)
}

/// Quita un ítem de una colección.
pub fn quitar(conn: &Connection, coleccion_id: i64, item_id: i64) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM coleccion_inventario WHERE coleccion_id = ?1 AND inventario_id = ?2",
        params![coleccion_id, item_id],
    )?;
    Ok(())
}
