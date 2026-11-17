//! Persistencia de Bienestar. Las tablas y columnas salen de
//! [`Seccion::columnas`] (listas cerradas): nunca se interpola texto del usuario.

use rusqlite::types::Value;
use rusqlite::{Connection, params, params_from_iter};

use super::error::DbError;
use crate::domain::bienestar::{EntradasAsistente, Recomendacion, Registro, Seccion, TipoDato};
use crate::domain::dates;

fn lista_columnas(seccion: Seccion) -> String {
    seccion
        .columnas()
        .iter()
        .map(|columna| columna.clave)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Valor de BD a texto de pantalla/formulario.
fn a_texto(tipo: TipoDato, valor: Value) -> String {
    match (tipo, valor) {
        (_, Value::Null) => String::new(),
        (TipoDato::Booleano, Value::Integer(n)) => if n != 0 { "Sí" } else { "No" }.to_string(),
        (TipoDato::Fecha, Value::Text(texto)) => {
            dates::parse_date(&texto).map_or(texto, dates::display_date)
        }
        (_, Value::Integer(n)) => n.to_string(),
        (_, Value::Real(n)) => format!("{n}"),
        (_, Value::Text(texto)) => texto,
        (_, Value::Blob(_)) => String::new(),
    }
}

/// Registros de una sección, más recientes primero.
pub fn list(conn: &Connection, seccion: Seccion) -> Result<Vec<Registro>, DbError> {
    let orden = if seccion == Seccion::Salud {
        "id"
    } else {
        "fecha DESC, id DESC"
    };
    let sql = format!(
        "SELECT id, {} FROM {} ORDER BY {orden}",
        lista_columnas(seccion),
        seccion.tabla()
    );
    let mut stmt = conn.prepare(&sql)?;
    let columnas = seccion.columnas();
    let filas = stmt.query_map([], |row| {
        let mut valores = Vec::with_capacity(columnas.len());
        for (indice, columna) in columnas.iter().enumerate() {
            valores.push(a_texto(columna.tipo, row.get(indice + 1)?));
        }
        Ok(Registro {
            id: row.get(0)?,
            valores,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Inserta un registro (valores en el orden de las columnas).
pub fn insert(conn: &Connection, seccion: Seccion, valores: &[Value]) -> Result<i64, DbError> {
    let marcas = (1..=valores.len())
        .map(|indice| format!("?{indice}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({marcas})",
        seccion.tabla(),
        lista_columnas(seccion)
    );
    conn.execute(&sql, params_from_iter(valores))?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza un registro.
pub fn update(
    conn: &Connection,
    seccion: Seccion,
    id: i64,
    valores: &[Value],
) -> Result<(), DbError> {
    let asignaciones = seccion
        .columnas()
        .iter()
        .enumerate()
        .map(|(indice, columna)| format!("{} = ?{}", columna.clave, indice + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE {} SET {asignaciones} WHERE id = ?{}",
        seccion.tabla(),
        valores.len() + 1
    );
    let mut todos = valores.to_vec();
    todos.push(Value::Integer(id));
    let cambios = conn.execute(&sql, params_from_iter(todos))?;
    if cambios == 0 {
        return Err(DbError::NotFound {
            entidad: "bienestar",
            id,
        });
    }
    Ok(())
}

/// Borra un registro.
pub fn delete(conn: &Connection, seccion: Seccion, id: i64) -> Result<(), DbError> {
    conn.execute(
        &format!("DELETE FROM {} WHERE id = ?1", seccion.tabla()),
        params![id],
    )?;
    Ok(())
}

/// Alterna `omitido` de un entrenamiento.
pub fn alternar_omitido(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE bienestar_entrenamiento SET omitido = 1 - COALESCE(omitido, 0) WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// Promedios recientes que alimentan el asistente. `hoy` es ISO `YYYY-MM-DD`.
pub fn entradas_asistente(conn: &Connection, hoy: &str) -> Result<EntradasAsistente, DbError> {
    let mut e = EntradasAsistente {
        rendimiento: conn.query_row(
            "SELECT COALESCE(AVG(rendimiento_general), 0.0) FROM
               (SELECT rendimiento_general FROM partido ORDER BY fecha_hora DESC, id DESC LIMIT 10)",
            [],
            |row| row.get(0),
        )?,
        lesiones: conn.query_row(
            "SELECT COUNT(*) FROM lesion WHERE estado = 'ACTIVA'",
            [],
            |row| row.get(0),
        )?,
        ..EntradasAsistente::default()
    };
    (
        e.dormi,
        e.hidratacion,
        e.alcohol,
        e.nervios,
        e.confianza_habito,
        e.motivacion_habito,
    ) = conn.query_row(
        "SELECT COALESCE(AVG(dormi_bien), 0.0), COALESCE(AVG(hidratacion), 0.0),
                COALESCE(AVG(alcohol), 0.0), COALESCE(AVG(nervios), 0.0),
                COALESCE(AVG(confianza), 0.0), COALESCE(AVG(motivacion), 0.0)
           FROM (SELECT * FROM bienestar_habito ORDER BY fecha DESC, id DESC LIMIT 14)",
        [],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        },
    )?;
    e.comida = conn.query_row(
        "SELECT COALESCE(AVG(CASE calidad WHEN 'Buena' THEN 3 WHEN 'Regular' THEN 2 ELSE 1 END), 0.0)
           FROM (SELECT calidad FROM bienestar_comida ORDER BY fecha DESC, id DESC LIMIT 14)",
        [],
        |row| row.get(0),
    )?;
    (e.entrenamientos, e.intensidad, e.duracion) = conn.query_row(
        "SELECT COUNT(*), COALESCE(AVG(intensidad), 0.0), COALESCE(AVG(duracion_min), 0.0)
           FROM (SELECT intensidad, duracion_min FROM bienestar_entrenamiento
                  WHERE COALESCE(omitido, 0) = 0 ORDER BY fecha DESC, id DESC LIMIT 14)",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    e.intensos_semana = conn.query_row(
        "SELECT COUNT(*) FROM bienestar_entrenamiento
          WHERE COALESCE(omitido, 0) = 0 AND intensidad >= 8 AND fecha >= date(?1, '-6 day')
            AND fecha <= ?1",
        params![hoy],
        |row| row.get(0),
    )?;
    (
        e.confianza,
        e.motivacion,
        e.concentracion,
        e.estres,
        e.presion,
    ) = conn.query_row(
        "SELECT COALESCE(AVG(confianza), 0.0), COALESCE(AVG(motivacion), 0.0),
                COALESCE(AVG(concentracion), 0.0), COALESCE(AVG(estres), 0.0),
                COALESCE(AVG(presion), 0.0)
           FROM (SELECT * FROM bienestar_sesion_mental ORDER BY fecha DESC, id DESC LIMIT 10)",
        [],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )?;
    Ok(e)
}

/// Guarda una recomendación del asistente.
pub fn insert_recomendacion(
    conn: &Connection,
    fecha: &str,
    preparacion: i64,
    riesgo: i64,
    resumen: &str,
    rutina: &str,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO bienestar_recomendacion
            (fecha, score_preparacion, riesgo_lesion, resumen, rutina)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![fecha, preparacion, riesgo, resumen, rutina],
    )?;
    Ok(())
}

/// Últimas recomendaciones guardadas.
pub fn recomendaciones(conn: &Connection, limite: i64) -> Result<Vec<Recomendacion>, DbError> {
    let mut stmt = conn
        .prepare("SELECT fecha, resumen FROM bienestar_recomendacion ORDER BY id DESC LIMIT ?1")?;
    let filas = stmt.query_map(params![limite], |row| {
        Ok(Recomendacion {
            fecha: row.get(0)?,
            resumen: row.get(1)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}
