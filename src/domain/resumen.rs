//! Datos del Resumen Compartible y sus plantillas (§17).

use chrono::NaiveDateTime;

use super::models::{RachaTipo, Resultado};
use crate::util::text::escape_html;

/// Datos completos del resumen.
#[derive(Debug, Clone, PartialEq)]
pub struct DatosResumen {
    /// Nombre del jugador.
    pub jugador: String,
    /// Partidos.
    pub total_partidos: i64,
    /// Goles.
    pub total_goles: i64,
    /// Asistencias.
    pub total_asistencias: i64,
    /// Promedio de goles.
    pub promedio_goles: f64,
    /// Promedio de asistencias.
    pub promedio_asistencias: f64,
    /// Fecha de generación.
    pub fecha_gen: NaiveDateTime,
    /// Logros (hitos + badges).
    pub logros_total: i64,
    /// Mejor partido.
    pub mejor: MejorPartido,
    /// Racha.
    pub racha: Racha,
    /// Favoritos.
    pub favoritos: Favoritos,
    /// Resultados.
    pub resultados: Resultados,
    /// Último partido.
    pub ultimo: UltimoPartido,
    /// Perfil de juego.
    pub perfil: PerfilJuego,
}

/// Mejor partido por rendimiento.
#[derive(Debug, Clone, PartialEq)]
pub struct MejorPartido {
    /// Rendimiento.
    pub rendimiento: i64,
    /// Fecha.
    pub fecha: NaiveDateTime,
}

/// Racha actual.
#[derive(Debug, Clone, PartialEq)]
pub struct Racha {
    /// Tipo.
    pub tipo: RachaTipo,
    /// Descripción.
    pub descripcion: String,
}

/// Datos favoritos.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Favoritos {
    /// Camiseta más usada.
    pub camiseta: String,
    /// Cancha más usada.
    pub cancha: String,
    /// Rival más frecuente.
    pub rival: String,
    /// Posición más jugada.
    pub posicion: String,
}

/// Resultados totales.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Resultados {
    /// Victorias.
    pub victorias: i64,
    /// Empates.
    pub empates: i64,
    /// Derrotas.
    pub derrotas: i64,
}

/// Último partido jugado.
#[derive(Debug, Clone, PartialEq)]
pub struct UltimoPartido {
    /// Fecha.
    pub fecha: NaiveDateTime,
    /// Rival.
    pub rival: String,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Resultado.
    pub resultado: Resultado,
}

/// Perfil de juego.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PerfilJuego {
    /// Clima más frecuente.
    pub clima: String,
    /// Torneo más frecuente (si hay datos).
    pub torneo: String,
    /// Lesiones.
    pub lesiones: String,
}

/// Formato de salida del resumen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResumenFormato {
    /// Markdown.
    #[default]
    Markdown,
    /// HTML autocontenido.
    Html,
    /// Texto plano.
    Texto,
}

impl ResumenFormato {
    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Html => "HTML",
            Self::Texto => "Texto",
        }
    }

    /// Extensión del archivo.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Html => "html",
            Self::Texto => "txt",
        }
    }
}

impl DatosResumen {
    /// Renderiza en Markdown.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Resumen de carrera — {}\n\n", self.jugador));
        out.push_str(&format!(
            "**Generado:** {}\n\n",
            self.fecha_gen.format("%Y-%m-%d %H:%M")
        ));
        out.push_str("| Métrica | Valor |\n|---|---|\n");
        out.push_str(&format!("| Partidos | {} |\n", self.total_partidos));
        out.push_str(&format!("| Goles | {} |\n", self.total_goles));
        out.push_str(&format!("| Asistencias | {} |\n", self.total_asistencias));
        out.push_str(&format!(
            "| Promedio goles | {:.2} |\n",
            self.promedio_goles
        ));
        out.push_str(&format!(
            "| Promedio asistencias | {:.2} |\n",
            self.promedio_asistencias
        ));
        out.push_str(&format!("| Logros | {} |\n", self.logros_total));
        out.push_str(&format!(
            "| Victorias / Empates / Derrotas | {} / {} / {} |\n",
            self.resultados.victorias, self.resultados.empates, self.resultados.derrotas
        ));
        out.push_str(&format!(
            "\n## Mejor partido\n{:.2} de rendimiento el {}.\n",
            self.mejor.rendimiento as f64,
            self.mejor.fecha.format("%d/%m/%Y")
        ));
        out.push_str(&format!("\n## Racha\n{}.\n", self.racha.descripcion));
        out.push_str("\n## Favoritos\n");
        out.push_str(&format!(
            "- Camiseta: {}\n",
            texto_o_guion(&self.favoritos.camiseta)
        ));
        out.push_str(&format!(
            "- Cancha: {}\n",
            texto_o_guion(&self.favoritos.cancha)
        ));
        out.push_str(&format!(
            "- Rival: {}\n",
            texto_o_guion(&self.favoritos.rival)
        ));
        out.push_str(&format!(
            "- Posición: {}\n",
            texto_o_guion(&self.favoritos.posicion)
        ));
        out.push_str(&format!(
            "\n## Último partido\n{} vs {} · {} G, {} A · {}.\n",
            self.ultimo.fecha.format("%d/%m/%Y"),
            texto_o_guion(&self.ultimo.rival),
            self.ultimo.goles,
            self.ultimo.asistencias,
            self.ultimo.resultado.label()
        ));
        out.push_str(&format!(
            "\n## Perfil de juego\n- Clima: {}\n- Torneo: {}\n- Lesiones: {}\n",
            texto_o_guion(&self.perfil.clima),
            texto_o_guion(&self.perfil.torneo),
            texto_o_guion(&self.perfil.lesiones)
        ));
        out
    }

    /// Renderiza un HTML autocontenido con escapes.
    #[must_use]
    pub fn to_html(&self) -> String {
        let jugador = escape_html(&self.jugador);
        let filas = [
            ("Partidos", self.total_partidos.to_string()),
            ("Goles", self.total_goles.to_string()),
            ("Asistencias", self.total_asistencias.to_string()),
            ("Promedio goles", format!("{:.2}", self.promedio_goles)),
            (
                "Promedio asistencias",
                format!("{:.2}", self.promedio_asistencias),
            ),
            ("Logros", self.logros_total.to_string()),
            (
                "Victorias / Empates / Derrotas",
                format!(
                    "{} / {} / {}",
                    self.resultados.victorias, self.resultados.empates, self.resultados.derrotas
                ),
            ),
        ];
        let tarjetas: String = filas
            .iter()
            .map(|(etiqueta, valor)| {
                format!(
                    "<div class=card><span class=label>{}</span><span class=value>{}</span></div>",
                    escape_html(etiqueta),
                    escape_html(valor)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "<!doctype html>\n<html lang=es>\n<head>\n<meta charset=utf-8>\n<meta name=viewport content=\"width=device-width, initial-scale=1\">\n<title>Resumen — {jugador}</title>\n<style>\n:root{{--bg:#f6f7fb;--fg:#1b1f2a;--muted:#5b6475;--accent:#2563eb;--card:#ffffff;--border:#e3e7ef;}}\n@media (prefers-color-scheme: dark){{:root{{--bg:#0f1420;--fg:#e6ecf7;--muted:#94a3b8;--accent:#60a5fa;--card:#161d2c;--border:#263143;}}}}\n*{{box-sizing:border-box}}\nbody{{margin:0;padding:32px;font-family:system-ui,Segoe UI,Roboto,sans-serif;background:var(--bg);color:var(--fg);}}\nh1{{margin:0 0 4px;font-size:28px}}\n.meta{{color:var(--muted);margin-bottom:24px}}\n.grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:12px;margin-bottom:24px}}\n.card{{background:var(--card);border:1px solid var(--border);border-radius:12px;padding:14px;display:flex;flex-direction:column;gap:6px}}\n.label{{color:var(--muted);font-size:12px;text-transform:uppercase;letter-spacing:.04em}}\n.value{{font-size:22px;font-weight:650}}\nsection{{background:var(--card);border:1px solid var(--border);border-radius:12px;padding:16px 18px;margin-bottom:16px}}\nh2{{margin:0 0 8px;font-size:16px;color:var(--accent)}}\np{{margin:0;line-height:1.5}}\nul{{margin:0;padding-left:18px;line-height:1.6}}\n</style>\n</head>\n<body>\n<h1>{jugador}</h1>\n<div class=meta>Generado el {fecha}</div>\n<div class=grid>\n{tarjetas}\n</div>\n<section><h2>Mejor partido</h2><p>{mejor_rendimiento:.2} de rendimiento el {mejor_fecha}.</p></section>\n<section><h2>Racha</h2><p>{racha}</p></section>\n<section><h2>Favoritos</h2><ul><li>Camiseta: {camiseta}</li><li>Cancha: {cancha}</li><li>Rival: {rival}</li><li>Posición: {posicion}</li></ul></section>\n<section><h2>Último partido</h2><p>{ultimo_fecha} vs {ultimo_rival} · {ultimo_goles} G, {ultimo_asistencias} A · {ultimo_resultado}.</p></section>\n<section><h2>Perfil de juego</h2><ul><li>Clima: {clima}</li><li>Torneo: {torneo}</li><li>Lesiones: {lesiones}</li></ul></section>\n</body>\n</html>\n",
            jugador = jugador,
            fecha = self.fecha_gen.format("%d/%m/%Y %H:%M"),
            tarjetas = tarjetas,
            mejor_rendimiento = self.mejor.rendimiento as f64,
            mejor_fecha = self.mejor.fecha.format("%d/%m/%Y"),
            racha = escape_html(&self.racha.descripcion),
            camiseta = escape_html(&texto_o_guion(&self.favoritos.camiseta)),
            cancha = escape_html(&texto_o_guion(&self.favoritos.cancha)),
            rival = escape_html(&texto_o_guion(&self.favoritos.rival)),
            posicion = escape_html(&texto_o_guion(&self.favoritos.posicion)),
            ultimo_fecha = self.ultimo.fecha.format("%d/%m/%Y"),
            ultimo_rival = escape_html(&texto_o_guion(&self.ultimo.rival)),
            ultimo_goles = self.ultimo.goles,
            ultimo_asistencias = self.ultimo.asistencias,
            ultimo_resultado = self.ultimo.resultado.label(),
            clima = escape_html(&texto_o_guion(&self.perfil.clima)),
            torneo = escape_html(&texto_o_guion(&self.perfil.torneo)),
            lesiones = escape_html(&texto_o_guion(&self.perfil.lesiones)),
        )
    }

    /// Renderiza en texto plano.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("RESUMEN DE CARRERA — {}\n", self.jugador));
        out.push_str(&format!(
            "Generado: {}\n\n",
            self.fecha_gen.format("%Y-%m-%d %H:%M")
        ));
        out.push_str(&format!("Partidos: {}\n", self.total_partidos));
        out.push_str(&format!("Goles: {}\n", self.total_goles));
        out.push_str(&format!("Asistencias: {}\n", self.total_asistencias));
        out.push_str(&format!("Promedio goles: {:.2}\n", self.promedio_goles));
        out.push_str(&format!(
            "Promedio asistencias: {:.2}\n",
            self.promedio_asistencias
        ));
        out.push_str(&format!(
            "V/E/D: {} / {} / {}\n",
            self.resultados.victorias, self.resultados.empates, self.resultados.derrotas
        ));
        out.push_str(&format!("Logros: {}\n", self.logros_total));
        out.push_str(&format!(
            "\nMejor partido: {:.2} ({})\n",
            self.mejor.rendimiento as f64,
            self.mejor.fecha.format("%d/%m/%Y")
        ));
        out.push_str(&format!("Racha: {}\n", self.racha.descripcion));
        out.push_str(&format!(
            "Favoritos: camiseta {}, cancha {}, rival {}, posición {}\n",
            texto_o_guion(&self.favoritos.camiseta),
            texto_o_guion(&self.favoritos.cancha),
            texto_o_guion(&self.favoritos.rival),
            texto_o_guion(&self.favoritos.posicion)
        ));
        out
    }

    /// Nombre de archivo sugerido.
    #[must_use]
    pub fn nombre_archivo(&self, formato: ResumenFormato) -> String {
        let jugador: String = self
            .jugador
            .chars()
            .map(|ch| if ch.is_alphanumeric() { ch } else { '_' })
            .collect();
        format!(
            "Resumen_{}_{}.{}",
            jugador.trim_matches('_'),
            self.fecha_gen.format("%Y%m%d_%H%M%S"),
            formato.extension()
        )
    }
}

/// Formatea un texto vacío como guion.
#[must_use]
pub fn texto_o_guion(texto: &str) -> String {
    if texto.trim().is_empty() {
        "—".to_string()
    } else {
        texto.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn datos() -> DatosResumen {
        DatosResumen {
            jugador: "Ana <b>".into(),
            total_partidos: 10,
            total_goles: 7,
            total_asistencias: 3,
            promedio_goles: 0.7,
            promedio_asistencias: 0.3,
            fecha_gen: NaiveDate::from_ymd_opt(2026, 5, 10)
                .unwrap()
                .and_hms_opt(20, 0, 0)
                .unwrap(),
            logros_total: 4,
            mejor: MejorPartido {
                rendimiento: 10,
                fecha: NaiveDate::from_ymd_opt(2026, 4, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            },
            racha: Racha {
                tipo: RachaTipo::Positiva,
                descripcion: "3 victorias seguidas".into(),
            },
            favoritos: Favoritos {
                camiseta: "Titular".into(),
                cancha: "Sur".into(),
                rival: "Rival".into(),
                posicion: "Mediocampista".into(),
            },
            resultados: Resultados {
                victorias: 6,
                empates: 2,
                derrotas: 2,
            },
            ultimo: UltimoPartido {
                fecha: NaiveDate::from_ymd_opt(2026, 5, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                rival: "Rival".into(),
                goles: 2,
                asistencias: 1,
                resultado: Resultado::Victoria,
            },
            perfil: PerfilJuego {
                clima: "Despejado".into(),
                torneo: String::new(),
                lesiones: "2".into(),
            },
        }
    }

    #[test]
    fn markdown_contains_tables_and_sections() {
        let texto = datos().to_markdown();
        assert!(texto.contains("# Resumen de carrera — Ana <b>"));
        assert!(texto.contains("| Partidos | 10 |"));
        assert!(texto.contains("## Mejor partido"));
        assert!(texto.contains("## Favoritos"));
    }

    #[test]
    fn html_escapes_content_and_is_self_contained() {
        let html = datos().to_html();
        assert!(html.contains("Ana &lt;b&gt;"));
        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("prefers-color-scheme"));
        assert!(!html.contains("<b>"));
        assert!(html.contains("3 victorias seguidas"));
    }

    #[test]
    fn file_name_is_sanitized() {
        let nombre = datos().nombre_archivo(ResumenFormato::Html);
        assert!(nombre.starts_with("Resumen_Ana__b_20260510_200000."));
        assert!(nombre.ends_with(".html"));
        assert!(!nombre.contains('<'));
    }
}
