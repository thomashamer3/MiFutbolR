//! Hoja de cálculo OpenDocument (.ods) real: ZIP con `mimetype` primero y sin
//! comprimir, `META-INF/manifest.xml` y `content.xml` (una tabla por hoja).
//!
//! MiFutbolC escribía solo `content.xml` en texto plano, que LibreOffice no abre.

use super::zip::zip_stored;

const MIMETYPE: &str = "application/vnd.oasis.opendocument.spreadsheet";

/// Una hoja: nombre, cabeceras y filas de texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hoja {
    /// Nombre de la pestaña.
    pub nombre: String,
    /// Fila de títulos.
    pub cabeceras: Vec<String>,
    /// Filas de datos.
    pub filas: Vec<Vec<String>>,
}

pub(crate) fn xml(texto: &str) -> String {
    texto
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn celda(valor: &str) -> String {
    // Números como float para que la planilla pueda sumar y ordenar.
    match valor.trim().parse::<f64>() {
        Ok(numero) if numero.is_finite() && !valor.trim().is_empty() => format!(
            "<table:table-cell office:value-type=\"float\" office:value=\"{numero}\"><text:p>{}</text:p></table:table-cell>",
            xml(valor)
        ),
        _ => format!(
            "<table:table-cell office:value-type=\"string\"><text:p>{}</text:p></table:table-cell>",
            xml(valor)
        ),
    }
}

/// `content.xml` con todas las hojas.
#[must_use]
pub fn content_xml(hojas: &[Hoja]) -> String {
    let mut salida = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <office:document-content xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
         xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" \
         xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" office:version=\"1.2\">\
         <office:body><office:spreadsheet>",
    );
    for hoja in hojas {
        salida.push_str(&format!(
            "<table:table table:name=\"{}\">",
            xml(&hoja.nombre)
        ));
        salida.push_str("<table:table-row>");
        for cabecera in &hoja.cabeceras {
            salida.push_str(&format!(
                "<table:table-cell office:value-type=\"string\"><text:p>{}</text:p></table:table-cell>",
                xml(cabecera)
            ));
        }
        salida.push_str("</table:table-row>");
        for fila in &hoja.filas {
            salida.push_str("<table:table-row>");
            for valor in fila {
                salida.push_str(&celda(valor));
            }
            salida.push_str("</table:table-row>");
        }
        salida.push_str("</table:table>");
    }
    salida.push_str("</office:spreadsheet></office:body></office:document-content>");
    salida
}

/// Bytes del archivo .ods.
#[must_use]
pub fn ods(hojas: &[Hoja]) -> Vec<u8> {
    let manifest = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.2\">\
         <manifest:file-entry manifest:full-path=\"/\" manifest:version=\"1.2\" manifest:media-type=\"{MIMETYPE}\"/>\
         <manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/>\
         </manifest:manifest>"
    );
    let content = content_xml(hojas);
    zip_stored(&[
        ("mimetype", MIMETYPE.as_bytes()),
        ("META-INF/manifest.xml", manifest.as_bytes()),
        ("content.xml", content.as_bytes()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ods_starts_with_uncompressed_mimetype_and_typed_cells() {
        let hoja = Hoja {
            nombre: "Partidos".into(),
            cabeceras: vec!["Rival".into(), "Goles".into()],
            filas: vec![vec!["Los <Pibes> & Cía".into(), "3".into()]],
        };
        let bytes = ods(std::slice::from_ref(&hoja));
        assert_eq!(&bytes[30..38], b"mimetype");
        assert_eq!(&bytes[38..38 + MIMETYPE.len()], MIMETYPE.as_bytes());
        let content = content_xml(&[hoja]);
        assert!(content.contains("table:name=\"Partidos\""));
        assert!(content.contains("office:value-type=\"float\" office:value=\"3\""));
        assert!(content.contains("Los &lt;Pibes&gt; &amp; Cía"));
    }
}
