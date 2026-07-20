// ============================================================================
// study.rs — Motor de Enseñanza Autónoma (stub mínimo para compilación)
// ============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// Tipos básicos que main.rs/state.rs necesitan
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

// StudyEngine mínimo
#[derive(Clone)]
pub struct StudyEngine {
    pub profiles: Arc<Mutex<HashMap<String, UserLearningProfile>>>,
    pub knowledge_bases: Arc<Mutex<HashMap<String, UserKnowledgeBase>>>,
    pub projects: Arc<Mutex<HashMap<String, StudyProject>>>,
    pub data_dir: PathBuf,
}

impl StudyEngine {
    pub fn new(data_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(data_dir.join("profiles"));
        let _ = fs::create_dir_all(data_dir.join("knowledge"));
        let _ = fs::create_dir_all(data_dir.join("projects"));

        // Cargar perfiles guardados desde disco
        let profiles: HashMap<String, UserLearningProfile> = {
            let profiles_dir = data_dir.join("profiles");
            let mut map = HashMap::new();
            if profiles_dir.exists() {
                if let Ok(entries) = fs::read_dir(&profiles_dir) {
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("json") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(profile) = serde_json::from_str::<UserLearningProfile>(&content) {
                                    map.insert(profile.username.clone(), profile);
                                }
                            }
                        }
                    }
                }
            }
            map
        };

        // Cargar knowledge bases guardadas desde disco
        let knowledge_bases: HashMap<String, UserKnowledgeBase> = {
            let kb_dir = data_dir.join("knowledge");
            let mut map = HashMap::new();
            if kb_dir.exists() {
                if let Ok(entries) = fs::read_dir(&kb_dir) {
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("json") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(kb) = serde_json::from_str::<UserKnowledgeBase>(&content) {
                                    map.insert(kb.username.clone(), kb);
                                }
                            }
                        }
                    }
                }
            }
            map
        };

        // Cargar proyectos de estudio guardados desde disco
        let projects: HashMap<String, StudyProject> = {
            let proj_dir = data_dir.join("projects");
            let mut map = HashMap::new();
            if proj_dir.exists() {
                if let Ok(entries) = fs::read_dir(&proj_dir) {
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("json") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(project) = serde_json::from_str::<StudyProject>(&content) {
                                    map.insert(project.id.clone(), project);
                                }
                            }
                        }
                    }
                }
            }
            map
        };

        StudyEngine {
            profiles: Arc::new(Mutex::new(profiles)),
            knowledge_bases: Arc::new(Mutex::new(knowledge_bases)),
            projects: Arc::new(Mutex::new(projects)),
            data_dir,
        }
    }

    pub fn get_profile(&self, username: &str) -> Option<UserLearningProfile> {
        self.profiles.lock().unwrap().get(username).cloned()
    }

    pub fn get_or_create_profile(&self, username: &str) -> UserLearningProfile {
        let mut profiles = self.profiles.lock().unwrap();
        if let Some(p) = profiles.get(username) { return p.clone(); }
        let p = UserLearningProfile {
            username: username.to_string(),
            phase: StudyPhase::Exploration,
            exploration_started_at: Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
            last_updated: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            ..Default::default()
        };
        profiles.insert(username.to_string(), p.clone());
        p
    }

    pub fn save_profile(&self, profile: &UserLearningProfile) -> Result<(), String> {
        let path = self.data_dir.join("profiles").join(format!("{}.json", profile.username));
        let json = serde_json::to_string_pretty(profile).map_err(|e| format!("Error: {}", e))?;
        fs::write(&path, json).map_err(|e| format!("Error: {}", e))?;
        self.profiles.lock().unwrap().insert(profile.username.clone(), profile.clone());
        Ok(())
    }

    pub fn get_knowledge(&self, username: &str) -> Option<UserKnowledgeBase> {
        self.knowledge_bases.lock().unwrap().get(username).cloned()
    }

    pub fn get_or_create_knowledge(&self, username: &str) -> UserKnowledgeBase {
        let mut kbs = self.knowledge_bases.lock().unwrap();
        if let Some(kb) = kbs.get(username) { return kb.clone(); }
        let kb = UserKnowledgeBase {
            username: username.to_string(),
            last_updated: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            ..Default::default()
        };
        kbs.insert(username.to_string(), kb.clone());
        kb
    }

    pub fn save_knowledge(&self, kb: &UserKnowledgeBase) -> Result<(), String> {
        let path = self.data_dir.join("knowledge").join(format!("{}.json", kb.username));
        let json = serde_json::to_string_pretty(kb).map_err(|e| format!("Error: {}", e))?;
        fs::write(&path, json).map_err(|e| format!("Error: {}", e))?;
        self.knowledge_bases.lock().unwrap().insert(kb.username.clone(), kb.clone());
        Ok(())
    }

    pub fn record_knowledge_demonstration(&self, username: &str, topic: &str, evidence: &str, explicit: bool) -> Result<(), String> {
        let mut kb = self.get_or_create_knowledge(username);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let entry = kb.known_topics.entry(topic.to_string()).or_insert_with(|| TopicProficiency {
            topic: topic.to_string(), level: 0.0, evidence: Vec::new(), last_demonstrated: now, explicit: false,
        });
        entry.evidence.push(evidence.to_string());
        entry.last_demonstrated = now;
        if explicit { entry.explicit = true; }
        entry.level = (entry.level + if explicit { 0.15 } else { 0.05 }).min(1.0);
        kb.last_updated = now;
        self.save_knowledge(&kb)
    }

    pub fn knows_topic(&self, username: &str, topic: &str) -> bool {
        self.get_knowledge(username)
            .and_then(|kb| kb.known_topics.get(topic).map(|t| t.level))
            .unwrap_or(0.0) > 0.3
    }

    pub fn record_message_timestamp(&self, username: &str, is_user: bool) -> Result<(), String> {
        let mut profile = self.get_or_create_profile(username);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let secs = now % 86400;
        let hour = (secs / 3600) as u32;
        let minute = ((secs % 3600) / 60) as u32;
        let day = ((now / 86400 + 4) % 7) as u32;
        profile.message_timestamps.push(MessageTimestamp { hour, minute, day_of_week: day, unix_timestamp: now, is_user_message: is_user });
        if profile.message_timestamps.len() > 500 {
            profile.message_timestamps = profile.message_timestamps.split_off(profile.message_timestamps.len() - 500);
        }
        profile.last_updated = now;
        self.save_profile(&profile)
    }

    pub fn calculate_engagement(&self, username: &str) -> f64 {
        let profile = match self.get_profile(username) { Some(p) => p, None => return 0.0 };
        let user_ts: Vec<_> = profile.message_timestamps.iter().filter(|t| t.is_user_message).map(|t| t.unix_timestamp).collect();
        if user_ts.len() < 3 { return 0.5; }
        let avg_gap: f64 = user_ts.windows(2).map(|w| (w[1] - w[0]) as f64).sum::<f64>() / (user_ts.len() - 1) as f64;
        if avg_gap < 30.0 { 1.0 } else if avg_gap > 600.0 { 0.0 } else { 1.0 - ((avg_gap - 30.0) / 570.0).clamp(0.0, 1.0) }
    }

    pub fn detect_disengagement(&self, username: &str) -> bool {
        let profile = match self.get_profile(username) { Some(p) => p, None => return false };
        if profile.message_timestamps.is_empty() { return false; }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        now - profile.message_timestamps.last().unwrap().unix_timestamp > 900
    }

    pub fn create_study_project(&self, name: &str, description: &str, owner: &str) -> Result<StudyProject, String> {
        let id = format!("study_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let project = StudyProject {
            id: id.clone(), name: name.to_string(), description: description.to_string(),
            owner: owner.to_string(), members: vec![owner.to_string()],
            study_files: HashMap::new(), study_prompt: None, created_at: now, last_synced: now,
        };
        self.save_project(&project)?;
        Ok(project)
    }

    pub fn add_member_to_project(&self, project_id: &str, username: &str) -> Result<(), String> {
        let mut projects = self.projects.lock().unwrap();
        let project = projects.get_mut(project_id).ok_or_else(|| "Proyecto no encontrado.".to_string())?;
        if !project.members.contains(&username.to_string()) { project.members.push(username.to_string()); }
        let project = project.clone();
        drop(projects);
        self.save_project(&project)
    }

    pub fn get_user_projects(&self, username: &str) -> Vec<StudyProject> {
        self.projects.lock().unwrap().values().filter(|p| p.members.contains(&username.to_string())).cloned().collect()
    }

    pub fn save_project(&self, project: &StudyProject) -> Result<(), String> {
        let path = self.data_dir.join("projects").join(format!("{}.json", project.id));
        let json = serde_json::to_string_pretty(project).map_err(|e| format!("Error: {}", e))?;
        fs::write(&path, json).map_err(|e| format!("Error: {}", e))?;
        self.projects.lock().unwrap().insert(project.id.clone(), project.clone());
        Ok(())
    }

    pub fn build_study_system_prompt(&self, username: &str, base_prompt: &str) -> String {
        let profile = self.get_or_create_profile(username);
        let kb = self.get_or_create_knowledge(username);
        let mut prompt = base_prompt.to_string();
        prompt.push_str(&format!("\n\n## PERFIL DEL ESTUDIANTE: {}", username));
        if let Some(age) = profile.age { prompt.push_str(&format!("\nEdad: {}", age)); }
        if !profile.favorite_games.is_empty() { prompt.push_str(&format!("\nJuegos favoritos: {}", profile.favorite_games.join(", "))); }
        if !profile.hobbies.is_empty() { prompt.push_str(&format!("\nHobbies: {}", profile.hobbies.join(", "))); }
        if !profile.neurological_conditions.is_empty() { prompt.push_str(&format!("\nCondiciones: {}", profile.neurological_conditions.join(", "))); }
        prompt.push_str(&format!("\nFase: {:?}", profile.phase));
        prompt.push_str(&format!("\nEngagement: {:.2}", self.calculate_engagement(username)));
        if !kb.learning_summary.is_empty() { prompt.push_str(&format!("\nResumen de aprendizaje: {}", kb.learning_summary)); }
        prompt
    }

    pub fn record_hypothesis_start(&self, username: &str, method: &str, basis: &str, analogies: Vec<String>) -> Result<(), String> {
        let mut profile = self.get_or_create_profile(username);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        profile.hypothesis_history.push(TeachingHypothesis {
            method: method.to_string(), theoretical_basis: basis.to_string(),
            analogies_used: analogies, started_at: now, ended_at: None,
            metrics: HypothesisMetrics::default(), conclusion: None,
        });
        profile.last_updated = now;
        self.save_profile(&profile)
    }

    pub fn record_hypothesis_end(&self, username: &str, conclusion: &str, metrics: HypothesisMetrics) -> Result<(), String> {
        let mut profile = self.get_or_create_profile(username);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        if let Some(h) = profile.hypothesis_history.iter_mut().rev().find(|h| h.ended_at.is_none()) {
            h.ended_at = Some(now); h.metrics = metrics; h.conclusion = Some(conclusion.to_string());
        }
        // Check if 3+ effective hypotheses => transition to exploitation
        let effective = profile.hypothesis_history.iter()
            .filter(|h| matches!(h.conclusion.as_deref(), Some("efectivo") | Some("muy efectivo")))
            .count();
        if effective >= 3 && profile.phase == StudyPhase::Exploration {
            profile.phase = StudyPhase::Exploitation;
            profile.exploitation_started_at = Some(now);
        }
        profile.last_updated = now;
        self.save_profile(&profile)
    }

    pub fn record_demonstrated_skill(&self, username: &str, skill: &str, snippet: &str, context: &str) -> Result<(), String> {
        let mut kb = self.get_or_create_knowledge(username);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        kb.demonstrated_skills.push(DemonstratedSkill {
            skill: skill.to_string(), evidence_snippet: snippet.to_string(),
            context: context.to_string(), timestamp: now,
        });
        kb.last_updated = now;
        self.save_knowledge(&kb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_engine() -> StudyEngine {
        let tmp = std::env::temp_dir().join("iaf_test_study");
        let _ = std::fs::create_dir_all(&tmp);
        StudyEngine::new(tmp)
    }

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
        engine.record_knowledge_demonstration("student1", "rust", "fn main() {}", true).unwrap();
        engine.record_knowledge_demonstration("student1", "rust", "let x = 5;", true).unwrap();
        engine.record_knowledge_demonstration("student1", "rust", "struct Foo;", true).unwrap();
        assert!(engine.knows_topic("student1", "rust"));
    }

    #[test]
    fn test_engagement() {
        let engine = test_engine();
        let mut profile = engine.get_or_create_profile("user");
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        for i in 0..5 {
            profile.message_timestamps.push(MessageTimestamp {
                hour: 12, minute: i, day_of_week: 1,
                unix_timestamp: now - (5 - i) as u64 * 15,
                is_user_message: true,
            });
        }
        engine.save_profile(&profile).unwrap();
        let e = engine.calculate_engagement("user");
        assert!(e > 0.8);
    }

    #[test]
    fn test_profile_persistence() {
        let tmp = std::env::temp_dir().join("iaf_test_study_persist");
        let _ = std::fs::create_dir_all(&tmp);

        // Crear un perfil y guardarlo
        {
            let engine = StudyEngine::new(tmp.clone());
            let mut profile = engine.get_or_create_profile("persist_user");
            profile.age = Some(25);
            profile.favorite_games = vec!["Minecraft".to_string(), "Rust".to_string()];
            profile.hobbies = vec!["Programación".to_string()];
            profile.neurological_conditions = vec!["TDAH".to_string()];
            engine.save_profile(&profile).unwrap();
        }

        // Recargar desde disco y verificar
        {
            let engine = StudyEngine::new(tmp.clone());
            let loaded = engine.get_profile("persist_user");
            assert!(loaded.is_some(), "El perfil debería persistir en disco");
            let p = loaded.unwrap();
            assert_eq!(p.age, Some(25));
            assert_eq!(p.favorite_games, vec!["Minecraft".to_string(), "Rust".to_string()]);
            assert_eq!(p.hobbies, vec!["Programación".to_string()]);
            assert_eq!(p.neurological_conditions, vec!["TDAH".to_string()]);
        }

        // Limpiar
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_build_study_system_prompt() {
        let engine = test_engine();
        let mut profile = engine.get_or_create_profile("student_prompt");
        profile.age = Some(15);
        profile.favorite_games = vec!["Fortnite".to_string(), "Roblox".to_string()];
        profile.hobbies = vec!["Dibujo".to_string()];
        engine.save_profile(&profile).unwrap();

        let prompt = engine.build_study_system_prompt("student_prompt", "Eres un tutor.");
        assert!(prompt.contains("PERFIL DEL ESTUDIANTE"));
        assert!(prompt.contains("Fortnite"));
        assert!(prompt.contains("Dibujo"));
        assert!(prompt.contains("Edad: 15"));
        assert!(prompt.contains("Eres un tutor."));
    }
}
