//! Sincronización con PocketBase (port de `sync.c`, `sync_http.c` y
//! `sync_entidades.c`): inicio de sesión, subida de la cola de partidos,
//! camisetas, botines y lápidas, y bajada incremental por `updated_at_client`.
//!
//! Regla de conflictos (la de C): gana la marca más nueva. Si un partido cambió
//! en los dos lados y el servidor es más nuevo que el cambio local, queda un
//! conflicto para que el usuario elija. La contraseña solo viaja al iniciar
//! sesión; se guarda el token, que nunca se muestra ni se exporta.

use std::cmp::Ordering;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::db::sync_repo::{self, Descarga, Entidad, Pendiente, comparar_fechas};
use crate::db::{Database, DbError};

/// Colección de partidos en PocketBase.
pub const COLECCION_PARTIDOS: &str = "partidos_sync";

/// Configuración guardada en `sync_config`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    /// URL en la red local (Wi-Fi).
    pub url_local: String,
    /// URL remota (HTTPS).
    pub url_remota: String,
    /// Correo de la cuenta.
    pub email: String,
    /// Token de sesión (secreto).
    pub token: String,
    /// Id del usuario en PocketBase.
    pub user_id: String,
    /// Última marca descargada.
    pub cursor: String,
    /// Servidor que respondió por última vez.
    pub servidor_activo: String,
    /// Fecha de la última sincronización.
    pub ultima_sync: String,
    /// Sincronizar al iniciar.
    pub auto_sync: bool,
}

impl Config {
    /// Lee la configuración.
    pub fn cargar(conn: &Connection) -> Result<Self, DbError> {
        let leer = |clave| sync_repo::config_get(conn, clave);
        Ok(Self {
            url_local: leer("url_local")?,
            url_remota: leer("url_remota")?,
            email: leer("email")?,
            token: leer("token")?,
            user_id: leer("user_id")?,
            cursor: leer("cursor")?,
            servidor_activo: leer("servidor_activo")?,
            ultima_sync: leer("ultima_sync")?,
            auto_sync: leer("auto_sync")? == "1",
        })
    }

    /// ¿Hay sesión iniciada?
    #[must_use]
    pub fn con_sesion(&self) -> bool {
        !self.token.is_empty() && !self.user_id.is_empty()
    }
}

/// Resumen de una sincronización.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resultado {
    /// Registros subidos.
    pub enviados: u32,
    /// Registros remotos aplicados.
    pub importados: u32,
    /// Borrados remotos aplicados.
    pub eliminados: u32,
    /// Conflictos nuevos.
    pub conflictos: u32,
    /// Registros que fallaron.
    pub errores: u32,
    /// Servidor con el que se completó.
    pub servidor: String,
}

impl Resultado {
    /// Mensaje para la pantalla.
    #[must_use]
    pub fn resumen(&self) -> String {
        format!(
            "Sincronización completada con {}: {} enviados, {} importados, {} eliminados, {} conflictos, {} errores.",
            self.servidor,
            self.enviados,
            self.importados,
            self.eliminados,
            self.conflictos,
            self.errores
        )
    }
}

/// Normaliza una URL: sin espacios, `http://` si no trae esquema y sin barras
/// finales. Solo `http`/`https` (`sync_normalizar_url`).
#[must_use]
pub fn normalizar_url(entrada: &str) -> Option<String> {
    let texto = entrada.trim();
    if texto.is_empty()
        || texto
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '\\' | '%'))
    {
        return None;
    }
    let mut url = match texto.split_once("://") {
        Some(("http" | "https", resto)) if !resto.is_empty() => texto.to_string(),
        Some(_) => return None,
        None => format!("http://{texto}"),
    };
    while url.len() > 8 && url.ends_with('/') {
        url.pop();
    }
    Some(url)
}

/// Servidores a probar, en orden: el activo, el local y el remoto, sin
/// repetir y como mucho dos (`sync_servidores`).
#[must_use]
pub fn servidores(cfg: &Config) -> Vec<String> {
    let mut lista: Vec<String> = Vec::new();
    for url in [&cfg.servidor_activo, &cfg.url_local, &cfg.url_remota] {
        if let Some(url) = normalizar_url(url)
            && !lista.contains(&url)
            && lista.len() < 2
        {
            lista.push(url);
        }
    }
    lista
}

/// Guarda las URLs (vacío = conservar). Devuelve el error de la que no valga.
pub fn configurar(conn: &Connection, url_local: &str, url_remota: &str) -> Result<(), String> {
    for (clave, valor) in [("url_local", url_local), ("url_remota", url_remota)] {
        if valor.trim().is_empty() {
            continue;
        }
        let url = normalizar_url(valor)
            .ok_or_else(|| format!("URL inválida (usá http:// o https://): {valor}"))?;
        sync_repo::config_set(conn, clave, &url).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Activa o desactiva la sincronización al iniciar. Devuelve el nuevo estado.
pub fn alternar_auto(conn: &Connection) -> Result<bool, DbError> {
    let activo = !Config::cargar(conn)?.auto_sync;
    sync_repo::config_set(conn, "auto_sync", if activo { "1" } else { "0" })?;
    Ok(activo)
}

/// Cierra la sesión: borra token, usuario y cursor.
pub fn cerrar_sesion(conn: &Connection) -> Result<(), DbError> {
    for clave in ["token", "user_id", "cursor"] {
        sync_repo::config_set(conn, clave, "")?;
    }
    Ok(())
}

// --------------------------------------------------------------------- HTTP

/// Por qué falló una petición.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Fallo {
    /// Sin respuesta (red, DNS, tiempo agotado) o error local.
    Red(String),
    /// El servidor rechazó ese registro (4xx/5xx que no son de sesión).
    Http(String),
    /// 401/403: hay que refrescar el token.
    Sesion,
}

impl std::fmt::Display for Fallo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Red(mensaje) | Self::Http(mensaje) => f.write_str(mensaje),
            Self::Sesion => f.write_str("Sesión vencida"),
        }
    }
}

impl From<DbError> for Fallo {
    fn from(error: DbError) -> Self {
        Self::Red(error.to_string())
    }
}

/// Agente con los tiempos de MiFutbolC (5 s conexión, 25 s total). Los códigos
/// HTTP de error se leen como respuesta, no como error.
fn agente() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(Some(Duration::from_secs(25)))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Codifica para la query string (todo lo que no sea alfanumérico o `-_.`).
fn codificar(texto: &str) -> String {
    texto
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// Hace la petición y devuelve (código, cuerpo). `Err` solo si no hubo respuesta.
fn pedir(
    agente: &ureq::Agent,
    metodo: &str,
    url: &str,
    token: &str,
    cuerpo: Option<&Value>,
) -> Result<(u16, String), String> {
    let respuesta = match metodo {
        "GET" => {
            let mut peticion = agente.get(url).header("Accept", "application/json");
            if !token.is_empty() {
                peticion = peticion.header("Authorization", token);
            }
            peticion.call()
        }
        _ => {
            let mut peticion = if metodo == "PATCH" {
                agente.patch(url)
            } else {
                agente.post(url)
            };
            peticion = peticion.header("Accept", "application/json");
            if !token.is_empty() {
                peticion = peticion.header("Authorization", token);
            }
            match cuerpo {
                Some(cuerpo) => peticion
                    .content_type("application/json")
                    .send(cuerpo.to_string()),
                None => peticion.send_empty(),
            }
        }
    };
    let mut respuesta = respuesta.map_err(|error| format!("Error de red: {error}"))?;
    let status = respuesta.status().as_u16();
    let cuerpo = respuesta.body_mut().read_to_string().unwrap_or_default();
    Ok((status, cuerpo))
}

/// Mensaje del servidor (`message` de PocketBase) o el cuerpo crudo.
fn mensaje_http(status: u16, cuerpo: &str) -> String {
    let mensaje = serde_json::from_str::<Value>(cuerpo)
        .ok()
        .and_then(|json| json["message"].as_str().map(String::from))
        .unwrap_or_else(|| cuerpo.chars().take(200).collect());
    format!("HTTP {status}: {mensaje}")
}

/// Petición que espera JSON 2xx, clasificando el fallo.
fn pedir_json(
    agente: &ureq::Agent,
    metodo: &str,
    url: &str,
    token: &str,
    cuerpo: Option<&Value>,
) -> Result<Value, Fallo> {
    let (status, texto) = pedir(agente, metodo, url, token, cuerpo).map_err(Fallo::Red)?;
    match status {
        200..=299 => Ok(serde_json::from_str(&texto).unwrap_or(Value::Null)),
        401 | 403 => Err(Fallo::Sesion),
        _ => Err(Fallo::Http(mensaje_http(status, &texto))),
    }
}

/// `auth-with-password` contra un servidor: devuelve (token, id de usuario).
fn autenticar(
    agente: &ureq::Agent,
    base: &str,
    email: &str,
    password: &str,
) -> Result<(String, String), String> {
    let url = format!("{base}/api/collections/users/auth-with-password");
    let cuerpo = json!({"identity": email, "password": password});
    let (status, texto) = pedir(agente, "POST", &url, "", Some(&cuerpo))?;
    match status {
        200..=299 => {}
        400 | 401 => return Err("Correo o contraseña incorrectos".to_string()),
        otro => return Err(format!("El servidor rechazó el inicio de sesión ({otro})")),
    }
    let json: Value = serde_json::from_str(&texto)
        .map_err(|_| "Respuesta de autenticación inválida".to_string())?;
    match (json["token"].as_str(), json["record"]["id"].as_str()) {
        (Some(token), Some(id)) if !token.is_empty() && !id.is_empty() => {
            Ok((token.to_string(), id.to_string()))
        }
        _ => Err("El servidor no devolvió token/usuario".to_string()),
    }
}

/// Inicia sesión probando cada servidor configurado. Guarda token, usuario y
/// servidor activo y reinicia el cursor. La contraseña no se guarda.
pub fn iniciar_sesion(conn: &Connection, email: &str, password: &str) -> Result<String, String> {
    if email.trim().is_empty() || password.is_empty() {
        return Err("Correo y contraseña son obligatorios".to_string());
    }
    let cfg = Config::cargar(conn).map_err(|e| e.to_string())?;
    let lista = servidores(&cfg);
    if lista.is_empty() {
        return Err("Configurá primero la URL del servidor".to_string());
    }
    let agente = agente();
    let mut fallos = Vec::new();
    for servidor in lista {
        match autenticar(&agente, &servidor, email.trim(), password) {
            Ok((token, user_id)) => {
                for (clave, valor) in [
                    ("email", email.trim()),
                    ("token", token.as_str()),
                    ("user_id", user_id.as_str()),
                    ("servidor_activo", servidor.as_str()),
                    ("cursor", ""),
                ] {
                    sync_repo::config_set(conn, clave, valor).map_err(|e| e.to_string())?;
                }
                return Ok(servidor);
            }
            Err(error) => fallos.push(format!("{servidor}: {error}")),
        }
    }
    Err(format!("No se pudo iniciar sesión. {}", fallos.join("; ")))
}

// ---------------------------------------------------------- Sincronización

/// Estado de una pasada contra un servidor.
struct Pasada<'a> {
    conn: &'a mut Connection,
    agente: &'a ureq::Agent,
    base: String,
    cfg: Config,
    res: &'a mut Resultado,
}

impl Pasada<'_> {
    fn url(&self, coleccion: &str, resto: &str) -> String {
        format!("{}/api/collections/{coleccion}/records{resto}", self.base)
    }

    /// `auth-refresh`: guarda el token nuevo si el servidor lo da.
    fn refrescar_token(&mut self) -> bool {
        let url = format!("{}/api/collections/users/auth-refresh", self.base);
        let Ok(json) = pedir_json(self.agente, "POST", &url, &self.cfg.token, None) else {
            return false;
        };
        let Some(token) = json["token"].as_str().filter(|t| !t.is_empty()) else {
            return false;
        };
        self.cfg.token = token.to_string();
        sync_repo::config_set(self.conn, "token", token).is_ok()
    }

    /// Registro remoto con ese uuid, si existe.
    fn buscar(&self, coleccion: &str, uuid: &str) -> Result<Option<Value>, Fallo> {
        let filtro = codificar(&format!("uuid='{uuid}'"));
        let url = self.url(coleccion, &format!("?filter={filtro}&perPage=1"));
        let json = pedir_json(self.agente, "GET", &url, &self.cfg.token, None)?;
        Ok(json["items"].get(0).cloned())
    }

    /// Crea (POST) o actualiza (PATCH) un registro.
    fn enviar(
        &self,
        coleccion: &str,
        record_id: Option<&str>,
        registro: &Value,
    ) -> Result<(), Fallo> {
        let (metodo, url) = match record_id {
            Some(id) => ("PATCH", self.url(coleccion, &format!("/{}", codificar(id)))),
            None => ("POST", self.url(coleccion, "")),
        };
        pedir_json(self.agente, metodo, &url, &self.cfg.token, Some(registro)).map(|_| ())
    }

    fn conflicto(&mut self, uuid: &str, local: &Value, remoto: &Value) -> Result<(), Fallo> {
        sync_repo::guardar_conflicto(self.conn, uuid, local, remoto)?;
        sync_repo::marcar_conflicto(self.conn, uuid)?;
        self.res.conflictos += 1;
        Ok(())
    }

    /// Sube la cola de partidos (`sync_subir`). Cada fila se intenta una vez por
    /// pasada: un registro rechazado queda con error y no frena a los demás
    /// (en C el primero que fallaba bloqueaba la cola para siempre).
    fn subir_partidos(&mut self) -> Result<(), Fallo> {
        let mut ultimo = 0;
        while let Some(pendiente) = sync_repo::siguiente_pendiente(self.conn, ultimo)? {
            ultimo = pendiente.id;
            match self.subir_pendiente(&pendiente) {
                Ok(()) => {}
                Err(Fallo::Http(error)) => {
                    sync_repo::marcar_error(self.conn, pendiente.id, &error)?;
                    self.res.errores += 1;
                }
                Err(Fallo::Red(error)) => {
                    sync_repo::marcar_error(self.conn, pendiente.id, &error)?;
                    self.res.errores += 1;
                    return Err(Fallo::Red(error));
                }
                Err(Fallo::Sesion) => return Err(Fallo::Sesion),
            }
        }
        Ok(())
    }

    fn subir_pendiente(&mut self, pendiente: &Pendiente) -> Result<(), Fallo> {
        let uuid = pendiente.uuid.as_str();
        let remoto = self.buscar(COLECCION_PARTIDOS, uuid)?;
        let record_id = remoto
            .as_ref()
            .and_then(|r| r["id"].as_str())
            .map(String::from);
        let remoto_actualizado = remoto
            .as_ref()
            .and_then(|r| r["updated_at_client"].as_str())
            .unwrap_or("")
            .to_string();
        let marca = if pendiente.creado_en.is_empty() {
            sync_repo::ahora_iso()
        } else {
            pendiente.creado_en.replace(' ', "T")
        };
        let payload = if pendiente.operacion == "eliminar" {
            None
        } else {
            sync_repo::payload_partido(self.conn, uuid)?
        };
        let Some(mut payload) = payload else {
            // Borrado local (o el partido ya no existe): lápida para los demás.
            if let Some(remoto) = &remoto
                && record_id.is_some()
                && comparar_fechas(&remoto_actualizado, &marca) == Ordering::Greater
                && !remoto_actualizado.is_empty()
            {
                return self.conflicto(
                    uuid,
                    &json!({"uuid": uuid, "actualizado_en": marca}),
                    remoto,
                );
            }
            if let (Some(id), Some(remoto)) = (&record_id, &remoto) {
                let fecha = remoto["fecha"]
                    .as_str()
                    .map_or_else(sync_repo::ahora_iso, String::from);
                let lapida = json!({
                    "uuid": uuid, "owner": self.cfg.user_id, "deleted": true,
                    "updated_at_client": marca, "fecha": fecha, "payload": {}
                });
                self.enviar(COLECCION_PARTIDOS, Some(id), &lapida)?;
            }
            sync_repo::marcar_hecho(self.conn, uuid)?;
            if pendiente.operacion == "eliminar" {
                self.res.enviados += 1;
            }
            return Ok(());
        };
        payload["uuid"] = json!(uuid);
        if !pendiente.creado_en.is_empty() {
            payload["actualizado_en"] = json!(marca);
        }
        let local_actualizado = payload["actualizado_en"].as_str().unwrap_or("").to_string();
        if let Some(remoto) = &remoto
            && !remoto_actualizado.is_empty()
            && !local_actualizado.is_empty()
            && comparar_fechas(&remoto_actualizado, &local_actualizado) == Ordering::Greater
        {
            return self.conflicto(uuid, &payload, remoto);
        }
        let registro = registro_remoto(&payload, uuid, &self.cfg.user_id);
        self.enviar(COLECCION_PARTIDOS, record_id.as_deref(), &registro)?;
        sync_repo::marcar_hecho(self.conn, uuid)?;
        self.res.enviados += 1;
        Ok(())
    }

    /// Recorre las páginas de una colección (100 por página).
    fn paginas(
        &mut self,
        coleccion: &str,
        filtro: &str,
        orden: &str,
        mut cada: impl FnMut(&mut Self, &Value) -> Result<(), Fallo>,
    ) -> Result<(), Fallo> {
        let mut pagina = 1;
        loop {
            let url = self.url(
                coleccion,
                &format!("?page={pagina}&perPage=100&sort={orden}{filtro}"),
            );
            let json = pedir_json(self.agente, "GET", &url, &self.cfg.token, None)?;
            let items = json["items"].as_array().cloned().unwrap_or_default();
            if items.is_empty() {
                break;
            }
            for item in &items {
                cada(self, item)?;
            }
            let total = json["totalPages"].as_i64().unwrap_or(1);
            pagina += 1;
            if pagina > total {
                break;
            }
        }
        Ok(())
    }

    /// Baja los partidos cambiados desde el cursor (`sync_bajar`). El filtro usa
    /// el cursor del inicio durante toda la bajada; C lo movía entre páginas y
    /// la página 2 salteaba registros.
    fn bajar_partidos(&mut self) -> Result<(), Fallo> {
        let inicial = self.cfg.cursor.clone();
        let filtro = if inicial.is_empty() {
            String::new()
        } else {
            format!(
                "&filter={}",
                codificar(&format!("updated_at_client>\"{inicial}\""))
            )
        };
        let mut cursor = inicial;
        self.paginas(
            COLECCION_PARTIDOS,
            &filtro,
            "updated_at_client",
            |pasada, item| {
                let actualizado = item["updated_at_client"].as_str().unwrap_or("");
                if comparar_fechas(actualizado, &cursor) == Ordering::Greater {
                    cursor = actualizado.to_string();
                }
                pasada.bajar_partido(item)
            },
        )?;
        self.cfg.cursor = cursor;
        sync_repo::config_set(self.conn, "cursor", &self.cfg.cursor)?;
        sync_repo::config_set(self.conn, "ultima_sync", &sync_repo::ahora_iso())?;
        Ok(())
    }

    fn bajar_partido(&mut self, item: &Value) -> Result<(), Fallo> {
        let Some(uuid) = item["uuid"].as_str().filter(|u| !u.is_empty()) else {
            return Ok(());
        };
        if let Some(pendiente) = sync_repo::ultimo_pendiente(self.conn, uuid)? {
            // También cambió acá: conflicto solo si el servidor es más nuevo.
            let local = sync_repo::payload_partido(self.conn, uuid)?;
            let local_actualizado = if pendiente.creado_en.is_empty() {
                local
                    .as_ref()
                    .and_then(|p| p["actualizado_en"].as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                pendiente.creado_en.replace(' ', "T")
            };
            let remoto_actualizado = item["updated_at_client"].as_str().unwrap_or("");
            if !remoto_actualizado.is_empty()
                && !local_actualizado.is_empty()
                && remoto_mas_nuevo(remoto_actualizado, &local_actualizado)
            {
                let local = local.unwrap_or_else(|| json!({"uuid": uuid}));
                self.conflicto(uuid, &local, item)?;
            }
            return Ok(());
        }
        let borrado = item["deleted"] == Value::Bool(true);
        match sync_repo::aplicar_partido(self.conn, item, false) {
            Ok(true) if borrado => self.res.eliminados += 1,
            Ok(true) => self.res.importados += 1,
            Ok(false) => {}
            Err(_) => self.res.errores += 1,
        }
        Ok(())
    }

    /// Baja camisetas y botines (`sync_entidades_bajar`).
    fn bajar_entidades(&mut self) -> Result<(), Fallo> {
        for entidad in [Entidad::Camiseta, Entidad::Botin] {
            self.paginas(entidad.coleccion(), "", "updated", |pasada, item| {
                match sync_repo::aplicar_entidad(pasada.conn, entidad, item) {
                    Ok(Descarga::Importado) => pasada.res.importados += 1,
                    Ok(Descarga::Eliminado) => pasada.res.eliminados += 1,
                    Ok(Descarga::Omitido) => {}
                    Err(_) => pasada.res.errores += 1,
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    /// Sube camisetas, botines y lápidas (`sync_entidades_subir`): solo lo que
    /// es más nuevo que la copia del servidor.
    fn subir_entidades(&mut self) -> Result<(), Fallo> {
        for entidad in [Entidad::Camiseta, Entidad::Botin] {
            for fila in sync_repo::filas_entidad(self.conn, entidad)? {
                let remoto = match self.buscar(entidad.coleccion(), &fila.uuid) {
                    Ok(remoto) => remoto,
                    Err(Fallo::Sesion) => return Err(Fallo::Sesion),
                    Err(_) => {
                        self.res.errores += 1;
                        continue;
                    }
                };
                if let Some(remoto) = &remoto
                    && !remoto_mas_nuevo(
                        &fila.actualizado,
                        remoto["updated_at_client"].as_str().unwrap_or(""),
                    )
                {
                    continue;
                }
                let registro = json!({
                    "uuid": fila.uuid, "owner": self.cfg.user_id, "nombre": fila.nombre,
                    "deleted": false, "payload": fila.payload,
                    "updated_at_client": fila.actualizado,
                });
                let id = remoto.as_ref().and_then(|r| r["id"].as_str());
                match self.enviar(entidad.coleccion(), id, &registro) {
                    Ok(()) => self.res.enviados += 1,
                    Err(Fallo::Sesion) => return Err(Fallo::Sesion),
                    Err(_) => self.res.errores += 1,
                }
            }
        }
        for lapida in sync_repo::lapidas(self.conn)? {
            let Some(entidad) = Entidad::desde_tabla(&lapida.entidad) else {
                sync_repo::borrar_lapida(self.conn, lapida.id)?;
                continue;
            };
            let envio = match self.buscar(entidad.coleccion(), &lapida.uuid) {
                Ok(Some(remoto)) => {
                    let cuerpo = json!({"deleted": true, "updated_at_client": lapida.creado_en});
                    self.enviar(entidad.coleccion(), remoto["id"].as_str(), &cuerpo)
                }
                // Nunca llegó al servidor: nada que borrar.
                Ok(None) => Ok(()),
                Err(fallo) => Err(fallo),
            };
            match envio {
                Ok(()) => {
                    sync_repo::borrar_lapida(self.conn, lapida.id)?;
                    self.res.enviados += 1;
                }
                Err(Fallo::Sesion) => return Err(Fallo::Sesion),
                Err(_) => self.res.errores += 1,
            }
        }
        Ok(())
    }

    /// Una fase; con 401/403 refresca el token una sola vez y la repite (C podía
    /// reintentar sin fin si el servidor seguía respondiendo 401).
    fn fase(&mut self, correr: fn(&mut Self) -> Result<(), Fallo>) -> Result<(), Fallo> {
        match correr(self) {
            Err(Fallo::Sesion) if self.refrescar_token() => correr(self),
            otro => otro,
        }
    }
}

/// Regla de conflicto: ¿la marca `a` es estrictamente más nueva que `b`?
#[must_use]
pub fn remoto_mas_nuevo(a: &str, b: &str) -> bool {
    comparar_fechas(a, b) == Ordering::Greater
}

/// Registro de `partidos_sync` (`sync_registro_remoto`). Como en C,
/// `camiseta_uuid` lleva el nombre de la camiseta.
fn registro_remoto(payload: &Value, uuid: &str, owner: &str) -> Value {
    let mut registro = json!({
        "uuid": uuid,
        "owner": owner,
        "fecha": payload["fecha_hora"].as_str().unwrap_or("1970-01-01T00:00:00"),
        "goles": payload["goles"].as_i64().unwrap_or(0),
        "asistencias": payload["asistencias"].as_i64().unwrap_or(0),
        "updated_at_client": payload["actualizado_en"].as_str().unwrap_or("1970-01-01T00:00:00"),
        "deleted": false,
        "payload": payload,
    });
    if let Some(rival) = payload["rival_nombre"].as_str() {
        registro["rival"] = json!(rival);
    }
    if let Some(camiseta) = payload["camiseta_nombre"].as_str() {
        registro["camiseta_uuid"] = json!(camiseta);
    }
    registro
}

/// Sincroniza cola y cambios remotos (`sync_ejecutar`): por cada servidor,
/// sube partidos, baja y sube camisetas/botines y baja partidos. Si un servidor
/// falla prueba el siguiente.
pub fn ejecutar(conn: &mut Connection) -> Result<Resultado, String> {
    sync_repo::preparar(conn).map_err(|e| e.to_string())?;
    let cfg = Config::cargar(conn).map_err(|e| e.to_string())?;
    if !cfg.con_sesion() {
        return Err("Iniciá sesión de sincronización primero".to_string());
    }
    let lista = servidores(&cfg);
    if lista.is_empty() {
        return Err("Configurá una URL local o remota".to_string());
    }
    let agente = agente();
    let mut resultado = Resultado::default();
    let mut fallos = Vec::new();
    for servidor in lista {
        sync_repo::config_set(conn, "servidor_activo", &servidor).map_err(|e| e.to_string())?;
        let cfg = Config::cargar(conn).map_err(|e| e.to_string())?;
        let mut pasada = Pasada {
            conn: &mut *conn,
            agente: &agente,
            base: servidor.clone(),
            cfg,
            res: &mut resultado,
        };
        let completo = pasada
            .fase(Pasada::subir_partidos)
            .and_then(|()| pasada.fase(Pasada::bajar_entidades))
            .and_then(|()| pasada.fase(Pasada::subir_entidades))
            .and_then(|()| pasada.fase(Pasada::bajar_partidos));
        match completo {
            Ok(()) => {
                resultado.servidor = servidor;
                return Ok(resultado);
            }
            Err(fallo) => fallos.push(format!("{servidor}: {fallo}")),
        }
    }
    Err(format!(
        "No se pudo sincronizar con ninguna dirección. {}",
        fallos.join("; ")
    ))
}

/// Encola todo el historial y sincroniza (`sync_subir_todos`); el uuid evita duplicar.
pub fn subir_todos(conn: &mut Connection) -> Result<Resultado, String> {
    sync_repo::preparar(conn).map_err(|e| e.to_string())?;
    sync_repo::encolar_todos(conn).map_err(|e| e.to_string())?;
    ejecutar(conn)
}

/// Resuelve el primer conflicto: conservar la copia local (se vuelve a subir) o
/// la del servidor (se aplica forzada).
pub fn resolver_conflicto(conn: &mut Connection, conservar_local: bool) -> Result<bool, DbError> {
    let Some(conflicto) = sync_repo::primer_conflicto(conn)? else {
        return Ok(false);
    };
    if conservar_local {
        sync_repo::tocar_partido(conn, &conflicto.uuid)?;
    } else {
        sync_repo::aplicar_partido(conn, &conflicto.remoto, true)?;
        sync_repo::marcar_hecho(conn, &conflicto.uuid)?;
    }
    sync_repo::borrar_conflicto(conn, conflicto.id)?;
    Ok(true)
}

/// Datos de la pantalla de estado (sin el token).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Estado {
    /// Configuración (el token solo se usa para saber si hay sesión).
    pub config: Config,
    /// Partidos pendientes de subir.
    pub pendientes: i64,
    /// Conflictos sin resolver.
    pub conflictos: i64,
    /// Primer conflicto: (uuid, local, servidor) en una línea cada uno.
    pub conflicto: Option<(String, String, String)>,
}

/// Estado actual (`sync_menu_estado`).
pub fn estado(conn: &Connection) -> Result<Estado, DbError> {
    let resumen = |valor: &Value| {
        format!(
            "{} · goles {} · asist. {}",
            valor["fecha_hora"].as_str().unwrap_or("-"),
            valor["goles"].as_i64().unwrap_or(0),
            valor["asistencias"].as_i64().unwrap_or(0)
        )
    };
    Ok(Estado {
        config: Config::cargar(conn)?,
        pendientes: sync_repo::contar_pendientes(conn)?,
        conflictos: sync_repo::contar_conflictos(conn)?,
        conflicto: sync_repo::primer_conflicto(conn)?.map(|c| {
            (
                c.uuid,
                resumen(&c.local),
                resumen(c.remoto.get("payload").unwrap_or(&Value::Null)),
            )
        }),
    })
}

/// Sincronización en un hilo con su propia conexión a la base: la TUI nunca
/// espera la red.
#[derive(Debug, Default)]
pub struct SyncWorker {
    canal: Option<Receiver<Result<Resultado, String>>>,
}

impl SyncWorker {
    /// ¿Hay una sincronización en curso?
    #[must_use]
    pub const fn ocupado(&self) -> bool {
        self.canal.is_some()
    }

    /// Lanza la sincronización sobre la base de `ruta`. `false` si ya había una.
    pub fn lanzar(&mut self, ruta: PathBuf, todos: bool) -> bool {
        if self.canal.is_some() {
            return false;
        }
        let (envio, recepcion) = mpsc::channel();
        std::thread::spawn(move || {
            let resultado = Database::open(&ruta)
                .map_err(|error| error.to_string())
                .and_then(|mut db| {
                    if todos {
                        subir_todos(db.conn_mut())
                    } else {
                        ejecutar(db.conn_mut())
                    }
                });
            let _ = envio.send(resultado);
        });
        self.canal = Some(recepcion);
        true
    }

    /// Resultado listo, sin esperar.
    pub fn recibir(&mut self) -> Option<Result<Resultado, String>> {
        let resultado = match self.canal.as_ref()?.try_recv() {
            Ok(resultado) => resultado,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                Err("La sincronización terminó sin resultado".to_string())
            }
        };
        self.canal = None;
        Some(resultado)
    }

    /// Espera el resultado (solo tests).
    pub fn esperar(&mut self, limite: Duration) -> Option<Result<Resultado, String>> {
        let resultado = self.canal.as_ref()?.recv_timeout(limite).ok()?;
        self.canal = None;
        Some(resultado)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normaliza_urls_como_c() {
        assert_eq!(
            normalizar_url(" 192.168.0.10:8090/ ").as_deref(),
            Some("http://192.168.0.10:8090")
        );
        assert_eq!(
            normalizar_url("https://pb.example.com///").as_deref(),
            Some("https://pb.example.com")
        );
        assert_eq!(normalizar_url("ftp://x"), None);
        assert_eq!(normalizar_url("http://a b"), None);
        assert_eq!(normalizar_url("http://a%20"), None);
        assert_eq!(normalizar_url(""), None);
    }

    #[test]
    fn servidores_sin_repetir_y_como_mucho_dos() {
        let cfg = Config {
            servidor_activo: "http://b".into(),
            url_local: "http://a".into(),
            url_remota: "http://b/".into(),
            ..Config::default()
        };
        assert_eq!(servidores(&cfg), vec!["http://b", "http://a"]);
    }

    #[test]
    fn regla_de_conflicto_estricta() {
        assert!(remoto_mas_nuevo(
            "2026-01-02T00:00:00",
            "2026-01-01 23:59:59"
        ));
        assert!(!remoto_mas_nuevo(
            "2026-01-01T10:00:00",
            "2026-01-01 10:00:00"
        ));
        assert!(!remoto_mas_nuevo(
            "2025-12-31T10:00:00",
            "2026-01-01T10:00:00"
        ));
    }

    #[test]
    fn codifica_el_filtro() {
        assert_eq!(codificar("uuid='a-1'"), "uuid%3D%27a-1%27");
    }

    // ------------------------------------------- PocketBase falso (sin red real)

    use crate::db::Database;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// (método, ruta con query, Authorization, cuerpo).
    type Peticion = (String, String, String, String);

    struct Falso {
        base: String,
        peticiones: Arc<Mutex<Vec<Peticion>>>,
    }

    /// Servidor HTTP mínimo en 127.0.0.1:0 que responde con `responder`.
    fn falso(responder: impl Fn(&Peticion) -> (u16, String) + Send + 'static) -> Falso {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let peticiones = Arc::new(Mutex::new(Vec::new()));
        let registro = Arc::clone(&peticiones);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut lector = BufReader::new(stream.try_clone().unwrap());
                let mut linea = String::new();
                if lector.read_line(&mut linea).unwrap_or(0) == 0 {
                    continue;
                }
                let mut partes = linea.split_whitespace();
                let metodo = partes.next().unwrap_or("").to_string();
                let ruta = partes.next().unwrap_or("").to_string();
                let (mut largo, mut auth) = (0, String::new());
                loop {
                    let mut cabecera = String::new();
                    if lector.read_line(&mut cabecera).unwrap_or(0) == 0 || cabecera == "\r\n" {
                        break;
                    }
                    if let Some((nombre, valor)) = cabecera.split_once(':') {
                        match nombre.to_ascii_lowercase().as_str() {
                            "content-length" => largo = valor.trim().parse().unwrap_or(0),
                            "authorization" => auth = valor.trim().to_string(),
                            _ => {}
                        }
                    }
                }
                let mut cuerpo = vec![0; largo];
                let _ = lector.read_exact(&mut cuerpo);
                let peticion = (
                    metodo,
                    ruta,
                    auth,
                    String::from_utf8_lossy(&cuerpo).into_owned(),
                );
                let (status, respuesta) = responder(&peticion);
                registro.lock().unwrap().push(peticion);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{respuesta}",
                    respuesta.len()
                );
            }
        });
        Falso { base, peticiones }
    }

    fn vacio() -> (u16, String) {
        (200, r#"{"items":[],"totalPages":1}"#.to_string())
    }

    /// Base con un partido pendiente (`u-1`) y sesión contra `base`.
    fn base_con_sesion(base: &str) -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO cancha (id, nombre) VALUES (1, 'La Bombonera');
                 INSERT INTO camiseta (id, nombre) VALUES (1, 'Titular');
                 INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, mes_anio, goles, uuid, updated_at)
                 VALUES (1, 1, '2026-03-10 20:00:00', '2026-03', 2, 'u-1', '2026-03-10 22:00:00');",
            )
            .unwrap();
        configurar(db.conn(), base, "").unwrap();
        for (clave, valor) in [("token", "tok"), ("user_id", "usr1")] {
            sync_repo::config_set(db.conn(), clave, valor).unwrap();
        }
        db
    }

    #[test]
    fn login_guarda_el_token_y_nunca_la_contrasena() {
        let pb = falso(|(metodo, ruta, _, _)| {
            if metodo == "POST" && ruta.ends_with("/auth-with-password") {
                (
                    200,
                    r#"{"token":"tok-123","record":{"id":"usr1"}}"#.to_string(),
                )
            } else {
                (404, "{}".to_string())
            }
        });
        let db = Database::open_in_memory().unwrap();
        configurar(db.conn(), &pb.base, "").unwrap();
        assert_eq!(
            iniciar_sesion(db.conn(), " yo@x.com ", "clave-secreta").unwrap(),
            pb.base
        );
        let cfg = Config::cargar(db.conn()).unwrap();
        assert_eq!(
            (cfg.token.as_str(), cfg.user_id.as_str()),
            ("tok-123", "usr1")
        );
        assert_eq!(cfg.email, "yo@x.com");
        assert_eq!(cfg.servidor_activo, pb.base);
        let guardado: String = db
            .conn()
            .query_row(
                "SELECT group_concat(valor, '|') FROM sync_config",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!guardado.contains("clave-secreta"));
        let cuerpo: Value = serde_json::from_str(&pb.peticiones.lock().unwrap()[0].3).unwrap();
        assert_eq!(
            cuerpo,
            json!({"identity": "yo@x.com", "password": "clave-secreta"})
        );
    }

    #[test]
    fn login_rechazado_da_el_mensaje_de_c() {
        let pb = falso(|(_, _, _, cuerpo)| {
            if cuerpo.contains("mala") {
                (400, "{}".into())
            } else {
                (500, "{}".into())
            }
        });
        let db = Database::open_in_memory().unwrap();
        configurar(db.conn(), &pb.base, "").unwrap();
        let error = iniciar_sesion(db.conn(), "yo@x.com", "mala").unwrap_err();
        assert!(error.contains("Correo o contraseña incorrectos"), "{error}");
        let error = iniciar_sesion(db.conn(), "yo@x.com", "otra").unwrap_err();
        assert!(
            error.contains("rechazó el inicio de sesión (500)"),
            "{error}"
        );
        assert!(!Config::cargar(db.conn()).unwrap().con_sesion());
    }

    #[test]
    fn sin_servidor_falla_sin_colgarse() {
        // Puerto que se libera enseguida: conexión rechazada.
        let puerto = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let mut db = base_con_sesion(&format!("http://127.0.0.1:{puerto}"));
        let error = ejecutar(db.conn_mut()).unwrap_err();
        assert!(error.contains("No se pudo sincronizar"), "{error}");
        assert!(
            !error.contains("tok"),
            "el token no aparece en el error: {error}"
        );
        // El partido sigue en la cola para la próxima.
        assert_eq!(sync_repo::contar_pendientes(db.conn()).unwrap(), 1);
    }

    #[test]
    fn sube_la_cola_y_baja_partidos_y_camisetas() {
        let pb = falso(|(metodo, ruta, auth, _)| {
            assert_eq!(auth, "tok", "cada petición lleva el token");
            match (metodo.as_str(), ruta.as_str()) {
                ("GET", r) if r.contains("/partidos_sync/records?page=1") => (
                    200,
                    json!({"totalPages": 1, "items": [{
                        "uuid": "remoto-1", "updated_at_client": "2026-05-01T10:00:00",
                        "deleted": false,
                        "payload": {"fecha_hora": "2026-05-01T10:00:00", "goles": 4,
                                    "asistencias": 1, "cancha_nombre": "Remota",
                                    "camiseta_nombre": "Titular"}
                    }]})
                    .to_string(),
                ),
                ("GET", r) if r.contains("/camisetas_sync/records?page=1") => (
                    200,
                    json!({"totalPages": 1, "items": [{
                        "id": "c9", "uuid": "cam-remota", "nombre": "Suplente",
                        "updated": "2026-05-01 10:00:00.000Z",
                        "updated_at_client": "2026-05-01T10:00:00.000Z",
                        "deleted": false, "payload": {"nombre": "Suplente", "marca": "Adidas"}
                    }]})
                    .to_string(),
                ),
                ("GET", _) => vacio(),
                ("POST", _) => (200, "{}".to_string()),
                _ => (404, "{}".to_string()),
            }
        });
        let mut db = base_con_sesion(&pb.base);
        let resultado = ejecutar(db.conn_mut()).unwrap();
        assert_eq!(resultado.servidor, pb.base);
        assert_eq!(resultado.errores, 0);
        assert!(resultado.enviados >= 2, "partido + camiseta: {resultado:?}");
        assert_eq!(resultado.importados, 2, "{resultado:?}");
        let conn = db.conn();
        assert_eq!(sync_repo::contar_pendientes(conn).unwrap(), 0);
        let goles: i64 = conn
            .query_row(
                "SELECT goles FROM partido WHERE uuid = 'remoto-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(goles, 4);
        let marca: String = conn
            .query_row(
                "SELECT marca FROM camiseta WHERE uuid = 'cam-remota'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(marca, "Adidas");
        let cfg = Config::cargar(conn).unwrap();
        assert_eq!(cfg.cursor, "2026-05-01T10:00:00");
        assert!(!cfg.ultima_sync.is_empty());
        let peticiones = pb.peticiones.lock().unwrap();
        let subida = peticiones
            .iter()
            .find(|(m, r, _, _)| m == "POST" && r.ends_with("/partidos_sync/records"))
            .expect("POST del partido");
        let registro: Value = serde_json::from_str(&subida.3).unwrap();
        assert_eq!(registro["uuid"], "u-1");
        assert_eq!(registro["owner"], "usr1");
        assert_eq!(registro["goles"], 2);
        assert_eq!(registro["camiseta_uuid"], "Titular", "compatibilidad con C");
        assert_eq!(registro["payload"]["cancha_nombre"], "La Bombonera");
    }

    #[test]
    fn servidor_mas_nuevo_deja_conflicto_y_se_resuelve() {
        let pb = falso(|(metodo, ruta, _, _)| match metodo.as_str() {
            "GET" if ruta.contains("/partidos_sync/records?filter=") => (
                200,
                json!({"items": [{
                    "id": "r1", "uuid": "u-1", "updated_at_client": "2999-01-01T00:00:00",
                    "deleted": false,
                    "payload": {"uuid": "u-1", "fecha_hora": "2026-03-10T20:00:00", "goles": 7,
                                "asistencias": 3, "cancha_nombre": "La Bombonera",
                                "camiseta_nombre": "Titular"}
                }]})
                .to_string(),
            ),
            "GET" => vacio(),
            _ => (200, "{}".to_string()),
        });
        let mut db = base_con_sesion(&pb.base);
        let resultado = ejecutar(db.conn_mut()).unwrap();
        assert_eq!(resultado.conflictos, 1, "{resultado:?}");
        let estado = estado(db.conn()).unwrap();
        assert_eq!(estado.conflictos, 1);
        let (_, local, servidor) = estado.conflicto.unwrap();
        assert!(
            local.contains("goles 2") && servidor.contains("goles 7"),
            "{local} / {servidor}"
        );
        assert!(
            !pb.peticiones
                .lock()
                .unwrap()
                .iter()
                .any(|(m, _, _, _)| m == "PATCH"),
            "no pisa la copia más nueva del servidor"
        );
        assert!(resolver_conflicto(db.conn_mut(), false).unwrap());
        let goles: i64 = db
            .conn()
            .query_row("SELECT goles FROM partido WHERE uuid = 'u-1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(goles, 7);
        assert_eq!(sync_repo::contar_conflictos(db.conn()).unwrap(), 0);
        assert!(!resolver_conflicto(db.conn_mut(), true).unwrap());
    }

    #[test]
    fn rechazo_400_no_frena_y_401_refresca_el_token() {
        let pb = falso(|(metodo, ruta, auth, _)| {
            if ruta.ends_with("/auth-refresh") {
                return (200, r#"{"token":"nuevo"}"#.to_string());
            }
            if auth != "nuevo" {
                return (401, r#"{"message":"expired"}"#.to_string());
            }
            match metodo.as_str() {
                "GET" => vacio(),
                "POST" if ruta.contains("/partidos_sync/") => {
                    (400, r#"{"message":"Failed to create record."}"#.to_string())
                }
                _ => (200, "{}".to_string()),
            }
        });
        let mut db = base_con_sesion(&pb.base);
        let resultado = ejecutar(db.conn_mut()).unwrap();
        assert_eq!(resultado.errores, 1, "{resultado:?}");
        assert_eq!(Config::cargar(db.conn()).unwrap().token, "nuevo");
        let (estado, error): (String, String) = db
            .conn()
            .query_row(
                "SELECT estado, ultimo_error FROM sync_dirty WHERE uuid = 'u-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(estado, "error");
        assert!(
            error.contains("HTTP 400") && error.contains("Failed to create"),
            "{error}"
        );
    }

    #[test]
    fn el_hilo_devuelve_el_resultado() {
        let pb = falso(|(metodo, _, _, _)| {
            if metodo == "GET" {
                vacio()
            } else {
                (200, "{}".into())
            }
        });
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("sync.db");
        {
            let db = Database::open(&ruta).unwrap();
            configurar(db.conn(), &pb.base, "").unwrap();
            sync_repo::config_set(db.conn(), "token", "tok").unwrap();
            sync_repo::config_set(db.conn(), "user_id", "usr1").unwrap();
        }
        let mut worker = SyncWorker::default();
        assert!(worker.lanzar(ruta.clone(), false));
        assert!(worker.ocupado());
        assert!(!worker.lanzar(ruta, false), "una sola a la vez");
        let resultado = worker.esperar(Duration::from_secs(20)).unwrap().unwrap();
        assert_eq!(resultado.servidor, pb.base);
        assert!(!worker.ocupado());
    }
}
