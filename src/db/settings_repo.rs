//! Repositorio de preferencias (`settings`).

use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::models::{AppSettings, Language, Mode, TextSize, Theme};

/// Carga las preferencias; si faltan, devuelve los valores por defecto.
pub fn load(conn: &Connection) -> Result<AppSettings, DbError> {
    let row = conn
        .query_row(
            "SELECT theme, language, mode, text_size, image_viewer, botin_predeterminado
             FROM settings WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()?;

    let Some((theme, language, mode, text_size, image_viewer, botin)) = row else {
        return Ok(AppSettings::default());
    };

    Ok(AppSettings {
        theme: Theme::try_from(theme).unwrap_or(Theme::Dark),
        language: Language::try_from(language).unwrap_or(Language::Espanol),
        mode: Mode::try_from(mode).unwrap_or(Mode::Simple),
        text_size: TextSize::try_from(text_size).unwrap_or(TextSize::Medium),
        image_viewer,
        botin_predeterminado: botin,
    })
}

/// ¿El tutorial de inicio ya se completó?
pub fn tutorial_completado(conn: &Connection) -> Result<bool, DbError> {
    let valor = conn
        .query_row(
            "SELECT IFNULL(tutorial_completado, 0) FROM settings WHERE id = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .unwrap_or(0);
    Ok(valor != 0)
}

/// Marca el tutorial como completado.
pub fn marcar_tutorial(conn: &Connection) -> Result<(), DbError> {
    conn.execute(
        "UPDATE settings SET tutorial_completado = 1 WHERE id = 1",
        [],
    )?;
    Ok(())
}

/// Guarda las preferencias (actualiza o crea la fila única).
pub fn save(conn: &Connection, settings: &AppSettings) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE settings
            SET theme = ?1, language = ?2, mode = ?3, text_size = ?4,
                image_viewer = ?5, botin_predeterminado = ?6,
                updated_at = datetime('now','localtime')
          WHERE id = 1",
        params![
            settings.theme.as_i64(),
            settings.language.as_i64(),
            settings.mode.as_i64(),
            settings.text_size.as_i64(),
            settings.image_viewer,
            settings.botin_predeterminado,
        ],
    )?;
    if updated == 0 {
        conn.execute(
            "INSERT INTO settings
                (id, theme, language, mode, text_size, image_viewer, botin_predeterminado, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, datetime('now','localtime'))",
            params![
                settings.theme.as_i64(),
                settings.language.as_i64(),
                settings.mode.as_i64(),
                settings.text_size.as_i64(),
                settings.image_viewer,
                settings.botin_predeterminado,
            ],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn load_returns_seeded_defaults() {
        let db = Database::open_in_memory().unwrap();
        let settings = load(db.conn()).unwrap();
        assert_eq!(settings.theme, Theme::Light);
        assert_eq!(settings.language, Language::Espanol);
    }

    #[test]
    fn save_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let mut settings = load(db.conn()).unwrap();
        settings.theme = Theme::HighContrast;
        settings.text_size = TextSize::Large;
        save(db.conn(), &settings).unwrap();
        let loaded = load(db.conn()).unwrap();
        assert_eq!(loaded.theme, Theme::HighContrast);
        assert_eq!(loaded.text_size, TextSize::Large);
    }
}
