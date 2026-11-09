//! Química entre jugadores (IMP-24).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::quimica_repo;
use crate::domain::quimica::Dupla;

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

/// Lista duplas.
pub fn listar(conn: &Connection) -> Result<Vec<Dupla>, ServiceError> {
    Ok(quimica_repo::list(conn)?)
}

/// Jugadores disponibles.
pub fn jugadores(conn: &Connection) -> Result<Vec<(i64, String)>, ServiceError> {
    Ok(quimica_repo::jugadores(conn)?)
}

/// Crea una dupla.
pub fn crear(conn: &Connection, dupla: &Dupla) -> Result<i64, ServiceError> {
    validar(dupla)?;
    Ok(quimica_repo::insert(conn, dupla)?)
}

/// Actualiza una dupla.
pub fn actualizar(conn: &Connection, dupla: &Dupla) -> Result<(), ServiceError> {
    validar(dupla)?;
    quimica_repo::update(conn, dupla)?;
    Ok(())
}

/// Elimina una dupla.
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    quimica_repo::delete(conn, id)?;
    Ok(())
}

fn validar(dupla: &Dupla) -> Result<(), ServiceError> {
    if dupla.jugador1_id == dupla.jugador2_id {
        return Err(ServiceError::Message(
            "Elegí dos jugadores distintos".to_string(),
        ));
    }
    if dupla.partidos < 0 || dupla.victorias < 0 || dupla.goles < 0 || dupla.asistencias < 0 {
        return Err(ServiceError::Message(
            "Las estadísticas no pueden ser negativas".to_string(),
        ));
    }
    if dupla.victorias > dupla.partidos {
        return Err(ServiceError::Message(
            "Las victorias no pueden superar los partidos".to_string(),
        ));
    }
    Ok(())
}

/// Ordena por winrate y partidos (útil para tests).
#[must_use]
pub fn ordenar(mut duplas: Vec<Dupla>) -> Vec<Dupla> {
    duplas.sort_by(|a, b| {
        b.winrate()
            .total_cmp(&a.winrate())
            .then_with(|| b.partidos.cmp(&a.partidos))
            .then_with(|| b.aporte().total_cmp(&a.aporte()))
    });
    duplas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn dupla(a: i64, b: i64, partidos: i64, victorias: i64) -> Dupla {
        Dupla {
            id: 0,
            jugador1_id: a,
            jugador2_id: b,
            jugador1: String::new(),
            jugador2: String::new(),
            partidos,
            victorias,
            goles: 0,
            asistencias: 0,
            comentario: String::new(),
        }
    }

    #[test]
    fn validates_and_orders() {
        assert!(validar(&dupla(1, 1, 1, 0)).is_err());
        assert!(validar(&dupla(1, 2, 1, 2)).is_err());
        let ordenadas = ordenar(vec![dupla(1, 2, 10, 5), dupla(2, 3, 4, 4)]);
        assert_eq!(ordenadas[0].jugador2_id, 3);
        assert!((ordenadas[0].winrate() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn database_flow() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO equipo (nombre) VALUES ('E')", [])
            .unwrap();
        for (nombre, numero) in [("Ana", 1), ("Beto", 2)] {
            db.conn()
                .execute(
                    "INSERT INTO jugador (equipo_id, nombre, numero, posicion) VALUES (1, ?1, ?2, 3)",
                    rusqlite::params![nombre, numero],
                )
                .unwrap();
        }
        let id = crear(db.conn(), &dupla(1, 2, 10, 6)).unwrap();
        let lista = listar(db.conn()).unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].jugador1, "Ana");
        eliminar(db.conn(), id).unwrap();
        assert!(listar(db.conn()).unwrap().is_empty());
    }
}
