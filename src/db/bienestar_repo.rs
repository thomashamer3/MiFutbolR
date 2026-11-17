//! Persistencia de Bienestar. Las tablas y columnas salen de
//! [`Seccion::columnas`] (listas cerradas): nunca se interpola texto del usuario.

use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use super::error::DbError;
use crate::domain::bienestar::{
    EntradasAsistente, Recomendacion, Referencia, Registro, Seccion, TipoDato,
};
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
    let tabla = seccion.tabla();
    let columnas = seccion.columnas();
    // Una etiqueta por columna: la de la fila apuntada para las referencias.
    let etiquetas = columnas
        .iter()
        .map(|columna| match columna.tipo {
            TipoDato::Referencia(referencia) => format!(
                "(SELECT {} FROM {} WHERE id = {tabla}.{})",
                referencia.etiqueta_sql(),
                referencia.tabla(),
                columna.clave
            ),
            _ => "NULL".to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT {}, {}, {etiquetas} FROM {tabla} ORDER BY {}",
        seccion.clave(),
        lista_columnas(seccion),
        seccion.orden()
    );
    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map([], |row| {
        let mut valores = Vec::with_capacity(columnas.len());
        let mut vista = Vec::with_capacity(columnas.len());
        for (indice, columna) in columnas.iter().enumerate() {
            let valor = a_texto(columna.tipo, row.get(indice + 1)?);
            let etiqueta: Option<String> = row.get(indice + 1 + columnas.len())?;
            vista.push(etiqueta.unwrap_or_else(|| valor.clone()));
            valores.push(valor);
        }
        Ok(Registro {
            id: row.get(0)?,
            valores,
            vista,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Opciones `(id, etiqueta)` para elegir un registro referenciado.
pub fn opciones(
    conn: &Connection,
    referencia: Referencia,
) -> Result<Vec<(String, String)>, DbError> {
    let sql = format!(
        "SELECT id, {} FROM {} ORDER BY id DESC",
        referencia.etiqueta_sql(),
        referencia.tabla()
    );
    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?.to_string(),
            row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        ))
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Objetivo activo más reciente (por fecha de fin), como en MiFutbolC.
pub fn objetivo_activo(conn: &Connection) -> Result<Option<String>, DbError> {
    Ok(conn
        .query_row(
            "SELECT nombre FROM bienestar_objetivo WHERE estado = 'Activo'
              ORDER BY fecha_fin DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?)
}

/// Inserta un registro (valores en el orden de las columnas). En las secciones
/// con columna única, repetir el valor actualiza la fila existente.
pub fn insert(conn: &Connection, seccion: Seccion, valores: &[Value]) -> Result<i64, DbError> {
    let marcas = (1..=valores.len())
        .map(|indice| format!("?{indice}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut sql = format!(
        "INSERT INTO {} ({}) VALUES ({marcas})",
        seccion.tabla(),
        lista_columnas(seccion)
    );
    if let Some(unica) = seccion.unica() {
        let asignaciones = seccion
            .columnas()
            .iter()
            .filter(|columna| columna.clave != unica)
            .map(|columna| format!("{0} = excluded.{0}", columna.clave))
            .collect::<Vec<_>>()
            .join(", ");
        sql.push_str(&format!(
            " ON CONFLICT({unica}) DO UPDATE SET {asignaciones}"
        ));
    }
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
        "UPDATE {} SET {asignaciones} WHERE {} = ?{}",
        seccion.tabla(),
        seccion.clave(),
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
        &format!(
            "DELETE FROM {} WHERE {} = ?1",
            seccion.tabla(),
            seccion.clave()
        ),
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
    (e.dia_hidratacion, e.dia_alcohol) = conn.query_row(
        "SELECT COALESCE(AVG(CASE hidratacion WHEN 'Alta' THEN 3 WHEN 'Media' THEN 2 ELSE 1 END), 0.0),
                COALESCE(AVG(alcohol), 0.0)
           FROM (SELECT hidratacion, alcohol FROM bienestar_dia_nutricional
                  ORDER BY fecha DESC LIMIT 14)",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
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
