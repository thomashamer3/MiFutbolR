//! Informe PDF y exportación a ODS (port de `export_pdf.c` / `export_ods.c`).

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use rusqlite::types::ValueRef;

use crate::services::export_service::ExportError;
use crate::services::resumen_service;
use crate::util::ods::{Hoja, ods};
use crate::util::pdf::{Seccion, informe};
use crate::util::xlsx::xlsx;

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

// Bienestar como `export_bienestar.c`: objetivos, planes, comidas, hábitos y salud.
const BIENESTAR: &str = "SELECT 'objetivo' AS Tipo, o.nombre AS Nombre, COALESCE(o.notas, '') AS Descripción,
        o.fecha_inicio AS Fecha, o.estado AS Valor FROM bienestar_objetivo o
 UNION ALL SELECT 'plan', p.rutina_semanal, COALESCE(p.notas, ''), '', CAST(p.frecuencia_semanal AS TEXT)
   FROM bienestar_plan_entrenamiento p
 UNION ALL SELECT 'comida', c.descripcion, c.calidad, c.fecha, c.tipo FROM bienestar_comida c
 UNION ALL SELECT 'habito', COALESCE(h.notas, ''), COALESCE(h.estado_animico, ''), h.fecha,
        CAST(h.dormi_bien AS TEXT) FROM bienestar_habito h
 UNION ALL SELECT 'salud', COALESCE(s.tipo_sangre, ''), COALESCE(s.notas, ''),
        COALESCE(s.ultima_revision, ''), CAST(COALESCE(s.peso_kg, 0) AS TEXT) FROM bienestar_salud s
 ORDER BY 1, 4";
// Colecciones e inventario como `export_colecciones.c` (con etiquetas en vez de códigos).
const COLECCIONES: &str = "SELECT id AS Id, nombre AS Nombre, descripcion AS Descripción,
        COALESCE(fecha_creacion, '') AS Creada FROM coleccion ORDER BY id";
const INVENTARIO: &str =
    "SELECT i.id AS Id, COALESCE(NULLIF(i.nombre, ''), cam.nombre, '') AS Nombre,
        CASE i.tipo WHEN 1 THEN 'Camiseta' WHEN 2 THEN 'Botines' ELSE 'Accesorio' END AS Tipo,
        CASE i.estado WHEN 1 THEN 'Usado' ELSE 'Nuevo' END AS Estado, i.valor AS Valor,
        COALESCE(i.fecha_compra, '') AS Compra,
        (SELECT group_concat(c.nombre, ', ') FROM coleccion_inventario ci
           JOIN coleccion c ON c.id = ci.coleccion_id WHERE ci.inventario_id = i.id) AS Colecciones
   FROM inventario_item i LEFT JOIN camiseta cam ON cam.id = i.camiseta_id ORDER BY i.id";
const CONSEJOS_IA: &str =
    "SELECT datetime(fecha, 'unixepoch', 'localtime') AS Fecha, consejo AS Consejo,
        CASE seguido WHEN 1 THEN 'Sí' ELSE 'No' END AS Seguido
   FROM consejos_historial ORDER BY fecha DESC, id DESC";

fn sello() -> String {
    crate::domain::dates::ahora_fmt("%Y%m%d-%H%M%S")
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
        ("Bienestar", BIENESTAR),
        ("Colecciones", COLECCIONES),
        ("Inventario", INVENTARIO),
        ("Entrenador IA", CONSEJOS_IA),
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

const EQUIPOS: &str = "SELECT nombre AS Nombre,
        CASE tipo WHEN 1 THEN 'Momentáneo' ELSE 'Fijo' END AS Tipo,
        CASE tipo_futbol WHEN 1 THEN 'F7' WHEN 2 THEN 'F8' WHEN 3 THEN 'F11' ELSE 'F5' END AS Fútbol,
        num_jugadores AS Jugadores FROM equipo ORDER BY nombre";

/// Bytes del .xlsx con las hojas de `menu_exportar_xlsx` de C: partidos,
/// equipos, canchas, camisetas, lesiones y torneos. Las consultas de C
/// pedían columnas que no existen (`equipo.pais`, `camisetas.club`,
/// `lesion.gravedad`...) y dejaban las hojas vacías; acá son las del esquema.
pub fn xlsx_bytes(conn: &Connection) -> Result<Vec<u8>, ExportError> {
    let mut hojas = Vec::new();
    for (nombre, sql) in [
        ("Partidos", PARTIDOS),
        ("Equipos", EQUIPOS),
        ("Canchas", CANCHAS),
        ("Camisetas", CAMISETAS),
        ("Lesiones", LESIONES),
        ("Torneos", TORNEOS),
    ] {
        let (cabeceras, filas) = tabla(conn, sql)?;
        hojas.push(Hoja {
            nombre: nombre.into(),
            cabeceras,
            filas,
        });
    }
    Ok(xlsx(&hojas))
}

/// Exporta `mifutbol_{sello}.xlsx` en `dir` (un libro con todas las hojas;
/// C escribía un archivo por tabla).
pub fn exportar_xlsx(conn: &Connection, dir: &Path) -> Result<PathBuf, ExportError> {
    escribir(
        dir,
        &format!("mifutbol_{}.xlsx", sello()),
        &xlsx_bytes(conn)?,
    )
}

/// Líneas "Col: valor · Col: valor" de las primeras `limite` filas.
fn lineas_sql(conn: &Connection, sql: &str, limite: usize) -> Result<Vec<String>, ExportError> {
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

/// Resumen mensual de Bienestar (`mes` = `YYYY-MM`), como el informe personal
/// de `export_pdf.c`. Allá la tendencia comparaba `partido.fecha`, columna que
/// no existe; acá se usa `date(fecha_hora)`.
#[allow(clippy::cast_precision_loss)]
fn bienestar_mes(conn: &Connection, mes: &str) -> Result<Vec<String>, ExportError> {
    let (sesiones, minutos, intensidad, omitidos): (i64, i64, f64, i64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(duracion_min), 0), COALESCE(AVG(intensidad), 0.0),
                    COALESCE(SUM(omitido), 0)
               FROM bienestar_entrenamiento WHERE substr(fecha, 1, 7) = ?1",
            [mes],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(db)?;
    let (comidas, buenas, regulares, malas): (i64, i64, i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(calidad = 'Buena'), 0),
                    COALESCE(SUM(calidad = 'Regular'), 0), COALESCE(SUM(calidad = 'Mala'), 0)
               FROM bienestar_comida WHERE substr(fecha, 1, 7) = ?1",
            [mes],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(db)?;
    let (baja, media, alta): (i64, i64, i64) = conn
        .query_row(
            "SELECT COALESCE(SUM(hidratacion = 'Baja'), 0), COALESCE(SUM(hidratacion = 'Media'), 0),
                    COALESCE(SUM(hidratacion = 'Alta'), 0)
               FROM bienestar_dia_nutricional WHERE substr(fecha, 1, 7) = ?1",
            [mes],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(db)?;
    let mental: (i64, f64, f64, f64, f64, f64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(AVG(confianza), 0.0), COALESCE(AVG(estres), 0.0),
                    COALESCE(AVG(motivacion), 0.0), COALESCE(AVG(presion), 0.0),
                    COALESCE(AVG(concentracion), 0.0)
               FROM bienestar_sesion_mental WHERE substr(fecha, 1, 7) = ?1",
            [mes],
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
        )
        .map_err(db)?;
    let tendencia = |condicion: &str| -> Result<f64, ExportError> {
        conn.query_row(
            &format!(
                "SELECT COALESCE(AVG(rendimiento_general), 0.0) FROM partido
                  WHERE substr(fecha_hora, 1, 7) = ?1 AND date(fecha_hora) IN
                        (SELECT fecha FROM bienestar_sesion_mental
                          WHERE {condicion} AND substr(fecha, 1, 7) = ?1)"
            ),
            [mes],
            |row| row.get(0),
        )
        .map_err(db)
    };
    let (alta_conf, baja_conf) = (tendencia("confianza >= 8")?, tendencia("confianza <= 4")?);
    let pct_buenas = if comidas > 0 {
        buenas as f64 / comidas as f64 * 100.0
    } else {
        0.0
    };
    let mut lineas = vec![
        format!(
            "Entrenamiento: {sesiones} sesiones | minutos: {minutos} | intensidad promedio: {intensidad:.2}"
        ),
        format!("Entrenamientos omitidos: {omitidos}"),
        format!("Comidas: {comidas} | Buenas: {buenas} | Regulares: {regulares} | Malas: {malas}"),
        format!("% de comidas buenas: {pct_buenas:.1}%"),
        format!("Hidratación: baja {baja} | media {media} | alta {alta}"),
        format!(
            "Mental: {} sesiones | confianza {:.2} | estrés {:.2}",
            mental.0, mental.1, mental.2
        ),
        format!(
            "Motivación {:.2} | presión {:.2} | concentración {:.2}",
            mental.3, mental.4, mental.5
        ),
        format!("Rendimiento con confianza alta: {alta_conf:.2} | baja: {baja_conf:.2}"),
    ];
    if alta_conf > baja_conf {
        lineas.push("Tendencia: jugás mejor cuando la confianza está alta.".to_string());
    }
    Ok(lineas)
}

/// Resumen de partidos del mes (`escribir_resumen_partidos` de `export_pdf.c`;
/// allá filtraba por `partido.fecha`, que no existe, y la sección salía vacía).
fn partidos_mes(conn: &Connection, mes: &str) -> Result<Vec<String>, ExportError> {
    let (total, goles, asist, rend, cans, animo): (i64, f64, f64, f64, f64, f64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(AVG(goles), 0.0), COALESCE(AVG(asistencias), 0.0),
                    COALESCE(AVG(rendimiento_general), 0.0), COALESCE(AVG(cansancio), 0.0),
                    COALESCE(AVG(estado_animo), 0.0)
               FROM partido WHERE substr(fecha_hora, 1, 7) = ?1",
            [mes],
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
        )
        .map_err(db)?;
    Ok(vec![
        format!("Partidos: {total}"),
        format!("Promedio goles: {goles:.2} | asistencias: {asist:.2}"),
        format!("Rendimiento: {rend:.2} | cansancio: {cans:.2} | ánimo: {animo:.2}"),
    ])
}

/// `MM/AAAA` (como pide MiFutbolC) o `AAAA-MM` → `AAAA-MM`.
#[must_use]
pub fn mes_iso(texto: &str) -> Option<String> {
    let texto = texto.trim();
    let (anio, mes) = match texto.split_once('/') {
        Some((mes, anio)) => (anio, mes),
        None => texto.split_once('-')?,
    };
    let valido = anio.len() == 4
        && mes.len() == 2
        && anio.bytes().chain(mes.bytes()).all(|b| b.is_ascii_digit())
        && (1..=12).contains(&mes.parse::<u8>().ok()?);
    valido.then(|| format!("{anio}-{mes}"))
}

/// Bytes del Informe Personal Mensual (`generar_informe_personal_mensual_pdf`):
/// partidos, entrenamiento, alimentación y mentalidad del mes `AAAA-MM`.
pub fn pdf_bienestar_mes(
    conn: &Connection,
    mes: &str,
    usuario: Option<&str>,
) -> Result<Vec<u8>, ExportError> {
    let mes_display = format!("{}/{}", &mes[5..7], &mes[..4]);
    let mut encabezado = vec![format!("INFORME PERSONAL MENSUAL - {mes_display}")];
    if let Some(usuario) = usuario {
        encabezado.push(format!("Usuario: {usuario}"));
    }
    encabezado.push(format!(
        "Fecha: {}",
        crate::domain::dates::ahora_fmt("%d/%m/%Y %H:%M")
    ));
    let mut bienestar = bienestar_mes(conn, mes)?;
    let mental = bienestar.split_off(5);
    let alimentacion = bienestar.split_off(2);
    let secciones = [
        ("Resumen de Partidos", partidos_mes(conn, mes)?),
        ("Resumen de Entrenamiento", bienestar),
        ("Resumen de Alimentación", alimentacion),
        ("Resumen de Mentalidad", mental),
        ("Nota", vec!["Este informe es 100% local.".to_string()]),
    ]
    .into_iter()
    .map(|(titulo, lineas)| Seccion {
        titulo: titulo.into(),
        lineas,
    })
    .collect::<Vec<_>>();
    Ok(informe("Informe Personal Mensual", &encabezado, &secciones))
}

/// Exporta `Informe Personal MM-AAAA.pdf` (nombre de MiFutbolC) en `dir`.
pub fn exportar_bienestar_mes(
    conn: &Connection,
    mes: &str,
    usuario: Option<&str>,
    dir: &Path,
) -> Result<PathBuf, ExportError> {
    let mes = mes_iso(mes).ok_or_else(|| ExportError("Mes inválido: use MM/AAAA".into()))?;
    escribir(
        dir,
        &format!("Informe Personal {}-{}.pdf", &mes[5..7], &mes[..4]),
        &pdf_bienestar_mes(conn, &mes, usuario)?,
    )
}

/// Totales del Entrenador IA y últimos consejos.
fn entrenador_ia(conn: &Connection) -> Result<Vec<String>, ExportError> {
    let (nivel, riesgo, seguidos, total): (i64, f64, i64, i64) = conn
        .query_row(
            "SELECT COALESCE((SELECT nivel_intervencion FROM ia_config WHERE id = 1), 2),
                    COALESCE((SELECT ultimo_riesgo FROM ia_config WHERE id = 1), 0.0),
                    (SELECT COALESCE(SUM(seguido = 1), 0) FROM consejos_historial),
                    (SELECT COUNT(*) FROM consejos_historial)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(db)?;
    let mut lineas = vec![format!(
        "Nivel de intervención: {nivel} | último riesgo avisado: {riesgo:.2} | consejos: {total} (seguidos {seguidos})"
    )];
    lineas.extend(lineas_sql(conn, CONSEJOS_IA, 10)?);
    Ok(lineas)
}

/// Bytes del informe PDF.
pub fn pdf_bytes(conn: &Connection, usuario: Option<&str>) -> Result<Vec<u8>, ExportError> {
    let mut encabezado = vec!["Informe MiFutbolR".to_string()];
    if let Some(usuario) = usuario {
        encabezado.push(format!("Usuario: {usuario}"));
    }
    encabezado.push(format!(
        "Fecha: {}",
        crate::domain::dates::ahora_fmt("%d/%m/%Y %H:%M")
    ));

    let mes = crate::domain::dates::ahora_fmt("%Y-%m");
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
            lineas: lineas_sql(conn, PARTIDOS, 20)?,
        },
        Seccion {
            titulo: "Lesiones".into(),
            lineas: lineas_sql(conn, LESIONES, 50)?,
        },
        Seccion {
            titulo: "Finanzas".into(),
            lineas: lineas_sql(conn, FINANZAS, 50)?,
        },
        Seccion {
            titulo: format!("Bienestar del mes ({mes})"),
            lineas: bienestar_mes(conn, &mes)?,
        },
        Seccion {
            titulo: "Colecciones".into(),
            lineas: lineas_sql(
                conn,
                "SELECT c.nombre AS Colección, COUNT(ci.inventario_id) AS Ítems
                   FROM coleccion c LEFT JOIN coleccion_inventario ci ON ci.coleccion_id = c.id
                  GROUP BY c.id ORDER BY c.nombre",
                50,
            )?,
        },
        Seccion {
            titulo: "Entrenador IA".into(),
            lineas: entrenador_ia(conn)?,
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

// Botines y torneos para el Markdown (las demás consultas son las del ODS).
const BOTINES: &str = "SELECT nombre AS Nombre,
        CASE activa WHEN 1 THEN 'Sí' ELSE 'No' END AS Activo FROM botin ORDER BY nombre";
const TORNEOS: &str = "SELECT nombre AS Nombre, cantidad_equipos AS Equipos,
        fase_actual AS Fase FROM torneo ORDER BY nombre";

/// Tablas que no se exportan: internas de SQLite, historial de undo y credenciales
/// (hashes de contraseña de `usuario`/`cuenta`).
const EXCLUIDAS_JSON: &[&str] = &[
    "undo_control",
    "undo_historial",
    "usuario",
    "cuenta",
    "cuenta_sesion",
    // Sincronización: el token y el correo son secretos; la cola es interna.
    "sync_config",
    "sync_dirty",
    "sync_conflicto",
    "sync_tombstones",
];

/// Todas las tablas de usuario como `{tabla: [fila, ...]}`. Los BLOB salen como
/// `null` (el esquema no tiene columnas BLOB).
pub fn json_todo(conn: &Connection) -> Result<serde_json::Value, ExportError> {
    use serde_json::{Map, Value};
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master
              WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .map_err(db)?;
    let tablas = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(db)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db)?;
    let mut raiz = Map::new();
    for tabla in tablas
        .into_iter()
        .filter(|t| !EXCLUIDAS_JSON.contains(&t.as_str()))
    {
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM \"{tabla}\""))
            .map_err(db)?;
        let columnas: Vec<String> = stmt.column_names().map(str::to_string).collect();
        let filas = stmt
            .query_map([], |row| {
                let mut objeto = Map::new();
                for (i, columna) in columnas.iter().enumerate() {
                    let valor = match row.get_ref(i)? {
                        ValueRef::Integer(n) => Value::from(n),
                        ValueRef::Real(x) => Value::from(x),
                        ValueRef::Text(t) => Value::from(String::from_utf8_lossy(t).into_owned()),
                        ValueRef::Null | ValueRef::Blob(_) => Value::Null,
                    };
                    objeto.insert(columna.clone(), valor);
                }
                Ok(Value::Object(objeto))
            })
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)?;
        raiz.insert(tabla, Value::Array(filas));
    }
    Ok(Value::Object(raiz))
}

/// Informe Markdown con los listados principales (una tabla por sección).
pub fn markdown_todo(conn: &Connection) -> Result<String, ExportError> {
    let celda = |v: &String| v.replace('|', "\\|").replace('\n', " ");
    let mut texto = format!(
        "# MiFutbolR · Exportación completa\n\nGenerado {}\n",
        crate::domain::dates::ahora_fmt("%d/%m/%Y %H:%M")
    );
    for (titulo, sql) in [
        ("Partidos", PARTIDOS),
        ("Camisetas", CAMISETAS),
        ("Canchas", CANCHAS),
        ("Botines", BOTINES),
        ("Lesiones", LESIONES),
        ("Financiamiento", FINANZAS),
        ("Torneos", TORNEOS),
    ] {
        let (cabeceras, filas) = tabla(conn, sql)?;
        texto.push_str(&format!("\n## {titulo} ({})\n\n", filas.len()));
        if filas.is_empty() {
            texto.push_str("Sin registros.\n");
            continue;
        }
        texto.push_str(&format!("| {} |\n|", cabeceras.join(" | ")));
        texto.push_str(&" --- |".repeat(cabeceras.len()));
        texto.push('\n');
        for fila in filas {
            let valores: Vec<String> = fila.iter().map(celda).collect();
            texto.push_str(&format!("| {} |\n", valores.join(" | ")));
        }
    }
    Ok(texto)
}

/// "Exportar todo": `mifutbol_{sello}.json` y `mifutbol_{sello}.md` en `dir`.
pub fn exportar_todo(conn: &Connection, dir: &Path) -> Result<(PathBuf, PathBuf), ExportError> {
    let marca = sello();
    let json = serde_json::to_string_pretty(&json_todo(conn)?)?;
    Ok((
        escribir(dir, &format!("mifutbol_{marca}.json"), json.as_bytes())?,
        escribir(
            dir,
            &format!("mifutbol_{marca}.md"),
            markdown_todo(conn)?.as_bytes(),
        )?,
    ))
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
                    VALUES (1, 1, '2025-03-10 20:00:00', 'Los Ñandúes', 2, 1);
                 INSERT INTO bienestar_objetivo (nombre, fecha_inicio, fecha_fin)
                    VALUES ('Correr 10k', '2025-03-01', '2025-06-01');
                 INSERT INTO inventario_item (tipo, nombre) VALUES (3, 'Canilleras');
                 INSERT INTO consejos_historial (fecha, consejo, seguido) VALUES (1741600000, 'Descansar', 1);",
            )
            .unwrap();
        db
    }

    #[test]
    fn ods_has_one_sheet_per_table_with_rows() {
        let db = base();
        let bytes = ods_bytes(db.conn()).unwrap();
        let texto = String::from_utf8_lossy(&bytes);
        for hoja in [
            "Partidos",
            "Canchas",
            "Camisetas",
            "Lesiones",
            "Finanzas",
            "Bienestar",
            "Colecciones",
            "Inventario",
            "Entrenador IA",
        ] {
            assert!(texto.contains(&format!("table:name=\"{hoja}\"")), "{hoja}");
        }
        assert!(texto.contains("Los Ñandúes"));
        assert!(texto.contains("<text:p>Victoria</text:p>"));
        // Bienestar como export_bienestar.c, colecciones sembradas e inventario con etiquetas.
        assert!(texto.contains("<text:p>Correr 10k</text:p>"));
        assert!(texto.contains("<text:p>objetivo</text:p>"));
        assert!(texto.contains("<text:p>Equipamiento actual</text:p>"));
        assert!(texto.contains("<text:p>Accesorio</text:p>"));
        assert!(texto.contains("<text:p>Descansar</text:p>"));
    }

    #[test]
    fn xlsx_tiene_las_hojas_de_c_con_datos() {
        use crate::util::xlsx::tests::{entradas_zip, xml_bien_formado};
        let db = base();
        let entradas = entradas_zip(&xlsx_bytes(db.conn()).unwrap());
        let libro = &entradas
            .iter()
            .find(|(n, _)| n == "xl/workbook.xml")
            .unwrap()
            .1;
        for hoja in [
            "Partidos",
            "Equipos",
            "Canchas",
            "Camisetas",
            "Lesiones",
            "Torneos",
        ] {
            assert!(libro.contains(&format!("name=\"{hoja}\"")), "{hoja}");
        }
        let partidos = &entradas
            .iter()
            .find(|(n, _)| n == "xl/worksheets/sheet1.xml")
            .unwrap()
            .1;
        assert!(xml_bien_formado(partidos));
        assert!(partidos.contains(">Los Ñandúes</t>"));
        assert!(partidos.contains(">Victoria</t>"));
        let dir = tempfile::tempdir().unwrap();
        let ruta = exportar_xlsx(db.conn(), dir.path()).unwrap();
        assert!(ruta.extension().is_some_and(|e| e == "xlsx"));
        assert_eq!(&std::fs::read(ruta).unwrap()[..4], b"PK");
    }

    #[test]
    fn monthly_wellness_summary_matches_export_pdf() {
        let db = base();
        db.conn()
            .execute_batch(
                "INSERT INTO bienestar_entrenamiento (fecha, tipo, duracion_min, intensidad, omitido)
                    VALUES ('2025-03-02', 'Fuerza', 60, 8, 0), ('2025-03-05', 'Tecnica', 30, 4, 1),
                           ('2025-04-01', 'Fuerza', 90, 9, 0);
                 INSERT INTO bienestar_comida (fecha, tipo, calidad) VALUES
                    ('2025-03-02', 'Cena', 'Buena'), ('2025-03-03', 'Cena', 'Mala'),
                    ('2025-03-04', 'Cena', 'Buena'), ('2025-03-05', 'Cena', 'Regular');
                 INSERT INTO bienestar_dia_nutricional (fecha, hidratacion) VALUES
                    ('2025-03-02', 'Alta'), ('2025-03-03', 'Baja');
                 INSERT INTO bienestar_sesion_mental (fecha, tipo, confianza, estres) VALUES
                    ('2025-03-10', 'Charla', 9, 2), ('2025-03-11', 'Charla', 3, 6);
                 UPDATE partido SET rendimiento_general = 8;",
            )
            .unwrap();
        let lineas = bienestar_mes(db.conn(), "2025-03").unwrap();
        assert_eq!(
            lineas[0],
            "Entrenamiento: 2 sesiones | minutos: 90 | intensidad promedio: 6.00"
        );
        assert_eq!(lineas[1], "Entrenamientos omitidos: 1");
        assert_eq!(
            lineas[2],
            "Comidas: 4 | Buenas: 2 | Regulares: 1 | Malas: 1"
        );
        assert_eq!(lineas[3], "% de comidas buenas: 50.0%");
        assert_eq!(lineas[4], "Hidratación: baja 1 | media 0 | alta 1");
        assert_eq!(
            lineas[5],
            "Mental: 2 sesiones | confianza 6.00 | estrés 4.00"
        );
        // El partido del 10/03 coincide con la sesión de confianza alta.
        assert_eq!(
            lineas[7],
            "Rendimiento con confianza alta: 8.00 | baja: 0.00"
        );
        assert!(lineas[8].starts_with("Tendencia"));
    }

    #[test]
    fn monthly_wellness_pdf_has_c_sections() {
        let db = base();
        db.conn()
            .execute_batch(
                "INSERT INTO bienestar_entrenamiento (fecha, tipo, duracion_min, intensidad)
                    VALUES ('2025-03-02', 'Fuerza', 60, 8);",
            )
            .unwrap();
        assert_eq!(mes_iso("03/2025").as_deref(), Some("2025-03"));
        assert_eq!(mes_iso("2025-03").as_deref(), Some("2025-03"));
        assert_eq!(mes_iso("13/2025"), None);
        assert_eq!(mes_iso("3/2025"), None);
        let pdf: String = pdf_bienestar_mes(db.conn(), "2025-03", Some("Ana"))
            .unwrap()
            .iter()
            .map(|b| *b as char)
            .collect();
        assert!(pdf.starts_with("%PDF"));
        assert!(pdf.contains("(INFORME PERSONAL MENSUAL - 03/2025)"));
        assert!(pdf.contains("(Resumen de Partidos)"));
        assert!(
            pdf.contains("(Partidos: 1)"),
            "el partido de marzo se cuenta"
        );
        assert!(pdf.contains("(Resumen de Entrenamiento)"));
        assert!(pdf.contains("(Entrenamiento: 1 sesiones | minutos: 60"));
        assert!(pdf.contains("(Resumen de Mentalidad)"));
        assert!(pdf.contains("(Este informe es 100% local.)"));
        let dir = tempfile::tempdir().unwrap();
        let ruta = exportar_bienestar_mes(db.conn(), "03/2025", None, dir.path()).unwrap();
        assert!(ruta.ends_with("Informe Personal 03-2025.pdf"));
        assert!(exportar_bienestar_mes(db.conn(), "marzo", None, dir.path()).is_err());
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
        assert!(pdf.contains("(Bienestar del mes"));
        assert!(pdf.contains("(Colecciones)"));
        assert!(pdf.contains("(Entrenador IA)"));
    }

    #[test]
    fn exportar_todo_json_y_markdown() {
        let db = base();
        db.conn()
            .execute(
                "INSERT INTO sync_config (clave, valor) VALUES ('token', 'secreto-pb')",
                [],
            )
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let (json, md) = exportar_todo(db.conn(), dir.path()).unwrap();
        let texto = std::fs::read_to_string(json).unwrap();
        assert!(!texto.contains("secreto-pb"), "el token no se exporta");
        let valor: serde_json::Value = serde_json::from_str(&texto).unwrap();
        assert_eq!(valor["partido"][0]["rival_nombre"], "Los Ñandúes");
        assert_eq!(valor["partido"][0]["goles"], 2);
        assert!(valor.get("cancha").is_some());
        for excluida in [
            "undo_historial",
            "cuenta",
            "usuario",
            "sqlite_sequence",
            "sync_config",
            "sync_dirty",
            "sync_conflicto",
            "sync_tombstones",
        ] {
            assert!(valor.get(excluida).is_none(), "{excluida}");
        }
        let md = std::fs::read_to_string(md).unwrap();
        for cabecera in [
            "# MiFutbolR",
            "## Partidos (1)",
            "## Camisetas",
            "## Botines",
            "## Torneos",
        ] {
            assert!(md.contains(cabecera), "{cabecera}");
        }
        assert!(md.contains("| La Bombonera |"));
    }
}
