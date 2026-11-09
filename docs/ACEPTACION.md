# Criterios de aceptación — evidencia (2026-11-06)

Verificación de la checklist global de `ESPECIFICACION_PROYECTO_RUST_TUI.md` §22.4 y de
`Performance.md` §8 sobre el estado actual del repositorio.

## §22.4 Criterios globales

| Criterio | Estado | Evidencia |
|----------|--------|-----------|
| `cargo build --release` sin warnings | ✅ | `Finished release profile` · binario `mifutbol-rs.exe` ≈ 3,71 MB |
| `cargo test` verde | ✅ | **99 unitarias + integración = 157**, 0 fallos (nextest: 144 passed) |
| La app arranca sin BD y crea el esquema | ✅ | `Database::open` aplica migraciones 001–003; tests de UI y de BD abren bases nuevas |
| Importa una BD legacy sin pérdida de datos | ✅ | `legacy_import_service` + tests con base legacy sintética (fechas y montos convertidos) |
| Los 11 módulos son navegables y CRUD funcional | ✅ | 11 módulos con pantallas; pruebas de flujo por teclado en Camisetas, Canchas, Botines, Equipos, Partidos, Lesiones, Estadísticas, Financiamiento, Carrera, Resumen y Ajustes |
| Cálculos de estadísticas coinciden con el legacy | ⚠️ | Implementados según las fórmulas del §14; **no comparados contra un fixture real** de MiFutbolC (no disponible en el repo) |
| No hay `panic!` por entradas inválidas | ✅ | Escaneo de `src/`: sin `unwrap`/`expect`/`panic!` fuera de tests; validaciones de dominio en todas las altas |
| UI responde < 100 ms para listados de 10k partidos | ✅ | Benchmarks: listado paginado ≈ 215 µs, búsqueda ≈ 640 µs, conteo ≈ 0,99 ms, estadísticas completas ≈ 71 ms |

## Performance.md §8

### Arquitectura
- [x] TUI / servicios / repositorios separados.
- [x] Conexión SQLite centralizada (`Database`).
- [x] Migraciones SQL embebidas con `user_version`.
- [x] Modelos de dominio separados de vistas (DTOs por módulo).

### Rendimiento
- [x] Perfil release: `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, `strip = true`.
- [x] PRAGMAs: WAL, `foreign_keys`, `synchronous=NORMAL`, `temp_store=MEMORY`.
- [x] Sin clones innecesarios en las rutas calientes (paso de `&str`/`&[T]` cuando es posible).
- [x] Consultas preparadas (`prepare` + `query_map`).
- [x] Transacciones en operaciones multi-tabla.
- [x] Índices según las consultas reales (`002_indexes.sql`).
- [x] Sin SQL en el renderizado; la UI solo lee estado.

### Calidad
- [x] Pruebas unitarias.
- [x] Pruebas de repositorios y migraciones.
- [x] Clippy `-D warnings` y `cargo fmt --check` limpios.
- [x] Dependencias revisadas: sin Tokio, Actix, Axum ni ORM.
- [x] Benchmarks Criterion (`benches/queries.rs`).
- [ ] `cargo audit` (herramienta externa; no instalada en este entorno).

### Multiplataforma
- [x] Directorios de datos con `directories::ProjectDirs`.
- [x] CI en GitHub Actions: Ubuntu, Windows y macOS (`fmt`, `clippy`, `test`, `build --release`).
- [x] Compilación release verificada localmente en Windows.

## Limitaciones conocidas

- La paridad exacta con el legacy de estadísticas no está verificado con una BD real de
  MiFutbolC; las fórmulas siguen el §14.
- `cargo audit` y la ejecución real del workflow de CI quedan fuera de este entorno local.

## Release Windows 0.1

- Binario: `target/release/mifutbol-rs.exe` (4,81 MB) con `--version` → 0.1.0.
- Instalador Inno Setup: `dist/MiFutbolR-Setup-0.1.exe` (4,97 MB).
- Portable: `dist/MiFutbolR-0.1-win64.zip` (3,35 MB).
- Hashes: `dist/SHA256SUMS.txt`.
- Características del instalador: Program Files, menú Inicio, escritorio opcional,
  icono `assets/icono.ico`, inicio al finalizar, desinstalador.
- Migración de datos: `docs/MIGRACION_BD.md` (legacy 72 tablas → esquema Rust,
  133 filas importadas + 1 camiseta previa; tablas vacías como `amigo` no migradas).

## Mejoras de interfaz (ratatui-image / awesome-ratatui / tachyonfx)

- Paleta RGB de 8 temas, paneles redondeados y estilos reutilizables.
- Tabs de módulos y sidebar clicable en pantallas ≥140 columnas.
- Tablas con resultados/estados coloreados y vista previa lateral.
- Clic en menú, sidebar y filas de todas las tablas; rueda para navegar.
- Miniaturas con `ratatui-image` en listados y fichas; visor a pantalla completa con `v`.
- Animaciones con `tachyonfx`: fundido de pantalla (210 ms), modales (130 ms)
  y pulso de fila (150 ms); loop a 16 ms mientras hay efectos.
- Logo embebido en el menú y textos grandes con `tui-big-text`.
- Verificación: 100 unitarias + 57 de integración = **157 pruebas**, clippy y fmt limpios.


## Ola 0 — implementación (en curso)

- Migración `004_ola0.sql` aplicada sobre bases nuevas y existentes (v3 → v4).
- Panel "Hoy", calendario mensual, búsqueda global, paleta de comandos, logros (65),
  notificaciones con badge, tutorial de 4 pasos y exportación de listados a 4 formatos.
- Records & rankings (16 reportes) y undo con triggers + Ctrl+Z.
- Pruebas: **176** (109 unitarias + 67 de integración), 0 fallos.
- `cargo clippy --all-targets -- -D warnings` y `cargo fmt --check` limpios.
- **Ola 0 completa (IMP-01 a IMP-10).**

## Ola 1 — torneos

- Migración `006_ola1.sql`: `equipo_torneo` + índices.
- CRUD de torneos, asociación automática de equipos, fixture round robin simple/doble, carga de
  resultados, tabla de posiciones y exportación del listado (MVP; faltan copa/grupos/eliminación).
- Pruebas: **198** (123 unitarias + 75 de integración), `clippy` y `fmt` limpios.

## Ola 2 — tiendas y amigos

- Tiendas (D): CRUD con contacto, productos, envío y favoritos.
- Ranking de amigos (A): CRUD y clasificación con el usuario local incluido.
- Multimedia (M): video/foto/artículo/audio con filtro y partido asociado.
- Recordatorios (Y): recurrencia diaria/semanal/mensual, agenda fusionada con partidos y
  aparición en el calendario.
- Progresión (P): planes de entrenamiento y mediciones semanales (ataque, defensa,
  resistencia, velocidad y técnica).

### IMP-13 e IMP-16

- Estadísticas de canchas ampliadas a 10 reportes (tipo, superficie, techada, localidad, franja).
- Pantalla "Últimos" (U) con última victoria/derrota/empate/gol/asistencia y tiempo transcurrido.
- Comparador A/B (C) de camisetas y canchas con 9 métricas y ganador por métrica.
- Meta-análisis (Estadísticas → Meta): consistencia, outliers, contexto, impacto del
  cansancio/ánimo, eficiencia y tendencia.
- Temporadas (E): CRUD, 3 fases por defecto, asociación de torneos, resumen del rango,
  fatiga y comparación con la temporada anterior.
