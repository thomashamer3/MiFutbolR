//! Reglas de negocio del módulo Camisetas (§8.4).
//!
//! Valida antes de tocar la BD, garantiza nombre único, decide eliminación vs
//! desactivación y ejecuta el sorteo con un RNG inyectable (testeable).

use rand::Rng;
use rand::seq::SliceRandom;
use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::camiseta_repo;
use crate::domain::models::{
    Camiseta, CamisetaDependencias, CamisetaFilter, CamisetaRow, FichaCamiseta,
};
use crate::domain::validation::{self, DomainError};

/// Error de negocio del módulo.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Validación de dominio.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
    /// Nombre repetido.
    #[error("Ya existe una camiseta con el nombre «{0}»")]
    NombreDuplicado(String),
    /// Id inexistente.
    #[error("No existe la camiseta id={0}")]
    NotFound(i64),
}

/// Resultado de intentar eliminar una camiseta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoEliminacion {
    /// Se borró físicamente.
    Eliminada,
    /// Tenía historial: se desactivó en su lugar.
    Desactivada(CamisetaDependencias),
}

/// Extensiones de imagen aceptadas (§8.4).
const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];

/// Valida una camiseta completa.
pub fn validar(camiseta: &Camiseta) -> Result<(), DomainError> {
    validation::required(&camiseta.nombre, "nombre")?;
    validation::max_len(&camiseta.nombre, "nombre", 100)?;
    validation::max_len(&camiseta.color_principal, "color principal", 50)?;
    validation::max_len(&camiseta.color_secundario, "color secundario", 50)?;
    validation::max_len(&camiseta.marca, "marca", 60)?;
    validation::max_len(&camiseta.modelo, "modelo", 80)?;
    validation::max_len(&camiseta.temporada, "temporada", 40)?;
    validation::max_len(&camiseta.observaciones, "observaciones", 240)?;
    validation::max_len(&camiseta.proveedor, "proveedor", 120)?;
    validation::max_len(&camiseta.regalo_de, "regalo de", 100)?;

    if camiseta.costo.0 < 0 {
        return Err(DomainError::Message(
            "El costo no puede ser negativo".into(),
        ));
    }
    if camiseta.fue_regalo && camiseta.regalo_de.trim().is_empty() {
        return Err(DomainError::Required("regalo de"));
    }
    if let Some(fecha) = camiseta.fecha_compra {
        let year = chrono::Datelike::year(&fecha);
        validation::range_i64(i64::from(year), 1900, 2100, "fecha de compra")?;
    }
    validar_imagen(&camiseta.imagen_ruta)?;
    Ok(())
}

/// Valida la extensión de una ruta de imagen (vacía es válida).
pub fn validar_imagen(ruta: &str) -> Result<(), DomainError> {
    let trimmed = ruta.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let lower = trimmed.to_lowercase();
    if IMAGE_EXTENSIONS
        .iter()
        .any(|extension| lower.ends_with(&format!(".{extension}")))
    {
        Ok(())
    } else {
        Err(DomainError::Message(format!(
            "Extensión de imagen no permitida: {trimmed}"
        )))
    }
}

/// Crea una camiseta, comprobando el nombre único.
pub fn crear(conn: &Connection, camiseta: &mut Camiseta) -> Result<i64, ServiceError> {
    validar(camiseta)?;
    if camiseta_repo::existe_nombre(conn, &camiseta.nombre, None)? {
        return Err(ServiceError::NombreDuplicado(camiseta.nombre.clone()));
    }
    let id = camiseta_repo::insert(conn, camiseta)?;
    camiseta.id = id;
    Ok(id)
}

/// Actualiza una camiseta existente.
pub fn actualizar(conn: &Connection, camiseta: &Camiseta) -> Result<(), ServiceError> {
    if camiseta.id <= 0 {
        return Err(ServiceError::NotFound(camiseta.id));
    }
    validar(camiseta)?;
    camiseta_repo::get(conn, camiseta.id)?;
    if camiseta_repo::existe_nombre(conn, &camiseta.nombre, Some(camiseta.id))? {
        return Err(ServiceError::NombreDuplicado(camiseta.nombre.clone()));
    }
    camiseta_repo::update(conn, camiseta)?;
    Ok(())
}

/// Elimina si no tiene historial; si lo tiene, la desactiva.
pub fn eliminar(conn: &Connection, id: i64) -> Result<ResultadoEliminacion, ServiceError> {
    let dependencias = camiseta_repo::dependencias(conn, id)?;
    if dependencias.has_dependencies() {
        camiseta_repo::set_active(conn, id, false)?;
        Ok(ResultadoEliminacion::Desactivada(dependencias))
    } else {
        camiseta_repo::delete(conn, id)?;
        Ok(ResultadoEliminacion::Eliminada)
    }
}

/// Cambia el estado activo.
pub fn set_activa(conn: &Connection, id: i64, activa: bool) -> Result<(), ServiceError> {
    camiseta_repo::set_active(conn, id, activa)?;
    Ok(())
}

/// Ficha con estadísticas.
pub fn ficha(conn: &Connection, id: i64) -> Result<FichaCamiseta, ServiceError> {
    Ok(camiseta_repo::ficha(conn, id)?)
}

/// Lista camisetas aplicando el filtro.
pub fn listar(
    conn: &Connection,
    filter: &CamisetaFilter,
) -> Result<Vec<CamisetaRow>, ServiceError> {
    Ok(camiseta_repo::list(conn, filter)?)
}

/// Obtiene una camiseta por id.
pub fn obtener(conn: &Connection, id: i64) -> Result<Camiseta, ServiceError> {
    Ok(camiseta_repo::get(conn, id)?)
}

/// Sortea una camiseta usando el RNG recibido (determinista en tests).
pub fn sortear_con_rng<R: Rng + ?Sized>(
    conn: &Connection,
    rng: &mut R,
) -> Result<Option<Camiseta>, ServiceError> {
    let mut pendientes = camiseta_repo::draw_pending(conn)?;
    if pendientes.is_empty() {
        camiseta_repo::reset_draw(conn)?;
        pendientes = camiseta_repo::draw_pending(conn)?;
    }
    let Some(elegida) = pendientes.choose(rng).cloned() else {
        return Ok(None);
    };
    camiseta_repo::mark_drawn(conn, elegida.id)?;
    Ok(Some(elegida))
}

/// Sortea usando el RNG del sistema.
pub fn sortear(conn: &Connection) -> Result<Option<Camiseta>, ServiceError> {
    let mut rng = rand::thread_rng();
    sortear_con_rng(conn, &mut rng)
}

/// Reinicia el ciclo de sorteo.
pub fn reiniciar_sorteo(conn: &Connection) -> Result<usize, ServiceError> {
    Ok(camiseta_repo::reset_draw(conn)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::domain::Cents;
    use crate::domain::models::EstadoFisico;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn base(nombre: &str) -> Camiseta {
        Camiseta {
            id: 0,
            nombre: nombre.to_string(),
            color_principal: "Rojo".into(),
            color_secundario: "Blanco".into(),
            marca: "Nike".into(),
            modelo: "Dri-FIT".into(),
            temporada: "2026".into(),
            estado_fisico: EstadoFisico::Buena,
            fecha_compra: None,
            costo: Cents(0),
            observaciones: String::new(),
            proveedor: String::new(),
            fue_regalo: false,
            regalo_de: String::new(),
            imagen_ruta: String::new(),
            activa: true,
            sorteada: false,
        }
    }

    #[test]
    fn validates_required_fields() {
        let db = Database::open_in_memory().unwrap();
        let mut camiseta = base("   ");
        assert!(matches!(
            crear(db.conn(), &mut camiseta),
            Err(ServiceError::Domain(DomainError::Required("nombre")))
        ));

        let mut regalo = base("Regalo");
        regalo.fue_regalo = true;
        assert!(matches!(
            validar(&regalo),
            Err(DomainError::Required("regalo de"))
        ));

        let mut negativa = base("Negativa");
        negativa.costo = Cents(-1);
        assert!(validar(&negativa).is_err());
    }

    #[test]
    fn rejects_duplicate_names_case_insensitive() {
        let db = Database::open_in_memory().unwrap();
        let mut primera = base("Titular 10");
        crear(db.conn(), &mut primera).unwrap();

        let mut segunda = base("titular 10");
        assert!(matches!(
            crear(db.conn(), &mut segunda),
            Err(ServiceError::NombreDuplicado(_))
        ));
    }

    #[test]
    fn create_update_and_ficha() {
        let db = Database::open_in_memory().unwrap();
        let mut camiseta = base("Titular 10");
        camiseta.costo = Cents(150_000);
        let id = crear(db.conn(), &mut camiseta).unwrap();

        let mut cargada = camiseta_repo::get(db.conn(), id).unwrap();
        assert_eq!(cargada.nombre, "Titular 10");
        assert_eq!(cargada.costo, Cents(150_000));

        cargada.nombre = "Titular 9".into();
        actualizar(db.conn(), &cargada).unwrap();
        assert_eq!(
            camiseta_repo::get(db.conn(), id).unwrap().nombre,
            "Titular 9"
        );

        let ficha = ficha(db.conn(), id).unwrap();
        assert_eq!(ficha.partidos, 0);
    }

    #[test]
    fn deletion_deactivates_when_it_has_history() {
        let db = Database::open_in_memory().unwrap();
        let mut camiseta = base("Con historia");
        let id = crear(db.conn(), &mut camiseta).unwrap();

        db.conn()
            .execute("INSERT INTO cancha (nombre) VALUES ('Cancha 1')", [])
            .unwrap();
        let cancha_id = db.conn().last_insert_rowid();
        db.conn()
            .execute(
                "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora) VALUES (?1, ?2, '2026-01-01 20:00:00')",
                [cancha_id, id],
            )
            .unwrap();

        let resultado = eliminar(db.conn(), id).unwrap();
        assert!(matches!(resultado, ResultadoEliminacion::Desactivada(_)));
        assert!(!camiseta_repo::get(db.conn(), id).unwrap().activa);
    }

    #[test]
    fn deletion_removes_clean_rows() {
        let db = Database::open_in_memory().unwrap();
        let mut camiseta = base("Sin historia");
        let id = crear(db.conn(), &mut camiseta).unwrap();
        assert_eq!(
            eliminar(db.conn(), id).unwrap(),
            ResultadoEliminacion::Eliminada
        );
        assert!(camiseta_repo::get(db.conn(), id).is_err());
    }

    #[test]
    fn draw_cycle_covers_all_and_restarts() {
        let db = Database::open_in_memory().unwrap();
        for nombre in ["A", "B", "C"] {
            let mut camiseta = base(nombre);
            crear(db.conn(), &mut camiseta).unwrap();
        }
        let mut rng = StdRng::seed_from_u64(42);
        let mut vistos = Vec::new();
        for _ in 0..3 {
            let elegida = sortear_con_rng(db.conn(), &mut rng).unwrap().unwrap();
            assert!(!vistos.contains(&elegida.id), "sorteo repetido");
            vistos.push(elegida.id);
        }
        // El cuarto sorteo reinicia el ciclo automáticamente.
        let cuarta = sortear_con_rng(db.conn(), &mut rng).unwrap().unwrap();
        assert!(vistos.contains(&cuarta.id));
        let marcadas: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM camiseta WHERE sorteada = 1 AND activa = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(marcadas, 1);
    }

    #[test]
    fn reset_draw_clears_flags() {
        let db = Database::open_in_memory().unwrap();
        for nombre in ["A", "B"] {
            let mut camiseta = base(nombre);
            crear(db.conn(), &mut camiseta).unwrap();
        }
        let mut rng = StdRng::seed_from_u64(7);
        sortear_con_rng(db.conn(), &mut rng).unwrap();
        assert_eq!(reiniciar_sorteo(db.conn()).unwrap(), 2);
        let pendientes = camiseta_repo::draw_pending(db.conn()).unwrap();
        assert_eq!(pendientes.len(), 2);
    }

    #[test]
    fn image_extension_validation() {
        assert!(validar_imagen("").is_ok());
        assert!(validar_imagen("foto.PNG").is_ok());
        assert!(validar_imagen("C:/Imagenes/camiseta.webp").is_ok());
        assert!(validar_imagen("camiseta.exe").is_err());
    }
}
