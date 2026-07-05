use axum::{
    extract::{State, Json, Path as AxumPath},
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

mod agent;
mod embeddings;
mod scraper;
mod desktop;
mod state;

use crate::desktop::DesktopController;
use crate::agent::{discover_projects, run_agent_loop};
use crate::state::{AppState, Project, PromptConfig, ActiveAgentStatus};

use std::sync::OnceLock;

fn deepseek_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY no configurada"))
}

fn voyage_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var("VOYAGE_API_KEY").expect("VOYAGE_API_KEY no configurada"))
fn openrouter_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY no configurada"))
}