//! Comprobar actualizaciones (port de `verificar_actualizacion_disponible`,
//! `comparar_versiones` de `settings.c` y `actualizacion_corresponde_verificar`
//! de `main.c`).
//!
//! C consulta `releases/latest` de la API de GitHub, compara `tag_name` (o
//! `name`) con su versión y, si hay una más nueva, descarga y ejecuta el
//! instalador `.exe`. Aquí solo se consulta y se muestra el enlace: no se
//! descarga ni se instala nada.

use std::cmp::Ordering;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

/// Última release del repositorio de MiFutbolR (C usa `thomashamer3/MiFutbolC`).
pub const URL_ULTIMA_RELEASE: &str =
    "https://api.github.com/repos/thomashamer3/MiFutbolR/releases/latest";

/// Archivo con la hora del último intento (`UPDATE_CHECK_ARCHIVO` de C).
pub const ARCHIVO_MARCA: &str = "update_check.stamp";

/// Una consulta automática por día (`UPDATE_CHECK_INTERVALO_SEG`).
pub const INTERVALO_SEG: i64 = 24 * 60 * 60;

/// Versión de esta compilación.
pub const VERSION_ACTUAL: &str = env!("CARGO_PKG_VERSION");

/// Release publicada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// `tag_name` o, si falta, `name`.
    pub version: String,
    /// Página de la release (`html_url`).
    pub url: String,
}

/// `mayor.menor.parche`; acepta prefijo `v`/`V` y partes faltantes (= 0), como C.
fn tripleta(version: &str) -> [u64; 3] {
    let version = version.trim();
    let version = version.strip_prefix(['v', 'V']).unwrap_or(version);
    let mut partes = [0; 3];
    for (parte, texto) in partes.iter_mut().zip(version.split('.')) {
        let digitos: String = texto.chars().take_while(char::is_ascii_digit).collect();
        let Ok(valor) = digitos.parse() else { break };
        *parte = valor;
        if digitos.len() != texto.len() {
            break;
        }
    }
    partes
}

/// Compara dos versiones (`comparar_versiones` de C).
#[must_use]
pub fn comparar_versiones(a: &str, b: &str) -> Ordering {
    tripleta(a).cmp(&tripleta(b))
}

/// ¿Toca la consulta automática? Como C, si toca guarda ya la hora del intento
/// (aunque la red falle no se reintenta hasta el día siguiente).
pub fn corresponde_verificar(marca: &Path, ahora: i64) -> bool {
    let ultimo = std::fs::read_to_string(marca)
        .ok()
        .and_then(|texto| texto.trim().parse::<i64>().ok())
        .unwrap_or(0);
    if ultimo > 0 && ahora >= ultimo && ahora - ultimo < INTERVALO_SEG {
        return false;
    }
    let _ = std::fs::write(marca, format!("{ahora}\n"));
    true
}

/// Consulta la última release (5 s de conexión, 15 s en total).
pub fn consultar(url: &str) -> Result<Release, String> {
    let agente: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let texto = agente
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "MiFutbolR")
        .call()
        .map_err(|error| format!("No se pudo verificar actualizaciones: {error}"))?
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("No se pudo verificar actualizaciones: {error}"))?;
    let json: serde_json::Value =
        serde_json::from_str(&texto).map_err(|_| "Respuesta inválida de GitHub".to_string())?;
    let texto_de = |clave: &str| {
        json[clave]
            .as_str()
            .map(str::trim)
            .filter(|valor| !valor.is_empty())
            .map(String::from)
    };
    let version = texto_de("tag_name")
        .or_else(|| texto_de("name"))
        .ok_or_else(|| "La release no tiene versión".to_string())?;
    Ok(Release {
        version,
        url: texto_de("html_url").unwrap_or_default(),
    })
}

/// (¿hay una versión nueva?, mensaje), con los textos de C.
#[must_use]
pub fn mensaje(actual: &str, release: &Release) -> (bool, String) {
    match comparar_versiones(actual, &release.version) {
        Ordering::Less => (
            true,
            format!(
                "¡NUEVA VERSIÓN DISPONIBLE!\n\nVersión actual:  {actual}\nNueva versión:   {}\n\nDescargala en:\n{}",
                release.version, release.url
            ),
        ),
        Ordering::Greater => (
            false,
            format!("Estás usando una versión de desarrollo ({actual})."),
        ),
        Ordering::Equal => (false, format!("Estás usando la última versión ({actual}).")),
    }
}

/// Consulta en segundo plano; `manual` indica si la pidió el usuario.
#[derive(Default)]
pub struct ActualizacionWorker {
    canal: Option<(bool, Receiver<Result<Release, String>>)>,
}

impl ActualizacionWorker {
    /// Lanza la consulta. `false` si ya había una en curso.
    pub fn lanzar(&mut self, url: String, manual: bool) -> bool {
        if self.canal.is_some() {
            return false;
        }
        let (envio, recepcion) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = envio.send(consultar(&url));
        });
        self.canal = Some((manual, recepcion));
        true
    }

    /// (manual, resultado) si ya terminó, sin esperar.
    pub fn recibir(&mut self) -> Option<(bool, Result<Release, String>)> {
        let (manual, canal) = self.canal.as_ref()?;
        let resultado = match canal.try_recv() {
            Ok(resultado) => resultado,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                Err("La comprobación terminó sin resultado".to_string())
            }
        };
        let manual = *manual;
        self.canal = None;
        Some((manual, resultado))
    }

    /// Espera el resultado (solo tests).
    pub fn esperar(&mut self, limite: Duration) -> Option<(bool, Result<Release, String>)> {
        let (manual, canal) = self.canal.as_ref()?;
        let resultado = canal.recv_timeout(limite).ok()?;
        let manual = *manual;
        self.canal = None;
        Some((manual, resultado))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    /// Servidor HTTP en 127.0.0.1:0 que responde siempre `status` + `cuerpo`.
    pub(crate) fn servidor_falso(status: u16, cuerpo: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/releases/latest", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut lector = BufReader::new(stream.try_clone().unwrap());
                loop {
                    let mut linea = String::new();
                    if lector.read_line(&mut linea).unwrap_or(0) == 0 || linea == "\r\n" {
                        break;
                    }
                }
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}",
                    cuerpo.len()
                );
            }
        });
        url
    }

    #[test]
    fn compara_versiones_como_c() {
        assert_eq!(comparar_versiones("0.1.0", "v0.2.0"), Ordering::Less);
        assert_eq!(comparar_versiones("V1.2", "1.2.0"), Ordering::Equal);
        assert_eq!(comparar_versiones("4.6", "4.5.9"), Ordering::Greater);
        assert_eq!(comparar_versiones("1.10.0", "1.9.0"), Ordering::Greater);
        assert_eq!(comparar_versiones("1.2.0-beta", "1.2.0"), Ordering::Equal);
        assert_eq!(comparar_versiones("", "0.0.1"), Ordering::Less);
    }

    #[test]
    fn una_vez_por_dia() {
        let dir = tempfile::tempdir().unwrap();
        let marca = dir.path().join(ARCHIVO_MARCA);
        assert!(corresponde_verificar(&marca, 1_000_000));
        assert!(!corresponde_verificar(&marca, 1_000_000 + 3600));
        assert!(corresponde_verificar(&marca, 1_000_000 + INTERVALO_SEG));
        // Reloj atrasado: vuelve a consultar, como C.
        assert!(corresponde_verificar(&marca, 10));
    }

    #[test]
    fn consulta_servidor_falso() {
        let url = servidor_falso(
            200,
            r#"{"tag_name":"v9.0.0","name":"Nueva","html_url":"https://example.invalid/r"}"#,
        );
        let release = consultar(&url).unwrap();
        assert_eq!(release.version, "v9.0.0");
        let (nueva, texto) = mensaje(VERSION_ACTUAL, &release);
        assert!(nueva && texto.contains("v9.0.0") && texto.contains("example.invalid"));
        assert!(!mensaje("9.0.0", &release).0);

        let solo_nombre = servidor_falso(200, r#"{"tag_name":"","name":"1.0"}"#);
        assert_eq!(consultar(&solo_nombre).unwrap().version, "1.0");

        let error = servidor_falso(404, r#"{"message":"Not Found"}"#);
        assert!(consultar(&error).is_err());
    }

    #[test]
    fn hilo_devuelve_resultado() {
        let url = servidor_falso(200, r#"{"tag_name":"0.0.1"}"#);
        let mut worker = ActualizacionWorker::default();
        assert!(worker.lanzar(url.clone(), true));
        assert!(!worker.lanzar(url, false), "una consulta a la vez");
        let (manual, resultado) = worker.esperar(Duration::from_secs(10)).unwrap();
        assert!(manual);
        assert_eq!(resultado.unwrap().version, "0.0.1");
        assert!(worker.recibir().is_none());
    }
}
