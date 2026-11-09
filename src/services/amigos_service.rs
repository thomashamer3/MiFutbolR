//! Amigos y ranking (IMP-22).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::amigo_repo;
use crate::domain::amigos::{Amigo, RankingAmigo};

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

/// Lista amigos.
pub fn listar(conn: &Connection) -> Result<Vec<Amigo>, ServiceError> {
    Ok(amigo_repo::list(conn)?)
}

/// Crea un amigo.
pub fn crear(conn: &Connection, amigo: &Amigo) -> Result<i64, ServiceError> {
    validar(amigo)?;
    Ok(amigo_repo::insert(conn, amigo)?)
}

/// Actualiza un amigo.
pub fn actualizar(conn: &Connection, amigo: &Amigo) -> Result<(), ServiceError> {
    validar(amigo)?;
    amigo_repo::update(conn, amigo)?;
    Ok(())
}

/// Elimina un amigo.
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    amigo_repo::delete(conn, id)?;
    Ok(())
}

/// Ranking con el usuario local incluido.
pub fn ranking(conn: &Connection) -> Result<Vec<RankingAmigo>, ServiceError> {
    let mut filas: Vec<RankingAmigo> = amigo_repo::list(conn)?
        .into_iter()
        .map(|amigo| RankingAmigo {
            nombre: amigo.nombre,
            goles: amigo.goles,
            asistencias: amigo.asistencias,
            partidos: amigo.partidos,
            mejor_rendimiento: amigo.mejor_rendimiento,
            local: false,
        })
        .collect();
    filas.push(amigo_repo::locales(conn)?);
    filas.sort_by(|a, b| {
        b.goles
            .cmp(&a.goles)
            .then_with(|| b.asistencias.cmp(&a.asistencias))
            .then_with(|| b.mejor_rendimiento.cmp(&a.mejor_rendimiento))
            .then_with(|| a.nombre.cmp(&b.nombre))
    });
    Ok(filas)
}

fn validar(amigo: &Amigo) -> Result<(), ServiceError> {
    if amigo.nombre.trim().is_empty() {
        return Err(ServiceError::Message(
            "El nombre del amigo es obligatorio".to_string(),
        ));
    }
    if amigo.goles < 0 || amigo.asistencias < 0 || amigo.partidos < 0 {
        return Err(ServiceError::Message(
            "Las estadísticas no pueden ser negativas".to_string(),
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn amigo(nombre: &str, goles: i64) -> Amigo {
        Amigo {
            id: 0,
            nombre: nombre.to_string(),
            posicion: "Delantero".to_string(),
            goles,
            asistencias: 1,
            partidos: 5,
            mejor_rendimiento: 8,
            notas: String::new(),
        }
    }

    #[test]
    fn ranking_includes_local() {
        let db = Database::open_in_memory().unwrap();
        crear(db.conn(), &amigo("Ana", 7)).unwrap();
        crear(db.conn(), &amigo("Beto", 2)).unwrap();
        let ranking = ranking(db.conn()).unwrap();
        assert_eq!(ranking.len(), 3);
        assert_eq!(ranking[0].nombre, "Ana");
        assert!(ranking.iter().any(|fila| fila.local));
    }
}
