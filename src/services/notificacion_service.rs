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

/// Tipos de la notificación manual (`crear_notificacion` de C).
pub const TIPOS_MANUALES: &[&str] = &["general", "lesion", "finanza", "torneo", "cumpleanos"];

/// Crea una notificación a mano.
pub fn crear(conn: &Connection, tipo: &str, titulo: &str, mensaje: &str) -> Result<i64, DbError> {
    notificacion_repo::insert(conn, tipo, 0, titulo, mensaje, "manual", None)
}

/// Días que faltan para el próximo cumpleaños (0 = hoy). El 29/02 se festeja
/// el 28/02 en los años no bisiestos.
#[must_use]
pub fn dias_hasta_cumple(nacimiento: chrono::NaiveDate, hoy: chrono::NaiveDate) -> i64 {
    use chrono::Datelike;
    let en = |anio: i32| {
        chrono::NaiveDate::from_ymd_opt(anio, nacimiento.month(), nacimiento.day())
            .or_else(|| chrono::NaiveDate::from_ymd_opt(anio, 2, 28))
            .unwrap_or(hoy)
    };
    let este = en(hoy.year());
    let proximo = if este >= hoy {
        este
    } else {
        en(hoy.year() + 1)
    };
    (proximo - hoy).num_days()
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

    // C listaba los del mes; aquí se avisa desde 7 días antes, una vez por día.
    let hoy = Local::now().date_naive();
    for (id, nombre, fecha) in notificacion_repo::cumpleanos(conn)? {
        let Some(nacimiento) = crate::domain::dates::parse_date(&fecha) else {
            continue;
        };
        let dias = dias_hasta_cumple(nacimiento, hoy);
        if dias <= 7 && !notificacion_repo::existe_hoy(conn, "cumpleanos", "jugador", Some(id))? {
            let cuando = match dias {
                0 => "¡hoy!".to_string(),
                1 => "mañana".to_string(),
                n => format!("en {n} días"),
            };
            notificacion_repo::insert(
                conn,
                "cumpleanos",
                0,
                "Cumpleaños",
                &format!("{nombre} cumple años {cuando}"),
                "jugador",
                Some(id),
            )?;
            nuevas += 1;
        }
    }

    for (id, torneo, temporada, fin) in notificacion_repo::torneos_por_vencer(conn)? {
        if !notificacion_repo::existe_hoy(conn, "torneo", "torneo", Some(id))? {
            notificacion_repo::insert(
                conn,
                "torneo",
                1,
                "Torneo por vencer",
                &format!("{torneo}: la temporada {temporada} termina el {fin}"),
                "torneo",
                Some(id),
            )?;
            nuevas += 1;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use chrono::NaiveDate;

    fn fecha(a: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, d).unwrap()
    }

    #[test]
    fn dias_hasta_cumple_cruza_el_anio_y_el_29_de_febrero() {
        assert_eq!(dias_hasta_cumple(fecha(1990, 3, 5), fecha(2026, 3, 5)), 0);
        assert_eq!(dias_hasta_cumple(fecha(1990, 3, 8), fecha(2026, 3, 5)), 3);
        assert_eq!(dias_hasta_cumple(fecha(1990, 1, 2), fecha(2026, 12, 30)), 3);
        assert_eq!(dias_hasta_cumple(fecha(2000, 2, 29), fecha(2026, 2, 27)), 1);
    }

    #[test]
    fn genera_cumpleanos_y_torneos_por_vencer_una_vez_por_dia() {
        let db = Database::open_in_memory().unwrap();
        let hoy = Local::now().date_naive();
        let fin = hoy + chrono::Duration::days(10);
        db.conn()
            .execute_batch(&format!(
                "INSERT INTO equipo (id, nombre) VALUES (1, 'Rojos');
                 INSERT INTO jugador (equipo_id, nombre, numero, posicion, fecha_nacimiento)
                    VALUES (1, 'Ana', 9, 3, '{}'), (1, 'Beto', 8, 3, '');
                 INSERT INTO torneo (id, nombre) VALUES (1, 'Apertura'), (2, 'Viejo');
                 UPDATE torneo SET fase_actual = 'Finalizado' WHERE id = 2;
                 INSERT INTO temporada (id, nombre, anio, fecha_inicio, fecha_fin)
                    VALUES (1, '2026', 2026, '2026-01-01', '{}');
                 INSERT INTO torneo_temporada VALUES (1, 1), (1, 2);",
                hoy.format("1995-%m-%d"),
                fin.format("%Y-%m-%d")
            ))
            .unwrap();
        let antes = notificacion_repo::list(db.conn(), false).unwrap().len();
        generar(db.conn()).unwrap();
        let lista = notificacion_repo::list(db.conn(), false).unwrap();
        let mensajes: Vec<&str> = lista.iter().map(|n| n.mensaje.as_str()).collect();
        assert!(mensajes.contains(&"Ana cumple años ¡hoy!"), "{mensajes:?}");
        assert!(
            mensajes
                .iter()
                .any(|m| m.starts_with("Apertura: la temporada 2026"))
        );
        assert!(!mensajes.iter().any(|m| m.starts_with("Viejo")));
        let despues = lista.len();
        generar(db.conn()).unwrap();
        assert_eq!(
            notificacion_repo::list(db.conn(), false).unwrap().len(),
            despues
        );
        assert!(despues >= antes + 2);
        crear(db.conn(), "general", "Recordar", "Llevar pelota").unwrap();
        assert!(
            notificacion_repo::list(db.conn(), false)
                .unwrap()
                .iter()
                .any(|n| n.titulo == "Recordar" && n.tipo == "general")
        );
    }
}
