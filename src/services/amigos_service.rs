//! Amigos y ranking (IMP-22).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::amigo_repo;
use crate::domain::amigos::{Amigo, RankingAmigo};
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

/// "Comparar conmigo" (`comparar_con_amigo` de `ranking_amigos.c`): mis
/// totales contra los del amigo, lado a lado. C rotulaba "Rendimiento
/// promedio" a goles/partidos; aquí se llama por lo que es.
pub fn comparar(conn: &Connection, id: i64) -> Result<Reporte, ServiceError> {
    let amigo = amigo_repo::get(conn, id)?;
    let yo = amigo_repo::locales(conn)?;
    let por_partido = |goles: i64, partidos: i64| {
        if partidos > 0 {
            goles as f64 / partidos as f64
        } else {
            0.0
        }
    };
    let mut r = Reporte::new(format!("Comparar conmigo · {}", amigo.nombre));
    r.linea(format!(
        "{:<22} {:>10} {:>10}",
        "Estadística", "Yo", amigo.nombre
    ));
    for (etiqueta, mio, suyo) in [
        ("Goles totales", yo.goles, amigo.goles),
        ("Asistencias totales", yo.asistencias, amigo.asistencias),
        ("Partidos jugados", yo.partidos, amigo.partidos),
        (
            "Mejor rendimiento",
            yo.mejor_rendimiento,
            amigo.mejor_rendimiento,
        ),
    ] {
        r.linea(format!("{etiqueta:<22} {mio:>10} {suyo:>10}"));
    }
    r.linea(format!(
        "{:<22} {:>10.2} {:>10.2}",
        "Goles por partido",
        por_partido(yo.goles, yo.partidos),
        por_partido(amigo.goles, amigo.partidos)
    ));
    r.linea("");
    r.linea(format!(
        "Posición del amigo: {}",
        if amigo.posicion.trim().is_empty() {
            "No registrada"
        } else {
            &amigo.posicion
        }
    ));
    r.linea(format!("Diferencia de goles: {:+}", yo.goles - amigo.goles));
    r.linea(format!(
        "Diferencia de asistencias: {:+}",
        yo.asistencias - amigo.asistencias
    ));
    Ok(r)
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

    #[test]
    fn comparar_conmigo_muestra_diferencias() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO cancha (nombre) VALUES ('C');
                 INSERT INTO camiseta (nombre) VALUES ('K');
                 INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, goles, asistencias,
                    rendimiento_general) VALUES (1, 1, '2026-01-01 20:00:00', 4, 3, 9);",
            )
            .unwrap();
        let id = crear(db.conn(), &amigo("Ana", 7)).unwrap();
        let texto = comparar(db.conn(), id).unwrap().texto();
        assert!(texto.contains("Diferencia de goles: -3"), "{texto}");
        assert!(texto.contains("Diferencia de asistencias: +2"), "{texto}");
        assert!(texto.contains("Posición del amigo: Delantero"), "{texto}");
        assert!(comparar(db.conn(), 99).is_err());
    }
}
