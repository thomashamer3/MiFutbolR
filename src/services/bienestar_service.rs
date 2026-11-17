//! Bienestar: CRUD por sección y asistente de preparación (fórmulas de
//! MiFutbolC, `bienestar.c`).

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
    })
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
    Ok(calcular(&entradas))
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

/// Fórmulas del asistente de MiFutbolC (`bienestar.c`).
///
/// Diferencia documentada: MiFutbolC promedia comidas y "día nutricional"
/// (hidratación + alcohol) para la nutrición; ese registro no se portó, así que
/// la nutrición usa solo la calidad de las comidas (el alcohol ya penaliza en
/// hábitos).
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
    let nutricion = clamp((e.comida - 1.0) / 2.0 * 100.0);
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
}
