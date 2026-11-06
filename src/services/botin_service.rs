//! Reglas de negocio del módulo Botines (§10.4).

use rand::Rng;
use rand::seq::SliceRandom;
use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::botin_repo;
use crate::domain::models::{Botin, BotinDependencias, BotinFilter, BotinRow};
use crate::domain::validation::{self, DomainError};

/// Error de negocio.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Validación.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
    /// Nombre repetido.
    #[error("Ya existe un botín con el nombre «{0}»")]
    NombreDuplicado(String),
    /// Id inexistente.
    #[error("No existe el botín id={0}")]
    NotFound(i64),
}

/// Resultado de eliminar un botín.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoEliminacion {
    /// Se borró.
    Eliminada,
    /// Tenía partidos: se desactivó.
    Desactivada(BotinDependencias),
}

/// Valida un botín.
pub fn validar(botin: &Botin) -> Result<(), DomainError> {
    validation::required(&botin.nombre, "nombre")?;
    validation::max_len(&botin.nombre, "nombre", 100)?;
    validation::max_len(&botin.imagen_ruta, "imagen", 255)?;
    validation::image_path(&botin.imagen_ruta)
}

/// Crea un botín.
pub fn crear(conn: &Connection, botin: &mut Botin) -> Result<i64, ServiceError> {
    validar(botin)?;
    if botin_repo::existe_nombre(conn, &botin.nombre, None)? {
        return Err(ServiceError::NombreDuplicado(botin.nombre.clone()));
    }
    let id = botin_repo::insert(conn, botin)?;
    botin.id = id;
    Ok(id)
}

/// Actualiza un botín.
pub fn actualizar(conn: &Connection, botin: &Botin) -> Result<(), ServiceError> {
    if botin.id <= 0 {
        return Err(ServiceError::NotFound(botin.id));
    }
    validar(botin)?;
    botin_repo::get(conn, botin.id)?;
    if botin_repo::existe_nombre(conn, &botin.nombre, Some(botin.id))? {
        return Err(ServiceError::NombreDuplicado(botin.nombre.clone()));
    }
    botin_repo::update(conn, botin)?;
    Ok(())
}

/// Elimina o desactiva según dependencias.
pub fn eliminar(conn: &Connection, id: i64) -> Result<ResultadoEliminacion, ServiceError> {
    let dependencias = botin_repo::dependencias(conn, id)?;
    if dependencias.has_dependencies() {
        botin_repo::set_active(conn, id, false)?;
        Ok(ResultadoEliminacion::Desactivada(dependencias))
    } else {
        botin_repo::delete(conn, id)?;
        Ok(ResultadoEliminacion::Eliminada)
    }
}

/// Activa o desactiva un botín.
pub fn set_activa(conn: &Connection, id: i64, activa: bool) -> Result<(), ServiceError> {
    botin_repo::set_active(conn, id, activa)?;
    Ok(())
}

/// Lista botines.
pub fn listar(conn: &Connection, filter: &BotinFilter) -> Result<Vec<BotinRow>, ServiceError> {
    Ok(botin_repo::list(conn, filter)?)
}

/// Obtiene un botín.
pub fn obtener(conn: &Connection, id: i64) -> Result<Botin, ServiceError> {
    Ok(botin_repo::get(conn, id)?)
}

/// Fija el botín predeterminado (debe existir y estar activo).
pub fn fijar_predeterminado(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    if id == 0 {
        botin_repo::set_default(conn, 0)?;
        return Ok(());
    }
    let botin = botin_repo::get(conn, id)?;
    if !botin.activa {
        return Err(
            DomainError::Message("El botín predeterminado debe estar activo".into()).into(),
        );
    }
    botin_repo::set_default(conn, id)?;
    Ok(())
}

/// Sortea un botín con RNG inyectable.
pub fn sortear_con_rng<R: Rng + ?Sized>(
    conn: &Connection,
    rng: &mut R,
) -> Result<Option<Botin>, ServiceError> {
    let mut pendientes = botin_repo::draw_pending(conn)?;
    if pendientes.is_empty() {
        botin_repo::reset_draw(conn)?;
        pendientes = botin_repo::draw_pending(conn)?;
    }
    let Some(elegido) = pendientes.choose(rng).cloned() else {
        return Ok(None);
    };
    botin_repo::mark_drawn(conn, elegido.id)?;
    Ok(Some(elegido))
}

/// Sortea con el RNG del sistema.
pub fn sortear(conn: &Connection) -> Result<Option<Botin>, ServiceError> {
    let mut rng = rand::thread_rng();
    sortear_con_rng(conn, &mut rng)
}

/// Reinicia el ciclo de sorteo.
pub fn reiniciar_sorteo(conn: &Connection) -> Result<usize, ServiceError> {
    Ok(botin_repo::reset_draw(conn)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn base(nombre: &str) -> Botin {
        Botin {
            id: 0,
            nombre: nombre.into(),
            imagen_ruta: String::new(),
            activa: true,
            sorteada: false,
        }
    }

    #[test]
    fn duplicate_name_is_rejected() {
        let db = Database::open_in_memory().unwrap();
        let mut primero = base("Predator");
        crear(db.conn(), &mut primero).unwrap();
        let mut segundo = base("PREDATOR");
        assert!(matches!(
            crear(db.conn(), &mut segundo),
            Err(ServiceError::NombreDuplicado(_))
        ));
    }

    #[test]
    fn default_requires_active_boot() {
        let db = Database::open_in_memory().unwrap();
        let mut botin = base("Tiempo");
        let id = crear(db.conn(), &mut botin).unwrap();
        fijar_predeterminado(db.conn(), id).unwrap();
        let valor: i64 = db
            .conn()
            .query_row(
                "SELECT botin_predeterminado FROM settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(valor, id);

        botin_repo::set_active(db.conn(), id, false).unwrap();
        assert!(fijar_predeterminado(db.conn(), id).is_err());
    }

    #[test]
    fn draw_cycle_and_restart() {
        let db = Database::open_in_memory().unwrap();
        for nombre in ["A", "B"] {
            let mut botin = base(nombre);
            crear(db.conn(), &mut botin).unwrap();
        }
        let mut rng = StdRng::seed_from_u64(3);
        let uno = sortear_con_rng(db.conn(), &mut rng).unwrap().unwrap();
        let dos = sortear_con_rng(db.conn(), &mut rng).unwrap().unwrap();
        assert_ne!(uno.id, dos.id);

        let tres = sortear_con_rng(db.conn(), &mut rng).unwrap().unwrap();
        assert!(tres.id == uno.id || tres.id == dos.id);
        assert_eq!(reiniciar_sorteo(db.conn()).unwrap(), 2);
    }

    #[test]
    fn deleting_the_default_clears_settings() {
        let db = Database::open_in_memory().unwrap();
        let mut botin = base("Unico");
        let id = crear(db.conn(), &mut botin).unwrap();
        fijar_predeterminado(db.conn(), id).unwrap();
        assert_eq!(
            eliminar(db.conn(), id).unwrap(),
            ResultadoEliminacion::Eliminada
        );
        let valor: i64 = db
            .conn()
            .query_row(
                "SELECT botin_predeterminado FROM settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(valor, 0);
    }

    #[test]
    fn list_marks_default_and_counts_matches() {
        let db = Database::open_in_memory().unwrap();
        let mut botin = base("Messi");
        let id = crear(db.conn(), &mut botin).unwrap();
        fijar_predeterminado(db.conn(), id).unwrap();
        let rows = listar(db.conn(), &BotinFilter::default()).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].predeterminado);
        assert!(!rows[0].sorteada);
    }
}
