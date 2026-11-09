//! Persistencia de progresión (IMP-23).

use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::progresion::{PlanEntrenamiento, ProgresionFila};

/// Lista planes.
pub fn planes(conn: &Connection) -> Result<Vec<PlanEntrenamiento>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, nombre, descripcion, duracion_semanas, sesiones_por_semana
           FROM entrenamiento_plan ORDER BY nombre",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok(PlanEntrenamiento {
            id: row.get(0)?,
            nombre: row.get(1)?,
            descripcion: row.get(2)?,
            duracion_semanas: row.get(3)?,
            sesiones_por_semana: row.get(4)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Lista jugadores (id, nombre).
pub fn jugadores(conn: &Connection) -> Result<Vec<(i64, String)>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT j.id, j.nombre || ' · ' || e.nombre
           FROM jugador j JOIN equipo e ON e.id = j.equipo_id
          ORDER BY e.nombre, j.nombre",
    )?;
    let filas = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Lista mediciones.
pub fn list(conn: &Connection) -> Result<Vec<ProgresionFila>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.jugador_id, j.nombre, p.plan_id, IFNULL(pl.nombre, ''),
                p.semana, p.ataque, p.defensa, p.resistencia, p.velocidad, p.tecnica
           FROM progresion_jugador p
           JOIN jugador j ON j.id = p.jugador_id
           LEFT JOIN entrenamiento_plan pl ON pl.id = p.plan_id
          ORDER BY j.nombre, p.semana",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok(ProgresionFila {
            id: row.get(0)?,
            jugador_id: row.get(1)?,
            jugador: row.get(2)?,
            plan_id: row.get(3)?,
            plan: row.get(4)?,
            semana: row.get(5)?,
            ataque: row.get(6)?,
            defensa: row.get(7)?,
            resistencia: row.get(8)?,
            velocidad: row.get(9)?,
            tecnica: row.get(10)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Obtiene una medición.
pub fn get(conn: &Connection, id: i64) -> Result<ProgresionFila, DbError> {
    conn.query_row(
        "SELECT p.id, p.jugador_id, j.nombre, p.plan_id, IFNULL(pl.nombre, ''),
                p.semana, p.ataque, p.defensa, p.resistencia, p.velocidad, p.tecnica
           FROM progresion_jugador p
           JOIN jugador j ON j.id = p.jugador_id
           LEFT JOIN entrenamiento_plan pl ON pl.id = p.plan_id
          WHERE p.id = ?1",
        params![id],
        |row| {
            Ok(ProgresionFila {
                id: row.get(0)?,
                jugador_id: row.get(1)?,
                jugador: row.get(2)?,
                plan_id: row.get(3)?,
                plan: row.get(4)?,
                semana: row.get(5)?,
                ataque: row.get(6)?,
                defensa: row.get(7)?,
                resistencia: row.get(8)?,
                velocidad: row.get(9)?,
                tecnica: row.get(10)?,
            })
        },
    )
    .optional()?
    .ok_or(DbError::NotFound {
        entidad: "progresion_jugador",
        id,
    })
}

/// Inserta una medición.
pub fn insert(conn: &Connection, fila: &ProgresionFila) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO progresion_jugador
            (jugador_id, plan_id, semana, ataque, defensa, resistencia, velocidad, tecnica)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            fila.jugador_id,
            fila.plan_id,
            fila.semana,
            fila.ataque,
            fila.defensa,
            fila.resistencia,
            fila.velocidad,
            fila.tecnica
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza una medición.
pub fn update(conn: &Connection, fila: &ProgresionFila) -> Result<(), DbError> {
    conn.execute(
        "UPDATE progresion_jugador SET jugador_id = ?1, plan_id = ?2, semana = ?3,
                ataque = ?4, defensa = ?5, resistencia = ?6, velocidad = ?7, tecnica = ?8
          WHERE id = ?9",
        params![
            fila.jugador_id,
            fila.plan_id,
            fila.semana,
            fila.ataque,
            fila.defensa,
            fila.resistencia,
            fila.velocidad,
            fila.tecnica,
            fila.id
        ],
    )?;
    Ok(())
}

/// Borra una medición.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM progresion_jugador WHERE id = ?1", params![id])?;
    Ok(())
}

/// Inserta un plan.
pub fn insert_plan(conn: &Connection, plan: &PlanEntrenamiento) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO entrenamiento_plan (nombre, descripcion, duracion_semanas, sesiones_por_semana)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            plan.nombre,
            plan.descripcion,
            plan.duracion_semanas,
            plan.sesiones_por_semana
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza un plan.
pub fn update_plan(conn: &Connection, plan: &PlanEntrenamiento) -> Result<(), DbError> {
    conn.execute(
        "UPDATE entrenamiento_plan SET nombre = ?1, descripcion = ?2,
                duracion_semanas = ?3, sesiones_por_semana = ?4
          WHERE id = ?5",
        params![
            plan.nombre,
            plan.descripcion,
            plan.duracion_semanas,
            plan.sesiones_por_semana,
            plan.id
        ],
    )?;
    Ok(())
}

/// Borra un plan.
pub fn delete_plan(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM entrenamiento_plan WHERE id = ?1", params![id])?;
    Ok(())
}
