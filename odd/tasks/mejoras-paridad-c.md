# Mejoras tras el análisis: errores, flujo MiFutbolC, rendimiento, funciones y limpieza

## Objetivo
Implementar los 5 bloques propuestos en el análisis del 2026-11-23 ("Implementa Todo"), con
MiFutbolC (`D:\ANIME\Libros\Lenguaje C\Proyectos\MiFutbolC`) como referencia de flujo.

## Problema / por qué
El análisis encontró un bypass de contraseñas, un panic con terminales angostas, escrituras sin
transacción, búsquedas rotas con `%`/`_`, y un flujo de menú que se aleja del de C (opciones ocultas
tras teclas, Análisis ausente, modo Simple/Avanzado sin efecto, sin restaurar backup al inicio).

## Alcance autorizado
- B1 Errores 1–7 del análisis.
- B2 Flujo C: menú numerado completo (Calendario, Logros, Records, Análisis, Tutorial), números de
  dos dígitos que abren, modo Simple/Avanzado filtra, oferta de restaurar backup al arrancar,
  Esc/`q` coherentes, teclas globales que no se cuelen en módulos.
- B3 Rendimiento: búsqueda solo al cambiar el texto, `go_back` con `inicio_sucio`, filtros por
  `mes_anio`/rango, migración 015 con índices de FKs, caché de imágenes acotada.
- B4 Funciones de C: Análisis (básico, evolución mensual, química), reactivar camisetas,
  administrar backups (listar/restaurar/eliminar), VACUUM/ANALYZE/integridad, exportar todo a
  JSON y Markdown.
- B5 Limpieza: un helper de selección, un flujo de borrado común, textos de ayuda compartidos,
  quitar `Screen::Module`/`module_placeholder`.

## Ampliación (reabierto: "Implementa Todo" incluye todo lo que el análisis listó como faltante)
- B6 Sincronización PocketBase + sincronización/exportación MiFutbolMobile.
- B7 Exportar/Importar: XLSX, importar JSON/TXT/CSV/HTML, recordatorios import/export y filtro por
  tema, backup JSON de Colecciones.
- B8 Ajustes y arranque: backup automático al inicio, editor de menús personalizados, comprobar
  actualizaciones, restablecer valores, ver configuración, idioma y modo retro como en C.
- B9 Módulos: simular partido con equipos guardados, lesiones (diferencias, actualizar estados),
  estadísticas (reportes automáticos, filtros avanzados, tabla de posiciones global), amigos
  "comparar conmigo", notificaciones (crear, cumpleaños, torneos por vencer), historial de
  equipamiento, gráfico de uso de camisetas, administrar torneos/temporadas completo.
- B10 Bienestar: pantallas de análisis (entrenamiento vs rendimiento, alimentación), informe
  mensual PDF, abrir documentos con la app del sistema, optimizar imágenes al copiarlas,
  importador copia `MiFutbolC\images`.
- B11 Migrar chrono → jiff (RUSTSEC-2026-0543) y quitar el ignore de `deny.toml`.

## Restricciones
- Sin dependencias nuevas, salvo `jiff` para reemplazar `chrono` (B11).
- Tests y docs junto al comportamiento; commit Conventional por tarea.

## Configuración
- TDD: desactivado (fuente: feature anterior `recomendaciones.md`, sin configuración). Runner: `cargo test`.
- RDD: `gentle-ai` no disponible → disabled/unmanaged.
- Entrega: ask-on-risk; previsión > 400 líneas → rama única `feature/mejoras-paridad-c`, commit por
  bloque; división en PRs a decisión del usuario.
- Engram: espejo `odd/mejoras-paridad-c/tasks`.

## Tareas
- [x] B1 Errores. Ruta: delegated (writer trigger: ~8 archivos).
- [x] B2 Flujo C. Ruta: delegated (writer trigger: app.rs, ui, ajustes, main).
- [x] B3 Rendimiento. Ruta: delegated (writer trigger: repos + migración + app).
- [x] B4 Funciones de C. Ruta: delegated (writer trigger: servicios + pantallas nuevas).
- [x] B5 Limpieza. Ruta: delegated (writer trigger: app_*.rs).
- [x] B6 Sincronización PocketBase + MiFutbolMobile. Ruta: delegated (writer trigger). Commit `e9cd489`.
- [x] B7 Exportar/Importar. Ruta: delegated (writer trigger). Commits `1242ee7`, `ed27f87`, `6b6c979`, `711d2c3`.
- [x] B8 Ajustes y arranque. Ruta: delegated (writer trigger). Commit `fdcb3ab`.
- [ ] B9 Módulos faltantes. Ruta: delegated (writer trigger).
- [ ] B10 Bienestar restante. Ruta: delegated (writer trigger).
- [ ] B11 chrono → jiff. Ruta: delegated (writer trigger: todo el crate).

## Criterios de aceptación
- Cada bloque: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`,
  `cargo fmt --check` OK; tests nuevos para cada error y cada lógica nueva.

## Progreso / evidencia
- Línea base: 307 tests OK, clippy y fmt limpios (master `d589bde`).
  (Recuento real de `#[test]` en `d589bde`: 257; el 307 anterior no coincide con `cargo test`.)
- B1 (`786a43a`, verificado por el orquestador: 263 OK, clippy limpio; RDD unmanaged):
  - 1 Seguridad: `usuario_service::cambiar_password(conn, id, actual, nueva)` y
    `eliminar(conn, id, actual)` verifican la contraseña actual (acepta hash `fnv1$` de C);
    el formulario de cambio pide "Contraseña actual" si la cuenta tiene, y borrar abre la
    pantalla `Screen::UsuarioEliminar` tras confirmar (como `eliminar_mi_cuenta_local` de C).
  - 2 `ui::draw` muestra "Terminal demasiado pequeña (mín. 40x12)" bajo 40x12; modal y
    overlay de búsqueda acotados al área (sin `clamp` con máximo < mínimo).
  - 3 `like_escape` (escapa `\`, `%`, `_`) + `ESCAPE '\'` en los 27 `LIKE` de `src/db`
    (búsqueda global y filtros de listados).
  - 4 `torneo_repo::reemplazar_fixture`: borrado + inserción en la misma transacción.
  - 5 Transacciones en `undo_service::deshacer`, cada migración + `user_version`, y
    `temporadas_service::crear`. Migraciones revisadas: los `BEGIN` son cuerpos de triggers.
  - 6 Evento de recordatorio en Inicio abre `Module::Recordatorios`.
  - 7 `calendario_repo::fecha_de` usa `texto.get(..10)`; el resto de slices en código no
    test opera sobre ASCII verificado (`validar_mes`) o son tests.
  - Tests nuevos (6): contraseña actual (servicio + UI), terminal 30x10 con modal, LIKE
    literal (búsqueda y filtro), fixture revertido, undo fallido sin `suspendido=1`,
    `fecha_de` multibyte. Total 263 OK; `cargo clippy --all-targets -D warnings` y
    `cargo fmt --check` limpios.
- B2 (`200934a`, verificado por el orquestador: 275 OK, clippy limpio; RDD unmanaged):
  - 1 Menú en orden y nombres de `menu.c` `MENU_ITEMS`: `Module` suma Calendario, Logros,
    Análisis, Records y Tutorial; 1..24 como C y extras 25 Progresión, 26 Entrenador IA,
    27 Usuario; "0 Salir" al pie. Sincronización (25 en C) fuera de alcance. Análisis es un
    hub (`Screen::AnalisisMenu`, `ANALISIS_SECCIONES` en `app_comparador.rs`) con Comparador
    A/B y Entrenador IA; B4 suma básico/evolución/química.
  - 2 Números: `teclear_numero_menu` abre en cuanto el número deja de ser prefijo de otra
    opción visible ("3" ya, "1"+"5" = 15); Enter abre lo tecleado; Esc lo descarta; el bucle
    llama `vencer_numero_menu` y abre tras `ESPERA_NUMERO_MENU` (900 ms). `0` = Salir con
    confirmación. Sidebar, tabs y lista muestran el número fijo.
  - 3 `App::menu_modulos` filtra por modo como `DEFINE_GET_MENU` de `settings.c`: Simple =
    Calendario, Camisetas, Canchas, Botines, Partidos, Lesiones, Tutorial, Ajustes (+ Usuario,
    única vía a cuentas en Rust). Personalizado lee `custom_menus` con las claves de C
    (`Module::key`): oculta solo filas `enabled = 0` (sin fila = visible, porque Rust no tiene
    editor de menús y sus seeds no traen todas; C las ocultaría); Ajustes siempre visible.
    Diferencias con C: los números no se renumeran al filtrar (pedido); Torneos no se oculta
    siempre en Personalizado (en C `custom=0`, parece un error).
  - 4 `ofrecer_restaurar_backup_inicio` (como `menu.c`): al entrar al menú (portada `i`/Enter
    o login), una vez por sesión, si la base no tiene datos (`db::tiene_datos_usuario`, mismas
    11 tablas que C) y el backup más reciente de `Exportaciones/` sí, modal S/N con la
    cantidad de partidos → `restaurar_backup`. Solo modal; `App::new` no lo dispara (tests).
    Rust no hace backup automático al iniciar como C, así que solo cuentan los `backup_*.db`.
  - 5 Esc en el menú pide confirmar salir (igual que `q`); en la portada `q`/Esc piden
    confirmar y `i` sigue entrando como invitado. Footer y ayuda actualizados.
  - 6 Atajos de letra (`c l n t R T U C E D A M Y P L`) solo en el menú; `g ! ?`, Ctrl+F/K/Z
    siguen globales.
  - Tests: helper `abrir_opcion`; `test_app()` usa modo Avanzado (el defecto es Simple);
    navegación por índice ajustada al nuevo orden; el test del menú verifica las 27 opciones
    numeradas y "0 Salir". Nuevos (12):
    Simple filtra y conserva números, defecto Simple, Personalizado, dos cifras, pausa,
    0/Esc salen, nuevas opciones abren, hub Análisis, atajos no saltan desde módulos,
    portada q/Esc, oferta de restaurar backup, `menus_ocultos`. Total 275 OK
    (`cargo test --all-targets`), clippy `-D warnings` y `cargo fmt --check` limpios.
  - No hecho: editor de menús personalizados (C `menu_custom_menus`) — no pedido.
- B3 (`1fa740c`, verificado por el orquestador: 283 OK, clippy limpio; RDD unmanaged):
  - 1 Búsqueda global (`handle_overlay_key`): guarda el texto antes de la tecla y solo llama a
    `busqueda_service::buscar` si cambió; flechas/Enter ya no repiten los ~8 `LIKE`.
  - 2 `go_back` al menú recalcula el Inicio solo si `inicio_sucio` o si
    `conn.total_changes()` cambió desde el último `refrescar_inicio` (nuevo campo
    `App::inicio_cambios`). Así cualquier guardado/borrado/undo/import por la conexión de la
    app lo marca sin tocar cada formulario; restaurar backup (API de backup, no cuenta en
    `total_changes`) pone `inicio_sucio = true`. `handle_key` sin cambios.
  - 3 Filtros por mes con rango indexable `col >= ?1 AND col < ?2` (`dates::rango_mes`,
    diciembre → enero del año siguiente; mes inválido = rango vacío): `inicio_repo::mes`,
    `calendario_repo::eventos_mes` (partido, financiamiento, recordatorio) y
    `financiamiento_repo::gasto_mes`. No se usó `partido.mes_anio` aunque el formato es el
    mismo (`YYYY-MM`): las inserciones directas (tests, imports) lo dejan en `''`. Los índices
    de fecha ya existían (`idx_partido_fecha_hora`, `idx_financiamiento_fecha`,
    `idx_recordatorio_fecha`), así que 015 no suma índices de fecha.
  - 4 `migrations/015_fk_indexes.sql`: equipo(partido_id), media(partido_id),
    bienestar_sesion_mental(partido_id) (la auditoría decía `bienestar_entrenamiento`, que no
    tiene `partido_id`), partido_torneo(equipo1_id), partido_torneo(equipo2_id),
    equipo_historial(equipo_id), equipo_historial(torneo_id), progresion_jugador(plan_id),
    torneo(equipo_fijo_id). `progresion_jugador(jugador_id)` ya lo cubre el UNIQUE
    (jugador_id, plan_id, semana). `cuenta_sesion.usuario_id` sin índice (tabla mínima).
  - 5 `ImageCache`: `acotar` vacía el mapa (protocolos y logo) al llegar a 32 entradas, así los
    redimensionados no lo hacen crecer sin límite (atajo documentado: no es LRU).
  - Tests nuevos (8): búsqueda no reconsulta con flechas, `go_back` sin escrituras no
    recalcula / con escritura sí, `rango_mes`, borde diciembre→enero en inicio, calendario y
    gasto del mes, índices de 015 + `EXPLAIN QUERY PLAN` usa `idx_partido_fecha_hora`, caché
    de imágenes acotada. Total 283 OK (`cargo test --all-targets`), clippy
    `-D warnings` y `cargo fmt` limpios.
  - No hecho: benches (`benches/queries.rs` no cubre Inicio/Calendario); filtros por mes de
    `informe_service` (bienestar) siguen con `substr` — fuera de la lista auditada.
- B4 (4 commits; RDD unmanaged; 297 OK en `cargo test --all-targets`, clippy `-D warnings`
  y `cargo fmt` limpios en cada commit):
  - 4b `3d2b735` Camisetas: `a` en listado y ficha llama a `camiseta_service::set_activa`
    (antes sin uso). Las inactivas ya se veían (filtro `f`, indicador INACTIVA). Sin
    confirmación, igual que el `a` de canchas/equipos (C pide confirmar al reactivar).
  - 4a `f1912dd` Análisis (hub en el orden de C: básico, comparador, Entrenador IA, química,
    más evolución temporal): `domain::analisis` (cálculos puros), `analisis_repo::serie`,
    `analisis_service`, `app_analisis.rs`, `ui/screens/analisis.rs`. Básico = últimos 5 vs
    promedio (goles, asistencias, rendimiento, cansancio, ánimo), rachas
    (`actualizar_rachas`: empate corta ambas) y mensaje motivacional (umbral ±0,5).
    Evolución = las 8 opciones de `mostrar_evolucion_temporal` en una pantalla con BarChart de
    12 meses (`m` cambia métrica). Química = mejor dupla con la CTE `duplas_auto` de C
    (winrate sin empates, como C) + CRUD de duplas con `quimica_service` (antes sin uso). La
    parte `duplas_manual` de C no se porta: su tabla en C guarda datos por partido y la de
    Rust son agregados por dupla, que se listan aparte.
  - 4c `43a5cbe` Ajustes → Ctrl+G: `backup_service::listar` (backup_*.db, fecha, tamaño,
    partidos), Enter restaura (confirmación, reutiliza `restaurar_backup_desde` con copia
    pre_restore), `d` elimina (confirmación, solo `backup_*.db`), `b` crea. No lista
    `pre_restore_*` ni copias `.bak_` de migración (fuera del patrón pedido).
  - 4d+4e `b3aeb81` Ajustes → Ctrl+T: backup previo `backup_*_pre_vacuum.db` (como C), luego
    `Database::mantenimiento` = VACUUM + ANALYZE + `wal_checkpoint(TRUNCATE)` +
    `integrity()` (integrity_check y foreign_key_check), con tamaño antes/después en un modal.
    Ctrl+X: `informe_service::exportar_todo` → `mifutbol_<sello>.json` (todas las tablas de
    `sqlite_master` salvo `sqlite_*`, `undo_*`, `usuario`, `cuenta`, `cuenta_sesion` para no
    exportar hashes de contraseña; BLOB → null, el esquema no tiene BLOB) y
    `mifutbol_<sello>.md` (partidos, camisetas, canchas, botines, lesiones, financiamiento,
    torneos; reutiliza las consultas del ODS). Un solo commit porque comparten el bloque Ctrl
    de Ajustes.
  - Errores de C encontrados: `comparar_inicio_fin_anio` rotula ambos semestres "(Ene-Jun)";
    los `strftime` de evolución/mejor mes devuelven NULL con fechas DD/MM/AAAA heredadas
    (meses "DESCONOCIDO") y `ORDER BY fecha_hora` las ordena mal — Rust parsea ISO y salta
    las inválidas.
  - Tests nuevos: dominio (rachas, básico, evolución), servicio (básico/evolución desde BD,
    mejor combinación), `set_activa`, `backup_service`, `mantenimiento`, `exportar_todo`
    (JSON parseado con serde_json + cabeceras MD) y UI (camisetas `a`, 3 pantallas de
    Análisis, hub con 5 entradas, backups, Ctrl+T/Ctrl+X).
  - Engram `odd/mejoras-paridad-c/tasks`: sincronizado por el orquestador (obs. #71) tras cada bloque.
- Cierre: 10 commits en `feature/mejoras-paridad-c`, 89 archivos, +4193/−970; 298 tests OK, clippy y fmt limpios (verificado por el orquestador). Sin push ni PR (decisión del usuario).
- B5 (`28b58fe`; RDD unmanaged; 298 OK en `cargo test --all-targets`, clippy `-D warnings` y
  `cargo fmt --check` limpios; `git diff --stat`: 34 archivos, +417 / -618):
  - 1 Selección: `app::mover_seleccion` (vuelve al otro extremo; lista vacía = `None`) y
    `App::mover_tabla_seleccion` la usa. Fuera las 9 copias `move_*_selection`/`mover_usuario`
    y los bloques `min`/`saturating_sub` (amigos, logros, avisos, últimos, récords,
    progresión, planes, recordatorios, agenda, temporadas, torneos y su fixture); hubs
    (menú, carrera, estadísticas, análisis) también. Cambio visible: los listados que
    frenaban en el borde ahora dan la vuelta (`usuario_ui` ajustado). Test unitario nuevo.
  - 2 Borrado: `App::tras_eliminar(mensaje, es_ficha, recargar)` para camisetas, canchas,
    botines, equipos, jugadores, partidos, lesiones y transacciones; cada uno solo arma el
    mensaje (sus cuatro `ResultadoEliminacion` son tipos distintos). Los `confirm_delete_*`
    quedan: son 10 líneas y el de jugador arma el texto con nombre y número.
  - 3 Textos: `detail::AYUDA_VISTA_PREVIA` = "Enter: detalle   e: editar   d: borrar" en las 5
    vistas previas (lo más usado en la barra inferior); "d: borrar" también en finanzas,
    backups y la ayuda. La barra inferior sigue con literales (no se puede `concat!` de una
    `const`). El título de la ficha de equipo mostraba teclas: ahora "Equipos · detalle".
  - 4 Código muerto: fuera `Screen::Module` + `module_placeholder`, `ImageCache::limpiar` (sin
    uso) y el parámetro `colors` de `dibujar_miniatura`/`dibujar_logo`. El único
    `#[allow(dead_code)]` es el de `tests/common` (helpers compartidos): queda.
  - 5 Errores: `confirmar_eliminacion` recibe un `despues` que devuelve `Result` y muestra su
    error; modal o barra para avisos (`generar`, marcar leída/todas), `notificar_logros`,
    tutorial, bienvenida, refresco tras deshacer, refresco del clima y copia previa a
    restaurar (se avisa en el modal de éxito; la restauración sigue igual). Quedan los
    `let _` intencionales: `refresh_usuarios`/`refrescar_inicio` (siempre `Ok`, avisan
    solos), `PRAGMA optimize`, backup best-effort previo a migrar, logging, `DETACH`,
    `write!` a `String`.
  - No hecho: las listas de los overlays (búsqueda global, paleta) y el selector A/B del
    comparador siguen frenando en el borde (no son tablas).
- B6 (`e9cd489`; RDD unmanaged; 322 OK en `cargo test --all-targets` (298 + 24), clippy
  `-D warnings` y `cargo fmt --check` limpios; 20 archivos, +4212 / -27 — supera la
  heurística de ~400 líneas: es un port completo de 4 archivos de C más el formato móvil y
  no se partió en dos commits porque `app_sync.rs` y el bloque Ctrl de Ajustes cruzan 6a/6b):
  - 6a PocketBase: migración `016_sync.sql` (uuid en partido/camiseta/botin, columnas de
    botín que C sincroniza, `sync_config`, `sync_dirty`, `sync_conflicto`,
    `sync_tombstones`, triggers de cola y lápidas). `db::sync_repo` (cola, payload con los
    nombres/textos de C, aplicar remoto con deshacer suspendido, entidades, lápidas) y
    `services::sync_service` (ureq con 5 s/25 s, auth-with-password, auth-refresh, orden de
    `sync_ejecutar`: subir partidos → bajar entidades → subir entidades → bajar partidos;
    hasta 2 servidores; `SyncWorker` en hilo con conexión propia). Menú 25 con las 8 opciones
    de C (`app_sync.rs`, `ui/screens/sincronizacion.rs`); extras renumerados 26/27/28; visible
    en Simple y siempre en Personalizado (C no tiene getter). Auto-sync al entrar al menú en
    segundo plano, solo si hay sesión y la base es un archivo. La contraseña nunca se guarda;
    el token no se muestra ni se exporta (`EXCLUIDAS_JSON` + test que busca el token en el JSON).
  - 6b MiFutbolMobile: `services::mobile_service` exporta
    `Exportaciones/mifutbolmobile_import.json` (formato `mifutbolmobile-import` v1, claves y
    códigos de C: clima, categoría, tipo, tipoFutbol, tipoCancha/superficie como texto) e
    importa `Importaciones/partidos.json` (cancha se crea, camiseta debe existir, sin duplicar
    cancha+fecha+camiseta, una transacción). Marcador `.sync_partidos_auto`: al iniciar importa
    y renombra a `partidos_importados_<sello>.json`. Ajustes: Ctrl+W / Ctrl+Y / Ctrl+A.
  - Tests: 6 repo, 11 servicio de sync con PocketBase falso en `127.0.0.1:0` (login ok, 400,
    500, subida POST, bajada con páginas, conflicto + resolución, 400 no frena + 401 con
    refresh, puerto cerrado, hilo), 3 móvil (formato, importación sin duplicar, marcador +
    renombre), 4 UI (`tests/sincronizacion_ui.rs`) y ajustes de `ui_smoke`/`informe_service`.
    Sin red real en ningún test.
  - Errores de C corregidos: (1) un registro rechazado bloqueaba la cola para siempre (ahora
    queda en error y sigue); (2) con 401 persistente podía reintentar sin fin (ahora un refresh
    por fase); (3) la bajada cambiaba el filtro del cursor entre páginas y salteaba registros;
    (4) códigos de enum numéricos se leían como 0; (5) `sync_aplicar_remoto` hacía ROLLBACK
    sin BEGIN y creaba cancha/camiseta fuera de la transacción; (6) `sync_normalizar_url`
    anteponía http:// a esquemas como ftp:// (ahora se rechazan); (7) campos de texto NULL en
    columnas NOT NULL.
  - Se conserva por compatibilidad: `camiseta_uuid` lleva el nombre de la camiseta (como C);
    el filtro del cursor es `>` estricto (registros del mismo segundo pueden saltearse, igual
    que en C).
  - Diferencias: el borrado remoto de una camiseta en uso la desactiva; `esfuerzoPercibido`
    sale 0 (MiFutbolR no tiene la columna); `favorito` sale de `partido_meta` (C fijaba false);
    recordatorios desde la tabla (C leía `Importaciones/recordatorios.json`); el orden de
    claves del JSON es alfabético (serde_json sin `preserve_order`).
  - No hecho: variables de entorno `MIFUTBOL_SYNC_URL`/`MIFUTBOL_SYNC_EMAIL` de C; importar
    partidos desde una ruta elegida (solo `Importaciones/partidos.json`); el inicio de sesión
    corre en el hilo de la UI (acción explícita, máx. 5 s de conexión por servidor, como C);
    timeout de lectura no probado (tardaría 25 s), sí el puerto cerrado.
- B7 (RDD unmanaged; `cargo test --all-targets`, clippy `-D warnings` y `fmt --check` OK en
  cada commit):
  - 7a `1242ee7` XLSX real (paquete OOXML mínimo con `zip_stored`, `src/util/xlsx.rs`;
    hojas Partidos, Equipos, Canchas, Camisetas, Lesiones, Torneos) y pantalla
    `Screen::ExportarImportar` (Ajustes → Ctrl+D) con las 13 opciones; las teclas Ctrl viejas
    siguen. 326 OK. Validado además con Python `zipfile` + `ElementTree` (openpyxl no instalado).
  - 7b `ed27f87` `importacion_service`: camisetas, partidos y lesiones desde
    `Importaciones/<entidad>.{json,txt,csv,html}` en una transacción con el deshacer
    suspendido; backup previo `backup_*_pre_import.db`; filas mal formadas informadas
    ("línea N"/"fila N"/"elemento N"). 332 OK (+5 servicio, +1 UI).
  - 7c `6b6c979` Recordatorios: `f` recorre temáticas, `x` exporta, `i` une, `I` reemplaza
    (confirmación + backup previo) con `recordatorios.json` de C. 334 OK.
  - 7d `711d2c3` Colecciones: `x`/`i` backup JSON `colecciones_inventario_backup` v1, une sin
    duplicar. 336 OK.
  - Errores de C corregidos: XLSX era SpreadsheetML 2003 dentro de un ZIP (inválido) y sus
    consultas usaban columnas/tablas inexistentes (hojas vacías); las lesiones importadas
    nunca entraban (faltaba `camiseta_id` NOT NULL); JSON/TXT/CSV/HTML exportados por C no se
    releían bien (resultado/clima en texto → 0, "Atajaste" pegado al comentario, franja
    Dia/Tarde/Noche=1/2/3 contra Madrugada…Noche=1..6, `<span>` del resultado → 0, entidades
    HTML sin decodificar); camisetas HTML emparejaba celdas ignorando filas; camisetas TXT/CSV
    no leían su propio export y truncaban nombres; el partido creaba la cancha aunque faltara
    la camiseta; "Reemplazar" recordatorios también fusionaba (duplicados); el restore de
    colecciones enlazaba relaciones a filas ajenas con el id viejo.
  - No hecho / diferencias: estadísticas no se importan (C las importaba a una tabla que no
    lee nadie); `atajaste` no se importa (C tampoco); el submenú por entidad de C quedó en una
    opción por formato (importa todos los archivos presentes); el XLSX de C por tabla quedó en
    un libro con varias hojas; `fecha_fin` de recordatorios no se guarda; Historial de
    Equipamiento queda para B9 (en C es un módulo propio, `equipamiento_historial.c`).
- B8 (`fdcb3ab`; RDD unmanaged; 349 OK en `cargo test --all-targets --no-fail-fast` (336 + 13),
  clippy `-D warnings` y `cargo fmt --check` limpios; 20 archivos, +1131/-25; supera la heurística de ~400 líneas por los tests y el servicio nuevo). Un solo commit:
  las cinco partes comparten `app_ajustes.rs`, el bloque Ctrl de Ajustes y `tareas_de_inicio`.
  - 8a Backup de inicio (`backup_al_iniciar` de `db.c`): `backup_service::backup_inicio` copia a
    `Exportaciones/backup_<AAAAMMDD_HHMMSS>_inicio.db` (`_inicio_u<id>` por cuenta, para que la
    retención de un perfil no borre la de otro) solo si la base tiene datos; el del mismo día se
    reemplaza y quedan los últimos 7 días (C: copia fija + `Historial/<base>.1..7.db`). Lo ven
    Ctrl+G y la oferta de restaurar de B2 (prefijo `backup_`, orden por nombre). No es
    configurable en C (el "Auto-Backup" por intervalo de `backup.c` solo corre desde
    recordatorios: no portado).
  - Tareas de arranque: `App::tareas_de_inicio` (backup, actualizaciones, aviso retro) al
    entrar al menú o iniciar sesión, una vez por base, solo si `tareas_inicio` (lo activa
    `main.rs`; `App::new` no, así los tests no escriben backups ni usan la red). Se conserva al
    cambiar de cuenta (`reemplazar_app`).
  - 8b Ajustes → Ctrl+N: `Screen::MenusPersonalizados` con `[X]`/`[ ]` para las 27 opciones
    (Ajustes no se lista: siempre visible); Enter/Espacio → `settings_repo::set_menu_visible`
    (`INSERT OR REPLACE` en `custom_menus`, como `set_custom_menu_enabled`).
  - 8c `actualizacion_service`: `releases/latest` de la API de GitHub (repo
    `thomashamer3/MiFutbolR`; C usa `.../MiFutbolC`), `tag_name` o `name`, `comparar_versiones`
    de C (prefijo v, mayor.menor.parche) contra `CARGO_PKG_VERSION`; una vez por día con
    `update_check.stamp` en `data_dir` (se escribe antes de consultar, como C); hilo + mpsc
    (`ActualizacionWorker`, `tick_actualizacion` en el bucle). La automática solo avisa si hay
    versión nueva (modal o barra); Ctrl+U siempre informa. Campo "Buscar actualizaciones al
    iniciar" (migración `017_ajustes.sql`). C descarga y ejecuta el instalador `.exe`: aquí
    solo se muestra el enlace `html_url`.
  - 8d Ctrl+F "Ver configuración actual" (modal: tema, idioma, modo, tamaño, visor, botín,
    actualizaciones, menús ocultos, usuario, BD, versión; sin contraseñas ni token — test que lo
    busca). Ctrl+Q restablece `AppSettings::default()` con confirmación y refresca el formulario.
    Diferencia: C además borraba la tabla `usuario` (nombre); aquí no, las cuentas viven en
    `cuentas.db`.
  - 8e Idioma: C tiene tablas reales (`lang.c` + `langs/*.json`, `tr(clave)`). Port:
    `crate::i18n::t(idioma, clave, español)` con la tabla inglesa de las claves de C para el
    menú principal (`Module::etiqueta`, `menu_<key>`), "Salir", el título y las etiquetas de
    Ajustes; el español queda como defecto en el código. Modo Retro en C no es una paleta: es
    "Hoy en tu historia" de Carrera (ya portado); se portó el aviso de inicio
    (`carrera_notificar_modo_retro_inicio`) como modal S/N que abre Carrera → Modo retro.
  - Errores corregidos: `carrera_repo::retro` comparaba con `date('now')` en UTC (de noche,
    hora local, no encontraba los recuerdos de hoy); `Database::restore_from` no migraba un
    backup de una versión anterior (ahora llama a `migrate`). Errores de C: `en.json` no tiene
    `menu_botines` (mostraba la clave) ni traduce Tiendas; `settings_save` usa
    `INSERT OR REPLACE` con 4 columnas y borra `image_viewer`.
  - Tests nuevos (13): `backup_inicio` (base vacía, 7 días, mismo día, no toca otros perfiles
    ni manuales), `comparar_versiones`, marca diaria, consulta a servidor falso
    (`127.0.0.1:0`; tag/name/404), hilo, `t()` + cobertura de claves, y `tests/arranque_ui.rs`
    (backup de inicio + lista de backups, aviso retro, Ctrl+U y arranque contra servidor falso,
    desactivar, editor de menús en Personalizado, ver/restablecer sin secretos, menú en inglés).
  - No hecho: Auto-Backup por intervalo de `backup.c`; descargar/instalar releases; traducir
    pantallas de módulos, barra de pestañas y textos de ayuda (C tampoco los traduce); hitos
    del Modo Retro de C (primer partido/gol/rival/camiseta, mejor racha) — el aviso cuenta solo
    partidos, como la pantalla retro de Rust; la oferta de restaurar de B2 sigue mirando todos
    los `backup_*.db` de `Exportaciones/` (compartida entre perfiles).
