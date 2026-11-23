//! Libro de Excel (.xlsx) real: paquete OOXML mínimo con cadenas en línea.
//!
//! MiFutbolC metía un XML de Excel 2003 (`<Workbook>` SpreadsheetML) dentro de
//! un ZIP con extensión .xlsx; Excel y LibreOffice lo rechazan. Acá van las
//! partes que exige el formato: `[Content_Types].xml`, `_rels/.rels`,
//! `xl/workbook.xml`, sus relaciones, `xl/styles.xml` y una hoja por tabla.

use super::ods::{Hoja, xml};
use super::zip::zip_stored;

const NS_MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const NS_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const NS_PKG_REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const CT_HOJA: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";

/// Estilos: 0 normal, 1 negrita (cabeceras).
const STYLES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
<fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font>\
<font><b/><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts>\
<fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill>\
<fill><patternFill patternType=\"gray125\"/></fill></fills>\
<borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders>\
<cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs>\
<cellXfs count=\"2\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/>\
<xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/></cellXfs>\
</styleSheet>";

/// Letra de columna: 0 → A, 25 → Z, 26 → AA.
fn columna(mut indice: usize) -> String {
    let mut letras = Vec::new();
    loop {
        letras.push(b'A' + (indice % 26) as u8);
        if indice < 26 {
            break;
        }
        indice = indice / 26 - 1;
    }
    letras.iter().rev().map(|b| *b as char).collect()
}

fn celda(referencia: &str, valor: &str, estilo: u8) -> String {
    let estilo = if estilo == 0 {
        String::new()
    } else {
        format!(" s=\"{estilo}\"")
    };
    // Números como número para que la planilla pueda sumar y ordenar (como el ODS).
    match valor.trim().parse::<f64>() {
        Ok(numero) if numero.is_finite() && estilo.is_empty() => {
            format!("<c r=\"{referencia}\"><v>{numero}</v></c>")
        }
        _ => format!(
            "<c r=\"{referencia}\" t=\"inlineStr\"{estilo}><is><t xml:space=\"preserve\">{}</t></is></c>",
            xml(valor)
        ),
    }
}

fn hoja_xml(hoja: &Hoja) -> String {
    let mut salida = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"{NS_MAIN}\"><sheetData>"
    );
    let filas = std::iter::once((&hoja.cabeceras, 1)).chain(hoja.filas.iter().map(|f| (f, 0)));
    for (numero, (fila, estilo)) in filas.enumerate() {
        salida.push_str(&format!("<row r=\"{}\">", numero + 1));
        for (col, valor) in fila.iter().enumerate() {
            let referencia = format!("{}{}", columna(col), numero + 1);
            salida.push_str(&celda(&referencia, valor, estilo));
        }
        salida.push_str("</row>");
    }
    salida.push_str("</sheetData></worksheet>");
    salida
}

/// Excel limita el nombre de hoja a 31 caracteres y prohíbe `[]:*?/\`.
fn nombre_hoja(nombre: &str) -> String {
    nombre
        .chars()
        .map(|c| if "[]:*?/\\".contains(c) { '_' } else { c })
        .take(31)
        .collect()
}

/// Bytes del archivo .xlsx: una hoja por elemento de `hojas`.
#[must_use]
pub fn xlsx(hojas: &[Hoja]) -> Vec<u8> {
    let mut tipos = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
         <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
         <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
         <Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>\
         <Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>",
    );
    let mut libro = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><workbook xmlns=\"{NS_MAIN}\" xmlns:r=\"{NS_REL}\"><sheets>"
    );
    let mut relaciones = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"{NS_PKG_REL}\">"
    );
    let mut partes: Vec<(String, String)> = Vec::new();
    for (indice, hoja) in hojas.iter().enumerate() {
        let n = indice + 1;
        tipos.push_str(&format!(
            "<Override PartName=\"/xl/worksheets/sheet{n}.xml\" ContentType=\"{CT_HOJA}\"/>"
        ));
        libro.push_str(&format!(
            "<sheet name=\"{}\" sheetId=\"{n}\" r:id=\"rId{n}\"/>",
            xml(&nombre_hoja(&hoja.nombre))
        ));
        relaciones.push_str(&format!(
            "<Relationship Id=\"rId{n}\" Type=\"{NS_REL}/worksheet\" Target=\"worksheets/sheet{n}.xml\"/>"
        ));
        partes.push((format!("xl/worksheets/sheet{n}.xml"), hoja_xml(hoja)));
    }
    tipos.push_str("</Types>");
    libro.push_str("</sheets></workbook>");
    relaciones.push_str(&format!(
        "<Relationship Id=\"rId{}\" Type=\"{NS_REL}/styles\" Target=\"styles.xml\"/></Relationships>",
        hojas.len() + 1
    ));
    let raiz = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"{NS_PKG_REL}\">\
         <Relationship Id=\"rId1\" Type=\"{NS_REL}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>"
    );
    let mut entradas: Vec<(&str, &[u8])> = vec![
        ("[Content_Types].xml", tipos.as_bytes()),
        ("_rels/.rels", raiz.as_bytes()),
        ("xl/workbook.xml", libro.as_bytes()),
        ("xl/_rels/workbook.xml.rels", relaciones.as_bytes()),
        ("xl/styles.xml", STYLES.as_bytes()),
    ];
    entradas.extend(partes.iter().map(|(n, d)| (n.as_str(), d.as_bytes())));
    zip_stored(&entradas)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Entradas (nombre, contenido) leyendo el directorio central del ZIP.
    pub(crate) fn entradas_zip(zip: &[u8]) -> Vec<(String, String)> {
        let u16_en = |i: usize| usize::from(u16::from_le_bytes([zip[i], zip[i + 1]]));
        let u32_en = |i: usize| u32::from_le_bytes(zip[i..i + 4].try_into().unwrap()) as usize;
        let eocd = zip.len() - 22;
        let mut pos = u32_en(eocd + 16);
        let mut salida = Vec::new();
        for _ in 0..u16_en(eocd + 10) {
            let largo_nombre = u16_en(pos + 28);
            let nombre =
                String::from_utf8(zip[pos + 46..pos + 46 + largo_nombre].to_vec()).unwrap();
            let local = u32_en(pos + 42);
            let inicio = local + 30 + u16_en(local + 26) + u16_en(local + 28);
            let datos = &zip[inicio..inicio + u32_en(pos + 24)];
            salida.push((nombre, String::from_utf8(datos.to_vec()).unwrap()));
            pos += 46 + largo_nombre + u16_en(pos + 30) + u16_en(pos + 32);
        }
        salida
    }

    /// Etiquetas balanceadas: cada apertura tiene su cierre en orden.
    pub(crate) fn xml_bien_formado(texto: &str) -> bool {
        let mut pila: Vec<&str> = Vec::new();
        let mut resto = texto;
        while let Some(inicio) = resto.find('<') {
            let Some(fin) = resto[inicio..].find('>') else {
                return false;
            };
            let etiqueta = &resto[inicio + 1..inicio + fin];
            resto = &resto[inicio + fin + 1..];
            if etiqueta.starts_with('?') || etiqueta.ends_with('/') {
                continue;
            }
            let nombre = etiqueta
                .trim_start_matches('/')
                .split_whitespace()
                .next()
                .unwrap_or("");
            if etiqueta.starts_with('/') {
                if pila.pop() != Some(nombre) {
                    return false;
                }
            } else {
                pila.push(nombre);
            }
        }
        pila.is_empty() && !resto.contains('>')
    }

    #[test]
    fn columnas_como_excel() {
        assert_eq!(columna(0), "A");
        assert_eq!(columna(25), "Z");
        assert_eq!(columna(26), "AA");
        assert_eq!(columna(27), "AB");
        assert_eq!(columna(701), "ZZ");
        assert_eq!(columna(702), "AAA");
    }

    #[test]
    fn paquete_ooxml_con_partes_y_celdas() {
        let hojas = [
            Hoja {
                nombre: "Partidos".into(),
                cabeceras: vec!["Rival".into(), "Goles".into()],
                filas: vec![vec!["Los <Pibes> & Cía".into(), "3".into()]],
            },
            Hoja {
                nombre: "Hoja/rara: con un nombre larguísimo de más".into(),
                cabeceras: vec!["A".into()],
                filas: vec![],
            },
        ];
        let entradas = entradas_zip(&xlsx(&hojas));
        let nombres: Vec<&str> = entradas.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            nombres,
            [
                "[Content_Types].xml",
                "_rels/.rels",
                "xl/workbook.xml",
                "xl/_rels/workbook.xml.rels",
                "xl/styles.xml",
                "xl/worksheets/sheet1.xml",
                "xl/worksheets/sheet2.xml",
            ]
        );
        for (nombre, contenido) in &entradas {
            assert!(xml_bien_formado(contenido), "{nombre}: {contenido}");
        }
        let hoja1 = &entradas[5].1;
        assert!(hoja1.contains("<c r=\"A2\" t=\"inlineStr\"><is><t xml:space=\"preserve\">Los &lt;Pibes&gt; &amp; Cía</t></is></c>"));
        assert!(hoja1.contains("<c r=\"B2\"><v>3</v></c>"));
        assert!(hoja1.contains("<c r=\"B1\" t=\"inlineStr\" s=\"1\">"));
        let libro = &entradas[2].1;
        assert!(libro.contains("name=\"Hoja_rara_ con un nombre larguí\""));
        assert!(entradas[0].1.contains("/xl/worksheets/sheet2.xml"));
        assert!(entradas[3].1.contains("Id=\"rId3\""));
    }
}
