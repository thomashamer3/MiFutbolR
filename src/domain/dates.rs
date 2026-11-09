//! Fechas, horas y formatos legacy.
//!
//! ADR-08: en la base de datos las fechas son ISO-8601
//! (`YYYY-MM-DD` o `YYYY-MM-DD HH:MM:SS`). La UI muestra `DD/MM/YYYY`.
//! El proyecto C usa formatos mixtos; aquí se toleran al importar.

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

/// Formatos de fecha aceptados en entradas del usuario.
pub const USER_DATE_INPUT: &str = "%d/%m/%Y";

/// Formato canónico de fecha en la BD.
pub const ISO_DATE: &str = "%Y-%m-%d";

/// Formato canónico de fecha-hora en la BD.
pub const ISO_DATETIME: &str = "%Y-%m-%d %H:%M:%S";

const DATE_FORMATS: &[&str] = &["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%Y/%m/%d"];

const DATETIME_FORMATS: &[&str] = &[
    "%Y-%m-%d %H:%M:%S",
    "%Y-%m-%d %H:%M",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%dT%H:%M",
    "%d/%m/%Y %H:%M:%S",
    "%d/%m/%Y %H:%M",
    "%d/%m/%Y",
    "%Y-%m-%d",
];

/// Parsea una fecha tolerando ISO y `DD/MM/YYYY`.
#[must_use]
pub fn parse_date(input: &str) -> Option<NaiveDate> {
    let text = input.trim();
    if text.is_empty() {
        return None;
    }
    DATE_FORMATS
        .iter()
        .find_map(|fmt| NaiveDate::parse_from_str(text, fmt).ok())
}

/// Parsea fecha-hora tolerando ISO, `DD/MM/YYYY HH:MM` y fecha sola.
#[must_use]
pub fn parse_datetime(input: &str) -> Option<NaiveDateTime> {
    let text = input.trim();
    if text.is_empty() {
        return None;
    }
    DATETIME_FORMATS
        .iter()
        .find_map(|fmt| NaiveDateTime::parse_from_str(text, fmt).ok())
        .or_else(|| parse_date(text).and_then(|d| d.and_hms_opt(0, 0, 0)))
}

/// Parsea una hora `HH:MM` (o `HH:MM:SS`).
#[must_use]
pub fn parse_time(input: &str) -> Option<NaiveTime> {
    let text = input.trim();
    NaiveTime::parse_from_str(text, "%H:%M:%S")
        .ok()
        .or_else(|| NaiveTime::parse_from_str(text, "%H:%M").ok())
}

/// Serializa una fecha a ISO para la BD.
#[must_use]
pub fn to_iso_date(date: NaiveDate) -> String {
    date.format(ISO_DATE).to_string()
}

/// Serializa una fecha-hora a ISO (con segundos) para la BD.
#[must_use]
pub fn to_iso_datetime(datetime: NaiveDateTime) -> String {
    datetime.format(ISO_DATETIME).to_string()
}

/// Describe el tiempo transcurrido entre dos fechas.
#[must_use]
pub fn tiempo_transcurrido(desde: NaiveDateTime, ahora: NaiveDateTime) -> String {
    let segundos = (ahora - desde).num_seconds().max(0);
    if segundos < 60 {
        return "hace instantes".to_string();
    }
    let minutos = segundos / 60;
    if minutos < 60 {
        return format!("hace {minutos} min");
    }
    let horas = minutos / 60;
    if horas < 24 {
        return format!("hace {horas} h");
    }
    let dias = horas / 24;
    let anios = dias / 365;
    let meses = (dias % 365) / 30;
    let dias_restantes = dias % 30;
    let mut partes = Vec::new();
    if anios > 0 {
        partes.push(format!("{anios} a"));
    }
    if meses > 0 {
        partes.push(format!("{meses} m"));
    }
    if dias_restantes > 0 || partes.is_empty() {
        partes.push(format!("{dias_restantes} d"));
    }
    format!("hace {}", partes.join(" "))
}

/// Formato de fecha para la UI: `DD/MM/YYYY`.
#[must_use]
pub fn display_date(date: NaiveDate) -> String {
    date.format(USER_DATE_INPUT).to_string()
}

/// Formato de fecha-hora para la UI: `DD/MM/YYYY HH:MM`.
#[must_use]
pub fn display_datetime(datetime: NaiveDateTime) -> String {
    datetime.format("%d/%m/%Y %H:%M").to_string()
}

/// Clave `YYYY-MM` de un partido (columna legacy `mes_anio`).
#[must_use]
pub fn month_key(datetime: NaiveDateTime) -> String {
    format!("{:04}-{:02}", datetime.year(), datetime.month())
}

/// Clave `YYYY-MM` a partir de un texto ya normalizado.
#[must_use]
pub fn month_key_from_date(date: NaiveDate) -> String {
    format!("{:04}-{:02}", date.year(), date.month())
}

/// Franja horaria codificada como en el legacy (`partido.dia`).
///
/// `1` madrugada, `2` mañana, `3` mediodía, `4` tarde, `5` atardecer,
/// `6` noche.
#[must_use]
pub fn franja_code(datetime: NaiveDateTime) -> i64 {
    match datetime.hour() {
        0..=5 => 1,
        6..=11 => 2,
        12..=14 => 3,
        15..=18 => 4,
        19..=20 => 5,
        _ => 6,
    }
}

/// Hora actual local como `NaiveDateTime` (sin dependencia de zona horaria).
#[must_use]
pub fn now_local() -> NaiveDateTime {
    chrono::Local::now().naive_local()
}

/// Fecha de hoy local.
#[must_use]
pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_iso_and_local_dates() {
        let expected = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        assert_eq!(parse_date("2026-11-06"), Some(expected));
        assert_eq!(parse_date("06/11/2026"), Some(expected));
        assert_eq!(parse_date("06-11-2026"), Some(expected));
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("2026-13-01"), None);
    }

    #[test]
    fn parses_legacy_datetimes() {
        let expected = NaiveDate::from_ymd_opt(2026, 11, 6)
            .unwrap()
            .and_hms_opt(21, 30, 0)
            .unwrap();
        assert_eq!(parse_datetime("2026-11-06 21:30"), Some(expected));
        assert_eq!(parse_datetime("2026-11-06 21:30:00"), Some(expected));
        assert_eq!(parse_datetime("06/11/2026 21:30"), Some(expected));
        assert_eq!(
            parse_datetime("2026-11-06"),
            Some(
                NaiveDate::from_ymd_opt(2026, 11, 6)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
            )
        );
    }

    #[test]
    fn formats_iso_for_db() {
        let dt = NaiveDate::from_ymd_opt(2026, 1, 2)
            .unwrap()
            .and_hms_opt(3, 4, 5)
            .unwrap();
        assert_eq!(to_iso_datetime(dt), "2026-01-02 03:04:05");
        assert_eq!(to_iso_date(dt.date()), "2026-01-02");
        assert_eq!(display_datetime(dt), "02/01/2026 03:04");
    }

    #[test]
    fn month_key_matches_legacy() {
        let dt = NaiveDate::from_ymd_opt(2026, 7, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert_eq!(month_key(dt), "2026-07");
    }

    #[test]
    fn franja_boundaries() {
        let at = |h: u32| {
            NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(h, 0, 0)
                .unwrap()
        };
        assert_eq!(franja_code(at(0)), 1);
        assert_eq!(franja_code(at(5)), 1);
        assert_eq!(franja_code(at(6)), 2);
        assert_eq!(franja_code(at(11)), 2);
        assert_eq!(franja_code(at(12)), 3);
        assert_eq!(franja_code(at(14)), 3);
        assert_eq!(franja_code(at(15)), 4);
        assert_eq!(franja_code(at(18)), 4);
        assert_eq!(franja_code(at(19)), 5);
        assert_eq!(franja_code(at(20)), 5);
        assert_eq!(franja_code(at(21)), 6);
        assert_eq!(franja_code(at(23)), 6);
    }
}

#[cfg(test)]
mod tiempo_tests {
    use super::*;

    #[test]
    fn elapsed_text() {
        let base = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(tiempo_transcurrido(base, base), "hace instantes");
        assert_eq!(
            tiempo_transcurrido(base, base + chrono::Duration::minutes(5)),
            "hace 5 min"
        );
        assert_eq!(
            tiempo_transcurrido(base, base + chrono::Duration::hours(3)),
            "hace 3 h"
        );
        let texto = tiempo_transcurrido(base, base + chrono::Duration::days(400));
        assert!(texto.contains("1 a"));
    }
}
