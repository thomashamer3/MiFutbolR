//! Reglas de negocio del módulo Equipos (§11.4).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::equipo_repo;
use crate::domain::models::{
    Equipo, EquipoDependencias, EquipoFilter, EquipoRow, Jugador, TipoFutbol,
};
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
    /// Nombre de equipo repetido.
    #[error("Ya existe un equipo con el nombre «{0}»")]
    NombreDuplicado(String),
    /// Dorsal repetido.
    #[error("El dorsal {0} ya está usado en este equipo")]
    DorsalDuplicado(i64),
    /// Id inexistente.
    #[error("No existe el equipo id={0}")]
    NotFound(i64),
}

/// Resultado de eliminar un equipo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoEliminacion {
    /// Se borró.
    Eliminada,
    /// Tenía historial: se desactivó.
    Desactivada(EquipoDependencias),
}

/// Jugadores esperados según la modalidad.
#[must_use]
pub const fn jugadores_esperados(tipo: TipoFutbol) -> i64 {
    match tipo {
        TipoFutbol::F5 => 5,
        TipoFutbol::F7 => 7,
        TipoFutbol::F8 => 8,
        TipoFutbol::F11 => 11,
    }
}

/// Valida la cabecera de un equipo.
pub fn validar_equipo(equipo: &Equipo) -> Result<(), DomainError> {
    validation::required(&equipo.nombre, "nombre")?;
    validation::max_len(&equipo.nombre, "nombre", 128)?;
    validation::max_len(&equipo.imagen_ruta, "imagen", 255)?;
    validation::image_path(&equipo.imagen_ruta)?;
    validation::range_i64(equipo.num_jugadores, 1, 11, "cantidad de jugadores")?;
    let esperado = jugadores_esperados(equipo.tipo_futbol);
    if equipo.num_jugadores < esperado {
        return Err(DomainError::Message(format!(
            "Un equipo {esperado} necesita al menos {esperado} jugadores (equipo incompleto)"
        )));
    }
    Ok(())
}

/// Valida un jugador.
pub fn validar_jugador(jugador: &Jugador) -> Result<(), DomainError> {
    validation::required(&jugador.nombre, "nombre del jugador")?;
    validation::max_len(&jugador.nombre, "nombre del jugador", 128)?;
    validation::range_i64(jugador.numero, 0, 99, "dorsal")?;
    Ok(())
}

/// Crea un equipo con sus jugadores.
pub fn crear(conn: &mut Connection, equipo: &mut Equipo) -> Result<i64, ServiceError> {
    validar_equipo(equipo)?;
    validar_plantel(&equipo.jugadores)?;
    let id = equipo_repo::insert(conn, equipo)?;
    equipo.id = id;
    Ok(id)
}

/// Actualiza la cabecera de un equipo.
pub fn actualizar(conn: &Connection, equipo: &Equipo) -> Result<(), ServiceError> {
    if equipo.id <= 0 {
        return Err(ServiceError::NotFound(equipo.id));
    }
    validar_equipo(equipo)?;
    equipo_repo::get(conn, equipo.id)?;
    equipo_repo::update(conn, equipo)?;
    Ok(())
}

/// Elimina o desactiva según dependencias.
pub fn eliminar(conn: &Connection, id: i64) -> Result<ResultadoEliminacion, ServiceError> {
    let dependencias = equipo_repo::dependencias(conn, id)?;
    if dependencias.has_dependencies() {
        equipo_repo::set_active(conn, id, false)?;
        Ok(ResultadoEliminacion::Desactivada(dependencias))
    } else {
        equipo_repo::delete(conn, id)?;
        Ok(ResultadoEliminacion::Eliminada)
    }
}

/// Lista equipos.
pub fn listar(conn: &Connection, filter: &EquipoFilter) -> Result<Vec<EquipoRow>, ServiceError> {
    Ok(equipo_repo::list(conn, filter)?)
}

/// Obtiene un equipo.
pub fn obtener(conn: &Connection, id: i64) -> Result<Equipo, ServiceError> {
    Ok(equipo_repo::get(conn, id)?)
}

/// Activa/desactiva un equipo.
pub fn set_activa(conn: &Connection, id: i64, activa: bool) -> Result<(), ServiceError> {
    equipo_repo::set_active(conn, id, activa)?;
    Ok(())
}

/// Agrega un jugador (garantiza dorsal único y un solo capitán).
pub fn agregar_jugador(conn: &mut Connection, jugador: &mut Jugador) -> Result<i64, ServiceError> {
    validar_jugador(jugador)?;
    if equipo_repo::existe_dorsal(conn, jugador.equipo_id, jugador.numero, None)? {
        return Err(ServiceError::DorsalDuplicado(jugador.numero));
    }
    let equipo_id = jugador.equipo_id;
    let nombre = jugador.nombre.clone();
    let numero = jugador.numero;
    let posicion = jugador.posicion.as_i64();
    let es_capitan = jugador.es_capitan;

    let resultado = (|| -> Result<i64, DbError> {
        let tx = conn.transaction()?;
        if es_capitan {
            tx.execute(
                "UPDATE jugador SET es_capitan = 0 WHERE equipo_id = ?1",
                [equipo_id],
            )?;
        }
        tx.execute(
            "INSERT INTO jugador (equipo_id, nombre, numero, posicion, es_capitan)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![equipo_id, nombre, numero, posicion, i64::from(es_capitan)],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    })();

    let id = resultado?;
    jugador.id = id;
    Ok(id)
}

/// Actualiza un jugador (mismas reglas que el alta).
pub fn actualizar_jugador(conn: &mut Connection, jugador: &Jugador) -> Result<(), ServiceError> {
    validar_jugador(jugador)?;
    if jugador.id <= 0 {
        return Err(ServiceError::NotFound(jugador.id));
    }
    if equipo_repo::existe_dorsal(conn, jugador.equipo_id, jugador.numero, Some(jugador.id))? {
        return Err(ServiceError::DorsalDuplicado(jugador.numero));
    }
    let equipo_id = jugador.equipo_id;
    let jugador_id = jugador.id;
    let nombre = jugador.nombre.clone();
    let numero = jugador.numero;
    let posicion = jugador.posicion.as_i64();
    let es_capitan = jugador.es_capitan;

    let resultado = (|| -> Result<(), DbError> {
        let tx = conn.transaction()?;
        if es_capitan {
            tx.execute(
                "UPDATE jugador SET es_capitan = 0 WHERE equipo_id = ?1 AND id <> ?2",
                rusqlite::params![equipo_id, jugador_id],
            )?;
        }
        tx.execute(
            "UPDATE jugador SET nombre = ?1, numero = ?2, posicion = ?3, es_capitan = ?4 WHERE id = ?5",
            rusqlite::params![nombre, numero, posicion, i64::from(es_capitan), jugador_id],
        )?;
        tx.commit()?;
        Ok(())
    })();

    resultado?;
    Ok(())
}

/// Elimina un jugador.
pub fn eliminar_jugador(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    equipo_repo::delete_jugador(conn, id)?;
    Ok(())
}

fn validar_plantel(jugadores: &[Jugador]) -> Result<(), DomainError> {
    let mut dorsales = std::collections::HashSet::new();
    let mut capitanes = 0;
    for jugador in jugadores {
        validar_jugador(jugador)?;
        if !dorsales.insert(jugador.numero) {
            return Err(DomainError::Message(format!(
                "Dorsal repetido en el plantel: {}",
                jugador.numero
            )));
        }
        if jugador.es_capitan {
            capitanes += 1;
        }
    }
    if capitanes > 1 {
        return Err(DomainError::Message(
            "Solo puede haber un capitán por equipo".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::domain::models::{Posicion, TipoEquipo};

    fn equipo_base() -> Equipo {
        Equipo {
            id: 0,
            nombre: "Los Pibes".into(),
            tipo: TipoEquipo::Fijo,
            tipo_futbol: TipoFutbol::F5,
            num_jugadores: 5,
            partido_id: None,
            activa: true,
            imagen_ruta: String::new(),
            jugadores: Vec::new(),
        }
    }

    fn jugador(equipo_id: i64, nombre: &str, numero: i64) -> Jugador {
        Jugador {
            id: 0,
            equipo_id,
            nombre: nombre.into(),
            numero,
            posicion: Posicion::Defensor,
            es_capitan: false,
        }
    }

    #[test]
    fn validates_team_and_players() {
        let mut equipo = equipo_base();
        equipo.nombre = " ".into();
        assert!(validar_equipo(&equipo).is_err());

        let mut incompleto = equipo_base();
        incompleto.num_jugadores = 3;
        assert!(validar_equipo(&incompleto).is_err());

        let mut jugador = jugador(1, "Ana", 120);
        assert!(validar_jugador(&jugador).is_err());
        jugador.numero = 10;
        assert!(validar_jugador(&jugador).is_ok());
    }

    #[test]
    fn create_team_with_players_transactionally() {
        let mut db = Database::open_in_memory().unwrap();
        let mut equipo = equipo_base();
        equipo.jugadores = vec![jugador(0, "Uno", 1), jugador(0, "Dos", 2)];
        let id = crear(db.conn_mut(), &mut equipo).unwrap();
        let cargado = equipo_repo::get(db.conn(), id).unwrap();
        assert_eq!(cargado.jugadores.len(), 2);
        assert_eq!(cargado.jugadores[0].nombre, "Uno");
    }

    #[test]
    fn duplicate_dorsal_is_rejected() {
        let mut db = Database::open_in_memory().unwrap();
        let mut equipo = equipo_base();
        let id = crear(db.conn_mut(), &mut equipo).unwrap();
        let mut primero = jugador(id, "Uno", 10);
        agregar_jugador(db.conn_mut(), &mut primero).unwrap();
        let mut segundo = jugador(id, "Dos", 10);
        assert!(matches!(
            agregar_jugador(db.conn_mut(), &mut segundo),
            Err(ServiceError::DorsalDuplicado(10))
        ));
    }

    #[test]
    fn only_one_captain_remains() {
        let mut db = Database::open_in_memory().unwrap();
        let mut equipo = equipo_base();
        let id = crear(db.conn_mut(), &mut equipo).unwrap();

        let mut primero = jugador(id, "Uno", 10);
        primero.es_capitan = true;
        agregar_jugador(db.conn_mut(), &mut primero).unwrap();

        let mut segundo = jugador(id, "Dos", 7);
        segundo.es_capitan = true;
        agregar_jugador(db.conn_mut(), &mut segundo).unwrap();

        let capitanes: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM jugador WHERE equipo_id = ?1 AND es_capitan = 1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(capitanes, 1);
    }

    #[test]
    fn delete_deactivates_team_with_history() {
        let mut db = Database::open_in_memory().unwrap();
        let mut equipo = equipo_base();
        let id = crear(db.conn_mut(), &mut equipo).unwrap();

        db.conn_mut()
            .execute(
                "INSERT INTO torneo (nombre, equipo_fijo_id) VALUES ('Liga', ?1)",
                [id],
            )
            .unwrap();

        let resultado = eliminar(db.conn(), id).unwrap();
        assert!(matches!(resultado, ResultadoEliminacion::Desactivada(_)));
        assert!(!equipo_repo::get(db.conn(), id).unwrap().activa);
    }

    #[test]
    fn list_counts_players() {
        let mut db = Database::open_in_memory().unwrap();
        let mut equipo = equipo_base();
        equipo.jugadores = vec![jugador(0, "Uno", 1)];
        crear(db.conn_mut(), &mut equipo).unwrap();

        let rows = listar(db.conn(), &EquipoFilter::default()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].jugadores_count, 1);
        assert!(rows[0].activa);
        assert_eq!(rows[0].tipo_futbol, TipoFutbol::F5);
    }
}
