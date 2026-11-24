-- B9 Estadísticas → Reportes automáticos (`reportes.c`): reportes generados.
-- `reporte_config` de C no se porta: guardaba periodicidades que nunca se ejecutaban.
CREATE TABLE IF NOT EXISTS reporte_generado (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre           TEXT    NOT NULL,
    archivo          TEXT    NOT NULL,
    fecha_generacion TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);
