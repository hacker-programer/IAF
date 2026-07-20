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

    pub fn get_user_projects(&self, username: &str) -> Vec<StudyProject> {
        self.projects
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.members.contains(&username.to_string()))
            .cloned()
            .collect()
    }

    pub fn save_project(&self, project: &StudyProject) -> Result<(), String> {
        let dir = self.projects_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.json", project.id));
        let json =
            serde_json::to_string_pretty(project).map_err(|e| format!("Error serializando: {}", e))?;
        fs::write(&path, &json).map_err(|e| format!("Error escribiendo proyecto: {}", e))?;
        self.projects
            .lock()
            .unwrap()
            .insert(project.id.clone(), project.clone());
        Ok(())
    }

    // ========================================================================
    // Build Study System Prompt
    // ========================================================================

    pub fn build_study_system_prompt(&self, username: &str, base_prompt: &str) -> String {
        let profile = self.get_or_create_profile(username);
        let kb = self.get_or_create_knowledge(username);
        let mut prompt = base_prompt.to_string();
        prompt.push_str(&format!("\n\n## PERFIL DEL ESTUDIANTE: {}", username));
        if let Some(age) = profile.age {
            prompt.push_str(&format!("\nEdad: {}", age));
        }
        if !profile.favorite_games.is_empty() {
            prompt.push_str(&format!(
                "\nJuegos favoritos: {}",
                profile.favorite_games.join(", ")
            ));
        }
        if !profile.hobbies.is_empty() {
            prompt.push_str(&format!("\nHobbies: {}", profile.hobbies.join(", ")));
        }
        if !profile.neurological_conditions.is_empty() {
            prompt.push_str(&format!(
                "\nCondiciones: {}",
                profile.neurological_conditions.join(", ")
            ));
        }
        prompt.push_str(&format!("\nFase: {:?}", profile.phase));
        prompt.push_str(&format!(
            "\nEngagement: {:.2}",
            self.calculate_engagement(username)
        ));
        if !kb.learning_summary.is_empty() {
            prompt.push_str(&format!(
                "\nResumen de aprendizaje: {}",
                kb.learning_summary
            ));
        }
        prompt
    }

    // ========================================================================
    // Hypothesis Tracking
    // ========================================================================

    pub fn record_hypothesis_start(
        &self,
        username: &str,
        method: &str,
        basis: &str,
        analogies: Vec<String>,
    ) -> Result<(), String> {
        let mut profile = self.get_or_create_profile(username);
        let now = now_secs();
        profile.hypothesis_history.push(TeachingHypothesis {
            method: method.to_string(),
            theoretical_basis: basis.to_string(),
            analogies_used: analogies,
            started_at: now,
            ended_at: None,
            metrics: HypothesisMetrics::default(),
            conclusion: None,
        });
        profile.last_updated = now;
        self.save_profile(&profile)
    }

    pub fn record_hypothesis_end(
        &self,
        username: &str,
        conclusion: &str,
        metrics: HypothesisMetrics,
    ) -> Result<(), String> {
        let mut profile = self.get_or_create_profile(username);
        let now = now_secs();
        if let Some(h) = profile
            .hypothesis_history
            .iter_mut()
            .rev()
            .find(|h| h.ended_at.is_none())
        {
            h.ended_at = Some(now);
            h.metrics = metrics;
            h.conclusion = Some(conclusion.to_string());
        }
        // Si hay 3+ hipótesis efectivas, transicionar a explotación
        let effective = profile
            .hypothesis_history
            .iter()
            .filter(|h| {
                matches!(
                    h.conclusion.as_deref(),
                    Some("efectivo") | Some("muy efectivo")
                )
            })
            .count();
        if effective >= 3 && profile.phase == StudyPhase::Exploration {
            profile.phase = StudyPhase::Exploitation;
            profile.exploitation_started_at = Some(now);
        }
        profile.last_updated = now;
        self.save_profile(&profile)
    }

    // ========================================================================
    // Skill Tracking
    // ========================================================================

    pub fn record_demonstrated_skill(
        &self,
        username: &str,
        skill: &str,
        snippet: &str,
        context: &str,
    ) -> Result<(), String> {
        let mut kb = self.get_or_create_knowledge(username);
        let now = now_secs();
        kb.demonstrated_skills.push(DemonstratedSkill {
            skill: skill.to_string(),
            evidence_snippet: snippet.to_string(),
            context: context.to_string(),
            timestamp: now,
        });
        kb.last_updated = now;
        self.save_knowledge(&kb)
    }
}

// ============================================================================
// Utilidad
// ============================================================================

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ============================================================================
// Tests unitarios
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_engine() -> StudyEngine {
        let tmp = std::env::temp_dir().join("iaf_test_study");
        // Limpiar tests anteriores para empezar fresco
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
            "REG-STU-001 FAIL: El perfil debe guardarse en {}. No existe.",
            expected_path.display()
        );

        // Verificar que NO existe en la ruta antigua (.config/study/profiles/)
        let old_path = tmp.join("profiles").join("alumno1.json");
        assert!(
            !old_path.exists(),
            "REG-STU-001 FAIL: El perfil NO debe guardarse en la ruta antigua {}.",
            old_path.display()
        );
    }

    // ========================================================================
    // REG-STU-002: El knowledge base debe guardarse en learnings.json
    // ========================================================================

    #[test]
    fn reg_stu002_knowledge_saved_to_correct_path() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu002");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        let engine = StudyEngine::new(tmp.clone());
        engine
            .record_knowledge_demonstration("alumno1", "rust", "fn main() {}", true)
            .unwrap();

        let expected_path = tmp
            .join(".config")
            .join("data")
            .join("alumno1")
            .join("learnings.json");
        assert!(
            expected_path.exists(),
            "REG-STU-002 FAIL: learnings.json debe existir en {}.",
            expected_path.display()
        );
    }

    // ========================================================================
    // REG-STU-003: Los perfiles deben cargarse desde disco al inicializar
    // ========================================================================

    #[test]
    fn reg_stu003_profiles_loaded_on_startup() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu003");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        // Crear un perfil y guardarlo
        {
            let engine = StudyEngine::new(tmp.clone());
            let mut profile = engine.get_or_create_profile("persistente");
            profile.age = Some(25);
            profile.hobbies = vec!["lectura".to_string(), "música".to_string()];
            engine.save_profile(&profile).unwrap();
        }

        // Crear un NUEVO engine (simulando reinicio del servidor)
        {
            let engine2 = StudyEngine::new(tmp.clone());
            let loaded = engine2.get_profile("persistente");
            assert!(
                loaded.is_some(),
                "REG-STU-003 FAIL: El perfil debe cargarse al inicializar el engine."
            );
            let loaded = loaded.unwrap();
            assert_eq!(loaded.age, Some(25));
            assert!(loaded.hobbies.contains(&"lectura".to_string()));
            assert_eq!(loaded.username, "persistente");
        }
    }

    // ========================================================================
    // REG-STU-004: Los knowledge bases deben cargarse desde disco
    // ========================================================================

    #[test]
    fn reg_stu004_knowledge_loaded_on_startup() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu004");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        // Guardar knowledge
        {
            let engine = StudyEngine::new(tmp.clone());
            engine
                .record_knowledge_demonstration("alumno", "python", "print('hello')", true)
                .unwrap();
            engine
                .record_knowledge_demonstration("alumno", "python", "def foo():", true)
                .unwrap();
            engine
                .record_knowledge_demonstration("alumno", "python", "class Bar:", true)
                .unwrap();
        }

        // Nuevo engine debe cargar el knowledge
        {
            let engine2 = StudyEngine::new(tmp.clone());
            assert!(
                engine2.knows_topic("alumno", "python"),
                "REG-STU-004 FAIL: El knowledge debe persistir y cargarse al reiniciar."
            );
        }
    }

    // ========================================================================
    // REG-STU-005: El teaching method debe guardarse en teachingMethod.json
    // ========================================================================

    #[test]
    fn reg_stu005_teaching_method_saved_to_correct_path() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu005");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        let engine = StudyEngine::new(tmp.clone());
        let mut tm = engine.get_or_create_teaching_method("alumno1");
        tm.methods_tried.push(MethodRecord {
            name: "gamificacion".to_string(),
            performance: 0.75,
            hypothesis_why_failed: None,
            tested_at: now_secs(),
        });
        engine.save_teaching_method(&tm).unwrap();

        let expected_path = tmp
            .join(".config")
            .join("data")
            .join("alumno1")
            .join("teachingMethod.json");
        assert!(
            expected_path.exists(),
            "REG-STU-005 FAIL: teachingMethod.json debe existir en {}.",
            expected_path.display()
        );

        // Verificar que se carga al reiniciar
        let engine2 = StudyEngine::new(tmp.clone());
        let loaded = engine2.get_teaching_method("alumno1");
        assert!(loaded.is_some(), "REG-STU-005 FAIL: teachingMethod debe cargarse al reiniciar.");
        let loaded = loaded.unwrap();
        assert_eq!(loaded.methods_tried.len(), 1);
        assert_eq!(loaded.methods_tried[0].name, "gamificacion");
    }

    // ========================================================================
    // REG-STU-006: Múltiples usuarios deben persistir independientemente
    // ========================================================================

    #[test]
    fn reg_stu006_multiple_users_independent_persistence() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu006");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        {
            let engine = StudyEngine::new(tmp.clone());

            let mut p1 = engine.get_or_create_profile("user_a");
            p1.age = Some(15);
            engine.save_profile(&p1).unwrap();

            let mut p2 = engine.get_or_create_profile("user_b");
            p2.age = Some(30);
            engine.save_profile(&p2).unwrap();
        }

        {
            let engine2 = StudyEngine::new(tmp.clone());
            let a = engine2.get_profile("user_a").unwrap();
            let b = engine2.get_profile("user_b").unwrap();
            assert_eq!(a.age, Some(15));
            assert_eq!(b.age, Some(30));
            assert_eq!(a.username, "user_a");
            assert_eq!(b.username, "user_b");
        }
    }

    // ========================================================================
    // REG-STU-007: profile_exists_on_disk debe reflejar el estado real
    // ========================================================================

    #[test]
    fn reg_stu007_profile_exists_on_disk_is_accurate() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu007");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        let engine = StudyEngine::new(tmp.clone());

        // No existe todavía
        assert!(!engine.profile_exists_on_disk("fantasma"));

        // Guardar perfil
        let p = engine.get_or_create_profile("real");
        engine.save_profile(&p).unwrap();

        // Ahora debe existir
        assert!(engine.profile_exists_on_disk("real"));
    }

    // ========================================================================
    // REG-STU-008: Si no hay datos previos, el engine arranca vacío sin error
    // ========================================================================

    #[test]
    fn reg_stu008_empty_startup_is_safe() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu008");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        // Directorio completamente vacío (sin .config/data)
        let engine = StudyEngine::new(tmp.clone());
        assert!(engine.get_profile("nadie").is_none());
        assert!(engine.get_knowledge("nadie").is_none());
        assert!(engine.get_teaching_method("nadie").is_none());
    }

    // ========================================================================
    // REG-STU-009: study_engine debe ignorar directorios internos (_projects)
    // ========================================================================

    #[test]
    fn reg_stu009_internal_dirs_not_loaded_as_users() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu009");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        // Pre-crear _projects dir con un proyecto
        let projects_dir = tmp.join(".config").join("data").join("_projects");
        fs::create_dir_all(&projects_dir).unwrap();

        let engine = StudyEngine::new(tmp.clone());
        // _projects NO debe aparecer como usuario
        assert!(engine.get_profile("_projects").is_none());
    }

    // ========================================================================
    // REG-STU-010: save_profile crea el directorio si no existe
    // ========================================================================

    #[test]
    fn reg_stu010_save_creates_missing_directories() {
        let tmp = std::env::temp_dir().join("iaf_test_reg_stu010");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        let engine = StudyEngine::new(tmp.clone());

        // Guardar un perfil sin haber creado el directorio manualmente
        let p = engine.get_or_create_profile("nuevo_user");
        engine.save_profile(&p).unwrap();

        // Debe existir en disco
        assert!(engine.profile_exists_on_disk("nuevo_user"));
    }
}
