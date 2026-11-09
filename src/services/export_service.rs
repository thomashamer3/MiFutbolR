//! Exportación de listados a CSV, JSON, HTML y Markdown (IMP-08).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use serde_json::{Value, json};

/// Error de exportación.
#[derive(Debug)]
pub struct ExportError(pub String);

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ExportError {}

impl From<std::io::Error> for ExportError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}

impl From<serde_json::Error> for ExportError {
    fn from(value: serde_json::Error) -> Self {
        Self(value.to_string())
    }
}

fn escapar_csv(valor: &str) -> String {
    if valor.contains(',') || valor.contains('"') || valor.contains('\n') {
        format!("\"{}\"", valor.replace('"', "\"\""))
    } else {
        valor.to_string()
    }
}

fn escapar_html(valor: &str) -> String {
    valor
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escribir_csv(ruta: &Path, cabeceras: &[&str], filas: &[Vec<String>]) -> Result<(), ExportError> {
    let mut texto = String::new();
    texto.push_str(
        &cabeceras
            .iter()
            .map(|cabecera| escapar_csv(cabecera))
            .collect::<Vec<_>>()
            .join(","),
    );
    texto.push('\n');
    for fila in filas {
        texto.push_str(
            &fila
                .iter()
                .map(|valor| escapar_csv(valor))
                .collect::<Vec<_>>()
                .join(","),
        );
        texto.push('\n');
    }
    fs::write(ruta, texto)?;
    Ok(())
}

fn escribir_json(
    ruta: &Path,
    cabeceras: &[&str],
    filas: &[Vec<String>],
) -> Result<(), ExportError> {
    let mut objetos: Vec<Value> = Vec::with_capacity(filas.len());
    for fila in filas {
        let mut objeto = serde_json::Map::new();
        for (indice, cabecera) in cabeceras.iter().enumerate() {
            objeto.insert(
                (*cabecera).to_string(),
                Value::String(fila.get(indice).cloned().unwrap_or_default()),
            );
        }
        objetos.push(Value::Object(objeto));
    }
    fs::write(ruta, serde_json::to_string_pretty(&json!(objetos))?)?;
    Ok(())
}

fn escribir_html(
    ruta: &Path,
    entidad: &str,
    cabeceras: &[&str],
    filas: &[Vec<String>],
) -> Result<(), ExportError> {
    let mut texto = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>MiFutbolR</title>\
         <style>body{font-family:system-ui,sans-serif;margin:2rem;background:#f7f7f5;color:#222}\
         h1{font-size:1.4rem}table{border-collapse:collapse;width:100%;background:#fff}\
         th,td{border:1px solid #ddd;padding:.4rem .6rem;text-align:left}\
         th{background:#222;color:#fff}tr:nth-child(even){background:#f2f2f2}</style>",
    );
    texto.push_str(&format!(
        "<h1>MiFutbolR · {}</h1><p>Generado {}</p><table><thead><tr>",
        escapar_html(entidad),
        Local::now().format("%d/%m/%Y %H:%M")
    ));
    for cabecera in cabeceras {
        texto.push_str(&format!("<th>{}</th>", escapar_html(cabecera)));
    }
    texto.push_str("</tr></thead><tbody>");
    for fila in filas {
        texto.push_str("<tr>");
        for valor in fila {
            texto.push_str(&format!("<td>{}</td>", escapar_html(valor)));
        }
        texto.push_str("</tr>");
    }
    texto.push_str("</tbody></table>");
    fs::write(ruta, texto)?;
    Ok(())
}

fn escribir_markdown(
    ruta: &Path,
    entidad: &str,
    cabeceras: &[&str],
    filas: &[Vec<String>],
) -> Result<(), ExportError> {
    let mut texto = format!("# MiFutbolR · {entidad}\n\n");
    texto.push_str("| ");
    texto.push_str(&cabeceras.join(" | "));
    texto.push_str(" |\n|");
    for _ in cabeceras {
        texto.push_str(" --- |");
    }
    texto.push('\n');
    for fila in filas {
        texto.push_str("| ");
        texto.push_str(&fila.join(" | "));
        texto.push_str(" |\n");
    }
    fs::write(ruta, texto)?;
    Ok(())
}

/// Exporta el conjunto a los cuatro formatos y devuelve las rutas creadas.
pub fn exportar(
    dir: &Path,
    entidad: &str,
    cabeceras: &[&str],
    filas: &[Vec<String>],
) -> Result<Vec<PathBuf>, ExportError> {
    fs::create_dir_all(dir)?;
    let marca = Local::now().format("%Y%m%d-%H%M%S");
    let base = format!("{entidad}_{marca}");
    let rutas = [
        dir.join(format!("{base}.csv")),
        dir.join(format!("{base}.json")),
        dir.join(format!("{base}.html")),
        dir.join(format!("{base}.md")),
    ];
    escribir_csv(&rutas[0], cabeceras, filas)?;
    escribir_json(&rutas[1], cabeceras, filas)?;
    escribir_html(&rutas[2], entidad, cabeceras, filas)?;
    escribir_markdown(&rutas[3], entidad, cabeceras, filas)?;
    Ok(rutas.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_four_formats() {
        let dir = tempfile::tempdir().unwrap();
        let filas = vec![
            vec!["1".to_string(), "Azul, 2024".to_string()],
            vec!["2".to_string(), "Roja".to_string()],
        ];
        let rutas = exportar(dir.path(), "camisetas", &["ID", "Nombre"], &filas).unwrap();
        assert_eq!(rutas.len(), 4);
        let csv = fs::read_to_string(&rutas[0]).unwrap();
        assert!(csv.contains("\"Azul, 2024\""));
        let json = fs::read_to_string(&rutas[1]).unwrap();
        assert!(json.contains("Azul, 2024"));
        let html = fs::read_to_string(&rutas[2]).unwrap();
        assert!(html.contains("<table>"));
        let md = fs::read_to_string(&rutas[3]).unwrap();
        assert!(md.contains("| ID | Nombre |"));
    }
}
