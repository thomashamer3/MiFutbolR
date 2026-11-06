//! Reglas de negocio del módulo Carrera (§16).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::carrera_repo;
use crate::domain::models::{
    CarreraBadge, CarreraHall, CarreraIdentidad, CarreraPerfil, CarreraResumen, HitoRow,
    TimelineEvento,
};
use crate::domain::validation::{self, DomainError};

/// Error de negocio.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Validación.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
    /// Partido ya marcado como hito.
    #[error("Ese partido ya tiene un hito registrado")]
    HitoDuplicado,
}

/// Valida la ficha de identidad.
pub fn validar_identidad(identidad: &CarreraIdentidad) -> Result<(), DomainError> {
    validation::max_len(&identidad.nombre, "nombre", 128)?;
    validation::range_i64(identidad.edad, 0, 120, "edad")?;
    validation::range_i64(identidad.dorsal_favorito, 0, 99, "dorsal")?;
    validation::max_len(&identidad.posiciones, "posiciones", 200)?;
    validation::max_len(&identidad.estilo, "estilo", 200)?;
    validation::max_len(&identidad.objetivos, "objetivos", 320)?;
    validation::max_len(&identidad.historia, "historia", 512)?;
    if !identidad.pie_habil.trim().is_empty()
        && !matches!(
            identidad.pie_habil.trim().to_lowercase().as_str(),
            "derecho" | "izquierdo" | "ambidiestro"
        )
    {
        return Err(DomainError::Message(
            "Pie hábil inválido (Derecho/Izquierdo/Ambidiestro)".to_string(),
        ));
    }
    if let Some(altura) = identidad.altura_cm {
        validation::range_f64(altura, 50.0, 250.0, "altura")?;
    }
    if let Some(peso) = identidad.peso_kg {
        validation::range_f64(peso, 20.0, 250.0, "peso")?;
    }
    Ok(())
}

/// Carga la identidad.
pub fn identidad(conn: &Connection) -> Result<CarreraIdentidad, ServiceError> {
    Ok(carrera_repo::identidad(conn)?)
}

/// Guarda la identidad.
pub fn guardar_identidad(
    conn: &Connection,
    identidad: &CarreraIdentidad,
) -> Result<(), ServiceError> {
    validar_identidad(identidad)?;
    carrera_repo::guardar_identidad(conn, identidad)?;
    Ok(())
}

/// Resumen con badges calculados.
pub fn resumen(conn: &Connection) -> Result<CarreraResumen, ServiceError> {
    let mut resumen = carrera_repo::resumen(conn)?;
    let (hat_tricks, rendimiento_perfecto, recuperaciones, racha5) =
        carrera_repo::metricas_badges(conn)?;
    resumen.badges = badges(
        &resumen,
        hat_tricks,
        rendimiento_perfecto,
        recuperaciones,
        racha5,
    );
    Ok(resumen)
}

fn badges(
    resumen: &CarreraResumen,
    hat_tricks: i64,
    rendimiento_perfecto: i64,
    recuperaciones: i64,
    racha5: i64,
) -> Vec<CarreraBadge> {
    fn badge(nombre: &str, descripcion: &str, obtenido: bool) -> CarreraBadge {
        CarreraBadge {
            nombre: nombre.to_string(),
            descripcion: descripcion.to_string(),
            obtenido,
        }
    }
    vec![
        badge(
            "Primer partido",
            "Jugar al menos 1 partido",
            resumen.partidos >= 1,
        ),
        badge("Primer gol", "Convertir al menos 1 gol", resumen.goles >= 1),
        badge("Hat-trick", "3 goles en un partido", hat_tricks > 0),
        badge("10 partidos", "Jugar 10 partidos", resumen.partidos >= 10),
        badge("50 partidos", "Jugar 50 partidos", resumen.partidos >= 50),
        badge("100 goles", "Convertir 100 goles", resumen.goles >= 100),
        badge("Campeón", "Ganar un torneo", resumen.titulos >= 1),
        badge("Múltiple campeón", "Ganar 3 torneos", resumen.titulos >= 3),
        badge("Podio", "3 podios en torneos", resumen.podios >= 3),
        badge(
            "Temporadas",
            "Registrar una temporada",
            resumen.temporadas >= 1,
        ),
        badge("Racha de 5", "5 victorias consecutivas", racha5 > 0),
        badge(
            "Mejor partido",
            "Rendimiento perfecto (10)",
            rendimiento_perfecto > 0,
        ),
        badge("Recuperación", "Volver de una lesión", recuperaciones > 0),
    ]
}

/// Lista hitos.
pub fn hitos(conn: &Connection) -> Result<Vec<HitoRow>, ServiceError> {
    Ok(carrera_repo::hitos(conn)?)
}

/// Obtiene un hito.
pub fn hito(conn: &Connection, id: i64) -> Result<HitoRow, ServiceError> {
    Ok(carrera_repo::hito(conn, id)?)
}

/// Crea un hito.
pub fn crear_hito(
    conn: &Connection,
    partido_id: i64,
    tipo_hito: &str,
    nota: &str,
) -> Result<i64, ServiceError> {
    validar_hito(partido_id, tipo_hito, nota)?;
    if carrera_repo::existe_hito_partido(conn, partido_id, None)? {
        return Err(ServiceError::HitoDuplicado);
    }
    Ok(carrera_repo::insertar_hito(
        conn, partido_id, tipo_hito, nota,
    )?)
}

/// Actualiza un hito.
pub fn actualizar_hito(
    conn: &Connection,
    id: i64,
    partido_id: i64,
    tipo_hito: &str,
    nota: &str,
) -> Result<(), ServiceError> {
    validar_hito(partido_id, tipo_hito, nota)?;
    carrera_repo::hito(conn, id)?;
    if carrera_repo::existe_hito_partido(conn, partido_id, Some(id))? {
        return Err(ServiceError::HitoDuplicado);
    }
    carrera_repo::actualizar_hito(conn, id, partido_id, tipo_hito, nota)?;
    Ok(())
}

/// Elimina un hito.
pub fn eliminar_hito(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    carrera_repo::eliminar_hito(conn, id)?;
    Ok(())
}

fn validar_hito(partido_id: i64, tipo_hito: &str, nota: &str) -> Result<(), DomainError> {
    validation::positive(partido_id, "partido")?;
    validation::required(tipo_hito, "tipo de hito")?;
    validation::max_len(tipo_hito, "tipo de hito", 100)?;
    validation::max_len(nota, "nota", 300)?;
    Ok(())
}

/// Línea de tiempo.
pub fn timeline(conn: &Connection) -> Result<Vec<TimelineEvento>, ServiceError> {
    Ok(carrera_repo::timeline(conn)?)
}

/// Hall of Fame.
pub fn hall_of_fame(conn: &Connection) -> Result<CarreraHall, ServiceError> {
    Ok(carrera_repo::hall_of_fame(conn)?)
}

/// Perfil dinámico.
pub fn perfil(conn: &Connection) -> Result<CarreraPerfil, ServiceError> {
    Ok(carrera_repo::perfil(conn)?)
}

/// Recuerdos del mismo día en años anteriores.
pub fn retro(conn: &Connection) -> Result<Vec<crate::domain::models::HallPartido>, ServiceError> {
    Ok(carrera_repo::retro(conn)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn identidad_base() -> CarreraIdentidad {
        CarreraIdentidad {
            nombre: "Jugador".into(),
            edad: 30,
            pie_habil: "Derecho".into(),
            posiciones: "Mediocampista".into(),
            altura_cm: Some(180.0),
            peso_kg: Some(75.0),
            estilo: "Técnico".into(),
            dorsal_favorito: 10,
            objetivos: String::new(),
            historia: String::new(),
        }
    }

    fn preparar() -> (Database, i64) {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO cancha (nombre) VALUES ('Sur')", [])
            .unwrap();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
            .unwrap();
        for (fecha, goles, resultado, rendimiento) in [
            ("2026-03-01 20:00:00", 3, 1, 9),
            ("2026-03-08 20:00:00", 1, 1, 7),
            ("2026-03-15 20:00:00", 0, 3, 5),
            ("2026-03-22 20:00:00", 2, 1, 8),
            ("2026-03-29 20:00:00", 1, 1, 7),
            ("2026-04-05 20:00:00", 2, 1, 10),
        ] {
            db.conn()
                .execute(
                    "INSERT INTO partido
                        (cancha_id, camiseta_id, fecha_hora, mes_anio, goles, asistencias,
                         resultado, rendimiento_general, clima, dia, tipo_partido)
                     VALUES (1, 1, ?1, ?2, ?3, 1, ?4, ?5, 1, 2, 1)",
                    rusqlite::params![fecha, &fecha[..7], goles, resultado, rendimiento],
                )
                .unwrap();
        }
        let partido_id = db.conn().last_insert_rowid();
        (db, partido_id)
    }

    #[test]
    fn identidad_roundtrip_and_validation() {
        let db = Database::open_in_memory().unwrap();
        let ficha = identidad_base();
        guardar_identidad(db.conn(), &ficha).unwrap();
        let cargada = identidad(db.conn()).unwrap();
        assert_eq!(cargada.nombre, "Jugador");
        assert_eq!(cargada.dorsal_favorito, 10);

        let mut invalida = identidad_base();
        invalida.edad = 200;
        assert!(validar_identidad(&invalida).is_err());
        let mut pie = identidad_base();
        pie.pie_habil = "Mano".into();
        assert!(validar_identidad(&pie).is_err());
    }

    #[test]
    fn hitos_reject_duplicates() {
        let (db, partido_id) = preparar();
        crear_hito(db.conn(), partido_id, "Primer gol", "Debut").unwrap();
        assert!(matches!(
            crear_hito(db.conn(), partido_id, "Hat-trick", ""),
            Err(ServiceError::HitoDuplicado)
        ));
        let hitos = hitos(db.conn()).unwrap();
        assert_eq!(hitos.len(), 1);
        assert_eq!(hitos[0].tipo_hito, "Primer gol");
    }

    #[test]
    fn summary_and_badges() {
        let (db, _) = preparar();
        let resumen = resumen(db.conn()).unwrap();
        assert_eq!(resumen.partidos, 6);
        assert_eq!(resumen.goles, 9);
        assert_eq!(resumen.victorias, 5);
        assert_eq!(resumen.derrotas, 1);
        assert_eq!(resumen.mejor_racha, 3);
        let hat_trick = resumen
            .badges
            .iter()
            .find(|badge| badge.nombre == "Hat-trick")
            .unwrap();
        assert!(hat_trick.obtenido);
        let cien = resumen
            .badges
            .iter()
            .find(|badge| badge.nombre == "100 goles")
            .unwrap();
        assert!(!cien.obtenido);
    }

    #[test]
    fn timeline_hall_and_profile() {
        let (db, _) = preparar();
        let eventos = timeline(db.conn()).unwrap();
        assert!(eventos.len() >= 6);
        assert!(eventos[0].fecha >= eventos[1].fecha);

        let hall = hall_of_fame(db.conn()).unwrap();
        assert_eq!(hall.partidos.len(), 5);
        assert_eq!(hall.partidos[0].rendimiento, 9);

        let perfil = perfil(db.conn()).unwrap();
        assert!(!perfil.etiqueta.is_empty());
    }
}
