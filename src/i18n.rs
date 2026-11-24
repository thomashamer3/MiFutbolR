//! Idioma de la interfaz (port de `lang.c` + `langs/en.json` de MiFutbolC).
//!
//! C carga una tabla `clave → texto` por idioma y `tr(clave)` la consulta. Aquí
//! el español sigue escrito en el código (es el idioma por defecto) y solo se
//! guarda la tabla inglesa, con las claves de C, para lo que C traduce: el menú
//! principal, "Salir"/"Volver" y las etiquetas de Ajustes.

use crate::domain::models::Language;

/// Textos en inglés de `langs/en.json` (C no traduce Botines ni Tiendas: su
/// `en.json` no tiene `menu_botines` y deja "Tiendas"; aquí se completan).
const EN: &[(&str, &str)] = &[
    ("menu_calendario", "Calendar"),
    ("menu_camisetas", "Shirts"),
    ("menu_canchas", "Pitches"),
    ("menu_botines", "Boots"),
    ("menu_equipos", "Teams"),
    ("menu_partidos", "Matches"),
    ("menu_lesiones", "Injuries"),
    ("menu_estadisticas", "Statistics"),
    ("menu_logros", "Achievements"),
    ("menu_financiamiento", "Financing"),
    ("menu_torneos", "Tournaments"),
    ("menu_temporada", "Season"),
    ("menu_analisis", "Analysis"),
    ("menu_bienestar", "Wellness"),
    ("menu_carrera", "Career"),
    ("menu_recordatorios", "Reminders"),
    ("menu_colecciones", "Collections"),
    ("menu_records_rankings", "Records & Rankings"),
    ("menu_tiendas", "Shops"),
    ("menu_media", "Media References"),
    ("menu_tutorial", "Tutorial"),
    ("menu_resumen_compartible", "Shareable Summary"),
    ("menu_ranking_amigos", "Friends Ranking"),
    ("menu_settings", "Settings"),
    ("menu_sincronizacion", "Sync (PocketBase)"),
    ("menu_progresion", "Progression"),
    ("menu_entrenador_ia", "Virtual Coach (AI)"),
    ("menu_usuario", "User"),
    ("menu_exit", "Exit"),
    ("menu_back", "Back"),
    ("menu_title", "Menu"),
    ("settings_theme", "Interface Theme"),
    ("settings_language", "Language"),
    ("settings_mode", "Mode"),
    ("settings_text_size", "Text size"),
];

/// Texto de `clave` en `idioma` (`tr` de C). En español, o si la clave no está
/// traducida, devuelve `espanol`.
#[must_use]
pub fn t<'a>(idioma: Language, clave: &str, espanol: &'a str) -> &'a str {
    match idioma {
        Language::Espanol => espanol,
        Language::English => EN
            .iter()
            .find(|(k, _)| *k == clave)
            .map_or(espanol, |(_, texto)| texto),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn espanol_por_defecto_e_ingles_con_respaldo() {
        assert_eq!(t(Language::Espanol, "menu_exit", "Salir"), "Salir");
        assert_eq!(t(Language::English, "menu_exit", "Salir"), "Exit");
        assert_eq!(t(Language::English, "sin_clave", "Hola"), "Hola");
        // Cada opción del menú tiene su traducción.
        for module in crate::app::Module::ALL {
            let clave = format!("menu_{}", module.key());
            assert!(EN.iter().any(|(k, _)| *k == clave), "{clave}");
        }
    }
}
