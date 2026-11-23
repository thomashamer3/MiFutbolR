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
- [ ] B3 Rendimiento. Ruta: delegated (writer trigger: repos + migración + app).
- [ ] B4 Funciones de C. Ruta: delegated (writer trigger: servicios + pantallas nuevas).
- [ ] B5 Limpieza. Ruta: delegated (writer trigger: app_*.rs).

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
- B2 (commit en este bloque; RDD unmanaged):
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
