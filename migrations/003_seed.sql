-- 003_seed.sql
INSERT OR IGNORE INTO settings (id, theme, language, mode, text_size, image_viewer, botin_predeterminado)
VALUES (1, 0, 0, 0, 1, '', 0);

INSERT OR IGNORE INTO carrera_identidad (id, nombre, edad, pie_habil, posiciones, estilo, dorsal_favorito)
VALUES (1, '', 0, 'Derecho', '', '', 0);

INSERT OR IGNORE INTO custom_menus (menu_name, enabled) VALUES
    ('camisetas', 1), ('canchas', 1), ('botines', 1), ('equipos', 1),
    ('partidos', 1), ('lesiones', 1), ('estadisticas', 1),
    ('financiamiento', 1), ('carrera', 1), ('resumen', 1), ('ajustes', 1);
