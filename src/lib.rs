// IAF Library — expone módulos públicos para tests de integración
pub mod utils;
pub mod state;
pub mod auth;
pub mod study;
pub mod desktop;
pub mod sync;
pub mod client_protocol;

/// System prompt para el modo estudio.
/// Contiene las reglas pedagógicas: anti-resúmenes, anti-.md, tests obligatorios,
/// transparencia de razonamiento, y filosofía de enseñanza por interacción en chat.
pub const STUDY_SYSTEM_PROMPT: &str = include_str!("../prompts/study_system_prompt.txt");
