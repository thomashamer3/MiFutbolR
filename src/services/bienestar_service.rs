//! Bienestar: CRUD por sección y asistente de preparación (fórmulas de
//! MiFutbolC, `bienestar.c`).

use std::path::Path;

use chrono::NaiveDate;
use rusqlite::Connection;
use rusqlite::types::Value;
use thiserror::Error;

use crate::db::DbError;
use crate::db::bienestar_repo;
use crate::domain::bienestar::{
    Asistente, EntradasAsistente, Recomendacion, Registro, Seccion, TipoDato,
};
use crate::domain::dates;

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

/// Registros de una sección.
pub fn listar(conn: &Connection, seccion: Seccion) -> Result<Vec<Registro>, ServiceError> {
    Ok(bienestar_repo::list(conn, seccion)?)
}

/// Crea (`id = None`) o actualiza un registro a partir de los textos del
/// formulario, en el orden de [`Seccion::columnas`]. Fecha vacía = `hoy`.
pub fn guardar(
    conn: &Connection,
    seccion: Seccion,
    id: Option<i64>,
    textos: &[String],
    hoy: NaiveDate,
) -> Result<i64, ServiceError> {
    let columnas = seccion.columnas();
    if textos.len() != columnas.len() {
        return Err(ServiceError::Message("Formulario incompleto".to_string()));
    }
    let valores = columnas
        .iter()
        .zip(textos)
        .map(|(columna, texto)| convertir(columna.tipo, texto, hoy, columna.etiqueta))
        .collect::<Result<Vec<_>, _>>()?;
    match id {
        Some(id) => {
            bienestar_repo::update(conn, seccion, id, &valores)?;
            Ok(id)
        }
        None => Ok(bienestar_repo::insert(conn, seccion, &valores)?),
    }
}

fn convertir(
    tipo: TipoDato,
    texto: &str,
    hoy: NaiveDate,
    etiqueta: &str,
) -> Result<Value, ServiceError> {
    let texto = texto.trim();
    let invalido = || ServiceError::Message(format!("{etiqueta}: valor inválido"));
    Ok(match tipo {
        TipoDato::Fecha if texto.is_empty() => Value::Text(dates::to_iso_date(hoy)),
        TipoDato::Fecha => Value::Text(dates::to_iso_date(
            dates::parse_date(texto).ok_or_else(invalido)?,
        )),
        TipoDato::Entero(min, max) => {
            let valor = if texto.is_empty() {
                min
            } else {
                texto.parse().map_err(|_| invalido())?
            };
            if !(min..=max).contains(&valor) {
                return Err(ServiceError::Message(format!(
                    "{etiqueta}: debe estar entre {min} y {max}"
                )));
            }
            Value::Integer(valor)
        }
        TipoDato::Decimal(_, _) if texto.is_empty() => Value::Null,
        TipoDato::Decimal(min, max) => {
            let valor: f64 = texto.replace(',', ".").parse().map_err(|_| invalido())?;
            if !valor.is_finite() || valor < min || valor > max {
                return Err(invalido());
            }
            Value::Real(valor)
        }
        TipoDato::Booleano => Value::Integer(i64::from(texto == "Sí")),
        TipoDato::Opcion(opciones) => {
            if !opciones.contains(&texto) {
                return Err(invalido());
            }
            Value::Text(texto.to_string())
        }
        TipoDato::Texto(max) => {
            if texto.chars().count() > max {
                return Err(ServiceError::Message(format!(
                    "{etiqueta}: máximo {max} caracteres"
                )));
            }
            Value::Text(texto.to_string())
        }
        TipoDato::Referencia(_) if texto.is_empty() => {
            return Err(ServiceError::Message(format!(
                "{etiqueta}: primero cargá uno en su pestaña"
            )));
        }
        TipoDato::Referencia(_) => Value::Integer(texto.parse().map_err(|_| invalido())?),
        TipoDato::Archivo if texto.is_empty() => {
            return Err(ServiceError::Message(format!("{etiqueta}: obligatorio")));
        }
        TipoDato::Archivo => Value::Text(texto.to_string()),
    })
}

/// Extensiones de imagen aceptadas (MiFutbolC, `bienestar.c`).
const EXT_IMAGEN: &[&str] = &["jpg", "jpeg", "png", "bmp", "webp"];
/// Documentos aceptados además de imágenes en los estudios médicos.
const EXT_DOCUMENTO: &[&str] = &["pdf", "docx", "doc", "txt", "xlsx", "csv"];

/// `true` si la ruta tiene extensión de imagen.
#[must_use]
pub fn es_imagen(ruta: &str) -> bool {
    extension(ruta).is_some_and(|ext| EXT_IMAGEN.contains(&ext.as_str()))
}

fn extension(ruta: &str) -> Option<String> {
    Path::new(ruta)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
}

fn posicion(seccion: Seccion, clave: &str) -> Option<usize> {
    seccion
        .columnas()
        .iter()
        .position(|columna| columna.clave == clave)
}

/// Copia el archivo elegido a `imagenes` como MiFutbolC y deja en `textos`
/// la ruta relativa `Imagenes/<nombre>`; en los estudios completa también el
/// nombre original y el tipo (imagen/documento). Una ruta que ya apunta a
/// `Imagenes/` (registro existente o importado) se deja tal cual.
pub fn adjuntar(
    seccion: Seccion,
    textos: &mut [String],
    imagenes: &Path,
) -> Result<(), ServiceError> {
    let Some(indice) = seccion.columna_archivo() else {
        return Ok(());
    };
    let estudio = seccion == Seccion::Estudios;
    let ruta = textos[indice].trim().to_string();
    let ya_copiado = ruta.starts_with("Imagenes/") || ruta.starts_with("imagenes/");
    if !ruta.is_empty() && !ya_copiado {
        let origen = Path::new(&ruta);
        if !origen.is_file() {
            return Err(ServiceError::Message(format!(
                "No existe el archivo: {ruta}"
            )));
        }
        let ext = extension(&ruta).unwrap_or_default();
        let permitido = EXT_IMAGEN.contains(&ext.as_str())
            || (estudio && EXT_DOCUMENTO.contains(&ext.as_str()));
        if !permitido {
            return Err(ServiceError::Message(format!(
                "Formato no soportado: .{ext}"
            )));
        }
        let (prefijo, clave) = if estudio {
            ("estudio", "control_id")
        } else {
            ("bienestar", "menu_key")
        };
        let destino = format!(
            "{prefijo}_{}_{}.{ext}",
            posicion(seccion, clave).map_or("", |i| textos[i].trim()),
            chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
        );
        std::fs::create_dir_all(imagenes)
            .and_then(|()| std::fs::copy(origen, imagenes.join(&destino)))
            .map_err(|error| ServiceError::Message(format!("No se pudo copiar: {error}")))?;
        if let Some(i) =
            posicion(seccion, "nombre_original").filter(|i| textos[*i].trim().is_empty())
        {
            textos[i] = origen
                .file_name()
                .map(|nombre| nombre.to_string_lossy().into_owned())
                .unwrap_or_default();
        }
        textos[indice] = format!("Imagenes/{destino}");
    }
    if let Some(i) = posicion(seccion, "tipo_archivo") {
        textos[i] = if es_imagen(&textos[indice]) {
            "imagen"
        } else {
            "documento"
        }
        .to_string();
    }
    Ok(())
}

/// Elimina un registro (el perfil de salud no se elimina).
pub fn eliminar(conn: &Connection, seccion: Seccion, id: i64) -> Result<(), ServiceError> {
    if seccion == Seccion::Salud {
        return Err(ServiceError::Message(
            "El perfil de salud se edita, no se elimina".to_string(),
        ));
    }
    bienestar_repo::delete(conn, seccion, id)?;
    Ok(())
}

/// Marca o desmarca un entrenamiento como omitido.
pub fn alternar_omitido(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    bienestar_repo::alternar_omitido(conn, id)?;
    Ok(())
}

/// Calcula el asistente con los datos actuales.
pub fn asistente(conn: &Connection, hoy: NaiveDate) -> Result<Asistente, ServiceError> {
    let entradas = bienestar_repo::entradas_asistente(conn, &dates::to_iso_date(hoy))?;
    let mut asistente = calcular(&entradas);
    let objetivo = bienestar_repo::objetivo_activo(conn)?
        .filter(|nombre| !nombre.is_empty())
        .unwrap_or_else(|| "Sin objetivo activo".to_string());
    asistente.rutina = format!(
        "Objetivo: {objetivo}
{}",
        asistente.rutina
    );
    Ok(asistente)
}

/// Guarda el resultado del asistente en el historial.
pub fn guardar_recomendacion(
    conn: &Connection,
    hoy: NaiveDate,
    asistente: &Asistente,
) -> Result<(), ServiceError> {
    bienestar_repo::insert_recomendacion(
        conn,
        &dates::to_iso_date(hoy),
        asistente.preparacion,
        asistente.riesgo,
        &asistente.resumen,
        &asistente.rutina,
    )?;
    Ok(())
}

/// Últimas recomendaciones guardadas.
pub fn recomendaciones(conn: &Connection) -> Result<Vec<Recomendacion>, ServiceError> {
    Ok(bienestar_repo::recomendaciones(conn, 5)?)
}

fn clamp(valor: f64) -> f64 {
    valor.clamp(0.0, 100.0)
}

/// Fórmulas del asistente de MiFutbolC (`bienestar.c`). La nutrición promedia
/// la calidad de las comidas y la hidratación del día nutricional, y resta el
/// alcohol de esos días; sin días nutricionales la hidratación cuenta 0, como allá.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn calcular(e: &EntradasAsistente) -> Asistente {
    let rendimiento = clamp(e.rendimiento * 10.0);
    let habitos = clamp(
        (e.dormi * 100.0
            + e.hidratacion / 10.0 * 100.0
            + (10.0 - e.nervios) / 10.0 * 100.0
            + e.confianza_habito / 10.0 * 100.0
            + e.motivacion_habito / 10.0 * 100.0)
            / 5.0
            - e.alcohol * 15.0,
    );
    let nutricion = clamp(
        (clamp((e.comida - 1.0) / 2.0 * 100.0) + clamp((e.dia_hidratacion - 1.0) / 2.0 * 100.0))
            / 2.0
            - e.dia_alcohol * 15.0,
    );
    let mut entrenamiento: f64 = if e.entrenamientos <= 0 {
        40.0
    } else if e.intensidad >= 8.0 {
        55.0
    } else if e.intensidad >= 6.0 {
        75.0
    } else if e.intensidad >= 4.0 {
        70.0
    } else {
        60.0
    };
    if e.entrenamientos > 0 && e.duracion > 90.0 {
        entrenamiento -= 5.0;
    }
    if e.entrenamientos > 0 && e.duracion < 30.0 {
        entrenamiento -= 5.0;
    }
    let mental = clamp(
        (e.confianza + e.motivacion + e.concentracion) / 3.0 / 10.0 * 100.0
            - (e.estres + e.presion) / 2.0 / 10.0 * 30.0,
    );
    let preparacion = clamp(
        rendimiento * 0.30
            + habitos * 0.20
            + nutricion * 0.20
            + mental * 0.20
            + entrenamiento * 0.10,
    ) as i64;
    let riesgo = if e.lesiones > 0 {
        (70 + e.lesiones * 5).clamp(70, 100)
    } else {
        ((100 - preparacion) as f64 * 0.6 + e.estres * 3.0 + e.intensos_semana as f64 * 5.0 + 0.5)
            .clamp(0.0, 100.0) as i64
    };
    let nivel = match (e.lesiones, preparacion) {
        (0, p) if p >= 75 => 2,
        (0, p) if p >= 50 => 1,
        _ => 0,
    };
    let mut rutina = rutina_semanal(e.lesiones > 0, nivel).to_string();
    if riesgo >= 70 {
        rutina.push_str("\nAlerta: riesgo de lesión elevado. Prioriza recuperación.");
    }
    Asistente {
        preparacion,
        riesgo,
        nivel,
        parciales: [rendimiento, habitos, nutricion, mental, entrenamiento].map(|v| v as i64),
        resumen: format!(
            "Prep {preparacion}/100 | Riesgo {riesgo}/100 | Lesiones {} | Rend {:.1}",
            e.lesiones, e.rendimiento
        ),
        rutina,
        alerta_intensidad: (e.intensos_semana >= 4).then(|| {
            format!(
                "Alerta: Muchos días seguidos de alta intensidad (últimos 7 días: {}).",
                e.intensos_semana
            )
        }),
    }
}

fn rutina_semanal(lesion: bool, nivel: u8) -> &'static str {
    if lesion {
        return "Semana de recuperación (lesión activa):\n\
                Lun: movilidad y estiramientos 20 min\n\
                Mar: kinesiología o ejercicios indicados\n\
                Mié: descanso\n\
                Jue: core suave y movilidad 20 min\n\
                Vie: caminata ligera 30 min\n\
                Sáb/Dom: descanso y evaluación de molestias";
    }
    match nivel {
        2 => {
            "Carga alta:\n\
             Lun: fuerza tren inferior 45 min\n\
             Mar: técnica con balón 40 min\n\
             Mié: resistencia por intervalos 35 min\n\
             Jue: fuerza tren superior y core 40 min\n\
             Vie: velocidad y cambios de dirección 30 min\n\
             Sáb: partido o fútbol reducido\n\
             Dom: recuperación activa"
        }
        1 => {
            "Carga moderada:\n\
             Lun: fuerza general 35 min\n\
             Mar: técnica con balón 30 min\n\
             Mié: descanso o movilidad\n\
             Jue: resistencia aeróbica 30 min\n\
             Vie: técnica y definición 30 min\n\
             Sáb: partido\n\
             Dom: descanso"
        }
        _ => {
            "Carga baja:\n\
             Lun: movilidad y core 20 min\n\
             Mar: descanso\n\
             Mié: técnica suave 25 min\n\
             Jue: caminata o bici 30 min\n\
             Vie: descanso\n\
             Sáb: partido si te sentís bien\n\
             Dom: descanso"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn hoy() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 17).unwrap()
    }

    #[test]
    fn crud_converts_and_validates_values() {
        let db = Database::open_in_memory().unwrap();
        let textos = |v: &[&str]| v.iter().map(|t| (*t).to_string()).collect::<Vec<_>>();
        let id = guardar(
            db.conn(),
            Seccion::Entrenamientos,
            None,
            &textos(&["", "Fuerza", "45", "8", "No", "Piernas"]),
            hoy(),
        )
        .unwrap();
        let lista = listar(db.conn(), Seccion::Entrenamientos).unwrap();
        assert_eq!(lista[0].valores[0], "17/11/2026", "fecha vacía = hoy");
        assert_eq!(lista[0].valores[4], "No");
        alternar_omitido(db.conn(), id).unwrap();
        assert_eq!(
            listar(db.conn(), Seccion::Entrenamientos).unwrap()[0].valores[4],
            "Sí"
        );

        let fuera_de_rango = guardar(
            db.conn(),
            Seccion::Entrenamientos,
            None,
            &textos(&["", "Fuerza", "45", "11", "No", ""]),
            hoy(),
        );
        assert!(fuera_de_rango.is_err());
        assert!(
            guardar(
                db.conn(),
                Seccion::Comidas,
                None,
                &textos(&["", "Cena", "Excelente", ""]),
                hoy()
            )
            .is_err()
        );

        // El perfil de salud ya existe (fila 1) y solo se edita.
        guardar(
            db.conn(),
            Seccion::Salud,
            Some(1),
            &textos(&["180", "75,5", "0+", "", ""]),
            hoy(),
        )
        .unwrap();
        let salud = listar(db.conn(), Seccion::Salud).unwrap();
        assert_eq!(salud.len(), 1);
        assert_eq!(salud[0].valores[1], "75.5");
        assert!(eliminar(db.conn(), Seccion::Salud, 1).is_err());
        eliminar(db.conn(), Seccion::Entrenamientos, id).unwrap();
        assert!(
            listar(db.conn(), Seccion::Entrenamientos)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn asistente_formulas_match_mifutbolc() {
        let e = EntradasAsistente {
            rendimiento: 8.0,
            dormi: 1.0,
            hidratacion: 8.0,
            nervios: 2.0,
            confianza_habito: 8.0,
            motivacion_habito: 8.0,
            comida: 3.0,
            dia_hidratacion: 3.0,
            entrenamientos: 4,
            intensidad: 6.5,
            duracion: 60.0,
            confianza: 8.0,
            motivacion: 8.0,
            concentracion: 8.0,
            estres: 2.0,
            presion: 2.0,
            ..EntradasAsistente::default()
        };
        let a = calcular(&e);
        // rend 80, hábitos 84, nutrición 100, mental 74, entrenamiento 75.
        assert_eq!(a.parciales, [80, 84, 100, 74, 75]);
        // 24 + 16.8 + 20 + 14.8 + 7.5 = 83.1 → 83.
        assert_eq!(a.preparacion, 83);
        // (100-83)*0.6 + 2*3 + 0 + 0.5 = 16.7 → 16.
        assert_eq!(a.riesgo, 16);
        assert_eq!(a.nivel, 2);
        assert!(a.rutina.starts_with("Carga alta"));
        assert_eq!(
            a.resumen,
            "Prep 83/100 | Riesgo 16/100 | Lesiones 0 | Rend 8.0"
        );
        assert!(a.alerta_intensidad.is_none());
    }

    #[test]
    fn asistente_injury_and_intensity_alerts() {
        let lesionado = calcular(&EntradasAsistente {
            lesiones: 2,
            ..EntradasAsistente::default()
        });
        assert_eq!(lesionado.riesgo, 80);
        assert_eq!(lesionado.nivel, 0);
        assert!(lesionado.rutina.contains("recuperación"));
        assert!(
            lesionado
                .rutina
                .contains("Alerta: riesgo de lesión elevado")
        );

        let sin_datos = calcular(&EntradasAsistente {
            intensos_semana: 4,
            ..EntradasAsistente::default()
        });
        // Sin entrenamientos el puntaje de entrenamiento es 40.
        assert_eq!(sin_datos.parciales[4], 40);
        assert!(sin_datos.alerta_intensidad.unwrap().contains("7 días: 4"));
    }

    #[test]
    fn asistente_reads_recent_records() {
        let db = Database::open_in_memory().unwrap();
        let t = |v: &[&str]| v.iter().map(|x| (*x).to_string()).collect::<Vec<_>>();
        for dia in [
            "12/11/2026",
            "13/11/2026",
            "15/11/2026",
            "16/11/2026",
            "01/10/2026",
        ] {
            guardar(
                db.conn(),
                Seccion::Entrenamientos,
                None,
                &t(&[dia, "Fuerza", "60", "9", "No", ""]),
                hoy(),
            )
            .unwrap();
        }
        let a = asistente(db.conn(), hoy()).unwrap();
        // El del 01/10 queda fuera de los últimos 7 días.
        assert!(a.alerta_intensidad.unwrap().contains("7 días: 4"));
        guardar_recomendacion(db.conn(), hoy(), &asistente(db.conn(), hoy()).unwrap()).unwrap();
        assert_eq!(recomendaciones(db.conn()).unwrap().len(), 1);
    }

    fn t(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| (*x).to_string()).collect()
    }

    #[test]
    fn nutrition_averages_meals_and_daily_hydration_minus_alcohol() {
        let base = EntradasAsistente {
            comida: 3.0,
            ..EntradasAsistente::default()
        };
        // Sin días nutricionales la hidratación cuenta 0 (como MiFutbolC): (100 + 0) / 2.
        assert_eq!(calcular(&base).parciales[2], 50);
        // Comidas 100, hidratación media 50 → 75; alcohol la mitad de los días → -7.5.
        let con_dias = EntradasAsistente {
            dia_hidratacion: 2.0,
            dia_alcohol: 0.5,
            ..base
        };
        assert_eq!(calcular(&con_dias).parciales[2], 67);
    }

    #[test]
    fn references_daily_upsert_and_active_goal() {
        let db = Database::open_in_memory().unwrap();
        let c = db.conn();
        assert!(
            guardar(
                c,
                Seccion::Planes,
                None,
                &t(&["", "3", "Lunes fuerza", ""]),
                hoy()
            )
            .is_err(),
            "sin objetivo no hay plan"
        );
        let objetivo = guardar(
            c,
            Seccion::Objetivos,
            None,
            &t(&["Correr 10k", "", "31/12/2026", "Activo", ""]),
            hoy(),
        )
        .unwrap();
        guardar(
            c,
            Seccion::Planes,
            None,
            &t(&[&objetivo.to_string(), "3", "Lunes fuerza", ""]),
            hoy(),
        )
        .unwrap();
        let plan = &listar(c, Seccion::Planes).unwrap()[0];
        assert_eq!(
            plan.valores[0],
            objetivo.to_string(),
            "el formulario edita el id"
        );
        assert_eq!(plan.vista[0], "Correr 10k", "el listado muestra el nombre");
        assert_eq!(
            bienestar_repo::opciones(c, crate::domain::bienestar::Referencia::Objetivo).unwrap(),
            vec![(objetivo.to_string(), "Correr 10k".to_string())]
        );
        let a = asistente(c, hoy()).unwrap();
        assert!(
            a.rutina.starts_with("Objetivo: Correr 10k\n"),
            "{}",
            a.rutina
        );
        // Borrar el objetivo borra sus planes en cascada.
        eliminar(c, Seccion::Objetivos, objetivo).unwrap();
        assert!(listar(c, Seccion::Planes).unwrap().is_empty());

        // Un día nutricional por fecha: repetirla actualiza (ON CONFLICT de MiFutbolC).
        guardar(
            c,
            Seccion::Nutricion,
            None,
            &t(&["", "Baja", "No", "", ""]),
            hoy(),
        )
        .unwrap();
        guardar(
            c,
            Seccion::Nutricion,
            None,
            &t(&["", "Alta", "Sí", "72,5", "ok"]),
            hoy(),
        )
        .unwrap();
        let dias = listar(c, Seccion::Nutricion).unwrap();
        assert_eq!(dias.len(), 1);
        assert_eq!(dias[0].valores[1..4], t(&["Alta", "Sí", "72.5"]));
        assert!(
            guardar(
                c,
                Seccion::Nutricion,
                None,
                &t(&["", "Mucha", "No", "", ""]),
                hoy()
            )
            .is_err()
        );
        let e = bienestar_repo::entradas_asistente(c, "2026-11-17").unwrap();
        assert!((e.dia_hidratacion - 3.0).abs() < 1e-9 && (e.dia_alcohol - 1.0).abs() < 1e-9);
    }

    #[test]
    fn attachments_are_copied_to_images_dir() {
        let db = Database::open_in_memory().unwrap();
        let c = db.conn();
        let dir = tempfile::tempdir().unwrap();
        let imagenes = dir.path().join("Imagenes");
        let pdf = dir.path().join("analisis sangre.pdf");
        std::fs::write(&pdf, b"%PDF").unwrap();
        let control = guardar(
            c,
            Seccion::Controles,
            None,
            &t(&["", "Clínico", "Dra. Pérez", "OK", ""]),
            hoy(),
        )
        .unwrap();

        let mut textos = t(&[&control.to_string(), "", "", "", pdf.to_str().unwrap()]);
        adjuntar(Seccion::Estudios, &mut textos, &imagenes).unwrap();
        assert_eq!(textos[1], "analisis sangre.pdf");
        assert_eq!(textos[2], "documento");
        let copia = textos[4].strip_prefix("Imagenes/").unwrap();
        assert!(copia.starts_with(&format!("estudio_{control}_")) && copia.ends_with(".pdf"));
        assert_eq!(std::fs::read(imagenes.join(copia)).unwrap(), b"%PDF");
        guardar(c, Seccion::Estudios, None, &textos, hoy()).unwrap();
        let estudio = &listar(c, Seccion::Estudios).unwrap()[0];
        assert_eq!(estudio.vista[0], "2026-11-17 · Clínico");
        assert_eq!(estudio.valores[3], "17/11/2026");

        // Una ruta ya copiada se conserva; formatos y rutas inexistentes se rechazan.
        let mut otra = textos.clone();
        adjuntar(Seccion::Estudios, &mut otra, &imagenes).unwrap();
        assert_eq!(otra[4], textos[4]);
        let exe = dir.path().join("x.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        let mut mala = t(&[&control.to_string(), "", "", "", exe.to_str().unwrap()]);
        assert!(adjuntar(Seccion::Estudios, &mut mala, &imagenes).is_err());
        let mut falta = t(&["1", "", "", "", "C:/no/existe.png"]);
        assert!(adjuntar(Seccion::Estudios, &mut falta, &imagenes).is_err());

        // Imagen de menú: solo imágenes, una por menú (clave de texto, sin id).
        let png = dir.path().join("menu.PNG");
        std::fs::write(&png, b"png").unwrap();
        let mut menu = t(&["salud", png.to_str().unwrap()]);
        adjuntar(Seccion::Imagenes, &mut menu, &imagenes).unwrap();
        assert!(menu[1].starts_with("Imagenes/bienestar_salud_") && menu[1].ends_with(".png"));
        guardar(c, Seccion::Imagenes, None, &menu, hoy()).unwrap();
        guardar(c, Seccion::Imagenes, None, &menu, hoy()).unwrap();
        let menus = listar(c, Seccion::Imagenes).unwrap();
        assert_eq!(menus.len(), 1);
        let mut pdf_menu = t(&["salud", pdf.to_str().unwrap()]);
        assert!(adjuntar(Seccion::Imagenes, &mut pdf_menu, &imagenes).is_err());
        eliminar(c, Seccion::Imagenes, menus[0].id).unwrap();
        assert!(listar(c, Seccion::Imagenes).unwrap().is_empty());
    }
}
