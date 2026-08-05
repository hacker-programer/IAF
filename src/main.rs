// Líneas 1-35 de 2318 en C:\Users\Fa\Desktop\IAF\src\main.rs
#![allow(dead_code, unused_imports, unused_variables, unused_mut, unused_assignments, unused_must_use)]
use axum::{
    extract::{State, Json, Path as AxumPath},
    response::IntoResponse,
    routing::{get, post, put, delete},
    Router,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

mod agent;
mod scraper;
mod validator;
mod desktop;
mod state;
mod sub_agent;
mod auth;
mod study;
mod sync;
mod client_protocol;
mod google_auth;
mod google_drive;
mod google_gmail;
mod google_docs;
mod task_scheduler;
mod file_editor;

use crate::state::{
    AppState, Project, PromptConfig, ActiveAgentStatus, ProcessRegistry, ToolResultStore, SubAgentManager,
    ChatSession, ChatMessage, CicleState, CiclePhase, CaptchaRequest, AuditStep,
};
use crate::desktop::DesktopController;
use crate::auth::{UserStore, ChallengeStore, SessionStore, UserLimits, WeeklySchedule, generate_keypair};
use crate::study::StudyEngine;
use crate::sync::SyncStore;
use crate::client_protocol::{
    ClientRequest, ConnectRequest, HeartbeatRequest,
    ClientResponseWrapper, PollRequest, ConnectedClient,
};
use crate::google_auth::{GoogleAuthStore, ALL_SCOPES};
use crate::google_drive::{GoogleDriveClient, DriveFile, DriveResult};
use crate::google_gmail::{GmailClient, GmailMessage, GmailResult};
use crate::google_docs::{GoogleDocsClient, GoogleSheetsClient, GoogleDocContent, GoogleSheetData};
use crate::task_scheduler::{
    TaskSchedulerStore, ScheduledTask, TaskAction, TaskFrequency, TaskStatus,
    TaskExecutionLog, DriveOrganizeRule,
};
use crate::file_editor::{EditMode, EditResult, edit_file, read_file_full, read_file_range};
use std::sync::OnceLock;
