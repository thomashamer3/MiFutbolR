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

## Fuera de alcance (no pedido en la propuesta)
Sincronización PocketBase, XLSX (ODS ya cubre hojas de cálculo), MiFutbolMobile, importar
JSON/TXT/CSV/HTML, migración chrono → jiff.

## Restricciones
- Sin dependencias nuevas.
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
  - Engram `odd/mejoras-paridad-c/tasks`: pendiente (lo sincroniza el orquestador).
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
