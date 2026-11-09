//! Composición del panel "Hoy" (IMP-01).

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::db::inicio_repo;
use crate::domain::inicio::InicioResumen;
use crate::domain::logros::LogroProgreso;
use crate::services::financiamiento_service;

/// Arma el resumen del panel con los datos actuales.
pub fn resumen(conn: &Connection) -> Result<InicioResumen, DbError> {
    let mes = chrono::Local::now().format("%Y-%m").to_string();
    let mut resumen = InicioResumen {
        proximo: inicio_repo::proximo_partido(conn)?,
        ultimo: inicio_repo::ultimo_partido(conn)?,
        racha: inicio_repo::racha(conn)?,
        mes: inicio_repo::mes(conn, &mes)?,
        lesiones_activas: inicio_repo::lesiones_activas(conn)?,
        presupuesto_alerta: None,
        logro_cercano: None,
    };

    if let Ok(Some(vista)) = financiamiento_service::presupuesto(conn, &mes)
        && !matches!(vista.alerta, crate::domain::models::AlertaPresupuesto::Ok)
    {
        resumen.presupuesto_alerta = Some(format!(
            "{} ({})",
            vista.alerta.label(),
            vista.gasto_real.display_ars()
        ));
    }

    let progresos = crate::services::logros_service::listar(conn, None)?;
    resumen.logro_cercano = progresos
        .into_iter()
        .filter(|logro| !logro.completado && logro.valor > 0)
        .max_by(|a, b| a.progreso().total_cmp(&b.progreso()));

    Ok(resumen)
}

/// Convierte una lista de progresos en el logro más cercano.
#[must_use]
pub fn cercano(progresos: &[LogroProgreso]) -> Option<LogroProgreso> {
    progresos
        .iter()
        .filter(|logro| !logro.completado && logro.valor > 0)
        .max_by(|a, b| a.progreso().total_cmp(&b.progreso()))
        .cloned()
}
