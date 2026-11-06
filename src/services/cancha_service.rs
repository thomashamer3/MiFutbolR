//! Reglas de negocio del módulo Canchas (§9.4).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::cancha_repo;
use crate::domain::models::{Cancha, CanchaDependencias, CanchaFilter, CanchaRow};
use crate::domain::validation::{self, DomainError};

/// Error de negocio del módulo.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Validación.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
    /// Id inexistente.
    #[error("No existe la cancha id={0}")]
    NotFound(i64),
}

/// Resultado de eliminar una cancha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoEliminacion {
    /// Se borró.
    Eliminada,
    /// Tenía partidos: se desactivó.
    Desactivada(CanchaDependencias),
}

/// Valida una cancha completa.
pub fn validar(cancha: &Cancha) -> Result<(), DomainError> {
    validation::required(&cancha.nombre, "nombre")?;
    validation::max_len(&cancha.nombre, "nombre", 100)?;
    validation::max_len(&cancha.telefono, "teléfono", 40)?;
    validation::max_len(&cancha.direccion, "dirección", 200)?;
    validation::max_len(&cancha.localidad, "localidad", 100)?;
    validation::max_len(&cancha.estado, "estado", 50)?;
    validation::max_len(&cancha.descripcion, "descripción", 200)?;
    validation::max_len(&cancha.contacto_alt, "contacto alternativo", 120)?;
    validation::max_len(&cancha.imagen_ruta, "imagen", 255)?;

    if !cancha.telefono.trim().is_empty()
        && !cancha
            .telefono
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '+' | ' ' | '-' | '(' | ')'))
    {
        return Err(DomainError::Message(
            "El teléfono solo admite dígitos, +, espacios y guiones".into(),
        ));
    }

    if cancha.cantidad_canchas < 1 {
        return Err(DomainError::Message(
            "La cantidad de canchas debe ser al menos 1".into(),
        ));
    }
    if cancha.precio_dia.0 < 0 || cancha.precio_noche.0 < 0 {
        return Err(DomainError::Message(
            "Los precios no pueden ser negativos".into(),
        ));
    }
    for (nombre, valor) in [
        ("apertura", cancha.horario_apertura_min),
        ("cierre", cancha.horario_cierre_min),
    ] {
        if let Some(minutos) = valor {
            validation::range_i64(minutos, 0, 1439, nombre)?;
        }
    }
    match (cancha.latitud, cancha.longitud) {
        (Some(lat), Some(lon)) => {
            validation::range_f64(lat, -90.0, 90.0, "latitud")?;
            validation::range_f64(lon, -180.0, 180.0, "longitud")?;
        }
        (None, None) => {}
        _ => {
            return Err(DomainError::Message(
                "Latitud y longitud deben cargarse juntas".into(),
            ));
        }
    }
    validation::image_path(&cancha.imagen_ruta)?;
    Ok(())
}

/// Crea una cancha (transacción padre + hijas).
pub fn crear(conn: &mut Connection, cancha: &mut Cancha) -> Result<i64, ServiceError> {
    validar(cancha)?;
    let id = cancha_repo::insert(conn, cancha)?;
    cancha.id = id;
    Ok(id)
}

/// Actualiza una cancha.
pub fn actualizar(conn: &mut Connection, cancha: &Cancha) -> Result<(), ServiceError> {
    if cancha.id <= 0 {
        return Err(ServiceError::NotFound(cancha.id));
    }
    validar(cancha)?;
    cancha_repo::get(conn, cancha.id)?;
    cancha_repo::update(conn, cancha)?;
    Ok(())
}

/// Elimina o desactiva según dependencias.
pub fn eliminar(conn: &Connection, id: i64) -> Result<ResultadoEliminacion, ServiceError> {
    let dependencias = cancha_repo::dependencias(conn, id)?;
    if dependencias.has_dependencies() {
        cancha_repo::set_active(conn, id, false)?;
        Ok(ResultadoEliminacion::Desactivada(dependencias))
    } else {
        cancha_repo::delete(conn, id)?;
        Ok(ResultadoEliminacion::Eliminada)
    }
}

/// Activa o desactiva una cancha.
pub fn set_activa(conn: &Connection, id: i64, activa: bool) -> Result<(), ServiceError> {
    cancha_repo::set_active(conn, id, activa)?;
    Ok(())
}

/// Lista canchas con filtro.
pub fn listar(conn: &Connection, filter: &CanchaFilter) -> Result<Vec<CanchaRow>, ServiceError> {
    Ok(cancha_repo::list(conn, filter)?)
}

/// Obtiene una cancha.
pub fn obtener(conn: &Connection, id: i64) -> Result<Cancha, ServiceError> {
    Ok(cancha_repo::get(conn, id)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::domain::Cents;
    use crate::domain::models::{ServiciosCancha, Superficie, TechadaEstado, TipoCancha};

    fn base() -> Cancha {
        Cancha {
            id: 0,
            nombre: "Complejo Sur".into(),
            telefono: "+54 11 1234-5678".into(),
            direccion: "Av. Siempre Viva 742".into(),
            localidad: "Quilmes".into(),
            tipos: vec![TipoCancha::F5, TipoCancha::F7],
            superficies: vec![Superficie::Sintetico],
            techada: TechadaEstado::Techada,
            tiene_iluminacion: true,
            horario_apertura_min: Some(8 * 60),
            horario_cierre_min: Some(23 * 60),
            precio_dia: Cents(2_000_000),
            precio_noche: Cents(2_500_000),
            servicios: ServiciosCancha {
                vestuarios: true,
                duchas: true,
                buffet: false,
                estacionamiento: true,
            },
            cantidad_canchas: 2,
            estado: "Habilitada".into(),
            descripcion: String::new(),
            contacto_alt: String::new(),
            latitud: Some(-34.7),
            longitud: Some(-58.25),
            tiene_grabacion: false,
            tiene_marcador: TechadaEstado::Techada,
            imagen_ruta: String::new(),
            activa: true,
        }
    }

    #[test]
    fn validates_required_and_ranges() {
        let mut cancha = base();
        cancha.nombre = "  ".into();
        assert!(validar(&cancha).is_err());

        let mut sin_cantidad = base();
        sin_cantidad.cantidad_canchas = 0;
        assert!(validar(&sin_cantidad).is_err());

        let mut coordenadas = base();
        coordenadas.latitud = Some(-34.7);
        coordenadas.longitud = None;
        assert!(validar(&coordenadas).is_err());

        let mut telefono = base();
        telefono.telefono = "abc".into();
        assert!(validar(&telefono).is_err());
    }

    #[test]
    fn create_roundtrip_keeps_children() {
        let mut db = Database::open_in_memory().unwrap();
        let mut cancha = base();
        let id = crear(db.conn_mut(), &mut cancha).unwrap();

        let cargada = cancha_repo::get(db.conn(), id).unwrap();
        assert_eq!(cargada.nombre, "Complejo Sur");
        assert_eq!(cargada.tipos, vec![TipoCancha::F5, TipoCancha::F7]);
        assert_eq!(cargada.superficies, vec![Superficie::Sintetico]);
        assert!(cargada.servicios.duchas);
        assert_eq!(cargada.precio_noche, Cents(2_500_000));
        assert_eq!(cargada.horario_cierre_min, Some(23 * 60));

        // El caché legacy se escribe con la máscara.
        let cache: i64 = db
            .conn()
            .query_row(
                "SELECT tipo_cancha_codigo FROM cancha WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cache, 1000 + 1 + 2);
    }

    #[test]
    fn update_replaces_children() {
        let mut db = Database::open_in_memory().unwrap();
        let mut cancha = base();
        let id = crear(db.conn_mut(), &mut cancha).unwrap();

        let mut cargada = cancha_repo::get(db.conn(), id).unwrap();
        cargada.tipos = vec![TipoCancha::F11];
        cargada.superficies = vec![Superficie::Natural, Superficie::Cemento];
        actualizar(db.conn_mut(), &cargada).unwrap();

        let actualizada = cancha_repo::get(db.conn(), id).unwrap();
        assert_eq!(actualizada.tipos, vec![TipoCancha::F11]);
        assert_eq!(
            actualizada.superficies,
            vec![Superficie::Natural, Superficie::Cemento]
        );
    }

    #[test]
    fn delete_deactivates_when_used_by_matches() {
        let mut db = Database::open_in_memory().unwrap();
        let mut cancha = base();
        let id = crear(db.conn_mut(), &mut cancha).unwrap();
        db.conn_mut()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Camiseta')", [])
            .unwrap();
        let camiseta_id = db.conn().last_insert_rowid();
        db.conn()
            .execute(
                "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora)
                 VALUES (?1, ?2, '2026-04-01 20:00:00')",
                rusqlite::params![id, camiseta_id],
            )
            .unwrap();

        let resultado = eliminar(db.conn(), id).unwrap();
        assert!(matches!(resultado, ResultadoEliminacion::Desactivada(_)));
        assert!(!cancha_repo::get(db.conn(), id).unwrap().activa);
    }

    #[test]
    fn list_aggregates_and_search() {
        let mut db = Database::open_in_memory().unwrap();
        for nombre in ["Complejo Sur", "Complejo Norte"] {
            let mut cancha = base();
            cancha.nombre = nombre.into();
            crear(db.conn_mut(), &mut cancha).unwrap();
        }
        let filter = CanchaFilter {
            texto: "norte".into(),
            ..CanchaFilter::default()
        };
        let rows = listar(db.conn(), &filter).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].nombre, "Complejo Norte");
        assert_eq!(rows[0].partidos, 0);
    }
}
