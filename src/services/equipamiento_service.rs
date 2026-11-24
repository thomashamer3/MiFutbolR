//! Historial de equipamiento (`equipamiento_historial.c`): alta, edición,
//! baja y estadísticas de uso.

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::equipamiento_repo;
use crate::domain::equipamiento::{ESTADOS, Equipamiento, TIPOS, etiqueta};
use crate::domain::money::Cents;
use crate::domain::reporte::Reporte;

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

fn validar(e: &Equipamiento) -> Result<(), ServiceError> {
    let error = |texto: &str| Err(ServiceError::Message(texto.to_string()));
    if e.marca.trim().is_empty() && e.modelo.trim().is_empty() {
        return error("Cargá al menos la marca o el modelo");
    }
    if !TIPOS.iter().any(|(c, _)| *c == e.tipo)
        || !ESTADOS.iter().any(|(c, _)| *c == e.estado_fisico)
    {
        return error("Tipo o estado físico inválido");
    }
    if !(1..=10).contains(&e.rating) {
        return error("El rating va de 1 a 10");
    }
    if e.partidos_usados < 0 || i64::from(e.precio) < 0 {
        return error("Partidos y precio no pueden ser negativos");
    }
    Ok(())
}

/// Lista los ítems.
pub fn listar(conn: &Connection) -> Result<Vec<Equipamiento>, ServiceError> {
    Ok(equipamiento_repo::list(conn)?)
}

/// Crea un ítem.
pub fn crear(conn: &Connection, e: &Equipamiento) -> Result<i64, ServiceError> {
    validar(e)?;
    Ok(equipamiento_repo::insert(conn, e)?)
}

/// Actualiza un ítem.
pub fn actualizar(conn: &Connection, e: &Equipamiento) -> Result<(), ServiceError> {
    validar(e)?;
    Ok(equipamiento_repo::update(conn, e)?)
}

/// Elimina un ítem.
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    Ok(equipamiento_repo::delete(conn, id)?)
}

/// `equipamiento_historial_estadisticas`: totales, gasto, rating y por tipo.
pub fn estadisticas(conn: &Connection) -> Result<Reporte, ServiceError> {
    let items = equipamiento_repo::list(conn)?;
    let mut r = Reporte::new("Estadísticas de equipamiento");
    let total = items.len();
    let gastado = Cents::from(items.iter().map(|e| i64::from(e.precio)).sum::<i64>());
    let rating = if total > 0 {
        items.iter().map(|e| e.rating).sum::<i64>() as f64 / total as f64
    } else {
        0.0
    };
    r.linea(format!("Total items:      {total}"));
    r.linea(format!(
        "Items activos:    {}",
        items.iter().filter(|e| e.activo).count()
    ));
    r.linea(format!(
        "Partidos totales: {}",
        items.iter().map(|e| e.partidos_usados).sum::<i64>()
    ));
    r.linea(format!("Total gastado:    {}", gastado.display_ars()));
    r.linea(format!("Rating promedio:  {rating:.1}/10"));
    r.seccion("Por tipo");
    for (codigo, nombre) in TIPOS {
        let cantidad = items.iter().filter(|e| e.tipo == *codigo).count() as u64;
        if cantidad > 0 {
            r.linea(format!("  {nombre}: {cantidad}"));
            r.barras.push(((*nombre).to_string(), cantidad));
        }
    }
    r.seccion("Detalle");
    for e in &items {
        r.linea(format!(
            "  #{} [{}] {} {} · {} · {} partidos · {} · {}/10",
            e.id,
            if e.activo { "ACTIVO" } else { "INACTIVO" },
            e.marca,
            e.modelo,
            etiqueta(TIPOS, e.tipo),
            e.partidos_usados,
            etiqueta(ESTADOS, e.estado_fisico),
            e.rating
        ));
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn item(tipo: i64, precio: i64, rating: i64) -> Equipamiento {
        Equipamiento {
            id: 0,
            tipo,
            marca: "Nike".into(),
            modelo: "Mercurial".into(),
            fecha_compra: "2026-01-10".into(),
            precio: Cents::from(precio),
            partidos_usados: 4,
            estado_fisico: 2,
            rating,
            notas: String::new(),
            activo: true,
        }
    }

    #[test]
    fn crud_validacion_y_estadisticas() {
        let db = Database::open_in_memory().unwrap();
        let id = crear(db.conn(), &item(1, 1_000_000, 8)).unwrap();
        crear(db.conn(), &item(5, 500_000, 6)).unwrap();
        assert!(crear(db.conn(), &item(9, 0, 5)).is_err());
        assert!(crear(db.conn(), &item(1, 0, 11)).is_err());
        let mut sin_nombre = item(1, 0, 5);
        sin_nombre.marca.clear();
        sin_nombre.modelo.clear();
        assert!(crear(db.conn(), &sin_nombre).is_err());

        let mut editado = listar(db.conn()).unwrap()[0].clone();
        assert_eq!(editado.id, id);
        editado.activo = false;
        actualizar(db.conn(), &editado).unwrap();
        // Los inactivos van al final.
        assert_eq!(listar(db.conn()).unwrap()[1].id, id);

        let r = estadisticas(db.conn()).unwrap();
        let texto = r.texto();
        assert!(texto.contains("Items activos:    1"), "{texto}");
        assert!(texto.contains("Partidos totales: 8"), "{texto}");
        assert!(texto.contains("Rating promedio:  7.0/10"), "{texto}");
        assert_eq!(
            r.barras,
            [("Botines".to_string(), 1), ("Guantes".to_string(), 1)]
        );
        eliminar(db.conn(), id).unwrap();
        assert_eq!(listar(db.conn()).unwrap().len(), 1);
    }
}
