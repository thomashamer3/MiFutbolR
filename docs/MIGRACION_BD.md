# Migración de la base legacy (mifutbol.db)

Fecha: 2026-11-06 · Versión de la app: 0.1

## Origen

- Archivo: `mifutbol.db` (raíz del repo), 757.760 bytes.
- Es una base de **MiFutbolC**: `PRAGMA user_version = 10`, 72 tablas, 3 triggers.
- Integridad: `ok`; `foreign_key_check`: 0 errores.
- Contiene 14 camisetas, 13 canchas, 36 partidos, 45 movimientos, 14 jugadores, 2 equipos, 1 lesión, 1 identidad de carrera, 2 torneos y 1 temporada.

## Destino

- Esquema de **MiFutbolRS** (`user_version = 3`, 25 tablas) generado por las migraciones 001–003.
- Importación con `legacy_import_service` (Ajustes → `Ctrl+L` o CLI).

## Datos importados (133 filas)

| Tabla | Filas |
|-------|------:|
| camiseta | 14 |
| botin | 1 |
| cancha | 13 |
| partido | 36 |
| partido_meta | 1 |
| partido_tag | 1 |
| equipo | 2 |
| jugador | 14 |
| lesion | 1 |
| financiamiento | 45 |
| carrera_identidad | 1 |
| carrera_resumen_narrativo | 1 |
| torneo | 2 |
| temporada | 1 |

Además se conservó una camiseta creada en la app antes de migrar (`Hamer 10`), total **15 camisetas**. La base final quedó en `%LOCALAPPDATA%\mifutbol\mifutbol-rs\data\mifutbol.db`.

## Conversiones aplicadas

- `partido.fecha_hora` `YYYY-MM-DD HH:MM` → ISO con segundos; `mes_anio` recalculado.
- `lesion.fecha` `DD/MM/YYYY HH:MM` → ISO con segundos; `estado` normalizado a mayúsculas.
- `financiamiento.monto` REAL → `monto_centavos` (×100).
- `partido.precio` → `precio_centavos` (×100).
- `equipo.partido_id = -1` → `NULL`.
- `cancha.cantidad_canchas <= 0` → `1`.
- `cancha_tipo`/`cancha_superficie` reconstruidas desde los códigos legacy (bitmask de §9.2).
- `settings`: se conservan tema, idioma, modo, tamaño, visor y botín predeterminado.
- Favoritos (`partido_meta`) y etiquetas (`partido_tag`) ahora forman parte de la importación.

## Tablas legacy no utilizadas

De las 72 tablas legacy, **49 no existen en el esquema de MiFutbolRS** y no se importan: módulos de bienestar (comida, salud, hábitos, sueño…), IA, sincronización, tienda, multimedia, reportes, historial de comparaciones, etc.

**31 tablas estaban vacías**, entre ellas `amigo` (0 filas). Como no aportan datos ni funcionalidad al proyecto, **no se migran**. Ejemplos: `amigo`, `backup_config`, `bienestar_comida`, `bienestar_control_medico`, `bienestar_dia_nutricional`, `bienestar_sesion_mental`, `coleccion_inventario`, `equipamiento_historial`, `media`, `notificacion`, `reporte_generado`, `sync_conflicto`, `tienda`, `torneo_fases`.

### Tablas vacías en la base nueva

Quedan 7 tablas sin filas porque son opcionales o de módulos futuros: `carrera_partido_hito`, `entrenamiento_plan`, `equipo_historial`, `partido_torneo`, `presupuesto_mensual`, `progresion_jugador` y `usuario`. Se mantienen en el esquema para no romper migraciones ni funcionalidades futuras; la UI no las requiere.

## Respaldos

- `datos/mifutbol_legacy_original.db`: copia intacta del origen.
- `datos/mifutbol_app_pre_import_*.db`: base que tenía la app antes de reemplazarla.
- `datos/mifutbol_convertida.db`: copia de la base final migrada.

## Repetir la migración

```powershell
# Base destino vacía (o usar MIFUTBOL_DB para otra ruta)
.\target\release\mifutbol-rs.exe --importar-legacy mifutbol.db
```

También desde la TUI: **Ajustes → Ctrl+L**, indicar la ruta del `.db` legacy.
