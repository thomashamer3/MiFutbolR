//! Generación de alertas y notificaciones (IMP-06).

use chrono::Local;
use rusqlite::Connection;

use crate::db::error::DbError;
use crate::db::{inicio_repo, notificacion_repo};
use crate::domain::notificacion::Notificacion;
use crate::services::financiamiento_service;

/// Lista las notificaciones.
pub fn listar(conn: &Connection, solo_no_leidas: bool) -> Result<Vec<Notificacion>, DbError> {
    notificacion_repo::list(conn, solo_no_leidas)
}

/// Cantidad de no leídas.
pub fn no_leidas(conn: &Connection) -> Result<i64, DbError> {
    notificacion_repo::contar_no_leidas(conn)
}

/// Marca una o todas como leídas.
pub fn marcar(conn: &Connection, id: Option<i64>) -> Result<(), DbError> {
    match id {
        Some(id) => notificacion_repo::marcar_leida(conn, id),
        None => {
            notificacion_repo::marcar_todas(conn)?;
            Ok(())
        }
    }
}

/// Genera las alertas que correspondan y devuelve cuántas nuevas creó.
pub fn generar(conn: &Connection) -> Result<usize, DbError> {
    let mut nuevas = 0;

    for (id, jugador, tipo) in notificacion_repo::lesiones_recientes(conn)? {
        if !notificacion_repo::existe_hoy(conn, "lesion", "lesion", Some(id))? {
            notificacion_repo::insert(
                conn,
                "lesion",
                2,
                "Lesión activa",
                &format!("{jugador}: {tipo}"),
                "lesion",
                Some(id),
            )?;
            nuevas += 1;
        }
    }

    if let Some(partido) = inicio_repo::proximo_partido(conn)? {
        let faltan = partido
            .fecha_hora
            .signed_duration_since(Local::now().naive_local());
        if faltan.num_hours() >= 0
            && faltan.num_hours() <= 24
            && !notificacion_repo::existe_hoy(conn, "partido", "partido", Some(partido.id))?
        {
            notificacion_repo::insert(
                conn,
                "partido",
                1,
                "Partido próximo",
                &format!(
                    "vs {} el {}",
                    if partido.rival.is_empty() {
                        "rival"
                    } else {
                        &partido.rival
                    },
                    partido.fecha_hora.format("%d/%m %H:%M")
                ),
                "partido",
                Some(partido.id),
            )?;
            nuevas += 1;
        }
    }

    let mes = Local::now().format("%Y-%m").to_string();
    if let Ok(Some(vista)) = financiamiento_service::presupuesto(conn, &mes)
        && !matches!(vista.alerta, crate::domain::models::AlertaPresupuesto::Ok)
        && !notificacion_repo::existe_hoy(conn, "presupuesto", "presupuesto", None)?
    {
        notificacion_repo::insert(
            conn,
            "presupuesto",
            2,
            "Presupuesto",
            &format!(
                "{} · gasto {}",
                vista.alerta.label(),
                vista.gasto_real.display_ars()
            ),
            "presupuesto",
            None,
        )?;
        nuevas += 1;
    }

    let racha = inicio_repo::racha(conn)?;
    if racha.tipo == crate::domain::models::RachaTipo::Positiva
        && racha.cantidad >= 3
        && !notificacion_repo::existe_hoy(conn, "racha", "racha", None)?
    {
        notificacion_repo::insert(
            conn,
            "racha",
            0,
            "Racha ganadora",
            &format!("{} victorias consecutivas", racha.cantidad),
            "racha",
            None,
        )?;
        nuevas += 1;
    }

    Ok(nuevas)
}

/// Notifica los logros recién desbloqueados.
pub fn notificar_logros(
    conn: &Connection,
    nuevos: &[crate::domain::logros::LogroProgreso],
) -> Result<(), DbError> {
    for logro in nuevos {
        notificacion_repo::insert(
            conn,
            "logro",
            0,
            "¡Logro desbloqueado!",
            &format!("{}: {}", logro.def.nombre, logro.def.descripcion),
            "logro",
            None,
        )?;
    }
    Ok(())
}
