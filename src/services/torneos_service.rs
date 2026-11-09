//! Reglas de torneos, fixture y tabla (IMP-11).

use std::collections::HashMap;

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::torneo_repo;
use crate::domain::torneos::{
    PartidoTorneoRow, StandingRow, TipoTorneo, Torneo, TorneoEquipo, TorneoFicha,
};

/// Error del servicio de torneos.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
    /// Validación o regla.
    #[error("{0}")]
    Message(String),
}

/// Lista los torneos.
pub fn listar(conn: &Connection) -> Result<Vec<Torneo>, ServiceError> {
    Ok(torneo_repo::list(conn)?)
}

/// Crea un torneo y asocia los equipos activos.
pub fn crear(conn: &Connection, nombre: &str, tipo: TipoTorneo) -> Result<i64, ServiceError> {
    let nombre = validar_nombre(nombre)?;
    let id = torneo_repo::insert(conn, &nombre, tipo)?;
    torneo_repo::asociar_todos(conn, id)?;
    Ok(id)
}

/// Actualiza nombre y tipo.
pub fn actualizar(
    conn: &Connection,
    id: i64,
    nombre: &str,
    tipo: TipoTorneo,
) -> Result<(), ServiceError> {
    let nombre = validar_nombre(nombre)?;
    torneo_repo::update(conn, id, &nombre, tipo)?;
    Ok(())
}

/// Elimina un torneo.
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    torneo_repo::delete(conn, id)?;
    Ok(())
}

/// Asocia todos los equipos activos.
pub fn asociar_todos(conn: &Connection, id: i64) -> Result<usize, ServiceError> {
    Ok(torneo_repo::asociar_todos(conn, id)?)
}

/// Genera el fixture (round robin).
pub fn generar_fixture(conn: &Connection, id: i64) -> Result<usize, ServiceError> {
    let torneo = torneo_repo::get(conn, id)?;
    let equipos = torneo_repo::equipos(conn, id)?;
    if equipos.len() < 2 {
        return Err(ServiceError::Message(
            "Hacen falta al menos 2 equipos activos para generar el fixture".to_string(),
        ));
    }
    let ids: Vec<i64> = equipos.iter().map(|equipo| equipo.id).collect();
    let mut partidos = emparejamientos(&ids);
    if torneo.tipo == TipoTorneo::IdaYVuelta {
        let mut vuelta: Vec<(i64, i64)> = partidos
            .iter()
            .map(|(local, visitante)| (*visitante, *local))
            .collect();
        partidos.append(&mut vuelta);
    }
    torneo_repo::borrar_fixture(conn, id)?;
    Ok(torneo_repo::insertar_fixture(conn, id, &partidos)?)
}

/// Registra un resultado.
pub fn registrar_resultado(
    conn: &Connection,
    partido_id: i64,
    goles_local: i64,
    goles_visitante: i64,
) -> Result<(), ServiceError> {
    if goles_local < 0 || goles_visitante < 0 {
        return Err(ServiceError::Message(
            "Los goles no pueden ser negativos".to_string(),
        ));
    }
    torneo_repo::registrar_resultado(conn, partido_id, goles_local, goles_visitante)?;
    Ok(())
}

/// Ficha completa con tabla calculada.
pub fn ficha(conn: &Connection, id: i64) -> Result<TorneoFicha, ServiceError> {
    let torneo = torneo_repo::get(conn, id)?;
    let equipos = torneo_repo::equipos(conn, id)?;
    let partidos = torneo_repo::partidos(conn, id)?;
    let tabla = calcular_tabla(&equipos, &partidos);
    Ok(TorneoFicha {
        torneo,
        equipos,
        partidos,
        tabla,
    })
}

/// Tabla de posiciones a partir del fixture.
#[must_use]
pub fn calcular_tabla(equipos: &[TorneoEquipo], partidos: &[PartidoTorneoRow]) -> Vec<StandingRow> {
    let mut tabla: HashMap<i64, StandingRow> = equipos
        .iter()
        .map(|equipo| {
            (
                equipo.id,
                StandingRow {
                    equipo: equipo.nombre.clone(),
                    pj: 0,
                    pg: 0,
                    pe: 0,
                    pp: 0,
                    gf: 0,
                    gc: 0,
                    dif: 0,
                    puntos: 0,
                },
            )
        })
        .collect();

    for partido in partidos.iter().filter(|p| p.estado == "Jugado") {
        let (local, visitante) = (partido.local_id, partido.visitante_id);
        let (gl, gv) = (partido.goles_local, partido.goles_visitante);
        if let Some(fila) = tabla.get_mut(&local) {
            actualizar_fila(fila, gl, gv);
        }
        if let Some(fila) = tabla.get_mut(&visitante) {
            actualizar_fila(fila, gv, gl);
        }
    }

    let mut filas: Vec<StandingRow> = tabla.into_values().collect();
    filas.sort_by(|a, b| {
        b.puntos
            .cmp(&a.puntos)
            .then_with(|| b.dif.cmp(&a.dif))
            .then_with(|| b.gf.cmp(&a.gf))
            .then_with(|| a.equipo.cmp(&b.equipo))
    });
    filas
}

fn actualizar_fila(fila: &mut StandingRow, gf: i64, gc: i64) {
    fila.pj += 1;
    fila.gf += gf;
    fila.gc += gc;
    fila.dif = fila.gf - fila.gc;
    match gf.cmp(&gc) {
        std::cmp::Ordering::Greater => {
            fila.pg += 1;
            fila.puntos += 3;
        }
        std::cmp::Ordering::Equal => {
            fila.pe += 1;
            fila.puntos += 1;
        }
        std::cmp::Ordering::Less => fila.pp += 1,
    }
}

fn validar_nombre(nombre: &str) -> Result<String, ServiceError> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(ServiceError::Message(
            "El nombre del torneo es obligatorio".to_string(),
        ));
    }
    if nombre.chars().count() > 100 {
        return Err(ServiceError::Message(
            "El nombre del torneo no puede superar 100 caracteres".to_string(),
        ));
    }
    Ok(nombre.to_string())
}

/// Emparejamientos round robin (método del círculo).
#[must_use]
pub fn emparejamientos(equipos: &[i64]) -> Vec<(i64, i64)> {
    let mut ids = equipos.to_vec();
    if ids.len() < 2 {
        return Vec::new();
    }
    if ids.len() % 2 == 1 {
        ids.push(-1);
    }
    let total = ids.len();
    let rondas = total - 1;
    let mut partidos = Vec::with_capacity(rondas * total / 2);
    for _ in 0..rondas {
        for indice in 0..total / 2 {
            let local = ids[indice];
            let visitante = ids[total - 1 - indice];
            if local >= 0 && visitante >= 0 {
                partidos.push((local, visitante));
            }
        }
        if total > 2 {
            ids[1..].rotate_right(1);
        }
    }
    partidos
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn round_robin_counts_for_even_and_odd() {
        assert_eq!(emparejamientos(&[1, 2]).len(), 1);
        assert_eq!(emparejamientos(&[1, 2, 3, 4, 5, 6]).len(), 15);
        assert_eq!(emparejamientos(&[1, 2, 3, 4, 5, 6, 7]).len(), 21);
        assert!(emparejamientos(&[1]).is_empty());
    }

    #[test]
    fn standings_three_points_one_point() {
        let equipos = vec![
            TorneoEquipo {
                id: 1,
                nombre: "A".to_string(),
            },
            TorneoEquipo {
                id: 2,
                nombre: "B".to_string(),
            },
            TorneoEquipo {
                id: 3,
                nombre: "C".to_string(),
            },
        ];
        let partidos = vec![
            PartidoTorneoRow {
                id: 1,
                local_id: 1,
                local: "A".to_string(),
                visitante_id: 2,
                visitante: "B".to_string(),
                goles_local: 2,
                goles_visitante: 0,
                estado: "Jugado".to_string(),
                fase: "Fase de Grupos".to_string(),
            },
            PartidoTorneoRow {
                id: 2,
                local_id: 2,
                local: "B".to_string(),
                visitante_id: 3,
                visitante: "C".to_string(),
                goles_local: 1,
                goles_visitante: 1,
                estado: "Jugado".to_string(),
                fase: "Fase de Grupos".to_string(),
            },
        ];
        let tabla = calcular_tabla(&equipos, &partidos);
        assert_eq!(tabla[0].equipo, "A");
        assert_eq!(tabla[0].puntos, 3);
        assert_eq!(tabla[0].dif, 2);
        let b = tabla.iter().find(|fila| fila.equipo == "B").unwrap();
        assert_eq!(b.puntos, 1);
        assert_eq!(b.pj, 2);
    }

    #[test]
    fn database_flow_creates_fixture_and_standings() {
        let db = Database::open_in_memory().unwrap();
        for nombre in ["A", "B", "C", "D"] {
            db.conn()
                .execute(
                    "INSERT INTO equipo (nombre) VALUES (?1)",
                    rusqlite::params![nombre],
                )
                .unwrap();
        }
        let id = crear(db.conn(), "Copa", TipoTorneo::SoloIda).unwrap();
        assert_eq!(generar_fixture(db.conn(), id).unwrap(), 6);
        let datos = ficha(db.conn(), id).unwrap();
        assert_eq!(datos.partidos.len(), 6);
        let partido = datos.partidos[0].clone();
        registrar_resultado(db.conn(), partido.id, 3, 1).unwrap();
        let datos = ficha(db.conn(), id).unwrap();
        assert!(datos.tabla.iter().any(|fila| fila.puntos == 3));
    }
}
