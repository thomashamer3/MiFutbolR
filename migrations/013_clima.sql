-- 013_clima.sql — cola de clima histórico (Open-Meteo) por partido.
-- Las columnas de clima de `partido` y las coordenadas de `cancha` ya existen (001).

CREATE TABLE IF NOT EXISTS partido_clima_pendiente (
    partido_id INTEGER PRIMARY KEY REFERENCES partido(id) ON DELETE CASCADE,
    intentos   INTEGER NOT NULL DEFAULT 0,
    creado_en  TEXT    NOT NULL DEFAULT (datetime('now','localtime'))
);
