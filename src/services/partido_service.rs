//! Reglas de negocio del módulo Partidos (§12, §21.3).
//!
//! Normaliza los modos (amistoso/torneo/entrenamiento), calcula derivados,
//! valida rangos y coherencia, y delega la transacción financiera al repo.

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::helpers::Page;
use crate::db::partido_repo;
use crate::domain::dates;
use crate::domain::models::{
    Arbitraje, FranjaHoraria, Partido, PartidoFilter, PartidoResumen, PartidoRow, Resultado,
    TipoPartido,
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
    /// Id inexistente.
    #[error("No existe el partido id={0}")]
    NotFound(i64),
    /// Falta un prerrequisito del alta.
    #[error("{0}")]
    Prerrequisito(String),
}

/// Normaliza los campos dependientes del tipo de partido.
pub fn normalizar(partido: &mut Partido) -> Result<(), DomainError> {
    if partido.tipo_partido == TipoPartido::Entrenamiento {
        partido.resultado = Resultado::NoAplica;
        partido.rival_nombre.clear();
        partido.goles_equipo = -1;
        partido.goles_rival = -1;
        partido.arbitraje_score = 0;
    } else if partido.goles_equipo >= 0 && partido.goles_rival >= 0 {
        let esperado = match partido.goles_equipo.cmp(&partido.goles_rival) {
            std::cmp::Ordering::Greater => Resultado::Victoria,
            std::cmp::Ordering::Equal => Resultado::Empate,
            std::cmp::Ordering::Less => Resultado::Derrota,
        };
        if partido.resultado == Resultado::NoAplica {
            partido.resultado = esperado;
        } else if partido.resultado != esperado {
            return Err(DomainError::Message(
                "El resultado no coincide con el marcador".to_string(),
            ));
        }
    }
    Ok(())
}

/// Calcula los campos derivados (§12.5).
pub fn derivar(partido: &mut Partido) -> Result<(), DomainError> {
    partido.mes_anio = dates::month_key(partido.fecha_hora);
    partido.dia = FranjaHoraria::try_from(dates::franja_code(partido.fecha_hora))?;
    partido.condicion_cancha = partido.estado_cancha.label().to_string();
    partido.arbitraje = if partido.arbitraje_score == 0 {
        "Sin árbitro".to_string()
    } else {
        Arbitraje::from_i64(partido.arbitraje_score)?
            .label()
            .to_string()
    };
    partido.diferencia_gol = if partido.tipo_partido == TipoPartido::Entrenamiento {
        0
    } else if partido.goles_equipo >= 0 && partido.goles_rival >= 0 {
        partido.goles_equipo - partido.goles_rival
    } else {
        0
    };
    Ok(())
}

/// Valida un partido normalizado y derivado.
pub fn validar(partido: &Partido) -> Result<(), DomainError> {
    if partido.cancha_id <= 0 {
        return Err(DomainError::Required("cancha"));
    }
    if partido.camiseta_id <= 0 {
        return Err(DomainError::Required("camiseta"));
    }
    validation::non_negative(partido.goles, "goles")?;
    validation::non_negative(partido.asistencias, "asistencias")?;
    validation::non_negative(partido.goles_en_contra, "goles en contra")?;
    validation::range_i64(partido.rendimiento_general, 1, 10, "rendimiento")?;
    validation::range_i64(partido.cansancio, 1, 10, "cansancio")?;
    validation::range_i64(partido.estado_animo, 1, 10, "estado de ánimo")?;
    validation::range_i64(partido.rating_tecnico, 1, 10, "rating técnico")?;
    validation::range_i64(partido.rating_fisico, 1, 10, "rating físico")?;
    validation::range_i64(partido.rating_mental, 1, 10, "rating mental")?;
    validation::range_i64(partido.intensidad, 1, 10, "intensidad")?;
    validation::range_i64(partido.minutos_jugados, 0, 180, "minutos jugados")?;
    validation::range_i64(partido.arbitraje_score, 0, 5, "arbitraje")?;
    validation::non_negative(partido.precio.0, "precio")?;
    validation::max_len(&partido.comentario_personal, "comentario", 256)?;
    validation::max_len(&partido.rival_nombre, "rival", 100)?;
    validation::max_len(&partido.posicion_jugada, "posición", 40)?;
    validation::max_len(&partido.eventos_clave, "eventos clave", 300)?;
    validation::max_len(&partido.lo_mejor, "lo mejor", 300)?;
    validation::max_len(&partido.que_mejorar, "qué mejorar", 300)?;
    validation::max_len(&partido.goles_detalle, "detalle de goles", 512)?;
    validation::max_len(&partido.asistencias_detalle, "detalle de asistencias", 512)?;
    let tags = partido.tags.join(", ");
    validation::max_len(&tags, "etiquetas", 300)?;
    if let Some(temperatura) = partido.temperatura_c {
        validation::range_f64(temperatura, -50.0, 60.0, "temperatura")?;
    }
    let year = chrono::Datelike::year(&partido.fecha_hora);
    validation::range_i64(i64::from(year), 1900, 2100, "fecha")?;
    if partido.tipo_partido != TipoPartido::Entrenamiento
        && partido.resultado == Resultado::NoAplica
    {
        return Err(DomainError::Message(
            "Indique el resultado del partido".to_string(),
        ));
    }
    Ok(())
}

/// Crea un partido aplicando normalización, derivados y validación.
pub fn crear(conn: &mut Connection, partido: &mut Partido) -> Result<i64, ServiceError> {
    normalizar(partido)?;
    derivar(partido)?;
    validar(partido)?;
    let id = partido_repo::insert(conn, partido)?;
    Ok(id)
}

/// Actualiza un partido completo.
pub fn actualizar(conn: &mut Connection, partido: &mut Partido) -> Result<(), ServiceError> {
    if partido.id <= 0 {
        return Err(ServiceError::NotFound(partido.id));
    }
    normalizar(partido)?;
    derivar(partido)?;
    validar(partido)?;
    partido_repo::update(conn, partido)?;
    Ok(())
}

/// Elimina un partido y su transacción automática.
pub fn eliminar(conn: &mut Connection, id: i64) -> Result<(), ServiceError> {
    partido_repo::delete(conn, id)?;
    Ok(())
}

/// Lista paginada.
pub fn listar(
    conn: &Connection,
    filter: &PartidoFilter,
    page: Page,
) -> Result<Vec<PartidoRow>, ServiceError> {
    Ok(partido_repo::list(conn, filter, page)?)
}

/// Conteo del filtro.
pub fn contar(conn: &Connection, filter: &PartidoFilter) -> Result<i64, ServiceError> {
    Ok(partido_repo::count(conn, filter)?)
}

/// Resumen agregado.
pub fn resumen(conn: &Connection, filter: &PartidoFilter) -> Result<PartidoResumen, ServiceError> {
    Ok(partido_repo::resumen(conn, filter)?)
}

/// Obtiene un partido.
pub fn obtener(conn: &Connection, id: i64) -> Result<Partido, ServiceError> {
    Ok(partido_repo::get(conn, id)?)
}

/// Alterna el favorito y devuelve el nuevo estado.
pub fn alternar_favorito(conn: &Connection, id: i64) -> Result<bool, ServiceError> {
    let partido = partido_repo::get(conn, id)?;
    let nuevo = !partido.favorito;
    partido_repo::set_favorito(conn, id, nuevo)?;
    Ok(nuevo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::domain::Cents;
    use crate::domain::models::{
        Atajaste, Clima, DolorFisico, EstadoCancha, FormatoPartido, Tarjeta, TipoPartido,
    };

    fn base() -> Partido {
        Partido {
            id: 0,
            cancha_id: 1,
            camiseta_id: 1,
            botin_id: None,
            fecha_hora: chrono::NaiveDate::from_ymd_opt(2026, 5, 10)
                .unwrap()
                .and_hms_opt(21, 30, 0)
                .unwrap(),
            mes_anio: String::new(),
            goles: 2,
            asistencias: 1,
            resultado: Resultado::Victoria,
            rendimiento_general: 7,
            cansancio: 6,
            estado_animo: 8,
            comentario_personal: String::new(),
            clima: Clima::Despejado,
            dia: FranjaHoraria::NoDefinida,
            precio: Cents(15_000),
            tipo_partido: TipoPartido::Amistoso,
            rival_nombre: "Rival FC".into(),
            tipo_rival: "Amistoso".into(),
            posicion_jugada: "Mediocampista".into(),
            minutos_jugados: 90,
            intensidad: 7,
            condicion_cancha: String::new(),
            arbitraje: String::new(),
            eventos_clave: String::new(),
            rating_tecnico: 7,
            rating_fisico: 6,
            rating_mental: 8,
            estado_cancha: EstadoCancha::Buena,
            goles_equipo: 3,
            goles_rival: 1,
            formato_partido: FormatoPartido::F5,
            tarjeta: Tarjeta::No,
            goles_en_contra: 0,
            dolor_fisico: DolorFisico::Ninguna,
            temperatura_c: Some(18.0),
            arbitraje_score: 3,
            lo_mejor: String::new(),
            que_mejorar: String::new(),
            tags: vec!["amistoso".into()],
            goles_detalle: String::new(),
            asistencias_detalle: String::new(),
            atajaste_todo_el_partido: Atajaste::NoEspecificado,
            diferencia_gol: 0,
            favorito: false,
        }
    }

    fn preparar() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO cancha (nombre) VALUES ('Cancha')", [])
            .unwrap();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Camiseta')", [])
            .unwrap();
        db
    }

    #[test]
    fn derives_and_validates_match() {
        let mut db = preparar();
        let mut partido = base();
        let id = crear(db.conn_mut(), &mut partido).unwrap();
        let guardado = partido_repo::get(db.conn(), id).unwrap();
        assert_eq!(guardado.mes_anio, "2026-05");
        assert_eq!(guardado.dia, FranjaHoraria::Noche);
        assert_eq!(guardado.diferencia_gol, 2);
        assert_eq!(guardado.condicion_cancha, "Buena");
        assert_eq!(guardado.arbitraje, "Normal");
    }

    #[test]
    fn incoherent_result_is_rejected() {
        let mut db = preparar();
        let mut partido = base();
        partido.resultado = Resultado::Empate;
        assert!(crear(db.conn_mut(), &mut partido).is_err());
    }

    #[test]
    fn training_normalizes_fields() {
        let mut db = preparar();
        let mut partido = base();
        partido.tipo_partido = TipoPartido::Entrenamiento;
        partido.resultado = Resultado::Victoria;
        partido.rival_nombre = "Rival".into();
        partido.arbitraje_score = 4;
        let id = crear(db.conn_mut(), &mut partido).unwrap();
        let guardado = partido_repo::get(db.conn(), id).unwrap();
        assert_eq!(guardado.resultado, Resultado::NoAplica);
        assert_eq!(guardado.rival_nombre, "");
        assert_eq!(guardado.goles_equipo, -1);
        assert_eq!(guardado.arbitraje_score, 0);
        assert_eq!(guardado.arbitraje, "Sin árbitro");
        assert_eq!(guardado.diferencia_gol, 0);
    }

    #[test]
    fn price_creates_and_syncs_financial_transaction() {
        let mut db = preparar();
        let mut partido = base();
        let id = crear(db.conn_mut(), &mut partido).unwrap();

        let (categoria, monto, partido_id): (i64, i64, i64) = db
            .conn()
            .query_row(
                "SELECT categoria, monto_centavos, partido_id FROM financiamiento WHERE partido_id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(categoria, 5);
        assert_eq!(monto, 15_000);
        assert_eq!(partido_id, id);

        // Actualizar el precio modifica la transacción.
        let mut guardado = partido_repo::get(db.conn(), id).unwrap();
        guardado.precio = Cents(20_000);
        actualizar(db.conn_mut(), &mut guardado).unwrap();
        let monto: i64 = db
            .conn()
            .query_row(
                "SELECT monto_centavos FROM financiamiento WHERE partido_id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(monto, 20_000);

        // Precio 0 borra la transacción automática.
        guardado.precio = Cents::ZERO;
        actualizar(db.conn_mut(), &mut guardado).unwrap();
        let cantidad: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM financiamiento WHERE partido_id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cantidad, 0);
    }

    #[test]
    fn delete_removes_dependencies() {
        let mut db = preparar();
        let mut partido = base();
        let id = crear(db.conn_mut(), &mut partido).unwrap();
        eliminar(db.conn_mut(), id).unwrap();
        let partidos: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM partido", [], |row| row.get(0))
            .unwrap();
        let finanzas: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM financiamiento", [], |row| row.get(0))
            .unwrap();
        assert_eq!(partidos, 0);
        assert_eq!(finanzas, 0);
    }

    #[test]
    fn list_filters_counts_and_summary() {
        let mut db = preparar();
        for (indice, (resultado, goles_equipo, goles_rival)) in
            [(Resultado::Victoria, 3, 1), (Resultado::Derrota, 1, 3)]
                .into_iter()
                .enumerate()
        {
            let mut partido = base();
            partido.resultado = resultado;
            partido.goles_equipo = goles_equipo;
            partido.goles_rival = goles_rival;
            partido.fecha_hora = chrono::NaiveDate::from_ymd_opt(2026, 5, 10 + indice as u32)
                .unwrap()
                .and_hms_opt(20, 0, 0)
                .unwrap();
            crear(db.conn_mut(), &mut partido).unwrap();
        }
        let filter = PartidoFilter::default();
        assert_eq!(contar(db.conn(), &filter).unwrap(), 2);
        let rows = listar(db.conn(), &filter, Page::ALL).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].fecha_hora >= rows[1].fecha_hora);

        let filtrado = PartidoFilter {
            resultado: Some(Resultado::Victoria),
            ..PartidoFilter::default()
        };
        assert_eq!(contar(db.conn(), &filtrado).unwrap(), 1);
        let resumen = resumen(db.conn(), &filter).unwrap();
        assert_eq!(resumen.total, 2);
        assert_eq!(resumen.victorias, 1);
        assert_eq!(resumen.derrotas, 1);
    }
}
