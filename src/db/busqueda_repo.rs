//! Búsqueda global por entidades (IMP-03).

use rusqlite::{Connection, params};

use super::error::DbError;
use super::helpers::like_pattern;
use crate::domain::busqueda::{ResultadoBusqueda, TipoResultado};
use crate::domain::dates;

const LIMITE_POR_TIPO: i64 = 6;

fn empujar(
    out: &mut Vec<ResultadoBusqueda>,
    tipo: TipoResultado,
    id: i64,
    titulo: String,
    subtitulo: String,
) {
    out.push(ResultadoBusqueda {
        tipo,
        id,
        titulo,
        subtitulo,
    });
}

/// Busca en todas las entidades principales.
pub fn buscar(
    conn: &Connection,
    texto: &str,
    limite: usize,
) -> Result<Vec<ResultadoBusqueda>, DbError> {
    let consulta = texto.trim();
    if consulta.is_empty() {
        return Ok(Vec::new());
    }
    let patron = like_pattern(consulta);
    let mut out = Vec::new();

    let mut stmt = conn.prepare(
        "SELECT p.id, p.rival_nombre, p.fecha_hora, c.nombre
           FROM partido p JOIN cancha c ON c.id = p.cancha_id
          WHERE p.rival_nombre LIKE ?1 COLLATE NOCASE ESCAPE '\\'
             OR p.comentario_personal LIKE ?1 COLLATE NOCASE ESCAPE '\\'
             OR p.tags LIKE ?1 COLLATE NOCASE ESCAPE '\\'
          ORDER BY p.fecha_hora DESC LIMIT ?2",
    )?;
    let filas = stmt.query_map(params![patron, LIMITE_POR_TIPO], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for fila in filas {
        let (id, rival, fecha, cancha) = fila?;
        let fecha = dates::parse_datetime(&fecha)
            .map(dates::display_datetime)
            .unwrap_or(fecha);
        let titulo = if rival.trim().is_empty() {
            format!("Partido #{id}")
        } else {
            format!("vs {rival}")
        };
        empujar(
            &mut out,
            TipoResultado::Partido,
            id,
            titulo,
            format!("{fecha} · {cancha}"),
        );
    }

    let mut stmt = conn.prepare(
        "SELECT id, nombre, marca FROM camiseta
          WHERE nombre LIKE ?1 COLLATE NOCASE ESCAPE '\\' OR marca LIKE ?1 COLLATE NOCASE ESCAPE '\\'
          ORDER BY nombre LIMIT ?2",
    )?;
    let filas = stmt.query_map(params![patron, LIMITE_POR_TIPO], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for fila in filas {
        let (id, nombre, marca) = fila?;
        empujar(
            &mut out,
            TipoResultado::Camiseta,
            id,
            nombre,
            if marca.is_empty() {
                "Camiseta".to_string()
            } else {
                marca
            },
        );
    }

    let mut stmt = conn.prepare(
        "SELECT id, nombre, localidad FROM cancha
          WHERE nombre LIKE ?1 COLLATE NOCASE ESCAPE '\\' OR localidad LIKE ?1 COLLATE NOCASE ESCAPE '\\'
          ORDER BY nombre LIMIT ?2",
    )?;
    let filas = stmt.query_map(params![patron, LIMITE_POR_TIPO], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for fila in filas {
        let (id, nombre, localidad) = fila?;
        empujar(
            &mut out,
            TipoResultado::Cancha,
            id,
            nombre,
            if localidad.is_empty() {
                "Cancha".to_string()
            } else {
                localidad
            },
        );
    }

    let mut stmt = conn.prepare(
        "SELECT id, nombre, tipo FROM equipo
          WHERE nombre LIKE ?1 COLLATE NOCASE ESCAPE '\\'
          ORDER BY nombre LIMIT ?2",
    )?;
    let filas = stmt.query_map(params![patron, LIMITE_POR_TIPO], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    for fila in filas {
        let (id, nombre, tipo) = fila?;
        empujar(
            &mut out,
            TipoResultado::Equipo,
            id,
            nombre,
            if tipo == 0 {
                "Equipo fijo"
            } else {
                "Equipo momentáneo"
            }
            .to_string(),
        );
    }

    let mut stmt = conn.prepare(
        "SELECT j.id, j.nombre, e.nombre FROM jugador j
           JOIN equipo e ON e.id = j.equipo_id
          WHERE j.nombre LIKE ?1 COLLATE NOCASE ESCAPE '\\'
          ORDER BY j.nombre LIMIT ?2",
    )?;
    let filas = stmt.query_map(params![patron, LIMITE_POR_TIPO], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for fila in filas {
        let (id, nombre, equipo) = fila?;
        empujar(&mut out, TipoResultado::Jugador, id, nombre, equipo);
    }

    let mut stmt = conn.prepare(
        "SELECT id, jugador, tipo, descripcion FROM lesion
          WHERE jugador LIKE ?1 COLLATE NOCASE ESCAPE '\\' OR tipo LIKE ?1 COLLATE NOCASE ESCAPE '\\'
             OR descripcion LIKE ?1 COLLATE NOCASE ESCAPE '\\'
          ORDER BY fecha DESC LIMIT ?2",
    )?;
    let filas = stmt.query_map(params![patron, LIMITE_POR_TIPO], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for fila in filas {
        let (id, jugador, tipo, descripcion) = fila?;
        empujar(
            &mut out,
            TipoResultado::Lesion,
            id,
            format!("{jugador}: {tipo}"),
            descripcion,
        );
    }

    let mut stmt = conn.prepare(
        "SELECT id, descripcion, fecha, monto_centavos FROM financiamiento
          WHERE descripcion LIKE ?1 COLLATE NOCASE ESCAPE '\\' OR item_especifico LIKE ?1 COLLATE NOCASE ESCAPE '\\'
          ORDER BY fecha DESC LIMIT ?2",
    )?;
    let filas = stmt.query_map(params![patron, LIMITE_POR_TIPO], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    for fila in filas {
        let (id, descripcion, fecha, monto) = fila?;
        empujar(
            &mut out,
            TipoResultado::Financiamiento,
            id,
            descripcion,
            format!("{fecha} · {}", crate::domain::Cents(monto).display_ars()),
        );
    }

    out.truncate(limite.max(1));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn finds_entities_across_tables() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute("INSERT INTO camiseta (nombre) VALUES ('Azul 2024')", [])
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO cancha (nombre, localidad) VALUES ('El Potrero', 'Lanús')",
                [],
            )
            .unwrap();
        let resultados = buscar(db.conn(), "azul", 20).unwrap();
        assert_eq!(resultados.len(), 1);
        assert_eq!(resultados[0].tipo, TipoResultado::Camiseta);
        assert!(
            buscar(db.conn(), "lanús", 20)
                .unwrap()
                .iter()
                .any(|r| r.tipo == TipoResultado::Cancha)
        );
        assert!(buscar(db.conn(), "   ", 20).unwrap().is_empty());
    }

    #[test]
    fn percent_and_underscore_are_literal() {
        let db = Database::open_in_memory().unwrap();
        for nombre in ["Rebaja 50%", "Modelo 500", "a_b", "axb"] {
            db.conn()
                .execute("INSERT INTO camiseta (nombre) VALUES (?1)", [nombre])
                .unwrap();
        }
        let nombres = |texto: &str| -> Vec<String> {
            buscar(db.conn(), texto, 20)
                .unwrap()
                .into_iter()
                .map(|r| r.titulo)
                .collect()
        };
        assert_eq!(nombres("50%"), ["Rebaja 50%"]);
        assert!(nombres("5%").is_empty(), "5% no debe coincidir con 500");
        assert_eq!(nombres("a_b"), ["a_b"]);

        // Los filtros de listado también tratan el texto como literal.
        let filtro = crate::domain::models::CamisetaFilter {
            texto: "5%".to_string(),
            ..Default::default()
        };
        assert!(
            crate::db::camiseta_repo::list(db.conn(), &filtro)
                .unwrap()
                .is_empty()
        );
    }
}
