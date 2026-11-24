//! Recordatorios y agenda (IMP-18).

use std::path::{Path, PathBuf};

use jiff::civil::Date;
use rusqlite::Connection;
use serde_json::{Value, json};
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
pub fn siguiente(fecha: Date, periodicidad: Periodicidad) -> Date {
    match periodicidad {
        Periodicidad::UnaVez => fecha,
        Periodicidad::Diaria => fecha + jiff::Span::new().days(1),
        Periodicidad::Semanal => fecha + jiff::Span::new().days(7),
        Periodicidad::Mensual => fecha
            .checked_add(jiff::Span::new().months(1))
            .unwrap_or_else(|_| dates::sumar_dias(fecha, 30)),
    }
}

/// Nombre del archivo de C (`Exportaciones/` al exportar, `Importaciones/` al importar).
pub const ARCHIVO: &str = "recordatorios.json";

/// Exporta en el formato de `recordatorios.c`: `{id, fecha "AAAA-MM-DD HH:MM",
/// nota, tematica, periodicidad, fecha_fin}` más `titulo` y `completado`, que C
/// ignora al leer. `fecha_fin` sale vacía: MiFutbolR no la guarda.
pub fn exportar(conn: &Connection, dir: &Path) -> Result<PathBuf, ServiceError> {
    let items: Vec<Value> = recordatorio_repo::list(conn)?
        .iter()
        .map(|r| {
            let fecha = r.fecha.strftime("%Y-%m-%d").to_string();
            json!({
                "id": r.id,
                "fecha": if r.hora.is_empty() { fecha } else { format!("{fecha} {}", r.hora) },
                "nota": r.nota,
                "tematica": r.tematica,
                "periodicidad": r.periodicidad.as_i64(),
                "fecha_fin": "",
                "titulo": r.titulo,
                "completado": r.completado,
            })
        })
        .collect();
    let io = |e: std::io::Error| ServiceError::Message(e.to_string());
    std::fs::create_dir_all(dir).map_err(io)?;
    let ruta = dir.join(ARCHIVO);
    let texto =
        serde_json::to_string_pretty(&items).map_err(|e| ServiceError::Message(e.to_string()))?;
    std::fs::write(&ruta, texto).map_err(io)?;
    Ok(ruta)
}

/// Importa el array de C. `reemplazar` borra antes los existentes; si no, se
/// suman con ids nuevos (en C, "Reemplazar" también fusionaba después y
/// duplicaba todo). Devuelve (importados, errores por elemento).
pub fn importar(
    conn: &mut Connection,
    ruta: &Path,
    reemplazar: bool,
) -> Result<(usize, Vec<String>), ServiceError> {
    let texto = std::fs::read_to_string(ruta)
        .map_err(|e| ServiceError::Message(format!("{}: {e}", ruta.display())))?;
    let Ok(Value::Array(items)) =
        serde_json::from_str::<Value>(texto.trim_start_matches('\u{feff}'))
    else {
        return Err(ServiceError::Message(
            "El archivo no contiene un array JSON de recordatorios".to_string(),
        ));
    };
    let mut nuevos = Vec::new();
    let mut errores = Vec::new();
    for (indice, item) in items.iter().enumerate() {
        match desde_json(item) {
            Ok(r) => nuevos.push(r),
            Err(error) => errores.push(format!("elemento {}: {error}", indice + 1)),
        }
    }
    let tx = conn.transaction().map_err(DbError::from)?;
    tx.execute("UPDATE undo_control SET suspendido = 1 WHERE id = 1", [])
        .map_err(DbError::from)?;
    if reemplazar {
        tx.execute("DELETE FROM recordatorio", [])
            .map_err(DbError::from)?;
    }
    for r in &nuevos {
        recordatorio_repo::insert(&tx, r)?;
    }
    tx.execute("UPDATE undo_control SET suspendido = 0 WHERE id = 1", [])
        .map_err(DbError::from)?;
    tx.commit().map_err(DbError::from)?;
    Ok((nuevos.len(), errores))
}

/// Un elemento de C. Sin `titulo` (C no lo tiene) se usa la temática o la nota.
fn desde_json(item: &Value) -> Result<Recordatorio, String> {
    if !item.is_object() {
        return Err("no es un objeto".to_string());
    }
    let texto = |clave: &str| item[clave].as_str().unwrap_or("").trim().to_string();
    let crudo = texto("fecha");
    let fecha = dates::parse_datetime(&crudo).ok_or_else(|| format!("fecha inválida «{crudo}»"))?;
    let nota = texto("nota");
    let tematica = texto("tematica");
    let titulo = [
        texto("titulo"),
        tematica.clone(),
        nota.chars().take(60).collect(),
    ]
    .into_iter()
    .find(|t| !t.is_empty())
    .unwrap_or_else(|| "Recordatorio".to_string());
    Ok(Recordatorio {
        id: 0,
        titulo,
        nota,
        fecha: fecha.date(),
        hora: if crudo.contains(':') {
            fecha.strftime("%H:%M").to_string()
        } else {
            String::new()
        },
        periodicidad: Periodicidad::from_i64(item["periodicidad"].as_i64().unwrap_or(0)),
        tematica,
        completado: item["completado"].as_bool().unwrap_or(false),
        ultima_gen: String::new(),
    })
}

/// Temáticas distintas (sin distinguir mayúsculas), en orden alfabético.
#[must_use]
pub fn tematicas(items: &[Recordatorio]) -> Vec<String> {
    let mut temas: Vec<String> = Vec::new();
    for item in items {
        let tema = item.tematica.trim();
        if !tema.is_empty()
            && !temas
                .iter()
                .any(|t| t.to_lowercase() == tema.to_lowercase())
        {
            temas.push(tema.to_string());
        }
    }
    temas.sort_by_key(|t| t.to_lowercase());
    temas
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
    fn exporta_e_importa_el_formato_de_c() {
        let mut db = Database::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join(ARCHIVO);
        std::fs::write(
            &ruta,
            r#"[{"id":1,"fecha":"2030-05-10 18:30","nota":"Llevar canilleras","tematica":"Partido","periodicidad":2,"fecha_fin":""},
                {"id":2,"fecha":"25/12/2030","nota":"","tematica":"","periodicidad":0},
                {"id":3,"fecha":"mañana"}, 5]"#,
        )
        .unwrap();
        let (importados, errores) = importar(db.conn_mut(), &ruta, false).unwrap();
        assert_eq!(importados, 2);
        assert_eq!(
            errores,
            [
                "elemento 3: fecha inválida «mañana»",
                "elemento 4: no es un objeto"
            ]
        );
        let lista = recordatorio_repo::list(db.conn()).unwrap();
        let partido = lista.iter().find(|r| r.tematica == "Partido").unwrap();
        assert_eq!(partido.titulo, "Partido");
        assert_eq!(partido.hora, "18:30");
        assert_eq!(partido.periodicidad, Periodicidad::Semanal);
        let navidad = lista.iter().find(|r| r.tematica.is_empty()).unwrap();
        assert_eq!(
            (navidad.titulo.as_str(), navidad.hora.as_str()),
            ("Recordatorio", "")
        );
        // Fusionar suma; reemplazar deja solo lo importado (C duplicaba en ambos casos).
        importar(db.conn_mut(), &ruta, false).unwrap();
        assert_eq!(recordatorio_repo::list(db.conn()).unwrap().len(), 4);
        importar(db.conn_mut(), &ruta, true).unwrap();
        assert_eq!(recordatorio_repo::list(db.conn()).unwrap().len(), 2);
        let exportado = exportar(db.conn(), dir.path()).unwrap();
        let valor: Value =
            serde_json::from_str(&std::fs::read_to_string(exportado).unwrap()).unwrap();
        let fechas: Vec<&str> = valor
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["fecha"].as_str().unwrap())
            .collect();
        assert!(fechas.contains(&"2030-05-10 18:30"), "{fechas:?}");
        assert!(fechas.contains(&"2030-12-25"), "{fechas:?}");
        assert_eq!(
            tematicas(&recordatorio_repo::list(db.conn()).unwrap()),
            ["Partido"]
        );
    }

    #[test]
    fn advances_daily_and_monthly() {
        let base = crate::domain::dates::fecha(2026, 1, 31).unwrap();
        assert_eq!(
            siguiente(base, Periodicidad::Diaria),
            base + jiff::Span::new().days(1)
        );
        assert_eq!(
            siguiente(base, Periodicidad::Semanal),
            base + jiff::Span::new().days(7)
        );
        assert_eq!(
            siguiente(base, Periodicidad::Mensual),
            crate::domain::dates::fecha(2026, 2, 28).unwrap()
        );
    }

    #[test]
    fn recurrence_is_materialized() {
        let db = Database::open_in_memory().unwrap();
        let ayer = dates::today() - jiff::Span::new().days(2);
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
