//! Escritor PDF 1.4 mínimo para informes de texto (sin dependencias).
//!
//! Helvetica y Helvetica-Bold (fuentes base, no se incrustan) con
//! WinAnsiEncoding: el texto se pasa a cp1252, así los acentos, la ñ, ¿ y ¡ se
//! ven bien. shortcut: el corte de línea estima el ancho por cantidad de
//! caracteres (sin métricas de fuente); alcanza para texto corrido.

const ANCHO: f32 = 595.0;
const ALTO: f32 = 842.0;
const MARGEN: f32 = 40.0;
const CUERPO: f32 = 10.0;
const INTERLINEA: f32 = 14.0;
const PIE: f32 = 30.0;
/// Azul de la portada de MiFutbolC (26, 89, 166).
const AZUL: (f32, f32, f32) = (26.0 / 255.0, 89.0 / 255.0, 166.0 / 255.0);

/// Sección del informe: título y líneas de texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seccion {
    /// Título de la sección.
    pub titulo: String,
    /// Líneas (se cortan solas si son largas).
    pub lineas: Vec<String>,
}

/// Byte cp1252 de un carácter (`?` si no existe en esa codificación).
fn cp1252(c: char) -> u8 {
    match c as u32 {
        0x20..=0x7E | 0xA0..=0xFF => c as u32 as u8,
        _ => match c {
            '€' => 0x80,
            '…' => 0x85,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' | '·' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            _ => b'?',
        },
    }
}

/// Literal de cadena PDF: escapa `( ) \` y pasa lo no ASCII a octal.
fn literal(texto: &str) -> String {
    let mut salida = String::from("(");
    for c in texto.chars() {
        match cp1252(c) {
            b'(' => salida.push_str("\\("),
            b')' => salida.push_str("\\)"),
            b'\\' => salida.push_str("\\\\"),
            byte if byte < 0x80 => salida.push(byte as char),
            byte => salida.push_str(&format!("\\{byte:03o}")),
        }
    }
    salida.push(')');
    salida
}

/// Corta una línea en trozos de como mucho `maximo` caracteres, por palabras.
fn cortar(linea: &str, maximo: usize) -> Vec<String> {
    let mut trozos = Vec::new();
    let mut actual = String::new();
    for palabra in linea.split_whitespace() {
        let largo = actual.chars().count();
        if largo > 0 && largo + 1 + palabra.chars().count() > maximo {
            trozos.push(std::mem::take(&mut actual));
        }
        if !actual.is_empty() {
            actual.push(' ');
        }
        actual.push_str(palabra);
    }
    trozos.push(actual);
    trozos
}

fn texto(salida: &mut String, fuente: &str, tamano: f32, x: f32, y: f32, contenido: &str) {
    salida.push_str(&format!(
        "BT /{fuente} {tamano} Tf {x:.1} {y:.1} Td {} Tj ET\n",
        literal(contenido)
    ));
}

/// Genera el PDF: portada con banda azul y título, líneas de encabezado y secciones.
#[must_use]
pub fn informe(titulo: &str, encabezado: &[String], secciones: &[Seccion]) -> Vec<u8> {
    // Caracteres por línea estimados para Helvetica 10 pt (~0,52 em de promedio).
    let maximo = ((ANCHO - 2.0 * MARGEN) / (CUERPO * 0.52)) as usize;
    let mut paginas: Vec<String> = Vec::new();
    let mut pagina = String::new();

    // Portada.
    pagina.push_str(&format!(
        "{:.3} {:.3} {:.3} rg 0 {:.1} {ANCHO} 140 re f\n",
        AZUL.0,
        AZUL.1,
        AZUL.2,
        ALTO - 140.0
    ));
    pagina.push_str("1 1 1 rg\n");
    texto(&mut pagina, "F2", 24.0, MARGEN, ALTO - 80.0, titulo);
    pagina.push_str("0 0 0 rg\n");
    let mut y = ALTO - 170.0;
    for linea in encabezado {
        texto(&mut pagina, "F1", CUERPO, MARGEN, y, linea);
        y -= INTERLINEA;
    }
    y -= INTERLINEA;

    let mut nueva_pagina = |pagina: &mut String, y: &mut f32| {
        paginas.push(std::mem::take(pagina));
        *y = ALTO - MARGEN;
    };
    for seccion in secciones {
        if y < MARGEN + PIE + 3.0 * INTERLINEA {
            nueva_pagina(&mut pagina, &mut y);
        }
        pagina.push_str(&format!(
            "0.92 0.94 0.97 rg {MARGEN} {:.1} {:.1} 18 re f 0 0 0 rg\n",
            y - 5.0,
            ANCHO - 2.0 * MARGEN
        ));
        texto(&mut pagina, "F2", 12.0, MARGEN + 4.0, y, &seccion.titulo);
        y -= INTERLINEA + 6.0;
        for linea in &seccion.lineas {
            for trozo in cortar(linea, maximo) {
                if y < MARGEN + PIE {
                    nueva_pagina(&mut pagina, &mut y);
                }
                texto(&mut pagina, "F1", CUERPO, MARGEN, y, &trozo);
                y -= INTERLINEA;
            }
        }
        y -= INTERLINEA / 2.0;
    }
    paginas.push(pagina);

    // Objetos: 1 catálogo, 2 páginas, 3-4 fuentes, luego (página, contenido) por hoja.
    let total = paginas.len();
    let mut objetos: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {total} >>",
            (0..total)
                .map(|i| format!("{} 0 R", 5 + i * 2))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".into(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .into(),
    ];
    for (indice, mut contenido) in paginas.into_iter().enumerate() {
        texto(
            &mut contenido,
            "F1",
            8.0,
            MARGEN,
            MARGEN / 2.0,
            &format!("MiFutbolR - Página {} de {total}", indice + 1),
        );
        objetos.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {ANCHO} {ALTO}] \
             /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
            6 + indice * 2
        ));
        objetos.push(format!(
            "<< /Length {} >>\nstream\n{contenido}endstream",
            contenido.len()
        ));
    }

    let mut salida = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objetos.len());
    for (indice, objeto) in objetos.iter().enumerate() {
        offsets.push(salida.len());
        salida.extend_from_slice(format!("{} 0 obj\n{objeto}\nendobj\n", indice + 1).as_bytes());
    }
    let xref = salida.len();
    salida.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objetos.len() + 1).as_bytes(),
    );
    for offset in offsets {
        salida.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    salida.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objetos.len() + 1
        )
        .as_bytes(),
    );
    salida
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texto_pdf(bytes: &[u8]) -> String {
        // Un carácter por byte, así los offsets del xref valen como índices.
        bytes
            .iter()
            .map(|b| if b.is_ascii() { *b as char } else { '?' })
            .collect()
    }

    #[test]
    fn xref_offsets_point_to_objects_and_pages_are_counted() {
        let secciones = vec![Seccion {
            titulo: "Partidos".into(),
            lineas: (0..120)
                .map(|i| format!("Partido {i} · ¿Ganó? ¡Sí! Ñandú (bien)"))
                .collect(),
        }];
        let bytes = informe("MiFutbolR", &["Usuario: Ana".into()], &secciones);
        let pdf = texto_pdf(&bytes);
        assert!(pdf.starts_with("%PDF-1.4"));
        assert!(pdf.trim_end().ends_with("%%EOF"));

        let startxref: usize = pdf
            .rsplit("startxref\n")
            .next()
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(pdf[startxref..].starts_with("xref"));
        let entradas: Vec<&str> = pdf[startxref..]
            .lines()
            .skip(3)
            .take_while(|l| l.ends_with(" n "))
            .collect();
        for (indice, entrada) in entradas.iter().enumerate() {
            let offset: usize = entrada[..10].parse().unwrap();
            assert!(
                pdf[offset..].starts_with(&format!("{} 0 obj", indice + 1)),
                "objeto {}",
                indice + 1
            );
        }
        // 120 líneas no entran en una página: hay más de una y /Count coincide.
        let paginas = pdf.matches("/Type /Page ").count();
        assert!(paginas > 1);
        assert!(pdf.contains(&format!("/Count {paginas}")));
        assert!(pdf.contains(&format!("Página {paginas} de {paginas}").replace('á', "\\341")));
    }

    #[test]
    fn spanish_text_is_cp1252_and_escaped() {
        assert_eq!(
            literal("Ñandú (¿sí?) \\"),
            "(\\321and\\372 \\(\\277s\\355?\\) \\\\)"
        );
        assert_eq!(literal("⚽"), "(?)");
        assert_eq!(cortar("uno dos tres", 7), vec!["uno dos", "tres"]);
    }
}
