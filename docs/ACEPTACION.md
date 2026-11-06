# Criterios de aceptación — evidencia (2026-11-06)

Verificación de la checklist global de `ESPECIFICACION_PROYECTO_RUST_TUI.md` §22.4 y de
`Performance.md` §8 sobre el estado actual del repositorio.

## §22.4 Criterios globales

| Criterio | Estado | Evidencia |
|----------|--------|-----------|
| `cargo build --release` sin warnings | ✅ | `Finished release profile` · binario `mifutbol-rs.exe` ≈ 3,71 MB |
| `cargo test` verde | ✅ | **95 unitarias + integración = 144**, 0 fallos (nextest: 144 passed) |
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

- Binario: `target/release/mifutbol-rs.exe` (3,73 MB) con `--version` → 0.1.0.
- Instalador Inno Setup: `dist/MiFutbolR-Setup-0.1.exe` (3,77 MB).
- Portable: `dist/MiFutbolR-0.1-win64.zip` (2,27 MB).
- Hashes: `dist/SHA256SUMS.txt`.
- Características del instalador: Program Files, menú Inicio, escritorio opcional,
  icono `assets/icono.ico`, inicio al finalizar, desinstalador.
- Migración de datos: `docs/MIGRACION_BD.md` (legacy 72 tablas → esquema Rust,
  133 filas importadas + 1 camiseta previa; tablas vacías como `amigo` no migradas).
