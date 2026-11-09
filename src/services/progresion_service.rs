//! Progresión (IMP-23).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::progresion_repo;
use crate::domain::progresion::{PlanEntrenamiento, ProgresionFila};

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

/// Lista mediciones.
pub fn listar(conn: &Connection) -> Result<Vec<ProgresionFila>, ServiceError> {
    Ok(progresion_repo::list(conn)?)
}

/// Lista planes.
pub fn planes(conn: &Connection) -> Result<Vec<PlanEntrenamiento>, ServiceError> {
    Ok(progresion_repo::planes(conn)?)
}

/// Lista jugadores.
pub fn jugadores(conn: &Connection) -> Result<Vec<(i64, String)>, ServiceError> {
    Ok(progresion_repo::jugadores(conn)?)
}

/// Crea una medición.
pub fn crear(conn: &Connection, fila: &ProgresionFila) -> Result<i64, ServiceError> {
    validar(fila)?;
    Ok(progresion_repo::insert(conn, fila)?)
}

/// Actualiza una medición.
pub fn actualizar(conn: &Connection, fila: &ProgresionFila) -> Result<(), ServiceError> {
    validar(fila)?;
    progresion_repo::update(conn, fila)?;
    Ok(())
}

/// Elimina una medición.
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    progresion_repo::delete(conn, id)?;
    Ok(())
}

/// Crea un plan.
pub fn crear_plan(conn: &Connection, plan: &PlanEntrenamiento) -> Result<i64, ServiceError> {
    validar_plan(plan)?;
    Ok(progresion_repo::insert_plan(conn, plan)?)
}

/// Actualiza un plan.
pub fn actualizar_plan(conn: &Connection, plan: &PlanEntrenamiento) -> Result<(), ServiceError> {
    validar_plan(plan)?;
    progresion_repo::update_plan(conn, plan)?;
    Ok(())
}

/// Elimina un plan.
pub fn eliminar_plan(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    progresion_repo::delete_plan(conn, id)?;
    Ok(())
}

fn validar(fila: &ProgresionFila) -> Result<(), ServiceError> {
    if fila.semana <= 0 {
        return Err(ServiceError::Message(
            "La semana debe ser mayor que cero".to_string(),
        ));
    }
    for (nombre, valor) in [
        ("ataque", fila.ataque),
        ("defensa", fila.defensa),
        ("resistencia", fila.resistencia),
        ("velocidad", fila.velocidad),
        ("técnica", fila.tecnica),
    ] {
        if !(0..=100).contains(&valor) {
            return Err(ServiceError::Message(format!(
                "{nombre} debe estar entre 0 y 100"
            )));
        }
    }
    Ok(())
}

fn validar_plan(plan: &PlanEntrenamiento) -> Result<(), ServiceError> {
    if plan.nombre.trim().is_empty() {
        return Err(ServiceError::Message(
            "El nombre del plan es obligatorio".to_string(),
        ));
    }
    if plan.duracion_semanas <= 0 || plan.sesiones_por_semana <= 0 {
        return Err(ServiceError::Message(
            "Duración y sesiones deben ser mayores que cero".to_string(),
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn plan_and_measurement_flow() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO equipo (nombre) VALUES ('Equipo')", [])
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO jugador (equipo_id, nombre, numero, posicion) VALUES (1, 'Ana', 10, 3)",
                [],
            )
            .unwrap();
        let plan = PlanEntrenamiento {
            id: 0,
            nombre: "Fuerza".to_string(),
            descripcion: String::new(),
            duracion_semanas: 4,
            sesiones_por_semana: 3,
        };
        let plan_id = crear_plan(db.conn(), &plan).unwrap();
        let fila = ProgresionFila {
            id: 0,
            jugador_id: 1,
            jugador: String::new(),
            plan_id: Some(plan_id),
            plan: String::new(),
            semana: 1,
            ataque: 60,
            defensa: 55,
            resistencia: 70,
            velocidad: 65,
            tecnica: 80,
        };
        crear(db.conn(), &fila).unwrap();
        let lista = listar(db.conn()).unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].jugador, "Ana");
        assert_eq!(lista[0].semana, 1);
        assert!((lista[0].promedio() - 66.0).abs() < f64::EPSILON);
    }
}
