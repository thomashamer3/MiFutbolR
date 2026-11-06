//! Reglas de negocio del módulo Lesiones (§13.4).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::lesion_repo;
use crate::domain::models::{Lesion, LesionFilter, LesionRow, LesionStats};
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
    /// Id inexistente.
    #[error("No existe la lesión id={0}")]
    NotFound(i64),
}

/// Valida una lesión.
pub fn validar(lesion: &Lesion) -> Result<(), DomainError> {
    validation::required(&lesion.jugador, "jugador")?;
    validation::max_len(&lesion.jugador, "jugador", 128)?;
    validation::required(&lesion.tipo, "tipo")?;
    validation::max_len(&lesion.tipo, "tipo", 100)?;
    validation::required(&lesion.descripcion, "descripción")?;
    validation::max_len(&lesion.descripcion, "descripción", 200)?;
    validation::positive(lesion.camiseta_id, "camiseta")?;
    let year = chrono::Datelike::year(&lesion.fecha);
    validation::range_i64(i64::from(year), 1900, 2100, "fecha")?;
    Ok(())
}

/// Indica si la fecha de la lesión está en el futuro (advertencia de UI).
#[must_use]
pub fn fecha_futura(lesion: &Lesion) -> bool {
    lesion.fecha > chrono::Local::now().naive_local()
}

/// Crea una lesión.
pub fn crear(conn: &Connection, lesion: &mut Lesion) -> Result<i64, ServiceError> {
    validar(lesion)?;
    let id = lesion_repo::insert(conn, lesion)?;
    lesion.id = id;
    Ok(id)
}

/// Actualiza una lesión.
pub fn actualizar(conn: &Connection, lesion: &Lesion) -> Result<(), ServiceError> {
    if lesion.id <= 0 {
        return Err(ServiceError::NotFound(lesion.id));
    }
    validar(lesion)?;
    lesion_repo::get(conn, lesion.id)?;
    lesion_repo::update(conn, lesion)?;
    Ok(())
}

/// Elimina una lesión.
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    lesion_repo::delete(conn, id)?;
    Ok(())
}

/// Lista lesiones.
pub fn listar(conn: &Connection, filter: &LesionFilter) -> Result<Vec<LesionRow>, ServiceError> {
    Ok(lesion_repo::list(conn, filter)?)
}

/// Obtiene una lesión.
pub fn obtener(conn: &Connection, id: i64) -> Result<Lesion, ServiceError> {
    Ok(lesion_repo::get(conn, id)?)
}

/// Estadísticas.
pub fn estadisticas(conn: &Connection) -> Result<LesionStats, ServiceError> {
    Ok(lesion_repo::stats(conn)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::domain::models::EstadoLesion;

    fn base() -> Lesion {
        Lesion {
            id: 0,
            jugador: "Jugador".into(),
            tipo: "Esguince".into(),
            descripcion: "Tobillo derecho".into(),
            fecha: chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
                .unwrap()
                .and_hms_opt(18, 0, 0)
                .unwrap(),
            camiseta_id: 1,
            estado: EstadoLesion::Activa,
            partido_id: None,
        }
    }

    fn preparar() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
            .unwrap();
        db
    }

    #[test]
    fn validates_required_fields() {
        let mut lesion = base();
        lesion.jugador = "  ".into();
        assert!(validar(&lesion).is_err());
        let mut sin_tipo = base();
        sin_tipo.tipo = String::new();
        assert!(validar(&sin_tipo).is_err());
        let mut sin_camiseta = base();
        sin_camiseta.camiseta_id = 0;
        assert!(validar(&sin_camiseta).is_err());
    }

    #[test]
    fn crud_roundtrip() {
        let db = preparar();
        let mut lesion = base();
        let id = crear(db.conn(), &mut lesion).unwrap();
        let cargada = obtener(db.conn(), id).unwrap();
        assert_eq!(cargada.jugador, "Jugador");
        assert_eq!(cargada.estado, EstadoLesion::Activa);

        let mut editada = cargada;
        editada.estado = EstadoLesion::Recuperado;
        actualizar(db.conn(), &editada).unwrap();
        assert_eq!(
            obtener(db.conn(), id).unwrap().estado,
            EstadoLesion::Recuperado
        );

        eliminar(db.conn(), id).unwrap();
        assert!(obtener(db.conn(), id).is_err());
    }

    #[test]
    fn filter_and_stats() {
        let db = preparar();
        for (tipo, mes) in [("Esguince", 1), ("Contractura", 3)] {
            let mut lesion = base();
            lesion.tipo = tipo.into();
            lesion.fecha = chrono::NaiveDate::from_ymd_opt(2026, mes, 10)
                .unwrap()
                .and_hms_opt(18, 0, 0)
                .unwrap();
            crear(db.conn(), &mut lesion).unwrap();
        }
        let filtro = LesionFilter {
            texto: "esgu".into(),
            ..LesionFilter::default()
        };
        assert_eq!(listar(db.conn(), &filtro).unwrap().len(), 1);

        let stats = estadisticas(db.conn()).unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.activas, 2);
        assert_eq!(stats.por_tipo.len(), 2);
        assert_eq!(stats.por_mes.len(), 2);
        assert!(stats.promedio_dias.is_some());
    }
}
