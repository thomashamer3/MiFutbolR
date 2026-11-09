//! Persistencia de notificaciones (IMP-06).

use rusqlite::{Connection, params};

use super::error::DbError;
use crate::domain::notificacion::Notificacion;

/// Lista notificaciones.
pub fn list(conn: &Connection, solo_no_leidas: bool) -> Result<Vec<Notificacion>, DbError> {
    let sql = if solo_no_leidas {
        "SELECT id, tipo, severidad, titulo, mensaje, leida, entidad, entidad_id, fecha
           FROM notificacion WHERE leida = 0
          ORDER BY severidad DESC, fecha DESC, id DESC"
    } else {
        "SELECT id, tipo, severidad, titulo, mensaje, leida, entidad, entidad_id, fecha
           FROM notificacion
          ORDER BY leida ASC, severidad DESC, fecha DESC, id DESC"
    };
    let mut stmt = conn.prepare(sql)?;
    let filas = stmt.query_map([], |row| {
        Ok(Notificacion {
            id: row.get(0)?,
            tipo: row.get(1)?,
            severidad: row.get(2)?,
            titulo: row.get(3)?,
            mensaje: row.get(4)?,
            leida: row.get::<_, i64>(5)? != 0,
            entidad: row.get(6)?,
            entidad_id: row.get(7)?,
            fecha: row.get(8)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Cuenta las no leídas.
pub fn contar_no_leidas(conn: &Connection) -> Result<i64, DbError> {
    let total = conn.query_row(
        "SELECT COUNT(*) FROM notificacion WHERE leida = 0",
        [],
        |row| row.get(0),
    )?;
    Ok(total)
}

/// Inserta una notificación.
pub fn insert(
    conn: &Connection,
    tipo: &str,
    severidad: i64,
    titulo: &str,
    mensaje: &str,
    entidad: &str,
    entidad_id: Option<i64>,
) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO notificacion (tipo, severidad, titulo, mensaje, entidad, entidad_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![tipo, severidad, titulo, mensaje, entidad, entidad_id],
    )?;
    Ok(conn.last_insert_rowid())
}

/// ¿Ya existe una notificación igual hoy?
pub fn existe_hoy(
    conn: &Connection,
    tipo: &str,
    entidad: &str,
    entidad_id: Option<i64>,
) -> Result<bool, DbError> {
    let total = conn.query_row(
        "SELECT COUNT(*) FROM notificacion
          WHERE tipo = ?1 AND entidad = ?2 AND IFNULL(entidad_id, 0) = ?3
            AND date(fecha) = date('now', 'localtime')",
        params![tipo, entidad, entidad_id.unwrap_or(0)],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(total > 0)
}

/// Marca una notificación como leída.
pub fn marcar_leida(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE notificacion SET leida = 1 WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// Marca todas como leídas.
pub fn marcar_todas(conn: &Connection) -> Result<usize, DbError> {
    Ok(conn.execute("UPDATE notificacion SET leida = 1 WHERE leida = 0", [])?)
}

/// Lesiones activas de los últimos 7 días.
pub fn lesiones_recientes(conn: &Connection) -> Result<Vec<(i64, String, String)>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, jugador, tipo FROM lesion
          WHERE estado IN ('ACTIVA', 'EN TRATAMIENTO', 'RECAIDA')
            AND fecha >= date('now', '-7 days')
          ORDER BY fecha DESC",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn insert_list_and_mark() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(contar_no_leidas(db.conn()).unwrap(), 0);
        let id = insert(db.conn(), "test", 1, "Título", "Mensaje", "x", Some(7)).unwrap();
        assert_eq!(contar_no_leidas(db.conn()).unwrap(), 1);
        assert!(existe_hoy(db.conn(), "test", "x", Some(7)).unwrap());
        assert!(!existe_hoy(db.conn(), "test", "x", Some(8)).unwrap());
        let lista = list(db.conn(), false).unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].id, id);
        marcar_leida(db.conn(), id).unwrap();
        assert_eq!(contar_no_leidas(db.conn()).unwrap(), 0);
    }
}
