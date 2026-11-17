# Login multiusuario con DB propia por cuenta

## Objetivo
Pantalla de inicio de sesión / registro (con o sin contraseña) donde cada cuenta
usa su propia base de datos, con una portada atractiva titulada MiFutbolR.

## Problema / por qué
Hoy las cuentas (`cuenta`, `cuenta_sesion`) viven dentro de la misma
`mifutbol.db`: todas comparten partidos, equipos, etc. No hay aislamiento.

## Alcance autorizado
- Registro de cuentas separado: `<data_dir>/cuentas.db` (solo migración 011).
- DB por cuenta: `<data_dir>/usuarios/<id>/mifutbol.db`.
- Login/registro reconstruyen `App` sobre la DB del usuario.
- "Continuar sin cuenta" = invitado sobre la DB por defecto (conserva datos previos).
- El binario siempre arranca en la portada de inicio de sesión.
- Rediseño de la portada (BigText MiFutbolR, lista de cuentas, atajos).
- Fuera de alcance: rendimiento (siguiente fase), migrar cuentas antiguas de la DB compartida.

## Restricciones
- Sin dependencias nuevas. Eliminar una cuenta NO borra su archivo de DB (evita pérdida de datos).
- `MIFUTBOL_DB` sigue definiendo la DB de invitado.

## Configuración
- TDD: desactivado (sin configuración de proyecto/sesión). Runner: `cargo test`.
- RDD: no disponible (`gentle-ai` no instalado) → disabled/unmanaged.
- Engram: herramientas `mem_*` no disponibles → espejo `odd/login-multiusuario/tasks` PENDIENTE.
- Entrega: `ask-on-risk`. Previsión ~350 líneas.

## Tareas
- [x] T1 Registro `cuentas.db` + rutas por usuario + cambio de DB en App + portada rediseñada + tests.
  - Ruta: delegated direct (writer trigger: 5+ archivos no triviales: config.rs, db/mod.rs, app.rs, app_usuario.rs, bienvenida.rs, main.rs, tests).
- [x] T2 Rendimiento: bucle de eventos sin redibujado en reposo, vaciado de eventos por frame
  (tope 64) e ignorar movimiento del mouse. Ruta: inline (1 archivo, src/app.rs::run).
- [x] T3 Rendimiento: `estadisticas_repo::rendimiento` con una sola pasada en lugar de 8 GROUP BY
  + test de equivalencia contra SQL. Ruta: inline (1 archivo).

## Criterios de aceptación
- Crear cuenta (con/sin contraseña) abre el menú usando `usuarios/<id>/mifutbol.db`.
- Datos creados por una cuenta no aparecen en otra.
- Login con contraseña incorrecta se rechaza.
- Cerrar sesión vuelve a la portada (DB de invitado).
- `cargo test`, `cargo clippy --all-targets` sin errores.

## Progreso / evidencia
- T1 (delegated direct, writer trigger): 9 archivos, ~+470/−200 (supera la heurística de 400 por
  el rediseño de la portada + reescritura de tests; no se dividió artificialmente).
- `cargo test`: todas las suites OK (206 tests, 0 fallos; incluye 4 nuevos en tests/usuario_ui.rs:
  DB propia, contraseña incorrecta, aislamiento de datos Ana/Beto, render 40x12/80x24/120x40).
- `cargo clippy --all-targets -- -D warnings`: limpio. `cargo fmt` aplicado.
- Vista previa renderizada con TestBackend (100x32, 80x24, formulario) revisada; se acortaron
  los atajos de la tarjeta para que no se corten.
- RDD: no disponible (gentle-ai ausente) → disabled/unmanaged. Engram: espejo pendiente.
- No probado en terminal real (solo TestBackend); ancho del emoji 🔒 puede variar por terminal.
- Commit: `8414732` feat(usuario): inicio de sesión multiusuario con DB propia por cuenta.

- Perfilado (release, 10k partidos): frame 0,2–0,3 ms; abrir BD existente 5,7 ms (migraciones ya se
  saltan); App::new 10 ms; `estadisticas_completas` 79 ms → cuello: GROUP BY con árbol temporal
  (~3,5 ms c/u, 7,5 ms con strftime). ANALYZE y transacción de lectura: sin mejora (<5 %), descartados.
- T3: `rendimiento` 29 → 11,5 ms; bench `estadisticas_completas` 79,4 → 62,0 ms (−22,8 %, p<0,05);
  resto de benches sin cambio significativo.
- T2: no medible con TestBackend; en reposo pasa de ~8 frames/s a 0. No probado en terminal real.
- `cargo test`: 207 OK, 0 fallos. Clippy `-D warnings` limpio.

## Siguiente paso
Opcional: llevar `canchas` (13 ms) y `goles` (10 ms) al mismo patrón de una pasada si se usan bases
muy grandes; con datos realistas (cientos de partidos) ya cargan en pocos ms.
