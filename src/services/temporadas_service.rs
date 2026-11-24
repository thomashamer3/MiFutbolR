//! Reglas de temporadas (IMP-12).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::temporada_repo;
use crate::domain::reporte::Reporte;
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
    let tx = conn.unchecked_transaction().map_err(DbError::from)?;
    let id = temporada_repo::insert(
        &tx,
        &ficha.nombre,
        ficha.anio,
        ficha.fecha_inicio,
        ficha.fecha_fin,
        &ficha.descripcion,
    )?;
    temporada_repo::insertar_fases_default(&tx, id, ficha.fecha_inicio, ficha.fecha_fin)?;
    temporada_repo::asociar_todos_torneos(&tx, id)?;
    tx.commit().map_err(DbError::from)?;
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

/// "Asociar Torneo a Temporada" de `temporada.c` (allá se llamaba con id 0 y no
/// asociaba nada): suma los torneos creados después de la temporada.
pub fn asociar_torneos(conn: &Connection, id: i64) -> Result<usize, ServiceError> {
    Ok(temporada_repo::asociar_todos_torneos(conn, id)?)
}

/// "Generar/Exportar Resumen de Temporada" de `temporada.c` (x en el reporte lo
/// guarda como TXT).
#[allow(clippy::cast_precision_loss)]
pub fn reporte(conn: &Connection, id: i64) -> Result<Reporte, ServiceError> {
    let f = ficha(conn, id)?;
    let t = &f.temporada;
    let s = &f.resumen;
    let mut r = Reporte::new(format!("Resumen de temporada: {}", t.nombre));
    r.linea(format!(
        "{} · {} al {} · {}",
        t.anio,
        crate::domain::dates::display_date(t.fecha_inicio),
        crate::domain::dates::display_date(t.fecha_fin),
        t.estado
    ));
    if !t.descripcion.is_empty() {
        r.linea(t.descripcion.clone());
    }
    r.seccion("Estadísticas generales");
    let promedio = if s.partidos > 0 {
        s.goles as f64 / s.partidos as f64
    } else {
        0.0
    };
    r.linea(format!("Partidos jugados: {}", s.partidos));
    r.linea(format!(
        "Goles totales: {} | asistencias: {}",
        s.goles, s.asistencias
    ));
    r.linea(format!("Promedio de goles por partido: {promedio:.2}"));
    r.linea(format!(
        "Victorias: {} | empates: {} | derrotas: {} | winrate: {:.1}%",
        s.victorias, s.empates, s.derrotas, s.winrate
    ));
    r.linea(format!("Rendimiento promedio: {:.2}", s.rendimiento));
    r.linea(format!("Lesiones registradas: {}", s.lesiones));
    r.linea(format!("Fatiga: {:.0}", f.fatiga));
    r.seccion("Fases");
    for fase in &f.fases {
        r.linea(format!(
            "{} ({}): {} a {}",
            fase.nombre, fase.tipo, fase.fecha_inicio, fase.fecha_fin
        ));
    }
    r.seccion("Torneos asociados");
    if f.torneos.is_empty() {
        r.linea("Sin torneos asociados (t en el detalle de la temporada).");
    }
    for torneo in &f.torneos {
        r.linea(torneo.clone());
    }
    Ok(r)
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
            fecha_inicio: crate::domain::dates::fecha(anio as i32, 1, 1).unwrap(),
            fecha_fin: crate::domain::dates::fecha(anio as i32, 12, 31).unwrap(),
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

        let texto = reporte(db.conn(), id).unwrap().texto();
        assert!(texto.contains("Partidos jugados: 1"), "{texto}");
        assert!(texto.contains("Promedio de goles por partido: 2.00"));
        assert!(texto.contains("Sin torneos asociados"));
        db.conn()
            .execute("INSERT INTO torneo (nombre) VALUES ('Copa')", [])
            .unwrap();
        assert_eq!(asociar_torneos(db.conn(), id).unwrap(), 1);
        assert_eq!(asociar_torneos(db.conn(), id).unwrap(), 0, "sin duplicar");
        assert!(reporte(db.conn(), id).unwrap().texto().contains("Copa"));

        let otra = crear(db.conn(), &temporada(0, "2025", 2025)).unwrap();
        let comparacion = comparar(db.conn(), id, otra).unwrap();
        assert_eq!(comparacion.filas.len(), 10);
        assert_eq!(comparacion.a, "2026");
    }
}
