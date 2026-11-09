//! Historial de escrituras y deshacer (IMP-07).
//!
//! Los triggers de la migración 005 guardan el JSON anterior/posterior de las
//! tablas principales. Deshacer aplica la operación inversa con los triggers
//! suspendidos para no realimentar el historial.

use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde_json::{Map, Value};

use crate::db::error::DbError;

/// Tablas con triggers de undo.
pub const TABLAS: &[&str] = &[
    "camiseta",
    "cancha",
    "botin",
    "equipo",
    "jugador",
    "partido",
    "lesion",
    "financiamiento",
];

/// Entrada del historial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoEntry {
    /// Id del historial.
    pub id: i64,
    /// Tabla afectada.
    pub tabla: String,
    /// Id del registro.
    pub registro_id: i64,
    /// crear/editar/borrar.
    pub operacion: String,
    /// Descripción legible.
    pub descripcion: String,
    /// JSON anterior.
    pub antes: Option<String>,
    /// JSON posterior.
    pub despues: Option<String>,
    /// Fecha.
    pub ts: String,
}

/// Última operación registrada.
pub fn ultima(conn: &Connection) -> Result<Option<UndoEntry>, DbError> {
    podar(conn)?;
    let fila = conn
        .query_row(
            "SELECT id, tabla, registro_id, operacion, descripcion, antes, despues, ts
               FROM undo_historial ORDER BY id DESC LIMIT 1",
            [],
            |row| {
                Ok(UndoEntry {
                    id: row.get(0)?,
                    tabla: row.get(1)?,
                    registro_id: row.get(2)?,
                    operacion: row.get(3)?,
                    descripcion: row.get(4)?,
                    antes: row.get(5)?,
                    despues: row.get(6)?,
                    ts: row.get(7)?,
                })
            },
        )
        .optional()?;
    Ok(fila)
}

/// Lista las últimas operaciones.
pub fn listar(conn: &Connection, limite: usize) -> Result<Vec<UndoEntry>, DbError> {
    podar(conn)?;
    let mut stmt = conn.prepare(
        "SELECT id, tabla, registro_id, operacion, descripcion, antes, despues, ts
           FROM undo_historial ORDER BY id DESC LIMIT ?1",
    )?;
    let filas = stmt.query_map(params![limite as i64], |row| {
        Ok(UndoEntry {
            id: row.get(0)?,
            tabla: row.get(1)?,
            registro_id: row.get(2)?,
            operacion: row.get(3)?,
            descripcion: row.get(4)?,
            antes: row.get(5)?,
            despues: row.get(6)?,
            ts: row.get(7)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Deshace la última operación y devuelve su descripción.
pub fn deshacer_ultima(conn: &Connection) -> Result<Option<String>, DbError> {
    let Some(entrada) = ultima(conn)? else {
        return Ok(None);
    };
    deshacer(conn, &entrada)?;
    Ok(Some(entrada.descripcion))
}

/// Deshace una entrada concreta.
pub fn deshacer(conn: &Connection, entrada: &UndoEntry) -> Result<(), DbError> {
    if !TABLAS.contains(&entrada.tabla.as_str()) {
        return Err(DbError::InvalidData(format!(
            "tabla no permitida para undo: {}",
            entrada.tabla
        )));
    }
    conn.execute_batch("UPDATE undo_control SET suspendido = 1 WHERE id = 1")?;
    let resultado = aplicar(conn, entrada);
    conn.execute_batch("UPDATE undo_control SET suspendido = 0 WHERE id = 1")?;
    resultado?;
    conn.execute(
        "DELETE FROM undo_historial WHERE id = ?1",
        params![entrada.id],
    )?;
    Ok(())
}

fn aplicar(conn: &Connection, entrada: &UndoEntry) -> Result<(), DbError> {
    match entrada.operacion.as_str() {
        "crear" => {
            conn.execute(
                &format!("DELETE FROM \"{}\" WHERE id = ?1", entrada.tabla),
                params![entrada.registro_id],
            )?;
        }
        "editar" => {
            let antes = entrada
                .antes
                .as_deref()
                .ok_or_else(|| DbError::InvalidData("falta el snapshot anterior".to_string()))?;
            actualizar(conn, &entrada.tabla, antes)?;
        }
        "borrar" => {
            let antes = entrada
                .antes
                .as_deref()
                .ok_or_else(|| DbError::InvalidData("falta el snapshot anterior".to_string()))?;
            insertar(conn, &entrada.tabla, antes)?;
        }
        otra => {
            return Err(DbError::InvalidData(format!(
                "operación de undo desconocida: {otra}"
            )));
        }
    }
    Ok(())
}

fn objeto(json: &str) -> Result<Map<String, Value>, DbError> {
    let valor: Value = serde_json::from_str(json)
        .map_err(|error| DbError::InvalidData(format!("JSON inválido: {error}")))?;
    match valor {
        Value::Object(mapa) => Ok(mapa),
        _ => Err(DbError::InvalidData(
            "se esperaba un objeto JSON".to_string(),
        )),
    }
}

fn a_sql(valor: &Value) -> SqlValue {
    match valor {
        Value::Null => SqlValue::Null,
        Value::Bool(valor) => SqlValue::Integer(i64::from(*valor)),
        Value::Number(numero) => numero.as_i64().map_or_else(
            || numero.as_f64().map_or(SqlValue::Null, SqlValue::Real),
            SqlValue::Integer,
        ),
        Value::String(texto) => SqlValue::Text(texto.clone()),
        _ => SqlValue::Null,
    }
}

fn actualizar(conn: &Connection, tabla: &str, json: &str) -> Result<(), DbError> {
    let mapa = objeto(json)?;
    let columnas: Vec<(&String, &Value)> = mapa
        .iter()
        .filter(|(columna, _)| columna.as_str() != "id")
        .collect();
    if columnas.is_empty() {
        return Ok(());
    }
    let asignaciones = columnas
        .iter()
        .enumerate()
        .map(|(indice, (columna, _))| {
            format!("\"{}\" = ?{}", columna.replace('"', "\"\""), indice + 2)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("UPDATE \"{tabla}\" SET {asignaciones} WHERE id = ?1");
    let mut valores: Vec<SqlValue> = Vec::with_capacity(columnas.len() + 1);
    let id = mapa.get("id").map_or(0, |valor| match valor {
        Value::Number(numero) => numero.as_i64().unwrap_or(0),
        _ => 0,
    });
    valores.push(SqlValue::Integer(id));
    valores.extend(columnas.iter().map(|(_, valor)| a_sql(valor)));
    conn.execute(&sql, params_from_iter(valores))?;
    Ok(())
}

fn insertar(conn: &Connection, tabla: &str, json: &str) -> Result<(), DbError> {
    let mapa = objeto(json)?;
    let columnas: Vec<(&String, &Value)> = mapa.iter().collect();
    if columnas.is_empty() {
        return Ok(());
    }
    let nombres = columnas
        .iter()
        .map(|(columna, _)| format!("\"{}\"", columna.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(", ");
    let marcadores = (1..=columnas.len())
        .map(|indice| format!("?{indice}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("INSERT OR REPLACE INTO \"{tabla}\" ({nombres}) VALUES ({marcadores})");
    let valores: Vec<SqlValue> = columnas.iter().map(|(_, valor)| a_sql(valor)).collect();
    conn.execute(&sql, params_from_iter(valores))?;
    Ok(())
}

fn podar(conn: &Connection) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM undo_historial
          WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50)",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn db() -> Database {
        Database::open_in_memory().unwrap()
    }

    #[test]
    fn undo_create_update_and_delete() {
        let db = db();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Azul')", [])
            .unwrap();

        // Deshacer creación.
        let entrada = ultima(db.conn()).unwrap().unwrap();
        assert_eq!(entrada.operacion, "crear");
        deshacer_ultima(db.conn()).unwrap();
        let total: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM camiseta", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total, 0);

        // Reinsertar y deshacer una edición.
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Azul')", [])
            .unwrap();
        let id = db.conn().last_insert_rowid();
        db.conn()
            .execute(
                "UPDATE camiseta SET nombre = 'Roja' WHERE id = ?1",
                params![id],
            )
            .unwrap();
        let entrada = ultima(db.conn()).unwrap().unwrap();
        assert_eq!(entrada.operacion, "editar");
        deshacer_ultima(db.conn()).unwrap();
        let nombre: String = db
            .conn()
            .query_row(
                "SELECT nombre FROM camiseta WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(nombre, "Azul");

        // Deshacer un borrado.
        db.conn()
            .execute("DELETE FROM camiseta WHERE id = ?1", params![id])
            .unwrap();
        let entrada = ultima(db.conn()).unwrap().unwrap();
        assert_eq!(entrada.operacion, "borrar");
        deshacer_ultima(db.conn()).unwrap();
        let nombre: String = db
            .conn()
            .query_row(
                "SELECT nombre FROM camiseta WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(nombre, "Azul");

        // El undo no debe haberse realimentado.
        let total: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM undo_historial", [], |row| row.get(0))
            .unwrap();
        assert!(total <= 2, "entradas: {total}");
    }
}
