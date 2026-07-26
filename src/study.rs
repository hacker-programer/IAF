// ============================================================================
// study.rs — Motor de Enseñanza Autónoma
// ============================================================================
// Persistencia según especificación base:
//   Perfil:          ./.config/data/<username>/profile.json
//   Knowledge Base:  ./.config/data/<username>/learnings.json
//   Teaching Method: ./.config/data/<username>/teachingMethod.json
//   Study Projects:  ./.config/data/_projects/<project_id>.json
//
// Al inicializar, StudyEngine escanea .config/data/ y carga todos los
// perfiles, knowledge bases y proyectos existentes desde disco.
// ============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ============================================================================
// Tipos de datos
// ============================================================================

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserLearningProfile {
    pub username: String,
    pub age: Option<u8>,
    pub high_capabilities: Option<String>,
    pub neurological_conditions: Vec<String>,
    pub favorite_games: Vec<String>,
    pub favorite_youtubers: Vec<String>,
    pub hobbies: Vec<String>,
    pub phase: StudyPhase,
    pub exploration_started_at: Option<u64>,
    pub exploitation_started_at: Option<u64>,
    pub hypothesis_history: Vec<TeachingHypothesis>,
    pub learning_style_summary: String,
    pub message_timestamps: Vec<MessageTimestamp>,
    pub last_updated: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum StudyPhase {
    NotStarted,
    Exploration,
    Exploitation,
}

impl Default for StudyPhase {
    fn default() -> Self { StudyPhase::NotStarted }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TeachingHypothesis {
    pub method: String,
    pub theoretical_basis: String,
    pub analogies_used: Vec<String>,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub metrics: HypothesisMetrics,
    pub conclusion: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct HypothesisMetrics {
    pub correct_answer_rate: f64,
    pub avg_response_time_secs: f64,
    pub message_count: u64,
    pub session_duration_secs: u64,
    pub follow_up_questions: u64,
    pub user_disengaged: bool,
    pub engagement_score: f64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MessageTimestamp {
    pub hour: u32,
    pub minute: u32,
    pub day_of_week: u32,
    pub unix_timestamp: u64,
    pub is_user_message: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserKnowledgeBase {
    pub username: String,
    pub known_topics: HashMap<String, TopicProficiency>,
    pub demonstrated_skills: Vec<DemonstratedSkill>,
    pub learning_summary: String,
    pub last_updated: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TopicProficiency {
    pub topic: String,
    pub level: f64,
    pub evidence: Vec<String>,
    pub last_demonstrated: u64,
    pub explicit: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DemonstratedSkill {
    pub skill: String,
    pub evidence_snippet: String,
    pub context: String,
    pub timestamp: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StudyProject {
    pub id: String,
    pub name: String,
    pub description: String,
    pub owner: String,
    pub members: Vec<String>,
    pub study_files: HashMap<String, StudyFileMeta>,
    pub study_prompt: Option<String>,
    pub created_at: u64,
    pub last_synced: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StudyFileMeta {
    pub path: String,
    pub content_hash: String,
    pub last_modified_by: String,
    pub last_modified_at: u64,
}

/// TeachingMethod — guardado en teachingMethod.json según especificación
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct TeachingMethod {
    pub username: String,
    pub phase: StudyPhase,
    pub methods_tried: Vec<MethodRecord>,
    pub methods_to_try: Vec<String>,
    pub chosen_method: Option<String>,
    pub failure_hypothesis: Option<String>,
    pub success_hypothesis: Option<String>,
    pub average_performance: Option<f64>,
    pub last_updated: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MethodRecord {
    pub name: String,
    pub performance: f64,
    pub hypothesis_why_failed: Option<String>,
    pub tested_at: u64,
}

// ============================================================================
// StudyEngine — Motor de enseñanza con persistencia correcta
// ============================================================================

#[derive(Clone)]
pub struct StudyEngine {
    pub profiles: Arc<Mutex<HashMap<String, UserLearningProfile>>>,
    pub knowledge_bases: Arc<Mutex<HashMap<String, UserKnowledgeBase>>>,
    pub projects: Arc<Mutex<HashMap<String, StudyProject>>>,
    pub teaching_methods: Arc<Mutex<HashMap<String, TeachingMethod>>>,
    pub base_workspace: PathBuf,
}

impl StudyEngine {
    /// Crea un nuevo StudyEngine.
    /// `base_workspace` es la raíz del proyecto (donde está .config/).
    /// Escanea .config/data/ y carga todos los datos existentes desde disco.
    pub fn new(base_workspace: PathBuf) -> Self {
        let data_root = base_workspace.join(".config").join("data");
        let _ = fs::create_dir_all(&data_root);

        let mut profiles: HashMap<String, UserLearningProfile> = HashMap::new();
        let mut knowledge_bases: HashMap<String, UserKnowledgeBase> = HashMap::new();
        let mut teaching_methods: HashMap<String, TeachingMethod> = HashMap::new();

        // Escanear directorios de usuario dentro de .config/data/
        if let Ok(entries) = fs::read_dir(&data_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                // Saltar directorios internos que empiezan con _
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if dir_name.starts_with('_') {
                    continue;
                }
                let username = dir_name.to_string();

                // Cargar perfil
                let profile_path = path.join("profile.json");
                if let Ok(content) = fs::read_to_string(&profile_path) {
                    if let Ok(profile) = serde_json::from_str::<UserLearningProfile>(&content) {
                        profiles.insert(username.clone(), profile);
                    }
                }

                // Cargar knowledge base (learnings.json)
                let kb_path = path.join("learnings.json");
                if let Ok(content) = fs::read_to_string(&kb_path) {
                    if let Ok(kb) = serde_json::from_str::<UserKnowledgeBase>(&content) {
                        knowledge_bases.insert(username.clone(), kb);
                    }
                }

                // Cargar teaching method
                let tm_path = path.join("teachingMethod.json");
                if let Ok(content) = fs::read_to_string(&tm_path) {
                    if let Ok(tm) = serde_json::from_str::<TeachingMethod>(&content) {
                        teaching_methods.insert(username.clone(), tm);
                    }
                }
            }
        }

        // Cargar proyectos de estudio desde _projects/
        let projects_dir = data_root.join("_projects");
        let _ = fs::create_dir_all(&projects_dir);
        let mut projects: HashMap<String, StudyProject> = HashMap::new();
        if let Ok(entries) = fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(project) = serde_json::from_str::<StudyProject>(&content) {
                        projects.insert(project.id.clone(), project);
                    }
                }
            }
        }

        let loaded_count = profiles.len();
        if loaded_count > 0 {
            eprintln!("[IAF StudyEngine] Cargados {} perfiles desde disco.", loaded_count);
        }

        StudyEngine {
            profiles: Arc::new(Mutex::new(profiles)),
            knowledge_bases: Arc::new(Mutex::new(knowledge_bases)),
            projects: Arc::new(Mutex::new(projects)),
            teaching_methods: Arc::new(Mutex::new(teaching_methods)),
            base_workspace,
        }
    }

    // ========================================================================
    // Helpers de rutas
    // ========================================================================

    /// Ruta al directorio de datos de un usuario: .config/data/<username>/
    fn user_data_dir(&self, username: &str) -> PathBuf {
        self.base_workspace
            .join(".config")
            .join("data")
            .join(username)
    }

    /// Ruta al archivo de perfil: .config/data/<username>/profile.json
    fn profile_path(&self, username: &str) -> PathBuf {
        self.user_data_dir(username).join("profile.json")
    }

    /// Ruta al archivo de knowledge base: .config/data/<username>/learnings.json
    fn knowledge_path(&self, username: &str) -> PathBuf {
        self.user_data_dir(username).join("learnings.json")
    }

    /// Ruta al archivo de teaching method: .config/data/<username>/teachingMethod.json
    fn teaching_method_path(&self, username: &str) -> PathBuf {
        self.user_data_dir(username).join("teachingMethod.json")
    }

    /// Directorio de proyectos de estudio: .config/data/_projects/
    fn projects_dir(&self) -> PathBuf {
        self.base_workspace
            .join(".config")
            .join("data")
            .join("_projects")
    }

    // ========================================================================
    // Perfil de usuario
    // ========================================================================

    pub fn get_profile(&self, username: &str) -> Option<UserLearningProfile> {
        self.profiles.lock().unwrap().get(username).cloned()
    }

    pub fn get_or_create_profile(&self, username: &str) -> UserLearningProfile {
        let mut profiles = self.profiles.lock().unwrap();
        if let Some(p) = profiles.get(username) {
            return p.clone();
        }
        let now = now_secs();
        let p = UserLearningProfile {
            username: username.to_string(),
            phase: StudyPhase::Exploration,
            exploration_started_at: Some(now),
            last_updated: now,
            ..Default::default()
        };
        profiles.insert(username.to_string(), p.clone());
        p
    }

    pub fn save_profile(&self, profile: &UserLearningProfile) -> Result<(), String> {
        let dir = self.user_data_dir(&profile.username);
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Error creando directorio para {}: {}", profile.username, e))?;
        let path = self.profile_path(&profile.username);
        let json =
            serde_json::to_string_pretty(profile).map_err(|e| format!("Error serializando: {}", e))?;
        fs::write(&path, &json).map_err(|e| format!("Error escribiendo perfil: {}", e))?;
        self.profiles
            .lock()
            .unwrap()
            .insert(profile.username.clone(), profile.clone());
        Ok(())
    }

    /// Verifica que el perfil existe en disco (para tests de regresión)
    pub fn profile_exists_on_disk(&self, username: &str) -> bool {
        self.profile_path(username).exists()
    }

    // ========================================================================
    // Knowledge Base (learnings.json)
    // ========================================================================

    pub fn get_knowledge(&self, username: &str) -> Option<UserKnowledgeBase> {
        self.knowledge_bases.lock().unwrap().get(username).cloned()
    }

    pub fn get_or_create_knowledge(&self, username: &str) -> UserKnowledgeBase {
        let mut kbs = self.knowledge_bases.lock().unwrap();
        if let Some(kb) = kbs.get(username) {
            return kb.clone();
        }
        let kb = UserKnowledgeBase {
            username: username.to_string(),
            last_updated: now_secs(),
            ..Default::default()
        };
        kbs.insert(username.to_string(), kb.clone());
        kb
    }

    pub fn save_knowledge(&self, kb: &UserKnowledgeBase) -> Result<(), String> {
        let dir = self.user_data_dir(&kb.username);
        let _ = fs::create_dir_all(&dir);
        let path = self.knowledge_path(&kb.username);
        let json =
            serde_json::to_string_pretty(kb).map_err(|e| format!("Error serializando: {}", e))?;
        fs::write(&path, &json).map_err(|e| format!("Error escribiendo knowledge: {}", e))?;
        self.knowledge_bases
            .lock()
            .unwrap()
            .insert(kb.username.clone(), kb.clone());
        Ok(())
    }

    /// Verifica que el knowledge base existe en disco
    pub fn knowledge_exists_on_disk(&self, username: &str) -> bool {
        self.knowledge_path(username).exists()
    }

    pub fn record_knowledge_demonstration(
        &self,
        username: &str,
        topic: &str,
        evidence: &str,
        explicit: bool,
    ) -> Result<(), String> {
        let mut kb = self.get_or_create_knowledge(username);
        let now = now_secs();
        let entry = kb
            .known_topics
            .entry(topic.to_string())
            .or_insert_with(|| TopicProficiency {
                topic: topic.to_string(),
                level: 0.0,
                evidence: Vec::new(),
                last_demonstrated: now,
                explicit: false,
            });
        entry.evidence.push(evidence.to_string());
        entry.last_demonstrated = now;
        if explicit {
            entry.explicit = true;
        }
        entry.level = (entry.level + if explicit { 0.15 } else { 0.05 }).min(1.0);
        kb.last_updated = now;
        self.save_knowledge(&kb)
    }

    pub fn knows_topic(&self, username: &str, topic: &str) -> bool {
        self.get_knowledge(username)
            .and_then(|kb| kb.known_topics.get(topic).map(|t| t.level))
            .unwrap_or(0.0)
            > 0.3
    }

    // ========================================================================
    // Teaching Method (teachingMethod.json)
    // ========================================================================

    pub fn get_teaching_method(&self, username: &str) -> Option<TeachingMethod> {
        self.teaching_methods.lock().unwrap().get(username).cloned()
    }

    pub fn get_or_create_teaching_method(&self, username: &str) -> TeachingMethod {
        let mut tms = self.teaching_methods.lock().unwrap();
        if let Some(tm) = tms.get(username) {
            return tm.clone();
        }
        let tm = TeachingMethod {
            username: username.to_string(),
            phase: StudyPhase::Exploration,
            last_updated: now_secs(),
            ..Default::default()
        };
        tms.insert(username.to_string(), tm.clone());
        tm
    }

    pub fn save_teaching_method(&self, tm: &TeachingMethod) -> Result<(), String> {
        let dir = self.user_data_dir(&tm.username);
        let _ = fs::create_dir_all(&dir);
        let path = self.teaching_method_path(&tm.username);
        let json =
            serde_json::to_string_pretty(tm).map_err(|e| format!("Error serializando: {}", e))?;
        fs::write(&path, &json)
            .map_err(|e| format!("Error escribiendo teachingMethod: {}", e))?;
        self.teaching_methods
            .lock()
            .unwrap()
            .insert(tm.username.clone(), tm.clone());
        Ok(())
    }

    /// Verifica que el teaching method existe en disco
    pub fn teaching_method_exists_on_disk(&self, username: &str) -> bool {
        self.teaching_method_path(username).exists()
    }

    // ========================================================================
    // Message Timestamps & Engagement
    // ========================================================================

    pub fn record_message_timestamp(&self, username: &str, is_user: bool) -> Result<(), String> {
        let mut profile = self.get_or_create_profile(username);
        let now = now_secs();
        let secs = now % 86400;
        let hour = (secs / 3600) as u32;
        let minute = ((secs % 3600) / 60) as u32;
        let day = ((now / 86400 + 4) % 7) as u32;
        profile.message_timestamps.push(MessageTimestamp {
            hour,
            minute,
            day_of_week: day,
            unix_timestamp: now,
            is_user_message: is_user,
        });
        if profile.message_timestamps.len() > 500 {
            let split_idx = profile.message_timestamps.len() - 500;
            profile.message_timestamps = profile.message_timestamps.split_off(split_idx);
        }
        profile.last_updated = now;
        self.save_profile(&profile)
    }

    pub fn calculate_engagement(&self, username: &str) -> f64 {
        let profile = match self.get_profile(username) {
            Some(p) => p,
            None => return 0.0,
        };
        let user_ts: Vec<_> = profile
            .message_timestamps
            .iter()
            .filter(|t| t.is_user_message)
            .map(|t| t.unix_timestamp)
            .collect();
        if user_ts.len() < 3 {
            return 0.5;
        }
        let avg_gap: f64 = user_ts
            .windows(2)
            .map(|w| (w[1] - w[0]) as f64)
            .sum::<f64>()
            / (user_ts.len() - 1) as f64;
        if avg_gap < 30.0 {
            1.0
        } else if avg_gap > 600.0 {
            0.0
        } else {
            1.0 - ((avg_gap - 30.0) / 570.0).clamp(0.0, 1.0)
        }
    }

    pub fn detect_disengagement(&self, username: &str) -> bool {
        let profile = match self.get_profile(username) {
            Some(p) => p,
            None => return false,
        };
        if profile.message_timestamps.is_empty() {
            return false;
        }
        now_secs() - profile.message_timestamps.last().unwrap().unix_timestamp > 900
    }

    // ========================================================================
    // Study Projects
    // ========================================================================

    pub fn create_study_project(
        &self,
        name: &str,
        description: &str,
        owner: &str,
    ) -> Result<StudyProject, String> {
        let id = format!("study_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let now = now_secs();
        let project = StudyProject {
            id: id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            owner: owner.to_string(),
            members: vec![owner.to_string()],
            study_files: HashMap::new(),
            study_prompt: None,
            created_at: now,
            last_synced: now,
        };
        self.save_project(&project)?;
        Ok(project)
    }

    pub fn add_member_to_project(&self, project_id: &str, username: &str) -> Result<(), String> {
        let mut projects = self.projects.lock().unwrap();
        let project = projects
            .get_mut(project_id)
            .ok_or_else(|| "Proyecto no encontrado.".to_string())?;
        if !project.members.contains(&username.to_string()) {
            project.members.push(username.to_string());
        }
        let project = project.clone();
        drop(projects);
        self.save_project(&project)
    }

    pub fn save_project(&self, project: &StudyProject) -> Result<(), String> {
        let dir = self.projects_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.json", project.id));
        let json = serde_json::to_string_pretty(project)
            .map_err(|e| format!("Error serializando proyecto: {}", e))?;
        fs::write(&path, &json).map_err(|e| format!("Error escribiendo proyecto: {}", e))?;
        self.projects
            .lock()
            .unwrap()
            .insert(project.id.clone(), project.clone());
        Ok(())
    }

    pub fn get_project(&self, project_id: &str) -> Option<StudyProject> {
        self.projects.lock().unwrap().get(project_id).cloned()
    }

    /// Devuelve los IDs de proyectos en los que el usuario es miembro
    pub fn list_user_projects(&self, username: &str) -> Vec<String> {
        self.projects
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.members.contains(&username.to_string()))
            .map(|p| p.id.clone())
            .collect()
    }

    /// Lista todos los usuarios que tienen perfil cargado
    pub fn list_users(&self) -> Vec<String> {
        self.profiles.lock().unwrap().keys().cloned().collect()
    }
}

// ============================================================================
// Utilidad: timestamp UNIX en segundos
// ============================================================================

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Contador atómico para generar directorios únicos por test.
    /// Evita race conditions cuando los tests se ejecutan en paralelo.
    static TEST_DIR_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_engine() -> StudyEngine {
        let id = TEST_DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        let tmp = std::env::temp_dir().join(format!("iaf_test_study_{}", id));
        // Limpiar test anterior con este ID si existe
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);
        StudyEngine::new(tmp)
    }

    // ========================================================================
    // Tests básicos
    // ========================================================================

    #[test]
    fn test_profile_crud() {
        let engine = test_engine();
        let profile = engine.get_or_create_profile("student1");
        assert_eq!(profile.username, "student1");
        assert_eq!(profile.phase, StudyPhase::Exploration);

        engine.save_profile(&profile).unwrap();
        let loaded = engine.get_profile("student1").unwrap();
        assert_eq!(loaded.username, "student1");
    }

    #[test]
    fn test_knowledge_tracking() {
        let engine = test_engine();
        // Necesita al menos 3 demostraciones explícitas para superar el umbral de 0.3
        engine
            .record_knowledge_demonstration("student1", "rust", "fn main() {}", true)
            .unwrap();
        engine
            .record_knowledge_demonstration("student1", "rust", "let x = 5;", true)
            .unwrap();
        engine
            .record_knowledge_demonstration("student1", "rust", "struct Foo;", true)
            .unwrap();
        assert!(engine.knows_topic("student1", "rust"));
    }

    #[test]
    fn test_engagement() {
        let engine = test_engine();
        let mut profile = engine.get_or_create_profile("user");
        let now = now_secs();
        for i in 0..5 {
            profile.message_timestamps.push(MessageTimestamp {
                hour: 12,
                minute: i,
                day_of_week: 1,
                unix_timestamp: now - (5 - i) as u64 * 15,
                is_user_message: true,
            });
        }
        engine.save_profile(&profile).unwrap();
        let e = engine.calculate_engagement("user");
        assert!(e > 0.8);
    }

    // ========================================================================
    // REG-STU-001: El perfil debe guardarse en .config/data/<user>/profile.json
    // ========================================================================

    #[test]
    fn reg_stu001_profile_saved_to_correct_path() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu001");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        let engine = StudyEngine::new(tmp.clone());
        let profile = engine.get_or_create_profile("alumno1");
        engine.save_profile(&profile).unwrap();

        // Verificar que el archivo existe en la ruta correcta
        let expected_path = tmp
            .join(".config")
            .join("data")
            .join("alumno1")
            .join("profile.json");
        assert!(
            expected_path.exists(),
            "El perfil debe guardarse en .config/data/alumno1/profile.json"
        );

        // Verificar que se puede cargar desde disco
        let content = std::fs::read_to_string(&expected_path).unwrap();
        let loaded: UserLearningProfile = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.username, "alumno1");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ========================================================================
    // REG-STU-002: Knowledge base debe guardarse en learnings.json
    // ========================================================================

    #[test]
    fn reg_stu002_knowledge_saved_to_correct_path() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu002");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        let engine = StudyEngine::new(tmp.clone());
        engine
            .record_knowledge_demonstration("alumno2", "python", "print('hello')", true)
            .unwrap();

        let expected_path = tmp
            .join(".config")
            .join("data")
            .join("alumno2")
            .join("learnings.json");
        assert!(
            expected_path.exists(),
            "La knowledge base debe guardarse en .config/data/alumno2/learnings.json"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ========================================================================
    // REG-STU-003: Los perfiles deben cargarse desde disco al iniciar
    // ========================================================================

    #[test]
    fn reg_stu003_profiles_loaded_on_startup() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu003");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        // Crear perfil directamente en disco
        let profile_dir = tmp.join(".config").join("data").join("alumno3");
        fs::create_dir_all(&profile_dir).unwrap();
        let profile_path = profile_dir.join("profile.json");
        let profile = UserLearningProfile {
            username: "alumno3".to_string(),
            age: Some(18),
            hobbies: vec!["dibujo".to_string()],
            ..Default::default()
        };
        fs::write(&profile_path, serde_json::to_string_pretty(&profile).unwrap()).unwrap();

        // Iniciar engine: debe cargar el perfil desde disco
        let engine = StudyEngine::new(tmp.clone());
        let loaded = engine.get_profile("alumno3");
        assert!(loaded.is_some(), "El perfil debe cargarse desde disco al iniciar");
        assert_eq!(loaded.unwrap().age, Some(18));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ========================================================================
    // REG-STU-004: Knowledge base debe cargarse desde disco al iniciar
    // ========================================================================

    #[test]
    fn reg_stu004_knowledge_loaded_on_startup() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu004");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        let kb_dir = tmp.join(".config").join("data").join("alumno4");
        fs::create_dir_all(&kb_dir).unwrap();
        let kb_path = kb_dir.join("learnings.json");

        let mut kb = UserKnowledgeBase {
            username: "alumno4".to_string(),
            ..Default::default()
        };
        kb.known_topics.insert(
            "rust".to_string(),
            TopicProficiency {
                topic: "rust".to_string(),
                level: 0.9,
                evidence: vec!["sabe traits".to_string()],
                last_demonstrated: 1000,
                explicit: true,
            },
        );
        fs::write(&kb_path, serde_json::to_string_pretty(&kb).unwrap()).unwrap();

        let engine = StudyEngine::new(tmp.clone());
        let loaded = engine.get_knowledge("alumno4");
        assert!(loaded.is_some());
        assert!(loaded.unwrap().known_topics.contains_key("rust"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ========================================================================
    // REG-STU-005: Teaching method se guarda en teachingMethod.json
    // ========================================================================

    #[test]
    fn reg_stu005_teaching_method_saved_to_correct_path() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu005");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        let engine = StudyEngine::new(tmp.clone());
        let tm = engine.get_or_create_teaching_method("alumno5");
        engine.save_teaching_method(&tm).unwrap();

        let expected_path = tmp
            .join(".config")
            .join("data")
            .join("alumno5")
            .join("teachingMethod.json");
        assert!(
            expected_path.exists(),
            "teachingMethod.json debe existir en .config/data/alumno5/"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ========================================================================
    // REG-STU-006: Múltiples usuarios independientes
    // ========================================================================

    #[test]
    fn reg_stu006_multiple_users_independent_persistence() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu006");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        let engine = StudyEngine::new(tmp.clone());

        let p1 = engine.get_or_create_profile("userA");
        let p2 = engine.get_or_create_profile("userB");
        engine.save_profile(&p1).unwrap();
        engine.save_profile(&p2).unwrap();

        let users = engine.list_users();
        assert!(users.contains(&"userA".to_string()));
        assert!(users.contains(&"userB".to_string()));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ========================================================================
    // REG-STU-007: profile_exists_on_disk es preciso
    // ========================================================================

    #[test]
    fn reg_stu007_profile_exists_on_disk_is_accurate() {
        let engine = test_engine();
        assert!(!engine.profile_exists_on_disk("nadie"));

        let profile = engine.get_or_create_profile("alumno7");
        engine.save_profile(&profile).unwrap();
        assert!(engine.profile_exists_on_disk("alumno7"));
    }

    // ========================================================================
    // REG-STU-008: Startup con datos vacíos es seguro
    // ========================================================================

    #[test]
    fn reg_stu008_empty_startup_is_safe() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu008");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);
        // No creamos nada en .config/data/ — debe iniciar vacío sin panics

        let engine = StudyEngine::new(tmp.clone());
        let users = engine.list_users();
        assert!(users.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ========================================================================
    // REG-STU-009: Directorios internos (_projects) no se cargan como usuarios
    // ========================================================================

    #[test]
    fn reg_stu009_internal_dirs_not_loaded_as_users() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu009");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        // Crear _projects (directorio interno que NO debe aparecer como usuario)
        let internal = tmp.join(".config").join("data").join("_projects");
        fs::create_dir_all(&internal).unwrap();

        let engine = StudyEngine::new(tmp.clone());
        let users = engine.list_users();
        assert!(!users.contains(&"_projects".to_string()));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ========================================================================
    // REG-STU-010: save crea directorios si no existen
    // ========================================================================

    #[test]
    fn reg_stu010_save_creates_missing_directories() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu010");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        let engine = StudyEngine::new(tmp.clone());
        // No creamos el directorio manualmente — save debe crearlo
        let profile = engine.get_or_create_profile("alumno10");
        engine.save_profile(&profile).unwrap();

        let expected_path = tmp
            .join(".config")
            .join("data")
            .join("alumno10")
            .join("profile.json");
        assert!(expected_path.exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
