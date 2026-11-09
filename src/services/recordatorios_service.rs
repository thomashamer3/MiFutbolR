//! Recordatorios y agenda (IMP-18).

use chrono::{Months, NaiveDate};
use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::recordatorio_repo;
use crate::domain::dates;
use crate::domain::recordatorios::{EventoAgenda, Periodicidad, Recordatorio};

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

/// Lista y avanza las recurrencias vencidas.
pub fn listar(conn: &Connection) -> Result<Vec<Recordatorio>, ServiceError> {
    avanzar_recurrentes(conn)?;
    Ok(recordatorio_repo::list(conn)?)
}

/// Crea un recordatorio.
pub fn crear(conn: &Connection, item: &Recordatorio) -> Result<i64, ServiceError> {
    validar(item)?;
    Ok(recordatorio_repo::insert(conn, item)?)
}

/// Actualiza un recordatorio.
pub fn actualizar(conn: &Connection, item: &Recordatorio) -> Result<(), ServiceError> {
    validar(item)?;
    recordatorio_repo::update(conn, item)?;
    Ok(())
}

/// Elimina un recordatorio.
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    recordatorio_repo::delete(conn, id)?;
    Ok(())
}

/// Alterna completado.
pub fn completar(
    conn: &Connection,
    item: &Recordatorio,
    completado: bool,
) -> Result<(), ServiceError> {
    let mut copia = item.clone();
    copia.completado = completado;
    recordatorio_repo::update(conn, &copia)?;
    Ok(())
}

/// Agenda fusionada.
pub fn agenda(conn: &Connection) -> Result<Vec<EventoAgenda>, ServiceError> {
    avanzar_recurrentes(conn)?;
    Ok(recordatorio_repo::agenda(conn)?)
}

/// Avanza las ocurrencias diarias/semanales/mensuales vencidas.
pub fn avanzar_recurrentes(conn: &Connection) -> Result<usize, ServiceError> {
    let hoy = dates::today();
    let mut cambios = 0;
    for item in recordatorio_repo::recurrentes(conn)? {
        let mut fecha = item.fecha;
        let mut iteraciones = 0;
        while fecha < hoy && iteraciones < 400 {
            fecha = siguiente(fecha, item.periodicidad);
            iteraciones += 1;
        }
        if fecha != item.fecha && fecha >= hoy {
            recordatorio_repo::avanzar_fecha(conn, item.id, fecha, hoy)?;
            cambios += 1;
        }
    }
    Ok(cambios)
}

/// Siguiente fecha según periodicidad.
#[must_use]
pub fn siguiente(fecha: NaiveDate, periodicidad: Periodicidad) -> NaiveDate {
    match periodicidad {
        Periodicidad::UnaVez => fecha,
        Periodicidad::Diaria => fecha + chrono::Duration::days(1),
        Periodicidad::Semanal => fecha + chrono::Duration::days(7),
        Periodicidad::Mensual => fecha
            .checked_add_months(Months::new(1))
            .unwrap_or(fecha + chrono::Duration::days(30)),
    }
}

fn validar(item: &Recordatorio) -> Result<(), ServiceError> {
    if item.titulo.trim().is_empty() {
        return Err(ServiceError::Message(
            "El título del recordatorio es obligatorio".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn advances_daily_and_monthly() {
        let base = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        assert_eq!(
            siguiente(base, Periodicidad::Diaria),
            base + chrono::Duration::days(1)
        );
        assert_eq!(
            siguiente(base, Periodicidad::Semanal),
            base + chrono::Duration::days(7)
        );
        assert_eq!(
            siguiente(base, Periodicidad::Mensual),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
    }

    #[test]
    fn recurrence_is_materialized() {
        let db = Database::open_in_memory().unwrap();
        let ayer = dates::today() - chrono::Duration::days(2);
        let item = Recordatorio {
            id: 0,
            titulo: "Agua".to_string(),
            nota: String::new(),
            fecha: ayer,
            hora: "08:00".to_string(),
            periodicidad: Periodicidad::Diaria,
            tematica: "Salud".to_string(),
            completado: false,
            ultima_gen: String::new(),
        };
        crear(db.conn(), &item).unwrap();
        let lista = listar(db.conn()).unwrap();
        assert!(lista[0].fecha >= dates::today());
        assert_eq!(lista[0].fecha, dates::today());
    }
}
