//! Evaluación y desbloqueo de logros (IMP-04).

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::db::logros_repo;
use crate::domain::logros::{LogroProgreso, definiciones};

/// Lista el catálogo con el progreso actual.
pub fn listar(conn: &Connection, camiseta_id: Option<i64>) -> Result<Vec<LogroProgreso>, DbError> {
    let valores = logros_repo::valores(conn, camiseta_id)?;
    let desbloqueados = logros_repo::desbloqueados(conn)?;
    let mut progresos: Vec<LogroProgreso> = definiciones()
        .into_iter()
        .map(|def| {
            let valor = valores.get(def.tipo.as_str()).copied().unwrap_or(0);
            let completado = valor >= def.objetivo;
            LogroProgreso {
                desbloqueado: desbloqueados.contains(&def.id),
                def,
                valor,
                completado,
            }
        })
        .collect();
    progresos.sort_by(|a, b| {
        b.completado
            .cmp(&a.completado)
            .then_with(|| b.progreso().total_cmp(&a.progreso()))
    });
    Ok(progresos)
}

/// Registra los logros recién completados y los devuelve.
pub fn registrar_nuevos(
    conn: &Connection,
    progresos: &[LogroProgreso],
) -> Result<Vec<LogroProgreso>, DbError> {
    let ids: Vec<String> = progresos
        .iter()
        .filter(|logro| logro.completado && !logro.desbloqueado)
        .map(|logro| logro.def.id.clone())
        .collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    logros_repo::registrar(conn, &ids)?;
    let mut nuevos = Vec::new();
    for logro in progresos {
        if ids.contains(&logro.def.id) {
            let mut copia = logro.clone();
            copia.desbloqueado = true;
            nuevos.push(copia);
        }
    }
    Ok(nuevos)
}

/// Cuenta cuántos logros están completados.
#[must_use]
pub fn completados(progresos: &[LogroProgreso]) -> usize {
    progresos.iter().filter(|logro| logro.completado).count()
}
