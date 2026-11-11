//! Normalización y recorte de texto para búsquedas y UI.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Normaliza para búsquedas: minúsculas, sin tildes y con espacios colapsados.
#[must_use]
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_space = true;
    for ch in text.trim().chars() {
        let lower = ch.to_lowercase().next().unwrap_or(ch);
        let mapped = match lower {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'è'..='ë' => 'e',
            'ì'..='ï' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ù'..='ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        };
        if mapped.is_whitespace() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else {
            out.push(mapped);
            last_space = false;
        }
    }
    out
}

/// Recorta a un ancho de terminal, agregando `…` si hace falta.
#[must_use]
pub fn truncate_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text.width() <= max_width {
        return text.to_string();
    }
    let mut out = String::new();
    let mut width = 0;
    let budget = max_width.saturating_sub(1);
    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if width + ch_width > budget {
            break;
        }
        out.push(ch);
        width += ch_width;
    }
    out.push('…');
    out
}

/// Escapa texto para insertarlo en HTML (Resumen Compartible).
#[must_use]
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// Une dos textos con un separador, omitiendo vacíos.
#[must_use]
pub fn join_non_empty(left: &str, right: &str, separator: &str) -> String {
    match (left.trim().is_empty(), right.trim().is_empty()) {
        (true, true) => String::new(),
        (true, false) => right.trim().to_string(),
        (false, true) => left.trim().to_string(),
        (false, false) => format!("{}{}{}", left.trim(), separator, right.trim()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_removes_accents_and_case() {
        assert_eq!(normalize("  Cancha N°1 — Peñarol "), "cancha n°1 — penarol");
        assert_eq!(normalize("ÁRBITRO"), "arbitro");
        assert_eq!(normalize("a   b"), "a b");
    }

    #[test]
    fn truncate_respects_width() {
        assert_eq!(truncate_width("hola", 10), "hola");
        assert_eq!(truncate_width("hola mundo", 8), "hola mu…");
        assert_eq!(truncate_width("hola", 0), "");
    }

    #[test]
    fn escape_html_blocks_tags() {
        assert_eq!(
            escape_html("<script>alert('x')</script>"),
            "&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"
        );
    }

    #[test]
    fn join_skips_empty_parts() {
        assert_eq!(join_non_empty("a", "", " · "), "a");
        assert_eq!(join_non_empty("", "b", " · "), "b");
        assert_eq!(join_non_empty("a", "b", " · "), "a · b");
    }
}
