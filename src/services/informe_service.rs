//! Informe PDF y exportación a ODS (port de `export_pdf.c` / `export_ods.c`).

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use rusqlite::types::ValueRef;

use crate::services::export_service::ExportError;
use crate::services::resumen_service;
use crate::util::ods::{Hoja, ods};
use crate::util::pdf::{Seccion, informe};

fn db(error: impl std::fmt::Display) -> ExportError {
    ExportError(error.to_string())
}

/// Cabeceras (alias de columna) y filas de texto de una consulta.
fn tabla(conn: &Connection, sql: &str) -> Result<(Vec<String>, Vec<Vec<String>>), ExportError> {
    let mut stmt = conn.prepare(sql).map_err(db)?;
    let cabeceras: Vec<String> = stmt.column_names().map(str::to_string).collect();
    let columnas = cabeceras.len();
    let filas = stmt
        .query_map([], |row| {
            (0..columnas)
                .map(|i| {
                    Ok(match row.get_ref(i)? {
                        ValueRef::Null => String::new(),
                        ValueRef::Integer(n) => n.to_string(),
                        ValueRef::Real(x) => format!("{x:.2}"),
                        ValueRef::Text(t) | ValueRef::Blob(t) => {
                            String::from_utf8_lossy(t).into_owned()
                        }
                    })
                })
                .collect::<Result<Vec<_>, rusqlite::Error>>()
        })
        .map_err(db)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db)?;
    Ok((cabeceras, filas))
}

const PARTIDOS: &str = "SELECT p.fecha_hora AS Fecha, COALESCE(c.nombre, '') AS Cancha,
        p.rival_nombre AS Rival, p.goles AS Goles, p.asistencias AS Asistencias,
        CASE p.resultado WHEN 1 THEN 'Victoria' WHEN 2 THEN 'Empate' WHEN 3 THEN 'Derrota' ELSE '' END AS Resultado,
        p.rendimiento_general AS Rendimiento, p.temperatura_c AS \"Temperatura °C\"
   FROM partido p LEFT JOIN cancha c ON c.id = p.cancha_id ORDER BY p.fecha_hora DESC";
const CANCHAS: &str = "SELECT nombre AS Nombre, direccion AS Dirección, localidad AS Localidad,
        latitud AS Latitud, longitud AS Longitud, precio_hora_dia_centavos / 100.0 AS \"Precio día\"
   FROM cancha ORDER BY nombre";
const CAMISETAS: &str = "SELECT nombre AS Nombre, color_principal AS Color, marca AS Marca,
        temporada AS Temporada, costo_centavos / 100.0 AS Costo FROM camiseta ORDER BY nombre";
const LESIONES: &str =
    "SELECT fecha AS Fecha, tipo AS Tipo, descripcion AS Descripción, estado AS Estado
   FROM lesion ORDER BY fecha DESC";
const FINANZAS: &str =
    "SELECT fecha AS Fecha, CASE tipo WHEN 0 THEN 'Ingreso' ELSE 'Gasto' END AS Tipo,
        categoria AS Categoría, descripcion AS Descripción, monto_centavos / 100.0 AS Monto
   FROM financiamiento ORDER BY fecha DESC";

fn sello() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn escribir(dir: &Path, nombre: &str, bytes: &[u8]) -> Result<PathBuf, ExportError> {
    std::fs::create_dir_all(dir)?;
    let ruta = dir.join(nombre);
    std::fs::write(&ruta, bytes)?;
    Ok(ruta)
}

/// Bytes del .ods: una hoja por tabla.
pub fn ods_bytes(conn: &Connection) -> Result<Vec<u8>, ExportError> {
    let mut hojas = Vec::new();
    for (nombre, sql) in [
        ("Partidos", PARTIDOS),
        ("Canchas", CANCHAS),
        ("Camisetas", CAMISETAS),
        ("Lesiones", LESIONES),
        ("Finanzas", FINANZAS),
    ] {
        let (cabeceras, filas) = tabla(conn, sql)?;
        hojas.push(Hoja {
            nombre: nombre.into(),
            cabeceras,
            filas,
        });
    }
    Ok(ods(&hojas))
}

/// Exporta `mifutbol_{sello}.ods` en `dir`.
pub fn exportar_ods(conn: &Connection, dir: &Path) -> Result<PathBuf, ExportError> {
    escribir(dir, &format!("mifutbol_{}.ods", sello()), &ods_bytes(conn)?)
}

/// Líneas "Col: valor · Col: valor" de las primeras `limite` filas.
fn lineas(conn: &Connection, sql: &str, limite: usize) -> Result<Vec<String>, ExportError> {
    let (cabeceras, filas) = tabla(conn, sql)?;
    if filas.is_empty() {
        return Ok(vec!["Sin registros.".into()]);
    }
    Ok(filas
        .iter()
        .take(limite)
        .map(|fila| {
            cabeceras
                .iter()
                .zip(fila)
                .filter(|(_, valor)| !valor.is_empty())
                .map(|(cabecera, valor)| format!("{cabecera}: {valor}"))
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .collect())
}

/// Bytes del informe PDF.
pub fn pdf_bytes(conn: &Connection, usuario: Option<&str>) -> Result<Vec<u8>, ExportError> {
    let mut encabezado = vec!["Informe MiFutbolR".to_string()];
    if let Some(usuario) = usuario {
        encabezado.push(format!("Usuario: {usuario}"));
    }
    encabezado.push(format!(
        "Fecha: {}",
        chrono::Local::now().format("%d/%m/%Y %H:%M")
    ));

    let resumen = match resumen_service::generar(conn) {
        Ok(datos) => datos.to_text().lines().map(str::to_string).collect(),
        Err(_) => vec!["Todavía no hay partidos registrados.".to_string()],
    };
    let secciones = vec![
        Seccion {
            titulo: "Estadísticas generales".into(),
            lineas: resumen,
        },
        Seccion {
            titulo: "Partidos recientes".into(),
            lineas: lineas(conn, PARTIDOS, 20)?,
        },
        Seccion {
            titulo: "Lesiones".into(),
            lineas: lineas(conn, LESIONES, 50)?,
        },
        Seccion {
            titulo: "Finanzas".into(),
            lineas: lineas(conn, FINANZAS, 50)?,
        },
    ];
    Ok(informe("MiFutbolR", &encabezado, &secciones))
}

/// Exporta `informe_mifutbol_{sello}.pdf` en `dir`.
pub fn exportar_pdf(
    conn: &Connection,
    usuario: Option<&str>,
    dir: &Path,
) -> Result<PathBuf, ExportError> {
    escribir(
        dir,
        &format!("informe_mifutbol_{}.pdf", sello()),
        &pdf_bytes(conn, usuario)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn base() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO cancha (id, nombre) VALUES (1, 'La Bombonera');
                 INSERT INTO camiseta (id, nombre) VALUES (1, 'Azul');
                 INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, rival_nombre, goles, resultado)
                    VALUES (1, 1, '2025-03-10 20:00:00', 'Los Ñandúes', 2, 1);",
            )
            .unwrap();
        db
    }

    #[test]
    fn ods_has_one_sheet_per_table_with_rows() {
        let db = base();
        let bytes = ods_bytes(db.conn()).unwrap();
        let texto = String::from_utf8_lossy(&bytes);
        for hoja in ["Partidos", "Canchas", "Camisetas", "Lesiones", "Finanzas"] {
            assert!(texto.contains(&format!("table:name=\"{hoja}\"")), "{hoja}");
        }
        assert!(texto.contains("Los Ñandúes"));
        assert!(texto.contains("<text:p>Victoria</text:p>"));
    }

    #[test]
    fn pdf_report_includes_user_and_sections() {
        let db = base();
        let bytes = pdf_bytes(db.conn(), Some("Ana")).unwrap();
        let pdf: String = bytes.iter().map(|b| *b as char).collect();
        assert!(pdf.starts_with("%PDF-1.4"));
        assert!(pdf.contains("(Usuario: Ana)"));
        assert!(pdf.contains("(Estad\\355sticas generales)"));
        assert!(pdf.contains("Los \\321and\\372es"));
    }
}
