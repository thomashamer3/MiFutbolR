//! Colecciones e inventario (paridad con MiFutbolC, `colecciones.c`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use thiserror::Error;

use crate::db::DbError;
use crate::db::coleccion_repo;
use crate::domain::colecciones::{Coleccion, InventarioItem};

/// Error del servicio.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
    /// Validación.
    #[error("{0}")]
    Message(String),
}

/// Inventario completo.
pub fn listar_items(conn: &Connection) -> Result<Vec<InventarioItem>, ServiceError> {
    Ok(coleccion_repo::list_items(conn)?)
}

/// Ítems de una colección.
pub fn items_de(conn: &Connection, coleccion_id: i64) -> Result<Vec<InventarioItem>, ServiceError> {
    Ok(coleccion_repo::items_de(conn, coleccion_id)?)
}

/// Crea o actualiza un ítem. Una camiseta vinculada toma su nombre de la camiseta.
pub fn guardar_item(conn: &Connection, item: &InventarioItem) -> Result<i64, ServiceError> {
    let mut item = item.clone();
    if item.tipo != 1 {
        item.camiseta_id = None;
    }
    if item.camiseta_id.is_some() {
        item.nombre.clear();
    } else if item.nombre.trim().is_empty() {
        return Err(ServiceError::Message(
            "Indicá un nombre o vinculá una camiseta".to_string(),
        ));
    }
    if !item.valor.is_finite() || item.valor < 0.0 {
        return Err(ServiceError::Message(
            "El valor no puede ser negativo".to_string(),
        ));
    }
    coleccion_repo::save_item(conn, &item).map_err(|error| match error {
        DbError::Sqlite(ref inner) if inner.to_string().contains("UNIQUE") => {
            ServiceError::Message("Esa camiseta ya está en el inventario".to_string())
        }
        otro => otro.into(),
    })
}

/// Elimina un ítem.
pub fn eliminar_item(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    coleccion_repo::delete_item(conn, id)?;
    Ok(())
}

/// Agrega al inventario las camisetas que faltan; devuelve cuántas.
pub fn sincronizar_camisetas(conn: &Connection) -> Result<usize, ServiceError> {
    Ok(coleccion_repo::sincronizar_camisetas(conn)?)
}

/// Colecciones.
pub fn listar_colecciones(conn: &Connection) -> Result<Vec<Coleccion>, ServiceError> {
    Ok(coleccion_repo::list_colecciones(conn)?)
}

/// Crea (`id == 0`) o actualiza una colección.
pub fn guardar_coleccion(
    conn: &Connection,
    id: i64,
    nombre: &str,
    descripcion: &str,
) -> Result<i64, ServiceError> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(ServiceError::Message(
            "El nombre de la colección es obligatorio".to_string(),
        ));
    }
    coleccion_repo::save_coleccion(conn, id, nombre, descripcion.trim()).map_err(
        |error| match error {
            DbError::Sqlite(ref inner) if inner.to_string().contains("UNIQUE") => {
                ServiceError::Message("Ya existe una colección con ese nombre".to_string())
            }
            otro => otro.into(),
        },
    )
}

/// Elimina una colección (los ítems siguen en el inventario).
pub fn eliminar_coleccion(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    coleccion_repo::delete_coleccion(conn, id)?;
    Ok(())
}

/// Agrega un ítem a una colección; `false` si ya estaba.
pub fn agregar(conn: &Connection, coleccion_id: i64, item_id: i64) -> Result<bool, ServiceError> {
    Ok(coleccion_repo::agregar(conn, coleccion_id, item_id)?)
}

/// Quita un ítem de una colección.
pub fn quitar(conn: &Connection, coleccion_id: i64, item_id: i64) -> Result<(), ServiceError> {
    coleccion_repo::quitar(conn, coleccion_id, item_id)?;
    Ok(())
}

/// Archivo del backup de C (`COLECCIONES_BACKUP_DEFAULT`).
pub const ARCHIVO_BACKUP: &str = "colecciones_inventario.json";

fn filas_json(conn: &Connection, sql: &str, claves: &[&str]) -> Result<Vec<Value>, DbError> {
    let mut stmt = conn.prepare(sql)?;
    let filas = stmt.query_map([], |row| {
        let mut objeto = serde_json::Map::new();
        for (i, clave) in claves.iter().enumerate() {
            let valor = match row.get_ref(i)? {
                ValueRef::Integer(n) => Value::from(n),
                ValueRef::Real(x) => Value::from(x),
                ValueRef::Text(t) => Value::from(String::from_utf8_lossy(t).into_owned()),
                ValueRef::Null | ValueRef::Blob(_) => Value::Null,
            };
            objeto.insert((*clave).to_string(), valor);
        }
        Ok(Value::Object(objeto))
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// `exportar_backup_colecciones_json` de C: `{tipo, version, colecciones,
/// inventario, relaciones}` en `dir/colecciones_inventario.json`.
pub fn exportar_backup(conn: &Connection, dir: &Path) -> Result<PathBuf, ServiceError> {
    let raiz = json!({
        "tipo": "colecciones_inventario_backup",
        "version": 1,
        "colecciones": filas_json(conn,
            "SELECT id, nombre, COALESCE(descripcion, ''), COALESCE(fecha_creacion, '')
               FROM coleccion ORDER BY id",
            &["id", "nombre", "descripcion", "fecha_creacion"])?,
        "inventario": filas_json(conn,
            "SELECT id, tipo, COALESCE(nombre, ''), estado, COALESCE(valor, 0.0),
                    COALESCE(fecha_compra, ''), COALESCE(camiseta_id, 0)
               FROM inventario_item ORDER BY id",
            &["id", "tipo", "nombre", "estado", "valor", "fecha_compra", "camiseta_id"])?,
        "relaciones": filas_json(conn,
            "SELECT coleccion_id, inventario_id, fecha_asociacion
               FROM coleccion_inventario ORDER BY coleccion_id, inventario_id",
            &["coleccion_id", "inventario_id", "fecha_asociacion"])?,
    });
    let io = |e: std::io::Error| ServiceError::Message(e.to_string());
    std::fs::create_dir_all(dir).map_err(io)?;
    let ruta = dir.join(ARCHIVO_BACKUP);
    let texto =
        serde_json::to_string_pretty(&raiz).map_err(|e| ServiceError::Message(e.to_string()))?;
    std::fs::write(&ruta, texto).map_err(io)?;
    Ok(ruta)
}

/// Cuántas colecciones e ítems se procesaron y cuántas relaciones nuevas hubo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResumenBackup {
    /// Colecciones creadas o actualizadas.
    pub colecciones: usize,
    /// Ítems creados o reutilizados.
    pub items: usize,
    /// Relaciones agregadas.
    pub relaciones: usize,
}

/// `importar_backup_colecciones_json` de C: une con lo existente (colección por
/// nombre, camiseta por `camiseta_id`, ítem genérico equivalente) en una
/// transacción con el deshacer suspendido. C enlazaba una relación con
/// cualquier fila que tuviera el id viejo si el ítem no se había importado;
/// acá solo se usan los ids que se importaron.
pub fn importar_backup(conn: &mut Connection, ruta: &Path) -> Result<ResumenBackup, ServiceError> {
    let error = |m: String| ServiceError::Message(m);
    let texto =
        std::fs::read_to_string(ruta).map_err(|e| error(format!("{}: {e}", ruta.display())))?;
    let raiz: Value = serde_json::from_str(texto.trim_start_matches('\u{feff}'))
        .map_err(|_| error("JSON inválido o corrupto".to_string()))?;
    let (Some(colecciones), Some(inventario), Some(relaciones)) = (
        raiz["colecciones"].as_array(),
        raiz["inventario"].as_array(),
        raiz["relaciones"].as_array(),
    ) else {
        return Err(error("Formato de backup no compatible".to_string()));
    };
    let entero = |v: &Value, clave: &str| v[clave].as_i64().unwrap_or(0);
    let texto_de = |v: &Value, clave: &str| v[clave].as_str().unwrap_or("").trim().to_string();

    let tx = conn.transaction().map_err(DbError::from)?;
    tx.execute("UPDATE undo_control SET suspendido = 1 WHERE id = 1", [])
        .map_err(DbError::from)?;
    let mut mapa_colecciones = HashMap::new();
    for c in colecciones {
        let nombre = texto_de(c, "nombre");
        if nombre.is_empty() {
            continue;
        }
        let descripcion = texto_de(c, "descripcion");
        tx.execute(
            "INSERT INTO coleccion (nombre, descripcion, fecha_creacion)
             VALUES (?1, ?2, COALESCE(NULLIF(?3, ''), CURRENT_TIMESTAMP))
             ON CONFLICT(nombre) DO UPDATE SET descripcion = excluded.descripcion",
            params![nombre, descripcion, texto_de(c, "fecha_creacion")],
        )
        .map_err(DbError::from)?;
        let id: i64 = tx
            .query_row(
                "SELECT id FROM coleccion WHERE nombre = ?1",
                [&nombre],
                |r| r.get(0),
            )
            .map_err(DbError::from)?;
        mapa_colecciones.insert(entero(c, "id"), id);
    }
    let mut mapa_items = HashMap::new();
    for item in inventario {
        let tipo = entero(item, "tipo");
        let estado = if entero(item, "estado") == 1 { 1 } else { 0 };
        let valor = item["valor"].as_f64().unwrap_or(0.0).max(0.0);
        let fecha = texto_de(item, "fecha_compra");
        let nuevo: Option<i64> = match tipo {
            1 => {
                let camiseta = entero(item, "camiseta_id");
                let existe: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM camiseta WHERE id = ?1)",
                        [camiseta],
                        |r| r.get(0),
                    )
                    .map_err(DbError::from)?;
                if !existe {
                    continue;
                }
                tx.execute(
                    "INSERT INTO inventario_item (tipo, nombre, estado, valor, fecha_compra, camiseta_id)
                     VALUES (1, '', ?1, ?2, ?3, ?4)
                     ON CONFLICT(camiseta_id) DO UPDATE SET estado = excluded.estado,
                        valor = excluded.valor, fecha_compra = excluded.fecha_compra",
                    params![estado, valor, fecha, camiseta],
                )
                .map_err(DbError::from)?;
                tx.query_row(
                    "SELECT id FROM inventario_item WHERE camiseta_id = ?1",
                    [camiseta],
                    |r| r.get(0),
                )
                .optional()
                .map_err(DbError::from)?
            }
            2 | 3 => {
                let nombre = Some(texto_de(item, "nombre"))
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| "Item importado".to_string());
                let existente: Option<i64> = tx
                    .query_row(
                        "SELECT id FROM inventario_item
                          WHERE tipo = ?1 AND COALESCE(nombre, '') = ?2 AND estado = ?3
                            AND ABS(COALESCE(valor, 0) - ?4) < 0.000001
                            AND COALESCE(fecha_compra, '') = ?5 AND camiseta_id IS NULL LIMIT 1",
                        params![tipo, nombre, estado, valor, fecha],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(DbError::from)?;
                match existente {
                    Some(id) => Some(id),
                    None => {
                        tx.execute(
                            "INSERT INTO inventario_item (tipo, nombre, estado, valor, fecha_compra)
                             VALUES (?1, ?2, ?3, ?4, ?5)",
                            params![tipo, nombre, estado, valor, fecha],
                        )
                        .map_err(DbError::from)?;
                        Some(tx.last_insert_rowid())
                    }
                }
            }
            _ => None,
        };
        if let Some(id) = nuevo {
            mapa_items.insert(entero(item, "id"), id);
        }
    }
    let mut nuevas = 0;
    for r in relaciones {
        let (Some(coleccion), Some(item)) = (
            mapa_colecciones.get(&entero(r, "coleccion_id")),
            mapa_items.get(&entero(r, "inventario_id")),
        ) else {
            continue;
        };
        nuevas += tx
            .execute(
                "INSERT OR IGNORE INTO coleccion_inventario (coleccion_id, inventario_id) VALUES (?1, ?2)",
                params![coleccion, item],
            )
            .map_err(DbError::from)?;
    }
    tx.execute("UPDATE undo_control SET suspendido = 0 WHERE id = 1", [])
        .map_err(DbError::from)?;
    tx.commit().map_err(DbError::from)?;
    Ok(ResumenBackup {
        colecciones: mapa_colecciones.len(),
        items: mapa_items.len(),
        relaciones: nuevas,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn item(tipo: i64, nombre: &str, camiseta_id: Option<i64>) -> InventarioItem {
        InventarioItem {
            id: 0,
            tipo,
            nombre: nombre.to_string(),
            nombre_visible: String::new(),
            estado: 0,
            valor: 100.0,
            fecha_compra: String::new(),
            camiseta_id,
            colecciones: 0,
        }
    }

    #[test]
    fn seeds_sync_and_membership() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();
        // Las tres colecciones por defecto de MiFutbolC.
        assert_eq!(listar_colecciones(conn).unwrap().len(), 3);

        conn.execute(
            "INSERT INTO camiseta (nombre) VALUES ('Titular'), ('Suplente')",
            [],
        )
        .unwrap();
        assert_eq!(sincronizar_camisetas(conn).unwrap(), 2);
        assert_eq!(sincronizar_camisetas(conn).unwrap(), 0, "idempotente");
        let items = listar_items(conn).unwrap();
        assert_eq!(items[0].nombre_visible, "Titular");

        let botin = guardar_item(conn, &item(2, "Predator", None)).unwrap();
        assert!(guardar_item(conn, &item(3, "  ", None)).is_err());
        // Una camiseta ya inventariada no se puede repetir.
        assert!(guardar_item(conn, &item(1, "", items[0].camiseta_id)).is_err());

        let actual = listar_colecciones(conn).unwrap()[0].id;
        assert!(agregar(conn, actual, botin).unwrap());
        assert!(!agregar(conn, actual, botin).unwrap());
        assert_eq!(items_de(conn, actual).unwrap().len(), 1);
        assert_eq!(listar_colecciones(conn).unwrap()[0].items, 1);
        quitar(conn, actual, botin).unwrap();
        assert!(items_de(conn, actual).unwrap().is_empty());

        // Borrar la camiseta deja el ítem desvinculado con nombre de reemplazo.
        conn.execute("DELETE FROM camiseta WHERE nombre = 'Suplente'", [])
            .unwrap();
        assert!(
            listar_items(conn)
                .unwrap()
                .iter()
                .any(|i| i.nombre_visible == "(Camiseta eliminada)")
        );
    }

    #[test]
    fn backup_json_ida_y_vuelta_sin_duplicar() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();
        conn.execute(
            "INSERT INTO camiseta (id, nombre) VALUES (5, 'Titular')",
            [],
        )
        .unwrap();
        sincronizar_camisetas(conn).unwrap();
        let retro = guardar_coleccion(conn, 0, "Retro", "Viejas").unwrap();
        let botin = guardar_item(conn, &item(2, "Predator", None)).unwrap();
        let camiseta = listar_items(conn).unwrap()[0].id;
        agregar(conn, retro, botin).unwrap();
        agregar(conn, retro, camiseta).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ruta = exportar_backup(conn, dir.path()).unwrap();
        let valor: Value = serde_json::from_str(&std::fs::read_to_string(&ruta).unwrap()).unwrap();
        assert_eq!(valor["tipo"], "colecciones_inventario_backup");
        assert_eq!(valor["relaciones"].as_array().unwrap().len(), 2);

        // Base nueva con la misma camiseta: entran colección, ítems y relaciones.
        let mut otra = Database::open_in_memory().unwrap();
        otra.conn()
            .execute(
                "INSERT INTO camiseta (id, nombre) VALUES (5, 'Titular')",
                [],
            )
            .unwrap();
        let resumen = importar_backup(otra.conn_mut(), &ruta).unwrap();
        assert_eq!(resumen.relaciones, 2, "{resumen:?}");
        assert_eq!(resumen.colecciones, 4);
        let de_retro = |db: &Database| {
            let id = listar_colecciones(db.conn())
                .unwrap()
                .into_iter()
                .find(|c| c.nombre == "Retro")
                .unwrap()
                .id;
            items_de(db.conn(), id).unwrap().len()
        };
        assert_eq!(de_retro(&otra), 2);
        // Reimportar no duplica ítems ni relaciones.
        let otra_vez = importar_backup(otra.conn_mut(), &ruta).unwrap();
        assert_eq!(otra_vez.relaciones, 0);
        assert_eq!(listar_items(otra.conn()).unwrap().len(), 2);

        // Sin la camiseta, su ítem no entra y la relación no se cuelga de otra fila.
        let mut sin_camiseta = Database::open_in_memory().unwrap();
        let resumen = importar_backup(sin_camiseta.conn_mut(), &ruta).unwrap();
        assert_eq!((resumen.items, resumen.relaciones), (1, 1));

        std::fs::write(&ruta, "{\"colecciones\": []}").unwrap();
        assert!(importar_backup(sin_camiseta.conn_mut(), &ruta).is_err());
    }

    #[test]
    fn coleccion_names_are_unique() {
        let db = Database::open_in_memory().unwrap();
        let id = guardar_coleccion(db.conn(), 0, "Retro", "").unwrap();
        assert!(guardar_coleccion(db.conn(), 0, "Retro", "").is_err());
        assert!(guardar_coleccion(db.conn(), 0, " ", "").is_err());
        eliminar_coleccion(db.conn(), id).unwrap();
        assert_eq!(listar_colecciones(db.conn()).unwrap().len(), 3);
    }
}
