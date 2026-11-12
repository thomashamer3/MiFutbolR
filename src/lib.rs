//! MiFutbolRS — gestión futbolística personal en TUI.
//!
//! Arquitectura en capas (ADR-06):
//! `ui` → `app` → `services` → `db` → SQLite, con `domain` puro en el centro.
//! La documentación de diseño vive en `skills/ESPECIFICACION_PROYECTO_RUST_TUI.md`.

pub mod app;
mod app_ajustes;
mod app_amigos;
mod app_botines;
mod app_canchas;
mod app_carrera;
mod app_comparador;
mod app_equipos;
mod app_estadisticas;
mod app_financiamiento;
mod app_inicio;
mod app_lesiones;
mod app_media;
mod app_partidos;
mod app_progresion;
mod app_recordatorios;
mod app_resumen;
mod app_temporadas;
mod app_tiendas;
mod app_torneos;
mod app_usuario;
pub mod config;
pub mod db;
pub mod domain;
pub mod services;
pub mod ui;
pub mod util;
