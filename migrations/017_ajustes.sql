-- B8 Ajustes: comprobar actualizaciones al iniciar (1 = sí). C siempre consulta
-- una vez por día; MiFutbolR permite desactivarlo desde Ajustes.
ALTER TABLE settings ADD COLUMN buscar_actualizaciones INTEGER NOT NULL DEFAULT 1;
