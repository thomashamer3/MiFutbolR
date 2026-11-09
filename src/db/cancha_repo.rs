//! Repositorio de canchas (§9.3). Incluye tablas hijas de tipos y superficies.

use rusqlite::{Connection, Row, Transaction, params};

use super::error::DbError;
use crate::domain::Cents;
use crate::domain::cancha_codes::{
    decode_superficies, decode_tipos, encode_superficies, encode_tipos,
};
use crate::domain::models::{
    Cancha, CanchaDependencias, CanchaFilter, CanchaRow, ServiciosCancha, Superficie,
    TechadaEstado, TipoCancha,
};

/// Columnas base de `cancha` en el orden de [`map_cancha`].
const COLUMNS: &str = "id, nombre, telefono, direccion, localidad, tipo_cancha_codigo,
    superficie_codigo, techada_estado_codigo, tiene_iluminacion, horario_apertura_min,
    horario_cierre_min, precio_hora_dia_centavos, precio_hora_noche_centavos,
    tiene_vestuarios, tiene_duchas, tiene_buffet, tiene_estacionamiento, cantidad_canchas,
    estado, descripcion, contacto_alt, latitud, longitud, tiene_grabacion, tiene_marcador,
    imagen_ruta, activa";

/// Lista canchas con agregados de partidos.
pub fn list(conn: &Connection, filter: &CanchaFilter) -> Result<Vec<CanchaRow>, DbError> {
    let sql = format!(
        "SELECT c.id, c.nombre, c.localidad, c.activa,
                COUNT(p.id) AS partidos,
                COALESCE(SUM(p.goles), 0) AS goles,
                COALESCE(SUM(p.asistencias), 0) AS asistencias,
                COALESCE(SUM(CASE WHEN p.resultado = 1 THEN 1 ELSE 0 END), 0) AS victorias,
                COALESCE(SUM(CASE WHEN p.resultado = 2 THEN 1 ELSE 0 END), 0) AS empates,
                COALESCE(SUM(CASE WHEN p.resultado = 3 THEN 1 ELSE 0 END), 0) AS derrotas,
                COALESCE(AVG(p.rendimiento_general), 0.0) AS rendimiento_promedio,
                c.imagen_ruta
           FROM cancha c
           LEFT JOIN partido p ON p.cancha_id = c.id
          WHERE (?1 = 0 OR c.activa = CASE ?1 WHEN 1 THEN 1 ELSE 0 END)
            AND (?2 = '' OR c.nombre LIKE '%' || ?2 || '%' COLLATE NOCASE
                          OR c.localidad LIKE '%' || ?2 || '%' COLLATE NOCASE)
          GROUP BY c.id
          ORDER BY {}",
        filter.orden.sql_order()
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(
            params![filter.estado.sql_code(), filter.texto.trim()],
            |row| {
                Ok(CanchaRow {
                    id: row.get(0)?,
                    nombre: row.get(1)?,
                    localidad: row.get(2)?,
                    activa: row.get::<_, i64>(3)? != 0,
                    partidos: row.get(4)?,
                    goles: row.get(5)?,
                    asistencias: row.get(6)?,
                    victorias: row.get(7)?,
                    empates: row.get(8)?,
                    derrotas: row.get(9)?,
                    rendimiento_promedio: row.get(10)?,
                    imagen_ruta: row.get(11)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Obtiene una cancha con sus tipos y superficies.
pub fn get(conn: &Connection, id: i64) -> Result<Cancha, DbError> {
    let sql = format!("SELECT {COLUMNS} FROM cancha WHERE id = ?1");
    let mut cancha: Cancha =
        conn.query_row(&sql, [id], map_cancha)
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    entidad: "Cancha",
                    id,
                },
                other => other.into(),
            })?;

    let tipos = child_tipos(conn, id)?;
    if !tipos.is_empty() {
        cancha.tipos = tipos;
    }
    let superficies = child_superficies(conn, id)?;
    if !superficies.is_empty() {
        cancha.superficies = superficies;
    }
    Ok(cancha)
}

/// Inserta una cancha y sus selecciones múltiples en una transacción.
pub fn insert(conn: &mut Connection, cancha: &Cancha) -> Result<i64, DbError> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO cancha
            (nombre, telefono, direccion, localidad, tipo_cancha_codigo, superficie_codigo,
             techada_estado_codigo, tiene_iluminacion, horario_apertura_min, horario_cierre_min,
             precio_hora_dia_centavos, precio_hora_noche_centavos, tiene_vestuarios,
             tiene_duchas, tiene_buffet, tiene_estacionamiento, cantidad_canchas, estado,
             descripcion, contacto_alt, latitud, longitud, tiene_grabacion, tiene_marcador,
             imagen_ruta, activa, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                 ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, datetime('now','localtime'))",
        params![
            cancha.nombre,
            cancha.telefono,
            cancha.direccion,
            cancha.localidad,
            encode_tipos(&cancha.tipos),
            encode_superficies(&cancha.superficies),
            cancha.techada.as_i64(),
            i64::from(cancha.tiene_iluminacion),
            cancha.horario_apertura_min.unwrap_or(-1),
            cancha.horario_cierre_min.unwrap_or(-1),
            cancha.precio_dia.0,
            cancha.precio_noche.0,
            i64::from(cancha.servicios.vestuarios),
            i64::from(cancha.servicios.duchas),
            i64::from(cancha.servicios.buffet),
            i64::from(cancha.servicios.estacionamiento),
            cancha.cantidad_canchas,
            cancha.estado,
            cancha.descripcion,
            cancha.contacto_alt,
            cancha.latitud,
            cancha.longitud,
            i64::from(cancha.tiene_grabacion),
            cancha.tiene_marcador.as_i64(),
            cancha.imagen_ruta,
            i64::from(cancha.activa),
        ],
    )?;
    let id = tx.last_insert_rowid();
    replace_children(&tx, id, cancha)?;
    tx.commit()?;
    Ok(id)
}

/// Actualiza una cancha y sus selecciones múltiples en una transacción.
pub fn update(conn: &mut Connection, cancha: &Cancha) -> Result<(), DbError> {
    let tx = conn.transaction()?;
    let updated = tx.execute(
        "UPDATE cancha
            SET nombre = ?1, telefono = ?2, direccion = ?3, localidad = ?4,
                tipo_cancha_codigo = ?5, superficie_codigo = ?6, techada_estado_codigo = ?7,
                tiene_iluminacion = ?8, horario_apertura_min = ?9, horario_cierre_min = ?10,
                precio_hora_dia_centavos = ?11, precio_hora_noche_centavos = ?12,
                tiene_vestuarios = ?13, tiene_duchas = ?14, tiene_buffet = ?15,
                tiene_estacionamiento = ?16, cantidad_canchas = ?17, estado = ?18,
                descripcion = ?19, contacto_alt = ?20, latitud = ?21, longitud = ?22,
                tiene_grabacion = ?23, tiene_marcador = ?24, imagen_ruta = ?25, activa = ?26,
                updated_at = datetime('now','localtime')
          WHERE id = ?27",
        params![
            cancha.nombre,
            cancha.telefono,
            cancha.direccion,
            cancha.localidad,
            encode_tipos(&cancha.tipos),
            encode_superficies(&cancha.superficies),
            cancha.techada.as_i64(),
            i64::from(cancha.tiene_iluminacion),
            cancha.horario_apertura_min.unwrap_or(-1),
            cancha.horario_cierre_min.unwrap_or(-1),
            cancha.precio_dia.0,
            cancha.precio_noche.0,
            i64::from(cancha.servicios.vestuarios),
            i64::from(cancha.servicios.duchas),
            i64::from(cancha.servicios.buffet),
            i64::from(cancha.servicios.estacionamiento),
            cancha.cantidad_canchas,
            cancha.estado,
            cancha.descripcion,
            cancha.contacto_alt,
            cancha.latitud,
            cancha.longitud,
            i64::from(cancha.tiene_grabacion),
            cancha.tiene_marcador.as_i64(),
            cancha.imagen_ruta,
            i64::from(cancha.activa),
            cancha.id,
        ],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Cancha",
            id: cancha.id,
        });
    }
    replace_children(&tx, cancha.id, cancha)?;
    tx.commit()?;
    Ok(())
}

/// Activa/desactiva una cancha.
pub fn set_active(conn: &Connection, id: i64, active: bool) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE cancha SET activa = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
        params![i64::from(active), id],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Cancha",
            id,
        });
    }
    Ok(())
}

/// Elimina una cancha.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    let deleted = conn.execute("DELETE FROM cancha WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(DbError::NotFound {
            entidad: "Cancha",
            id,
        });
    }
    Ok(())
}

/// Dependencias que impiden borrar una cancha.
pub fn dependencias(conn: &Connection, id: i64) -> Result<CanchaDependencias, DbError> {
    let partidos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM partido WHERE cancha_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    Ok(CanchaDependencias { partidos })
}

fn replace_children(tx: &Transaction<'_>, id: i64, cancha: &Cancha) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM cancha_tipo WHERE cancha_id = ?1", [id])?;
    tx.execute("DELETE FROM cancha_superficie WHERE cancha_id = ?1", [id])?;
    for tipo in &cancha.tipos {
        tx.execute(
            "INSERT OR IGNORE INTO cancha_tipo (cancha_id, tipo_codigo) VALUES (?1, ?2)",
            params![id, tipo.as_i64()],
        )?;
    }
    for superficie in &cancha.superficies {
        if superficie.as_i64() == 0 {
            continue;
        }
        tx.execute(
            "INSERT OR IGNORE INTO cancha_superficie (cancha_id, superficie_codigo) VALUES (?1, ?2)",
            params![id, superficie.as_i64()],
        )?;
    }
    Ok(())
}

fn child_tipos(conn: &Connection, id: i64) -> Result<Vec<TipoCancha>, DbError> {
    let mut stmt = conn
        .prepare("SELECT tipo_codigo FROM cancha_tipo WHERE cancha_id = ?1 ORDER BY tipo_codigo")?;
    let codes = stmt
        .query_map([id], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(codes
        .into_iter()
        .filter_map(|code| TipoCancha::try_from(code).ok())
        .collect())
}

fn child_superficies(conn: &Connection, id: i64) -> Result<Vec<Superficie>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT superficie_codigo FROM cancha_superficie WHERE cancha_id = ?1 ORDER BY superficie_codigo",
    )?;
    let codes = stmt
        .query_map([id], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(codes
        .into_iter()
        .filter_map(|code| Superficie::try_from(code).ok())
        .collect())
}

fn map_cancha(row: &Row<'_>) -> rusqlite::Result<Cancha> {
    let tipos_cache: i64 = row.get(5)?;
    let superficies_cache: i64 = row.get(6)?;
    let techada: i64 = row.get(7)?;
    let marcador: i64 = row.get(24)?;
    let apertura: i64 = row.get(9)?;
    let cierre: i64 = row.get(10)?;
    Ok(Cancha {
        id: row.get(0)?,
        nombre: row.get(1)?,
        telefono: row.get(2)?,
        direccion: row.get(3)?,
        localidad: row.get(4)?,
        tipos: decode_tipos(tipos_cache),
        superficies: decode_superficies(superficies_cache),
        techada: TechadaEstado::try_from(techada).unwrap_or(TechadaEstado::NoSe),
        tiene_iluminacion: row.get::<_, i64>(8)? != 0,
        horario_apertura_min: if apertura < 0 { None } else { Some(apertura) },
        horario_cierre_min: if cierre < 0 { None } else { Some(cierre) },
        precio_dia: Cents(row.get(11)?),
        precio_noche: Cents(row.get(12)?),
        servicios: ServiciosCancha {
            vestuarios: row.get::<_, i64>(13)? != 0,
            duchas: row.get::<_, i64>(14)? != 0,
            buffet: row.get::<_, i64>(15)? != 0,
            estacionamiento: row.get::<_, i64>(16)? != 0,
        },
        cantidad_canchas: row.get(17)?,
        estado: row.get(18)?,
        descripcion: row.get(19)?,
        contacto_alt: row.get(20)?,
        latitud: row.get(21)?,
        longitud: row.get(22)?,
        tiene_grabacion: row.get::<_, i64>(23)? != 0,
        tiene_marcador: TechadaEstado::try_from(marcador).unwrap_or(TechadaEstado::NoSe),
        imagen_ruta: row.get(25)?,
        activa: row.get::<_, i64>(26)? != 0,
    })
}
