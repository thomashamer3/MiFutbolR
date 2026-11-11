-- 005_undo.sql — historial de escrituras y deshacer (IMP-07).
-- Generado a partir del esquema v4; las tablas nuevas requieren su propio trigger.

CREATE TABLE IF NOT EXISTS undo_historial (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    tabla        TEXT    NOT NULL,
    registro_id  INTEGER NOT NULL,
    operacion    TEXT    NOT NULL CHECK (operacion IN ('crear','editar','borrar')), -- NOSONAR: literal repetido por diseño en trigger generado
    descripcion  TEXT    NOT NULL DEFAULT '',
    antes        TEXT,
    despues      TEXT,
    ts           TEXT    NOT NULL DEFAULT (datetime('now','localtime'))
);
CREATE INDEX IF NOT EXISTS idx_undo_fecha ON undo_historial(id DESC);

CREATE TABLE IF NOT EXISTS undo_control (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    suspendido INTEGER NOT NULL DEFAULT 0 CHECK (suspendido IN (0,1))
);
INSERT OR IGNORE INTO undo_control (id, suspendido) VALUES (1, 0);

-- camiseta (19 columnas)
CREATE TRIGGER IF NOT EXISTS undo_camiseta_ai AFTER INSERT ON camiseta WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('camiseta', NEW.id, 'crear', 'camiseta #' || NEW.id, NULL, json_object('id', NEW.id, 'nombre', NEW.nombre, 'color_principal', NEW.color_principal, 'color_secundario', NEW.color_secundario, 'marca', NEW.marca, 'modelo', NEW.modelo, 'temporada', NEW.temporada, 'estado_fisico', NEW.estado_fisico, 'fecha_compra', NEW.fecha_compra, 'costo_centavos', NEW.costo_centavos, 'observaciones', NEW.observaciones, 'proveedor', NEW.proveedor, 'fue_regalo', NEW.fue_regalo, 'regalo_de', NEW.regalo_de, 'imagen_ruta', NEW.imagen_ruta, 'activa', NEW.activa, 'sorteada', NEW.sorteada, 'created_at', NEW.created_at, 'updated_at', NEW.updated_at)); -- NOSONAR: literal repetido por diseño en trigger generado
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_camiseta_au AFTER UPDATE ON camiseta WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('camiseta', NEW.id, 'editar', 'camiseta #' || NEW.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'color_principal', OLD.color_principal, 'color_secundario', OLD.color_secundario, 'marca', OLD.marca, 'modelo', OLD.modelo, 'temporada', OLD.temporada, 'estado_fisico', OLD.estado_fisico, 'fecha_compra', OLD.fecha_compra, 'costo_centavos', OLD.costo_centavos, 'observaciones', OLD.observaciones, 'proveedor', OLD.proveedor, 'fue_regalo', OLD.fue_regalo, 'regalo_de', OLD.regalo_de, 'imagen_ruta', OLD.imagen_ruta, 'activa', OLD.activa, 'sorteada', OLD.sorteada, 'created_at', OLD.created_at, 'updated_at', OLD.updated_at), json_object('id', NEW.id, 'nombre', NEW.nombre, 'color_principal', NEW.color_principal, 'color_secundario', NEW.color_secundario, 'marca', NEW.marca, 'modelo', NEW.modelo, 'temporada', NEW.temporada, 'estado_fisico', NEW.estado_fisico, 'fecha_compra', NEW.fecha_compra, 'costo_centavos', NEW.costo_centavos, 'observaciones', NEW.observaciones, 'proveedor', NEW.proveedor, 'fue_regalo', NEW.fue_regalo, 'regalo_de', NEW.regalo_de, 'imagen_ruta', NEW.imagen_ruta, 'activa', NEW.activa, 'sorteada', NEW.sorteada, 'created_at', NEW.created_at, 'updated_at', NEW.updated_at));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_camiseta_ad AFTER DELETE ON camiseta WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('camiseta', OLD.id, 'borrar', 'camiseta #' || OLD.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'color_principal', OLD.color_principal, 'color_secundario', OLD.color_secundario, 'marca', OLD.marca, 'modelo', OLD.modelo, 'temporada', OLD.temporada, 'estado_fisico', OLD.estado_fisico, 'fecha_compra', OLD.fecha_compra, 'costo_centavos', OLD.costo_centavos, 'observaciones', OLD.observaciones, 'proveedor', OLD.proveedor, 'fue_regalo', OLD.fue_regalo, 'regalo_de', OLD.regalo_de, 'imagen_ruta', OLD.imagen_ruta, 'activa', OLD.activa, 'sorteada', OLD.sorteada, 'created_at', OLD.created_at, 'updated_at', OLD.updated_at), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- cancha (29 columnas)
CREATE TRIGGER IF NOT EXISTS undo_cancha_ai AFTER INSERT ON cancha WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('cancha', NEW.id, 'crear', 'cancha #' || NEW.id, NULL, json_object('id', NEW.id, 'nombre', NEW.nombre, 'telefono', NEW.telefono, 'direccion', NEW.direccion, 'localidad', NEW.localidad, 'tipo_cancha_codigo', NEW.tipo_cancha_codigo, 'superficie_codigo', NEW.superficie_codigo, 'techada_estado_codigo', NEW.techada_estado_codigo, 'tiene_iluminacion', NEW.tiene_iluminacion, 'horario_apertura_min', NEW.horario_apertura_min, 'horario_cierre_min', NEW.horario_cierre_min, 'precio_hora_dia_centavos', NEW.precio_hora_dia_centavos, 'precio_hora_noche_centavos', NEW.precio_hora_noche_centavos, 'tiene_vestuarios', NEW.tiene_vestuarios, 'tiene_duchas', NEW.tiene_duchas, 'tiene_buffet', NEW.tiene_buffet, 'tiene_estacionamiento', NEW.tiene_estacionamiento, 'cantidad_canchas', NEW.cantidad_canchas, 'estado', NEW.estado, 'descripcion', NEW.descripcion, 'contacto_alt', NEW.contacto_alt, 'latitud', NEW.latitud, 'longitud', NEW.longitud, 'tiene_grabacion', NEW.tiene_grabacion, 'tiene_marcador', NEW.tiene_marcador, 'imagen_ruta', NEW.imagen_ruta, 'activa', NEW.activa, 'created_at', NEW.created_at, 'updated_at', NEW.updated_at)); -- NOSONAR: literal repetido por diseño en trigger generado
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_cancha_au AFTER UPDATE ON cancha WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('cancha', NEW.id, 'editar', 'cancha #' || NEW.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'telefono', OLD.telefono, 'direccion', OLD.direccion, 'localidad', OLD.localidad, 'tipo_cancha_codigo', OLD.tipo_cancha_codigo, 'superficie_codigo', OLD.superficie_codigo, 'techada_estado_codigo', OLD.techada_estado_codigo, 'tiene_iluminacion', OLD.tiene_iluminacion, 'horario_apertura_min', OLD.horario_apertura_min, 'horario_cierre_min', OLD.horario_cierre_min, 'precio_hora_dia_centavos', OLD.precio_hora_dia_centavos, 'precio_hora_noche_centavos', OLD.precio_hora_noche_centavos, 'tiene_vestuarios', OLD.tiene_vestuarios, 'tiene_duchas', OLD.tiene_duchas, 'tiene_buffet', OLD.tiene_buffet, 'tiene_estacionamiento', OLD.tiene_estacionamiento, 'cantidad_canchas', OLD.cantidad_canchas, 'estado', OLD.estado, 'descripcion', OLD.descripcion, 'contacto_alt', OLD.contacto_alt, 'latitud', OLD.latitud, 'longitud', OLD.longitud, 'tiene_grabacion', OLD.tiene_grabacion, 'tiene_marcador', OLD.tiene_marcador, 'imagen_ruta', OLD.imagen_ruta, 'activa', OLD.activa, 'created_at', OLD.created_at, 'updated_at', OLD.updated_at), json_object('id', NEW.id, 'nombre', NEW.nombre, 'telefono', NEW.telefono, 'direccion', NEW.direccion, 'localidad', NEW.localidad, 'tipo_cancha_codigo', NEW.tipo_cancha_codigo, 'superficie_codigo', NEW.superficie_codigo, 'techada_estado_codigo', NEW.techada_estado_codigo, 'tiene_iluminacion', NEW.tiene_iluminacion, 'horario_apertura_min', NEW.horario_apertura_min, 'horario_cierre_min', NEW.horario_cierre_min, 'precio_hora_dia_centavos', NEW.precio_hora_dia_centavos, 'precio_hora_noche_centavos', NEW.precio_hora_noche_centavos, 'tiene_vestuarios', NEW.tiene_vestuarios, 'tiene_duchas', NEW.tiene_duchas, 'tiene_buffet', NEW.tiene_buffet, 'tiene_estacionamiento', NEW.tiene_estacionamiento, 'cantidad_canchas', NEW.cantidad_canchas, 'estado', NEW.estado, 'descripcion', NEW.descripcion, 'contacto_alt', NEW.contacto_alt, 'latitud', NEW.latitud, 'longitud', NEW.longitud, 'tiene_grabacion', NEW.tiene_grabacion, 'tiene_marcador', NEW.tiene_marcador, 'imagen_ruta', NEW.imagen_ruta, 'activa', NEW.activa, 'created_at', NEW.created_at, 'updated_at', NEW.updated_at));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_cancha_ad AFTER DELETE ON cancha WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('cancha', OLD.id, 'borrar', 'cancha #' || OLD.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'telefono', OLD.telefono, 'direccion', OLD.direccion, 'localidad', OLD.localidad, 'tipo_cancha_codigo', OLD.tipo_cancha_codigo, 'superficie_codigo', OLD.superficie_codigo, 'techada_estado_codigo', OLD.techada_estado_codigo, 'tiene_iluminacion', OLD.tiene_iluminacion, 'horario_apertura_min', OLD.horario_apertura_min, 'horario_cierre_min', OLD.horario_cierre_min, 'precio_hora_dia_centavos', OLD.precio_hora_dia_centavos, 'precio_hora_noche_centavos', OLD.precio_hora_noche_centavos, 'tiene_vestuarios', OLD.tiene_vestuarios, 'tiene_duchas', OLD.tiene_duchas, 'tiene_buffet', OLD.tiene_buffet, 'tiene_estacionamiento', OLD.tiene_estacionamiento, 'cantidad_canchas', OLD.cantidad_canchas, 'estado', OLD.estado, 'descripcion', OLD.descripcion, 'contacto_alt', OLD.contacto_alt, 'latitud', OLD.latitud, 'longitud', OLD.longitud, 'tiene_grabacion', OLD.tiene_grabacion, 'tiene_marcador', OLD.tiene_marcador, 'imagen_ruta', OLD.imagen_ruta, 'activa', OLD.activa, 'created_at', OLD.created_at, 'updated_at', OLD.updated_at), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- botin (5 columnas)
CREATE TRIGGER IF NOT EXISTS undo_botin_ai AFTER INSERT ON botin WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('botin', NEW.id, 'crear', 'botin #' || NEW.id, NULL, json_object('id', NEW.id, 'nombre', NEW.nombre, 'imagen_ruta', NEW.imagen_ruta, 'activa', NEW.activa, 'sorteada', NEW.sorteada)); -- NOSONAR: literal repetido por diseño en trigger generado
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_botin_au AFTER UPDATE ON botin WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('botin', NEW.id, 'editar', 'botin #' || NEW.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'imagen_ruta', OLD.imagen_ruta, 'activa', OLD.activa, 'sorteada', OLD.sorteada), json_object('id', NEW.id, 'nombre', NEW.nombre, 'imagen_ruta', NEW.imagen_ruta, 'activa', NEW.activa, 'sorteada', NEW.sorteada));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_botin_ad AFTER DELETE ON botin WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('botin', OLD.id, 'borrar', 'botin #' || OLD.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'imagen_ruta', OLD.imagen_ruta, 'activa', OLD.activa, 'sorteada', OLD.sorteada), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- equipo (10 columnas)
CREATE TRIGGER IF NOT EXISTS undo_equipo_ai AFTER INSERT ON equipo WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('equipo', NEW.id, 'crear', 'equipo #' || NEW.id, NULL, json_object('id', NEW.id, 'nombre', NEW.nombre, 'tipo', NEW.tipo, 'tipo_futbol', NEW.tipo_futbol, 'num_jugadores', NEW.num_jugadores, 'partido_id', NEW.partido_id, 'activa', NEW.activa, 'imagen_ruta', NEW.imagen_ruta, 'created_at', NEW.created_at, 'updated_at', NEW.updated_at)); -- NOSONAR: literal repetido por diseño en trigger generado
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_equipo_au AFTER UPDATE ON equipo WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('equipo', NEW.id, 'editar', 'equipo #' || NEW.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'tipo', OLD.tipo, 'tipo_futbol', OLD.tipo_futbol, 'num_jugadores', OLD.num_jugadores, 'partido_id', OLD.partido_id, 'activa', OLD.activa, 'imagen_ruta', OLD.imagen_ruta, 'created_at', OLD.created_at, 'updated_at', OLD.updated_at), json_object('id', NEW.id, 'nombre', NEW.nombre, 'tipo', NEW.tipo, 'tipo_futbol', NEW.tipo_futbol, 'num_jugadores', NEW.num_jugadores, 'partido_id', NEW.partido_id, 'activa', NEW.activa, 'imagen_ruta', NEW.imagen_ruta, 'created_at', NEW.created_at, 'updated_at', NEW.updated_at));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_equipo_ad AFTER DELETE ON equipo WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('equipo', OLD.id, 'borrar', 'equipo #' || OLD.id, json_object('id', OLD.id, 'nombre', OLD.nombre, 'tipo', OLD.tipo, 'tipo_futbol', OLD.tipo_futbol, 'num_jugadores', OLD.num_jugadores, 'partido_id', OLD.partido_id, 'activa', OLD.activa, 'imagen_ruta', OLD.imagen_ruta, 'created_at', OLD.created_at, 'updated_at', OLD.updated_at), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- jugador (6 columnas)
CREATE TRIGGER IF NOT EXISTS undo_jugador_ai AFTER INSERT ON jugador WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('jugador', NEW.id, 'crear', 'jugador #' || NEW.id, NULL, json_object('id', NEW.id, 'equipo_id', NEW.equipo_id, 'nombre', NEW.nombre, 'numero', NEW.numero, 'posicion', NEW.posicion, 'es_capitan', NEW.es_capitan)); -- NOSONAR: literal repetido por diseño en trigger generado
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_jugador_au AFTER UPDATE ON jugador WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('jugador', NEW.id, 'editar', 'jugador #' || NEW.id, json_object('id', OLD.id, 'equipo_id', OLD.equipo_id, 'nombre', OLD.nombre, 'numero', OLD.numero, 'posicion', OLD.posicion, 'es_capitan', OLD.es_capitan), json_object('id', NEW.id, 'equipo_id', NEW.equipo_id, 'nombre', NEW.nombre, 'numero', NEW.numero, 'posicion', NEW.posicion, 'es_capitan', NEW.es_capitan));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_jugador_ad AFTER DELETE ON jugador WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('jugador', OLD.id, 'borrar', 'jugador #' || OLD.id, json_object('id', OLD.id, 'equipo_id', OLD.equipo_id, 'nombre', OLD.nombre, 'numero', OLD.numero, 'posicion', OLD.posicion, 'es_capitan', OLD.es_capitan), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- partido (51 columnas)
CREATE TRIGGER IF NOT EXISTS undo_partido_ai AFTER INSERT ON partido WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('partido', NEW.id, 'crear', 'partido #' || NEW.id, NULL, json_object('id', NEW.id, 'cancha_id', NEW.cancha_id, 'camiseta_id', NEW.camiseta_id, 'botin_id', NEW.botin_id, 'fecha_hora', NEW.fecha_hora, 'mes_anio', NEW.mes_anio, 'goles', NEW.goles, 'asistencias', NEW.asistencias, 'resultado', NEW.resultado, 'rendimiento_general', NEW.rendimiento_general, 'cansancio', NEW.cansancio, 'estado_animo', NEW.estado_animo, 'comentario_personal', NEW.comentario_personal, 'clima', NEW.clima, 'dia', NEW.dia, 'precio_centavos', NEW.precio_centavos, 'tipo_partido', NEW.tipo_partido, 'rival_nombre', NEW.rival_nombre, 'tipo_rival', NEW.tipo_rival, 'posicion_jugada', NEW.posicion_jugada, 'minutos_jugados', NEW.minutos_jugados, 'intensidad', NEW.intensidad, 'condicion_cancha', NEW.condicion_cancha, 'arbitraje', NEW.arbitraje, 'eventos_clave', NEW.eventos_clave, 'rating_tecnico', NEW.rating_tecnico, 'rating_fisico', NEW.rating_fisico, 'rating_mental', NEW.rating_mental, 'estado_cancha', NEW.estado_cancha, 'goles_equipo', NEW.goles_equipo, 'goles_rival', NEW.goles_rival, 'formato_partido', NEW.formato_partido, 'tarjeta', NEW.tarjeta, 'goles_en_contra', NEW.goles_en_contra, 'dolor_fisico', NEW.dolor_fisico, 'temperatura_c', NEW.temperatura_c, 'arbitraje_score', NEW.arbitraje_score, 'lo_mejor', NEW.lo_mejor, 'que_mejorar', NEW.que_mejorar, 'tags', NEW.tags, 'goles_detalle', NEW.goles_detalle, 'asistencias_detalle', NEW.asistencias_detalle, 'atajaste_todo_el_partido', NEW.atajaste_todo_el_partido, 'diferencia_gol', NEW.diferencia_gol, 'precip_mm', NEW.precip_mm, 'wind_kmh', NEW.wind_kmh, 'weather_code', NEW.weather_code, 'clima_json', NEW.clima_json, 'apparent_temp_c', NEW.apparent_temp_c, 'created_at', NEW.created_at, 'updated_at', NEW.updated_at)); -- NOSONAR: literal repetido por diseño en trigger generado
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_partido_au AFTER UPDATE ON partido WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('partido', NEW.id, 'editar', 'partido #' || NEW.id, json_object('id', OLD.id, 'cancha_id', OLD.cancha_id, 'camiseta_id', OLD.camiseta_id, 'botin_id', OLD.botin_id, 'fecha_hora', OLD.fecha_hora, 'mes_anio', OLD.mes_anio, 'goles', OLD.goles, 'asistencias', OLD.asistencias, 'resultado', OLD.resultado, 'rendimiento_general', OLD.rendimiento_general, 'cansancio', OLD.cansancio, 'estado_animo', OLD.estado_animo, 'comentario_personal', OLD.comentario_personal, 'clima', OLD.clima, 'dia', OLD.dia, 'precio_centavos', OLD.precio_centavos, 'tipo_partido', OLD.tipo_partido, 'rival_nombre', OLD.rival_nombre, 'tipo_rival', OLD.tipo_rival, 'posicion_jugada', OLD.posicion_jugada, 'minutos_jugados', OLD.minutos_jugados, 'intensidad', OLD.intensidad, 'condicion_cancha', OLD.condicion_cancha, 'arbitraje', OLD.arbitraje, 'eventos_clave', OLD.eventos_clave, 'rating_tecnico', OLD.rating_tecnico, 'rating_fisico', OLD.rating_fisico, 'rating_mental', OLD.rating_mental, 'estado_cancha', OLD.estado_cancha, 'goles_equipo', OLD.goles_equipo, 'goles_rival', OLD.goles_rival, 'formato_partido', OLD.formato_partido, 'tarjeta', OLD.tarjeta, 'goles_en_contra', OLD.goles_en_contra, 'dolor_fisico', OLD.dolor_fisico, 'temperatura_c', OLD.temperatura_c, 'arbitraje_score', OLD.arbitraje_score, 'lo_mejor', OLD.lo_mejor, 'que_mejorar', OLD.que_mejorar, 'tags', OLD.tags, 'goles_detalle', OLD.goles_detalle, 'asistencias_detalle', OLD.asistencias_detalle, 'atajaste_todo_el_partido', OLD.atajaste_todo_el_partido, 'diferencia_gol', OLD.diferencia_gol, 'precip_mm', OLD.precip_mm, 'wind_kmh', OLD.wind_kmh, 'weather_code', OLD.weather_code, 'clima_json', OLD.clima_json, 'apparent_temp_c', OLD.apparent_temp_c, 'created_at', OLD.created_at, 'updated_at', OLD.updated_at), json_object('id', NEW.id, 'cancha_id', NEW.cancha_id, 'camiseta_id', NEW.camiseta_id, 'botin_id', NEW.botin_id, 'fecha_hora', NEW.fecha_hora, 'mes_anio', NEW.mes_anio, 'goles', NEW.goles, 'asistencias', NEW.asistencias, 'resultado', NEW.resultado, 'rendimiento_general', NEW.rendimiento_general, 'cansancio', NEW.cansancio, 'estado_animo', NEW.estado_animo, 'comentario_personal', NEW.comentario_personal, 'clima', NEW.clima, 'dia', NEW.dia, 'precio_centavos', NEW.precio_centavos, 'tipo_partido', NEW.tipo_partido, 'rival_nombre', NEW.rival_nombre, 'tipo_rival', NEW.tipo_rival, 'posicion_jugada', NEW.posicion_jugada, 'minutos_jugados', NEW.minutos_jugados, 'intensidad', NEW.intensidad, 'condicion_cancha', NEW.condicion_cancha, 'arbitraje', NEW.arbitraje, 'eventos_clave', NEW.eventos_clave, 'rating_tecnico', NEW.rating_tecnico, 'rating_fisico', NEW.rating_fisico, 'rating_mental', NEW.rating_mental, 'estado_cancha', NEW.estado_cancha, 'goles_equipo', NEW.goles_equipo, 'goles_rival', NEW.goles_rival, 'formato_partido', NEW.formato_partido, 'tarjeta', NEW.tarjeta, 'goles_en_contra', NEW.goles_en_contra, 'dolor_fisico', NEW.dolor_fisico, 'temperatura_c', NEW.temperatura_c, 'arbitraje_score', NEW.arbitraje_score, 'lo_mejor', NEW.lo_mejor, 'que_mejorar', NEW.que_mejorar, 'tags', NEW.tags, 'goles_detalle', NEW.goles_detalle, 'asistencias_detalle', NEW.asistencias_detalle, 'atajaste_todo_el_partido', NEW.atajaste_todo_el_partido, 'diferencia_gol', NEW.diferencia_gol, 'precip_mm', NEW.precip_mm, 'wind_kmh', NEW.wind_kmh, 'weather_code', NEW.weather_code, 'clima_json', NEW.clima_json, 'apparent_temp_c', NEW.apparent_temp_c, 'created_at', NEW.created_at, 'updated_at', NEW.updated_at));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_partido_ad AFTER DELETE ON partido WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('partido', OLD.id, 'borrar', 'partido #' || OLD.id, json_object('id', OLD.id, 'cancha_id', OLD.cancha_id, 'camiseta_id', OLD.camiseta_id, 'botin_id', OLD.botin_id, 'fecha_hora', OLD.fecha_hora, 'mes_anio', OLD.mes_anio, 'goles', OLD.goles, 'asistencias', OLD.asistencias, 'resultado', OLD.resultado, 'rendimiento_general', OLD.rendimiento_general, 'cansancio', OLD.cansancio, 'estado_animo', OLD.estado_animo, 'comentario_personal', OLD.comentario_personal, 'clima', OLD.clima, 'dia', OLD.dia, 'precio_centavos', OLD.precio_centavos, 'tipo_partido', OLD.tipo_partido, 'rival_nombre', OLD.rival_nombre, 'tipo_rival', OLD.tipo_rival, 'posicion_jugada', OLD.posicion_jugada, 'minutos_jugados', OLD.minutos_jugados, 'intensidad', OLD.intensidad, 'condicion_cancha', OLD.condicion_cancha, 'arbitraje', OLD.arbitraje, 'eventos_clave', OLD.eventos_clave, 'rating_tecnico', OLD.rating_tecnico, 'rating_fisico', OLD.rating_fisico, 'rating_mental', OLD.rating_mental, 'estado_cancha', OLD.estado_cancha, 'goles_equipo', OLD.goles_equipo, 'goles_rival', OLD.goles_rival, 'formato_partido', OLD.formato_partido, 'tarjeta', OLD.tarjeta, 'goles_en_contra', OLD.goles_en_contra, 'dolor_fisico', OLD.dolor_fisico, 'temperatura_c', OLD.temperatura_c, 'arbitraje_score', OLD.arbitraje_score, 'lo_mejor', OLD.lo_mejor, 'que_mejorar', OLD.que_mejorar, 'tags', OLD.tags, 'goles_detalle', OLD.goles_detalle, 'asistencias_detalle', OLD.asistencias_detalle, 'atajaste_todo_el_partido', OLD.atajaste_todo_el_partido, 'diferencia_gol', OLD.diferencia_gol, 'precip_mm', OLD.precip_mm, 'wind_kmh', OLD.wind_kmh, 'weather_code', OLD.weather_code, 'clima_json', OLD.clima_json, 'apparent_temp_c', OLD.apparent_temp_c, 'created_at', OLD.created_at, 'updated_at', OLD.updated_at), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- lesion (10 columnas)
CREATE TRIGGER IF NOT EXISTS undo_lesion_ai AFTER INSERT ON lesion WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('lesion', NEW.id, 'crear', 'lesion #' || NEW.id, NULL, json_object('id', NEW.id, 'jugador', NEW.jugador, 'tipo', NEW.tipo, 'descripcion', NEW.descripcion, 'fecha', NEW.fecha, 'camiseta_id', NEW.camiseta_id, 'estado', NEW.estado, 'partido_id', NEW.partido_id, 'created_at', NEW.created_at, 'updated_at', NEW.updated_at)); -- NOSONAR: literal repetido por diseño en trigger generado
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_lesion_au AFTER UPDATE ON lesion WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('lesion', NEW.id, 'editar', 'lesion #' || NEW.id, json_object('id', OLD.id, 'jugador', OLD.jugador, 'tipo', OLD.tipo, 'descripcion', OLD.descripcion, 'fecha', OLD.fecha, 'camiseta_id', OLD.camiseta_id, 'estado', OLD.estado, 'partido_id', OLD.partido_id, 'created_at', OLD.created_at, 'updated_at', OLD.updated_at), json_object('id', NEW.id, 'jugador', NEW.jugador, 'tipo', NEW.tipo, 'descripcion', NEW.descripcion, 'fecha', NEW.fecha, 'camiseta_id', NEW.camiseta_id, 'estado', NEW.estado, 'partido_id', NEW.partido_id, 'created_at', NEW.created_at, 'updated_at', NEW.updated_at));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_lesion_ad AFTER DELETE ON lesion WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('lesion', OLD.id, 'borrar', 'lesion #' || OLD.id, json_object('id', OLD.id, 'jugador', OLD.jugador, 'tipo', OLD.tipo, 'descripcion', OLD.descripcion, 'fecha', OLD.fecha, 'camiseta_id', OLD.camiseta_id, 'estado', OLD.estado, 'partido_id', OLD.partido_id, 'created_at', OLD.created_at, 'updated_at', OLD.updated_at), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

-- financiamiento (9 columnas)
CREATE TRIGGER IF NOT EXISTS undo_financiamiento_ai AFTER INSERT ON financiamiento WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('financiamiento', NEW.id, 'crear', 'financiamiento #' || NEW.id, NULL, json_object('id', NEW.id, 'fecha', NEW.fecha, 'tipo', NEW.tipo, 'categoria', NEW.categoria, 'descripcion', NEW.descripcion, 'monto_centavos', NEW.monto_centavos, 'item_especifico', NEW.item_especifico, 'partido_id', NEW.partido_id, 'created_at', NEW.created_at)); -- NOSONAR: literal repetido por diseño en trigger generado
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_financiamiento_au AFTER UPDATE ON financiamiento WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('financiamiento', NEW.id, 'editar', 'financiamiento #' || NEW.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'tipo', OLD.tipo, 'categoria', OLD.categoria, 'descripcion', OLD.descripcion, 'monto_centavos', OLD.monto_centavos, 'item_especifico', OLD.item_especifico, 'partido_id', OLD.partido_id, 'created_at', OLD.created_at), json_object('id', NEW.id, 'fecha', NEW.fecha, 'tipo', NEW.tipo, 'categoria', NEW.categoria, 'descripcion', NEW.descripcion, 'monto_centavos', NEW.monto_centavos, 'item_especifico', NEW.item_especifico, 'partido_id', NEW.partido_id, 'created_at', NEW.created_at));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
CREATE TRIGGER IF NOT EXISTS undo_financiamiento_ad AFTER DELETE ON financiamiento WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('financiamiento', OLD.id, 'borrar', 'financiamiento #' || OLD.id, json_object('id', OLD.id, 'fecha', OLD.fecha, 'tipo', OLD.tipo, 'categoria', OLD.categoria, 'descripcion', OLD.descripcion, 'monto_centavos', OLD.monto_centavos, 'item_especifico', OLD.item_especifico, 'partido_id', OLD.partido_id, 'created_at', OLD.created_at), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
