//! Reglas de temporadas (IMP-12).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::temporada_repo;
use crate::domain::temporadas::{
    ComparacionTemporadas, Temporada, TemporadaFicha, TemporadaResumen,
};

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

/// Lista temporadas.
pub fn listar(conn: &Connection) -> Result<Vec<Temporada>, ServiceError> {
    Ok(temporada_repo::list(conn)?)
}

/// Crea una temporada, sus fases y asocia los torneos existentes.
pub fn crear(conn: &Connection, ficha: &Temporada) -> Result<i64, ServiceError> {
    validar(ficha)?;
    let id = temporada_repo::insert(
        conn,
        &ficha.nombre,
        ficha.anio,
        ficha.fecha_inicio,
        ficha.fecha_fin,
        &ficha.descripcion,
    )?;
    temporada_repo::insertar_fases_default(conn, id, ficha.fecha_inicio, ficha.fecha_fin)?;
    temporada_repo::asociar_todos_torneos(conn, id)?;
    Ok(id)
}

/// Actualiza una temporada.
pub fn actualizar(conn: &Connection, ficha: &Temporada) -> Result<(), ServiceError> {
    validar(ficha)?;
    temporada_repo::update(
        conn,
        ficha.id,
        &ficha.nombre,
        ficha.anio,
        ficha.fecha_inicio,
        ficha.fecha_fin,
        &ficha.descripcion,
    )?;
    Ok(())
}

/// Elimina una temporada.
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    temporada_repo::delete(conn, id)?;
    Ok(())
}

/// Ficha con resumen y fatiga.
pub fn ficha(conn: &Connection, id: i64) -> Result<TemporadaFicha, ServiceError> {
    let temporada = temporada_repo::get(conn, id)?;
    let fases = temporada_repo::fases(conn, id)?;
    let torneos = temporada_repo::torneos(conn, id)?;
    let resumen = temporada_repo::resumen(conn, temporada.fecha_inicio, temporada.fecha_fin)?;
    let fatiga = temporada_repo::fatiga(conn, temporada.fecha_fin)?;
    Ok(TemporadaFicha {
        temporada,
        fases,
        torneos,
        resumen,
        fatiga,
    })
}

/// Compara dos temporadas.
pub fn comparar(conn: &Connection, a: i64, b: i64) -> Result<ComparacionTemporadas, ServiceError> {
    let ficha_a = ficha(conn, a)?;
    let ficha_b = ficha(conn, b)?;
    Ok(armar_comparacion(&ficha_a, &ficha_b))
}

/// Arma la comparación (pura).
#[must_use]
pub fn armar_comparacion(a: &TemporadaFicha, b: &TemporadaFicha) -> ComparacionTemporadas {
    let mut filas = Vec::new();
    let mut fila = |metrica: &str, va: String, vb: String| {
        filas.push((metrica.to_string(), va, vb));
    };
    fila(
        "Partidos",
        a.resumen.partidos.to_string(),
        b.resumen.partidos.to_string(),
    );
    fila(
        "Goles",
        a.resumen.goles.to_string(),
        b.resumen.goles.to_string(),
    );
    fila(
        "Asistencias",
        a.resumen.asistencias.to_string(),
        b.resumen.asistencias.to_string(),
    );
    fila(
        "Victorias",
        a.resumen.victorias.to_string(),
        b.resumen.victorias.to_string(),
    );
    fila(
        "Empates",
        a.resumen.empates.to_string(),
        b.resumen.empates.to_string(),
    );
    fila(
        "Derrotas",
        a.resumen.derrotas.to_string(),
        b.resumen.derrotas.to_string(),
    );
    fila(
        "Winrate %",
        format!("{:.1}", a.resumen.winrate),
        format!("{:.1}", b.resumen.winrate),
    );
    fila(
        "Rendimiento",
        format!("{:.2}", a.resumen.rendimiento),
        format!("{:.2}", b.resumen.rendimiento),
    );
    fila(
        "Lesiones",
        a.resumen.lesiones.to_string(),
        b.resumen.lesiones.to_string(),
    );
    fila(
        "Fatiga",
        format!("{:.0}", a.fatiga),
        format!("{:.0}", b.fatiga),
    );
    ComparacionTemporadas {
        a: a.temporada.nombre.clone(),
        b: b.temporada.nombre.clone(),
        filas,
    }
}

fn validar(ficha: &Temporada) -> Result<(), ServiceError> {
    if ficha.nombre.trim().is_empty() {
        return Err(ServiceError::Message(
            "El nombre de la temporada es obligatorio".to_string(),
        ));
    }
    if ficha.fecha_fin < ficha.fecha_inicio {
        return Err(ServiceError::Message(
            "La fecha de fin no puede ser anterior al inicio".to_string(),
        ));
    }
    Ok(())
}

/// Resumen vacío para tests.
#[must_use]
pub fn resumen_vacio() -> TemporadaResumen {
    TemporadaResumen::default()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::domain::temporadas::Temporada;

    fn temporada(id: i64, nombre: &str, anio: i64) -> Temporada {
        Temporada {
            id,
            nombre: nombre.to_string(),
            anio,
            fecha_inicio: chrono::NaiveDate::from_ymd_opt(anio as i32, 1, 1).unwrap(),
            fecha_fin: chrono::NaiveDate::from_ymd_opt(anio as i32, 12, 31).unwrap(),
            estado: "Activa".to_string(),
            descripcion: String::new(),
        }
    }

    #[test]
    fn creates_summary_and_compares() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO cancha (nombre) VALUES ('C')", [])
            .unwrap();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('K')", [])
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, goles, asistencias,
                                      rendimiento_general, resultado)
                 VALUES (1, 1, '2026-03-10 20:00:00', 2, 1, 8, 1)",
                [],
            )
            .unwrap();
        let id = crear(db.conn(), &temporada(0, "2026", 2026)).unwrap();
        let ficha = ficha(db.conn(), id).unwrap();
        assert_eq!(ficha.resumen.partidos, 1);
        assert_eq!(ficha.resumen.goles, 2);
        assert_eq!(ficha.resumen.victorias, 1);
        assert_eq!(ficha.fases.len(), 3);

        let otra = crear(db.conn(), &temporada(0, "2025", 2025)).unwrap();
        let comparacion = comparar(db.conn(), id, otra).unwrap();
        assert_eq!(comparacion.filas.len(), 10);
        assert_eq!(comparacion.a, "2026");
    }
}
