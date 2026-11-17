-- 014_bienestar_completo.sql — resto de Bienestar de MiFutbolC (planificación,
-- ejercicios, día nutricional, controles médicos, archivos de estudios e
-- imágenes de menú) y deshacer para las tablas de 012 y 014.
-- Mismas tablas y columnas que `db.c` de MiFutbolC para importar sin traducción;
-- las claves foráneas de las tablas hijas borran en cascada (MiFutbolC no las
-- declara con acción y dejaba filas huérfanas).

CREATE TABLE IF NOT EXISTS bienestar_objetivo (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre       TEXT NOT NULL,
    fecha_inicio TEXT NOT NULL,
    fecha_fin    TEXT NOT NULL,
    estado       TEXT NOT NULL DEFAULT 'Activo',
    notas        TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS bienestar_plan_entrenamiento (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    objetivo_id        INTEGER NOT NULL,
    frecuencia_semanal INTEGER NOT NULL,
    rutina_semanal     TEXT    NOT NULL,
    notas              TEXT    DEFAULT '',
    FOREIGN KEY (objetivo_id) REFERENCES bienestar_objetivo(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS bienestar_ejercicio (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre         TEXT NOT NULL,
    grupo_muscular TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS bienestar_entrenamiento_ejercicio (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    entrenamiento_id INTEGER NOT NULL,
    ejercicio_id     INTEGER NOT NULL,
    series           INTEGER DEFAULT 0,
    repeticiones     INTEGER DEFAULT 0,
    tiempo_min       INTEGER DEFAULT 0,
    FOREIGN KEY (entrenamiento_id) REFERENCES bienestar_entrenamiento(id) ON DELETE CASCADE,
    FOREIGN KEY (ejercicio_id) REFERENCES bienestar_ejercicio(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS bienestar_dia_nutricional (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    fecha         TEXT NOT NULL UNIQUE,
    hidratacion   TEXT NOT NULL,
    alcohol       INTEGER DEFAULT 0,
    peso_corporal REAL DEFAULT NULL,
    notas         TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS bienestar_control_medico (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    fecha       TEXT NOT NULL,
    tipo        TEXT NOT NULL,
    profesional TEXT DEFAULT '',
    resultado   TEXT DEFAULT '',
    notas       TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS bienestar_estudio_archivo (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    control_id      INTEGER NOT NULL,
    nombre_original TEXT NOT NULL,
    ruta_archivo    TEXT NOT NULL,
    tipo_archivo    TEXT DEFAULT '',
    fecha_subida    TEXT NOT NULL,
    FOREIGN KEY (control_id) REFERENCES bienestar_control_medico(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS bienestar_menu_imagen (
    menu_key    TEXT PRIMARY KEY,
    imagen_ruta TEXT DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_bienestar_plan_objetivo ON bienestar_plan_entrenamiento(objetivo_id);
CREATE INDEX IF NOT EXISTS idx_bienestar_ent_ej_entrenamiento ON bienestar_entrenamiento_ejercicio(entrenamiento_id);
CREATE INDEX IF NOT EXISTS idx_bienestar_ent_ej_ejercicio ON bienestar_entrenamiento_ejercicio(ejercicio_id);
CREATE INDEX IF NOT EXISTS idx_bienestar_control_fecha ON bienestar_control_medico(fecha);
CREATE INDEX IF NOT EXISTS idx_bienestar_estudio_control ON bienestar_estudio_archivo(control_id);

-- Deshacer (mismo patrón que 005). Quedan fuera: ia_config (configuración),
-- coleccion_inventario y bienestar_menu_imagen (sin columna `id`, que es la que
-- usa undo_service) y partido_clima_pendiente (cola técnica de 013).


-- bienestar_habito (11 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_habito_ai AFTER INSERT ON bienestar_habito WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_habito', NEW.id, 'crear', 'bienestar_habito #' || NEW.id, NULL, json_object('id', NEW.id, 'fecha', NEW.fecha, 'dormi_bien', NEW.dormi_bien, 'hidratacion', NEW.hidratacion, 'alcohol', NEW.alcohol, 'estado_animico', NEW.estado_animico, 'nervios', NEW.nervios, 'confianza', NEW.confianza, 'motivacion', NEW.motivacion, 'notas', NEW.notas, 'tipo_diario', NEW.tipo_diario));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_habito_au AFTER UPDATE ON bienestar_habito WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_habito', NEW.id, 'editar', 'bienestar_habito #' || NEW.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'dormi_bien', OLD.dormi_bien, 'hidratacion', OLD.hidratacion, 'alcohol', OLD.alcohol, 'estado_animico', OLD.estado_animico, 'nervios', OLD.nervios, 'confianza', OLD.confianza, 'motivacion', OLD.motivacion, 'notas', OLD.notas, 'tipo_diario', OLD.tipo_diario), json_object('id', NEW.id, 'fecha', NEW.fecha, 'dormi_bien', NEW.dormi_bien, 'hidratacion', NEW.hidratacion, 'alcohol', NEW.alcohol, 'estado_animico', NEW.estado_animico, 'nervios', NEW.nervios, 'confianza', NEW.confianza, 'motivacion', NEW.motivacion, 'notas', NEW.notas, 'tipo_diario', NEW.tipo_diario));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_habito_ad AFTER DELETE ON bienestar_habito WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_habito', OLD.id, 'borrar', 'bienestar_habito #' || OLD.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'dormi_bien', OLD.dormi_bien, 'hidratacion', OLD.hidratacion, 'alcohol', OLD.alcohol, 'estado_animico', OLD.estado_animico, 'nervios', OLD.nervios, 'confianza', OLD.confianza, 'motivacion', OLD.motivacion, 'notas', OLD.notas, 'tipo_diario', OLD.tipo_diario), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_entrenamiento (7 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_entrenamiento_ai AFTER INSERT ON bienestar_entrenamiento WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_entrenamiento', NEW.id, 'crear', 'bienestar_entrenamiento #' || NEW.id, NULL, json_object('id', NEW.id, 'fecha', NEW.fecha, 'tipo', NEW.tipo, 'duracion_min', NEW.duracion_min, 'intensidad', NEW.intensidad, 'omitido', NEW.omitido, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_entrenamiento_au AFTER UPDATE ON bienestar_entrenamiento WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_entrenamiento', NEW.id, 'editar', 'bienestar_entrenamiento #' || NEW.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'tipo', OLD.tipo, 'duracion_min', OLD.duracion_min, 'intensidad', OLD.intensidad, 'omitido', OLD.omitido, 'notas', OLD.notas), json_object('id', NEW.id, 'fecha', NEW.fecha, 'tipo', NEW.tipo, 'duracion_min', NEW.duracion_min, 'intensidad', NEW.intensidad, 'omitido', NEW.omitido, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_entrenamiento_ad AFTER DELETE ON bienestar_entrenamiento WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_entrenamiento', OLD.id, 'borrar', 'bienestar_entrenamiento #' || OLD.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'tipo', OLD.tipo, 'duracion_min', OLD.duracion_min, 'intensidad', OLD.intensidad, 'omitido', OLD.omitido, 'notas', OLD.notas), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_comida (5 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_comida_ai AFTER INSERT ON bienestar_comida WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_comida', NEW.id, 'crear', 'bienestar_comida #' || NEW.id, NULL, json_object('id', NEW.id, 'fecha', NEW.fecha, 'tipo', NEW.tipo, 'calidad', NEW.calidad, 'descripcion', NEW.descripcion));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_comida_au AFTER UPDATE ON bienestar_comida WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_comida', NEW.id, 'editar', 'bienestar_comida #' || NEW.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'tipo', OLD.tipo, 'calidad', OLD.calidad, 'descripcion', OLD.descripcion), json_object('id', NEW.id, 'fecha', NEW.fecha, 'tipo', NEW.tipo, 'calidad', NEW.calidad, 'descripcion', NEW.descripcion));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_comida_ad AFTER DELETE ON bienestar_comida WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_comida', OLD.id, 'borrar', 'bienestar_comida #' || OLD.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'tipo', OLD.tipo, 'calidad', OLD.calidad, 'descripcion', OLD.descripcion), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_sesion_mental (13 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_sesion_mental_ai AFTER INSERT ON bienestar_sesion_mental WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_sesion_mental', NEW.id, 'crear', 'bienestar_sesion_mental #' || NEW.id, NULL, json_object('id', NEW.id, 'fecha', NEW.fecha, 'tipo', NEW.tipo, 'momento', NEW.momento, 'partido_id', NEW.partido_id, 'confianza', NEW.confianza, 'estres', NEW.estres, 'motivacion', NEW.motivacion, 'miedos', NEW.miedos, 'presion', NEW.presion, 'concentracion', NEW.concentracion, 'pensamientos_clave', NEW.pensamientos_clave, 'texto_libre', NEW.texto_libre));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_sesion_mental_au AFTER UPDATE ON bienestar_sesion_mental WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_sesion_mental', NEW.id, 'editar', 'bienestar_sesion_mental #' || NEW.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'tipo', OLD.tipo, 'momento', OLD.momento, 'partido_id', OLD.partido_id, 'confianza', OLD.confianza, 'estres', OLD.estres, 'motivacion', OLD.motivacion, 'miedos', OLD.miedos, 'presion', OLD.presion, 'concentracion', OLD.concentracion, 'pensamientos_clave', OLD.pensamientos_clave, 'texto_libre', OLD.texto_libre), json_object('id', NEW.id, 'fecha', NEW.fecha, 'tipo', NEW.tipo, 'momento', NEW.momento, 'partido_id', NEW.partido_id, 'confianza', NEW.confianza, 'estres', NEW.estres, 'motivacion', NEW.motivacion, 'miedos', NEW.miedos, 'presion', NEW.presion, 'concentracion', NEW.concentracion, 'pensamientos_clave', NEW.pensamientos_clave, 'texto_libre', NEW.texto_libre));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_sesion_mental_ad AFTER DELETE ON bienestar_sesion_mental WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_sesion_mental', OLD.id, 'borrar', 'bienestar_sesion_mental #' || OLD.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'tipo', OLD.tipo, 'momento', OLD.momento, 'partido_id', OLD.partido_id, 'confianza', OLD.confianza, 'estres', OLD.estres, 'motivacion', OLD.motivacion, 'miedos', OLD.miedos, 'presion', OLD.presion, 'concentracion', OLD.concentracion, 'pensamientos_clave', OLD.pensamientos_clave, 'texto_libre', OLD.texto_libre), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_salud (7 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_salud_ai AFTER INSERT ON bienestar_salud WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_salud', NEW.id, 'crear', 'bienestar_salud #' || NEW.id, NULL, json_object('id', NEW.id, 'altura_cm', NEW.altura_cm, 'peso_kg', NEW.peso_kg, 'tipo_sangre', NEW.tipo_sangre, 'ultima_revision', NEW.ultima_revision, 'medidas', NEW.medidas, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_salud_au AFTER UPDATE ON bienestar_salud WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_salud', NEW.id, 'editar', 'bienestar_salud #' || NEW.id, json_object('id', OLD.id, 'altura_cm', OLD.altura_cm, 'peso_kg', OLD.peso_kg, 'tipo_sangre', OLD.tipo_sangre, 'ultima_revision', OLD.ultima_revision, 'medidas', OLD.medidas, 'notas', OLD.notas), json_object('id', NEW.id, 'altura_cm', NEW.altura_cm, 'peso_kg', NEW.peso_kg, 'tipo_sangre', NEW.tipo_sangre, 'ultima_revision', NEW.ultima_revision, 'medidas', NEW.medidas, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_salud_ad AFTER DELETE ON bienestar_salud WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_salud', OLD.id, 'borrar', 'bienestar_salud #' || OLD.id, json_object('id', OLD.id, 'altura_cm', OLD.altura_cm, 'peso_kg', OLD.peso_kg, 'tipo_sangre', OLD.tipo_sangre, 'ultima_revision', OLD.ultima_revision, 'medidas', OLD.medidas, 'notas', OLD.notas), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_recomendacion (6 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_recomendacion_ai AFTER INSERT ON bienestar_recomendacion WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_recomendacion', NEW.id, 'crear', 'bienestar_recomendacion #' || NEW.id, NULL, json_object('id', NEW.id, 'fecha', NEW.fecha, 'score_preparacion', NEW.score_preparacion, 'riesgo_lesion', NEW.riesgo_lesion, 'resumen', NEW.resumen, 'rutina', NEW.rutina));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_recomendacion_au AFTER UPDATE ON bienestar_recomendacion WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_recomendacion', NEW.id, 'editar', 'bienestar_recomendacion #' || NEW.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'score_preparacion', OLD.score_preparacion, 'riesgo_lesion', OLD.riesgo_lesion, 'resumen', OLD.resumen, 'rutina', OLD.rutina), json_object('id', NEW.id, 'fecha', NEW.fecha, 'score_preparacion', NEW.score_preparacion, 'riesgo_lesion', NEW.riesgo_lesion, 'resumen', NEW.resumen, 'rutina', NEW.rutina));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_recomendacion_ad AFTER DELETE ON bienestar_recomendacion WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_recomendacion', OLD.id, 'borrar', 'bienestar_recomendacion #' || OLD.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'score_preparacion', OLD.score_preparacion, 'riesgo_lesion', OLD.riesgo_lesion, 'resumen', OLD.resumen, 'rutina', OLD.rutina), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- coleccion (4 columnas)
CREATE TRIGGER IF NOT EXISTS undo_coleccion_ai AFTER INSERT ON coleccion WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('coleccion', NEW.id, 'crear', 'coleccion #' || NEW.id, NULL, json_object('id', NEW.id, 'nombre', NEW.nombre, 'descripcion', NEW.descripcion, 'fecha_creacion', NEW.fecha_creacion));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_coleccion_au AFTER UPDATE ON coleccion WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('coleccion', NEW.id, 'editar', 'coleccion #' || NEW.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'descripcion', OLD.descripcion, 'fecha_creacion', OLD.fecha_creacion), json_object('id', NEW.id, 'nombre', NEW.nombre, 'descripcion', NEW.descripcion, 'fecha_creacion', NEW.fecha_creacion));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_coleccion_ad AFTER DELETE ON coleccion WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('coleccion', OLD.id, 'borrar', 'coleccion #' || OLD.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'descripcion', OLD.descripcion, 'fecha_creacion', OLD.fecha_creacion), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- inventario_item (7 columnas)
CREATE TRIGGER IF NOT EXISTS undo_inventario_item_ai AFTER INSERT ON inventario_item WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('inventario_item', NEW.id, 'crear', 'inventario_item #' || NEW.id, NULL, json_object('id', NEW.id, 'tipo', NEW.tipo, 'nombre', NEW.nombre, 'estado', NEW.estado, 'valor', NEW.valor, 'fecha_compra', NEW.fecha_compra, 'camiseta_id', NEW.camiseta_id));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_inventario_item_au AFTER UPDATE ON inventario_item WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('inventario_item', NEW.id, 'editar', 'inventario_item #' || NEW.id, json_object('id', OLD.id, 'tipo', OLD.tipo, 'nombre', OLD.nombre, 'estado', OLD.estado, 'valor', OLD.valor, 'fecha_compra', OLD.fecha_compra, 'camiseta_id', OLD.camiseta_id), json_object('id', NEW.id, 'tipo', NEW.tipo, 'nombre', NEW.nombre, 'estado', NEW.estado, 'valor', NEW.valor, 'fecha_compra', NEW.fecha_compra, 'camiseta_id', NEW.camiseta_id));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_inventario_item_ad AFTER DELETE ON inventario_item WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('inventario_item', OLD.id, 'borrar', 'inventario_item #' || OLD.id, json_object('id', OLD.id, 'tipo', OLD.tipo, 'nombre', OLD.nombre, 'estado', OLD.estado, 'valor', OLD.valor, 'fecha_compra', OLD.fecha_compra, 'camiseta_id', OLD.camiseta_id), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- consejos_historial (4 columnas)
CREATE TRIGGER IF NOT EXISTS undo_consejos_historial_ai AFTER INSERT ON consejos_historial WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('consejos_historial', NEW.id, 'crear', 'consejos_historial #' || NEW.id, NULL, json_object('id', NEW.id, 'fecha', NEW.fecha, 'consejo', NEW.consejo, 'seguido', NEW.seguido));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_consejos_historial_au AFTER UPDATE ON consejos_historial WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('consejos_historial', NEW.id, 'editar', 'consejos_historial #' || NEW.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'consejo', OLD.consejo, 'seguido', OLD.seguido), json_object('id', NEW.id, 'fecha', NEW.fecha, 'consejo', NEW.consejo, 'seguido', NEW.seguido));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_consejos_historial_ad AFTER DELETE ON consejos_historial WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('consejos_historial', OLD.id, 'borrar', 'consejos_historial #' || OLD.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'consejo', OLD.consejo, 'seguido', OLD.seguido), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_objetivo (6 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_objetivo_ai AFTER INSERT ON bienestar_objetivo WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_objetivo', NEW.id, 'crear', 'bienestar_objetivo #' || NEW.id, NULL, json_object('id', NEW.id, 'nombre', NEW.nombre, 'fecha_inicio', NEW.fecha_inicio, 'fecha_fin', NEW.fecha_fin, 'estado', NEW.estado, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_objetivo_au AFTER UPDATE ON bienestar_objetivo WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_objetivo', NEW.id, 'editar', 'bienestar_objetivo #' || NEW.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'fecha_inicio', OLD.fecha_inicio, 'fecha_fin', OLD.fecha_fin, 'estado', OLD.estado, 'notas', OLD.notas), json_object('id', NEW.id, 'nombre', NEW.nombre, 'fecha_inicio', NEW.fecha_inicio, 'fecha_fin', NEW.fecha_fin, 'estado', NEW.estado, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_objetivo_ad AFTER DELETE ON bienestar_objetivo WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_objetivo', OLD.id, 'borrar', 'bienestar_objetivo #' || OLD.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'fecha_inicio', OLD.fecha_inicio, 'fecha_fin', OLD.fecha_fin, 'estado', OLD.estado, 'notas', OLD.notas), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_plan_entrenamiento (5 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_plan_entrenamiento_ai AFTER INSERT ON bienestar_plan_entrenamiento WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_plan_entrenamiento', NEW.id, 'crear', 'bienestar_plan_entrenamiento #' || NEW.id, NULL, json_object('id', NEW.id, 'objetivo_id', NEW.objetivo_id, 'frecuencia_semanal', NEW.frecuencia_semanal, 'rutina_semanal', NEW.rutina_semanal, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_plan_entrenamiento_au AFTER UPDATE ON bienestar_plan_entrenamiento WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_plan_entrenamiento', NEW.id, 'editar', 'bienestar_plan_entrenamiento #' || NEW.id, json_object('id', OLD.id, 'objetivo_id', OLD.objetivo_id, 'frecuencia_semanal', OLD.frecuencia_semanal, 'rutina_semanal', OLD.rutina_semanal, 'notas', OLD.notas), json_object('id', NEW.id, 'objetivo_id', NEW.objetivo_id, 'frecuencia_semanal', NEW.frecuencia_semanal, 'rutina_semanal', NEW.rutina_semanal, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_plan_entrenamiento_ad AFTER DELETE ON bienestar_plan_entrenamiento WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_plan_entrenamiento', OLD.id, 'borrar', 'bienestar_plan_entrenamiento #' || OLD.id, json_object('id', OLD.id, 'objetivo_id', OLD.objetivo_id, 'frecuencia_semanal', OLD.frecuencia_semanal, 'rutina_semanal', OLD.rutina_semanal, 'notas', OLD.notas), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_ejercicio (3 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_ejercicio_ai AFTER INSERT ON bienestar_ejercicio WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_ejercicio', NEW.id, 'crear', 'bienestar_ejercicio #' || NEW.id, NULL, json_object('id', NEW.id, 'nombre', NEW.nombre, 'grupo_muscular', NEW.grupo_muscular));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_ejercicio_au AFTER UPDATE ON bienestar_ejercicio WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_ejercicio', NEW.id, 'editar', 'bienestar_ejercicio #' || NEW.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'grupo_muscular', OLD.grupo_muscular), json_object('id', NEW.id, 'nombre', NEW.nombre, 'grupo_muscular', NEW.grupo_muscular));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_ejercicio_ad AFTER DELETE ON bienestar_ejercicio WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_ejercicio', OLD.id, 'borrar', 'bienestar_ejercicio #' || OLD.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'grupo_muscular', OLD.grupo_muscular), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_entrenamiento_ejercicio (6 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_entrenamiento_ejercicio_ai AFTER INSERT ON bienestar_entrenamiento_ejercicio WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_entrenamiento_ejercicio', NEW.id, 'crear', 'bienestar_entrenamiento_ejercicio #' || NEW.id, NULL, json_object('id', NEW.id, 'entrenamiento_id', NEW.entrenamiento_id, 'ejercicio_id', NEW.ejercicio_id, 'series', NEW.series, 'repeticiones', NEW.repeticiones, 'tiempo_min', NEW.tiempo_min));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_entrenamiento_ejercicio_au AFTER UPDATE ON bienestar_entrenamiento_ejercicio WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_entrenamiento_ejercicio', NEW.id, 'editar', 'bienestar_entrenamiento_ejercicio #' || NEW.id, json_object('id', OLD.id, 'entrenamiento_id', OLD.entrenamiento_id, 'ejercicio_id', OLD.ejercicio_id, 'series', OLD.series, 'repeticiones', OLD.repeticiones, 'tiempo_min', OLD.tiempo_min), json_object('id', NEW.id, 'entrenamiento_id', NEW.entrenamiento_id, 'ejercicio_id', NEW.ejercicio_id, 'series', NEW.series, 'repeticiones', NEW.repeticiones, 'tiempo_min', NEW.tiempo_min));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_entrenamiento_ejercicio_ad AFTER DELETE ON bienestar_entrenamiento_ejercicio WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_entrenamiento_ejercicio', OLD.id, 'borrar', 'bienestar_entrenamiento_ejercicio #' || OLD.id, json_object('id', OLD.id, 'entrenamiento_id', OLD.entrenamiento_id, 'ejercicio_id', OLD.ejercicio_id, 'series', OLD.series, 'repeticiones', OLD.repeticiones, 'tiempo_min', OLD.tiempo_min), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_dia_nutricional (6 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_dia_nutricional_ai AFTER INSERT ON bienestar_dia_nutricional WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_dia_nutricional', NEW.id, 'crear', 'bienestar_dia_nutricional #' || NEW.id, NULL, json_object('id', NEW.id, 'fecha', NEW.fecha, 'hidratacion', NEW.hidratacion, 'alcohol', NEW.alcohol, 'peso_corporal', NEW.peso_corporal, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_dia_nutricional_au AFTER UPDATE ON bienestar_dia_nutricional WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_dia_nutricional', NEW.id, 'editar', 'bienestar_dia_nutricional #' || NEW.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'hidratacion', OLD.hidratacion, 'alcohol', OLD.alcohol, 'peso_corporal', OLD.peso_corporal, 'notas', OLD.notas), json_object('id', NEW.id, 'fecha', NEW.fecha, 'hidratacion', NEW.hidratacion, 'alcohol', NEW.alcohol, 'peso_corporal', NEW.peso_corporal, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_dia_nutricional_ad AFTER DELETE ON bienestar_dia_nutricional WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_dia_nutricional', OLD.id, 'borrar', 'bienestar_dia_nutricional #' || OLD.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'hidratacion', OLD.hidratacion, 'alcohol', OLD.alcohol, 'peso_corporal', OLD.peso_corporal, 'notas', OLD.notas), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_control_medico (6 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_control_medico_ai AFTER INSERT ON bienestar_control_medico WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_control_medico', NEW.id, 'crear', 'bienestar_control_medico #' || NEW.id, NULL, json_object('id', NEW.id, 'fecha', NEW.fecha, 'tipo', NEW.tipo, 'profesional', NEW.profesional, 'resultado', NEW.resultado, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_control_medico_au AFTER UPDATE ON bienestar_control_medico WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_control_medico', NEW.id, 'editar', 'bienestar_control_medico #' || NEW.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'tipo', OLD.tipo, 'profesional', OLD.profesional, 'resultado', OLD.resultado, 'notas', OLD.notas), json_object('id', NEW.id, 'fecha', NEW.fecha, 'tipo', NEW.tipo, 'profesional', NEW.profesional, 'resultado', NEW.resultado, 'notas', NEW.notas));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_control_medico_ad AFTER DELETE ON bienestar_control_medico WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_control_medico', OLD.id, 'borrar', 'bienestar_control_medico #' || OLD.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'tipo', OLD.tipo, 'profesional', OLD.profesional, 'resultado', OLD.resultado, 'notas', OLD.notas), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- bienestar_estudio_archivo (6 columnas)
CREATE TRIGGER IF NOT EXISTS undo_bienestar_estudio_archivo_ai AFTER INSERT ON bienestar_estudio_archivo WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_estudio_archivo', NEW.id, 'crear', 'bienestar_estudio_archivo #' || NEW.id, NULL, json_object('id', NEW.id, 'control_id', NEW.control_id, 'nombre_original', NEW.nombre_original, 'ruta_archivo', NEW.ruta_archivo, 'tipo_archivo', NEW.tipo_archivo, 'fecha_subida', NEW.fecha_subida));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_estudio_archivo_au AFTER UPDATE ON bienestar_estudio_archivo WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_estudio_archivo', NEW.id, 'editar', 'bienestar_estudio_archivo #' || NEW.id, json_object('id', OLD.id, 'control_id', OLD.control_id, 'nombre_original', OLD.nombre_original, 'ruta_archivo', OLD.ruta_archivo, 'tipo_archivo', OLD.tipo_archivo, 'fecha_subida', OLD.fecha_subida), json_object('id', NEW.id, 'control_id', NEW.control_id, 'nombre_original', NEW.nombre_original, 'ruta_archivo', NEW.ruta_archivo, 'tipo_archivo', NEW.tipo_archivo, 'fecha_subida', NEW.fecha_subida));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_bienestar_estudio_archivo_ad AFTER DELETE ON bienestar_estudio_archivo WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('bienestar_estudio_archivo', OLD.id, 'borrar', 'bienestar_estudio_archivo #' || OLD.id, json_object('id', OLD.id, 'control_id', OLD.control_id, 'nombre_original', OLD.nombre_original, 'ruta_archivo', OLD.ruta_archivo, 'tipo_archivo', OLD.tipo_archivo, 'fecha_subida', OLD.fecha_subida), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
