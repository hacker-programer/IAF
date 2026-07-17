// ============================================================================
// study.rs — Motor de Enseñanza Autónoma con Fases de Exploración/Explotación
// ============================================================================
//
// Fase 1: EXPLORACIÓN — Construye un micro-perfil del usuario.
//   Pregunta: edad, altas capacidades, condiciones neurológicas, juegos favoritos,
//   youtubers favoritos, hobbies. Usa analogías basadas en sus intereses.
//   Aplica neurociencia, psicología y pedagogía para diseñar hipótesis de enseñanza.
//   Prueba métodos y mide efectividad (engagement, tasas de acierto, tiempos).
//
// Fase 2: EXPLOTACIÓN — Método optimizado encontrado.
//   Mide rendimiento. Si cae significativamente, teoriza causa y solución.
//   Guarda resumen de cómo aprende el usuario.
//
// Principios:
//   - Método socrático SOLO cuando detecta dificultad real.
//   - Forja personas autónomas, NO dependientes de IA.
//   - Enseña, NO hace el código por el alumno.
//   - Base de conocimiento semi-global (compartida entre proyectos, local del usuario).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;

// ============================================================================
// Perfil de Aprendizaje del Usuario
// ============================================================================

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserLearningProfile {
    pub username: String,
    /// Edad (opcional, se pregunta solo una vez)
    pub age: Option<u8>,
    /// "Sí", "No", "No sé"
    pub high_capabilities: Option<String>,
    /// Condiciones neurológicas (TDAH, TEA, dislexia, etc.)
    pub neurological_conditions: Vec<String>,
    /// Juegos favoritos (para analogías)
    pub favorite_games: Vec<String>,
    /// YouTubers favoritos (para analogías)
    pub favorite_youtubers: Vec<String>,
    /// Hobbies
    pub hobbies: Vec<String>,
    /// Fase actual del estudio
    pub phase: StudyPhase,
    /// Cuándo se inició la exploración
    pub exploration_started_at: Option<u64>,
    /// Cuándo se pasó a explotación
    pub exploitation_started_at: Option<u64>,
    /// Hipótesis de enseñanza probadas
    pub hypothesis_history: Vec<TeachingHypothesis>,
    /// Resumen del estilo de aprendizaje
    pub learning_style_summary: String,
    /// Timestamps de mensajes (para detectar abandono)
    pub message_timestamps: Vec<MessageTimestamp>,
    /// Última actualización
    pub last_updated: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum StudyPhase {
    /// Aún no se ha iniciado el perfilado
    NotStarted,
    /// Fase de exploración: construyendo perfil, probando métodos
    Exploration,
    /// Fase de explotación: método optimizado, midiendo rendimiento
    Exploitation,
}

impl Default for StudyPhase {
    fn default() -> Self { StudyPhase::NotStarted }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TeachingHypothesis {
    /// Descripción del método probado
    pub method: String,
    /// Base teórica (neurociencia, psicología, pedagogía)
    pub theoretical_basis: String,
    /// Analogías usadas
    pub analogies_used: Vec<String>,
    /// Timestamp de inicio
    pub started_at: u64,
    /// Timestamp de fin (None si sigue activa)
    pub ended_at: Option<u64>,
    /// Métricas de efectividad
    pub metrics: HypothesisMetrics,
    /// Conclusión: "efectivo", "inefectivo", "parcial"
    pub conclusion: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct HypothesisMetrics {
    /// Tasa de respuestas correctas
    pub correct_answer_rate: f64,
    /// Tiempo promedio de respuesta (segundos)
    pub avg_response_time_secs: f64,
    /// Número de mensajes intercambiados
    pub message_count: u64,
    /// Duración de la sesión en segundos
    pub session_duration_secs: u64,
    /// ¿El usuario hizo preguntas de seguimiento? (señal de interés)
    pub follow_up_questions: u64,
    /// ¿El usuario abandonó? (más de 10 min sin responder)
    pub user_disengaged: bool,
    /// Nivel de engagement (0.0 - 1.0) basado en velocidad de respuesta
    pub engagement_score: f64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MessageTimestamp {
    pub hour: u32,       // 0-23
    pub minute: u32,      // 0-59
    pub day_of_week: u32, // 0=Sun, 6=Sat
    pub unix_timestamp: u64,
    pub is_user_message: bool,
}

// ============================================================================
// Base de Conocimiento Semi-Global
// ============================================================================

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserKnowledgeBase {
    pub username: String,
    /// Temas conocidos con nivel de competencia (0.0 a 1.0)
    pub known_topics: HashMap<String, TopicProficiency>,
    /// Habilidades demostradas (con evidencia)
    pub demonstrated_skills: Vec<DemonstratedSkill>,
    /// Resumen generado por IA de cómo aprende
    pub learning_summary: String,
    /// Última actualización
    pub last_updated: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TopicProficiency {
    pub topic: String,
    /// 0.0 = no sabe nada, 1.0 = experto
    pub level: f64,
    /// Evidencia: fragmentos de código, respuestas correctas
    pub evidence: Vec<String>,
    /// Última vez que demostró conocimiento
    pub last_demonstrated: u64,
    /// ¿Fue demostrado explícitamente o inferido?
    pub explicit: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DemonstratedSkill {
    pub skill: String,
    pub evidence_snippet: String,
    pub context: String,
    pub timestamp: u64,
}

// ============================================================================
// Proyecto de Estudio Compartido
// ============================================================================

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StudyProject {
    pub id: String,
    pub name: String,
    pub description: String,
    pub owner: String,
    /// Miembros del proyecto (usernames)
    pub members: Vec<String>,
    /// Archivos de estudio sincronizados
    pub study_files: HashMap<String, StudyFileMeta>,
    /// Prompt de estudio local (personalizable por proyecto)
    pub study_prompt: Option<String>,
    pub created_at: u64,
    pub last_synced: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StudyFileMeta {
    pub path: String,
    pub content_hash: String,  // SHA256
    pub last_modified_by: String,
    pub last_modified_at: u64,
}

// ============================================================================
// StudyEngine — Motor principal de estudio
// ============================================================================

#[derive(Clone)]
pub struct StudyEngine {
    /// Perfiles de aprendizaje por usuario
    pub profiles: Arc<Mutex<HashMap<String, UserLearningProfile>>>,
    /// Bases de conocimiento por usuario
    pub knowledge_bases: Arc<Mutex<HashMap<String, UserKnowledgeBase>>>,
    /// Proyectos de estudio compartidos
    pub projects: Arc<Mutex<HashMap<String, StudyProject>>>,
    /// Directorio base para datos de estudio
    pub data_dir: PathBuf,
}

impl StudyEngine {
    pub fn new(data_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(data_dir.join("profiles"));
        let _ = fs::create_dir_all(data_dir.join("knowledge"));
        let _ = fs::create_dir_all(data_dir.join("projects"));

        let profiles = Self::load_all_profiles(&data_dir);
        let knowledge_bases = Self::load_all_knowledge(&data_dir);
        let projects = Self::load_all_projects(&data_dir);

        StudyEngine {
            profiles: Arc::new(Mutex::new(profiles)),
            knowledge_bases: Arc::new(Mutex::new(knowledge_bases)),
            projects: Arc::new(Mutex::new(projects)),
            data_dir,
        }
    }

    // =========================================================================
    // Carga y persistencia
    // =========================================================================

    fn load_all_profiles(data_dir: &PathBuf) -> HashMap<String, UserLearningProfile> {
        let dir = data_dir.join("profiles");
        let mut map = HashMap::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.filter_map(Result::ok) {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(profile) = serde_json::from_str::<UserLearningProfile>(&content) {
                            map.insert(profile.username.clone(), profile);
                        }
                    }
                }
            }
        }
        map
    }

    fn load_all_knowledge(data_dir: &PathBuf) -> HashMap<String, UserKnowledgeBase> {
        let dir = data_dir.join("knowledge");
        let mut map = HashMap::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.filter_map(Result::ok) {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(kb) = serde_json::from_str::<UserKnowledgeBase>(&content) {
                            map.insert(kb.username.clone(), kb);
                        }
                    }
                }
            }
        }
        map
    }

    fn load_all_projects(data_dir: &PathBuf) -> HashMap<String, StudyProject> {
        let dir = data_dir.join("projects");
        let mut map = HashMap::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.filter_map(Result::ok) {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(proj) = serde_json::from_str::<StudyProject>(&content) {
                            map.insert(proj.id.clone(), proj);
                        }
                    }
                }
            }
        }
        map
    }

    pub fn save_profile(&self, profile: &UserLearningProfile) -> Result<(), String> {
        let path = self.data_dir.join("profiles").join(format!("{}.json", profile.username));
        let json = serde_json::to_string_pretty(profile)
            .map_err(|e| format!("Error serializando perfil: {}", e))?;
        fs::write(&path, json).map_err(|e| format!("Error guardando perfil: {}", e))?;
        self.profiles.lock().insert(profile.username.clone(), profile.clone());
        Ok(())
    }

    pub fn save_knowledge(&self, kb: &UserKnowledgeBase) -> Result<(), String> {
        let path = self.data_dir.join("knowledge").join(format!("{}.json", kb.username));
        let json = serde_json::to_string_pretty(kb)
            .map_err(|e| format!("Error serializando knowledge base: {}", e))?;
        fs::write(&path, json).map_err(|e| format!("Error guardando knowledge base: {}", e))?;
        self.knowledge_bases.lock().insert(kb.username.clone(), kb.clone());
        Ok(())
    }

    pub fn save_project(&self, project: &StudyProject) -> Result<(), String> {
        let path = self.data_dir.join("projects").join(format!("{}.json", project.id));
        let json = serde_json::to_string_pretty(project)
            .map_err(|e| format!("Error serializando proyecto: {}", e))?;
        fs::write(&path, json).map_err(|e| format!("Error guardando proyecto: {}", e))?;
        self.projects.lock().insert(project.id.clone(), project.clone());
        Ok(())
    }

    // =========================================================================
    // API de Perfil
    // =========================================================================

    pub fn get_profile(&self, username: &str) -> Option<UserLearningProfile> {
        self.profiles.lock().get(username).cloned()
    }

    pub fn get_or_create_profile(&self, username: &str) -> UserLearningProfile {
        let mut profiles = self.profiles.lock();
        if let Some(p) = profiles.get(username) {
            return p.clone();
        }
        let profile = UserLearningProfile {
            username: username.to_string(),
            phase: StudyPhase::Exploration,
            exploration_started_at: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            ..Default::default()
        };
        profiles.insert(username.to_string(), profile.clone());
        profile
    }

    /// Registra un timestamp de mensaje para análisis de engagement.
    pub fn record_message_timestamp(&self, username: &str, is_user: bool) -> Result<(), String> {
        let mut profile = self.get_or_create_profile(username);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        // Convertir a hora local aprox (UTC para simplificar)
        let secs_since_midnight = now % 86400;
        let hour = (secs_since_midnight / 3600) as u32;
        let minute = ((secs_since_midnight % 3600) / 60) as u32;
        let day_of_week = ((now / 86400 + 4) % 7) as u32; // 1970-01-01 was Thursday

        profile.message_timestamps.push(MessageTimestamp {
            hour,
            minute,
            day_of_week,
            unix_timestamp: now,
            is_user_message: is_user,
        });

        // Limitar a últimos 500 timestamps
        if profile.message_timestamps.len() > 500 {
            profile.message_timestamps = profile.message_timestamps
                .split_off(profile.message_timestamps.len() - 500);
        }

        profile.last_updated = now;
        self.save_profile(&profile)
    }

    /// Detecta si el usuario probablemente abandonó la sesión.
    pub fn detect_disengagement(&self, username: &str) -> bool {
        let profile = match self.get_profile(username) {
            Some(p) => p,
            None => return false,
        };

        if profile.message_timestamps.is_empty() {
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let last_ts = profile.message_timestamps.last().unwrap().unix_timestamp;

        // Si el último mensaje fue hace más de 15 minutos, posible abandono
        now - last_ts > 900
    }

    /// Calcula el engagement score basado en velocidad de respuesta.
    pub fn calculate_engagement(&self, username: &str) -> f64 {
        let profile = match self.get_profile(username) {
            Some(p) => p,
            None => return 0.0,
        };

        let user_ts: Vec<_> = profile.message_timestamps.iter()
            .filter(|t| t.is_user_message)
            .map(|t| t.unix_timestamp)
            .collect();

        if user_ts.len() < 3 {
            return 0.5; // No hay suficientes datos
        }

        // Calcular tiempos entre respuestas del usuario
        let mut gaps = Vec::new();
        for i in 1..user_ts.len() {
            gaps.push(user_ts[i] - user_ts[i - 1]);
        }

        let avg_gap = gaps.iter().sum::<u64>() as f64 / gaps.len() as f64;

        // Engagement alto = gaps cortos (< 60s), bajo = gaps largos (> 300s)
        if avg_gap < 30.0 { return 1.0; }
        if avg_gap > 600.0 { return 0.0; }
        1.0 - ((avg_gap - 30.0) / 570.0).clamp(0.0, 1.0)
    }

    // =========================================================================
    // API de Conocimiento
    // =========================================================================

    pub fn get_knowledge(&self, username: &str) -> Option<UserKnowledgeBase> {
        self.knowledge_bases.lock().get(username).cloned()
    }

    pub fn get_or_create_knowledge(&self, username: &str) -> UserKnowledgeBase {
        let mut kbs = self.knowledge_bases.lock();
        if let Some(kb) = kbs.get(username) {
            return kb.clone();
        }
        let kb = UserKnowledgeBase {
            username: username.to_string(),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            ..Default::default()
        };
        kbs.insert(username.to_string(), kb.clone());
        kb
    }

    /// Registra conocimiento demostrado por el usuario.
    pub fn record_knowledge_demonstration(
        &self, username: &str, topic: &str, evidence: &str, explicit: bool,
    ) -> Result<(), String> {
        let mut kb = self.get_or_create_knowledge(username);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        let entry = kb.known_topics.entry(topic.to_string()).or_insert_with(|| TopicProficiency {
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
        // Incrementar nivel de competencia con cada demostración
        entry.level = (entry.level + if explicit { 0.15 } else { 0.05 }).min(1.0);

        // Truncar evidencia a últimas 20
        if entry.evidence.len() > 20 {
            entry.evidence = entry.evidence.split_off(entry.evidence.len() - 20);
        }

        kb.last_updated = now;
        self.save_knowledge(&kb)
    }

    /// Registra una habilidad demostrada con evidencia.
    pub fn record_demonstrated_skill(
        &self, username: &str, skill: &str, snippet: &str, context: &str,
    ) -> Result<(), String> {
        let mut kb = self.get_or_create_knowledge(username);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        kb.demonstrated_skills.push(DemonstratedSkill {
            skill: skill.to_string(),
            evidence_snippet: snippet.to_string(),
            context: context.to_string(),
            timestamp: now,
        });

        if kb.demonstrated_skills.len() > 100 {
            kb.demonstrated_skills = kb.demonstrated_skills.split_off(kb.demonstrated_skills.len() - 100);
        }

        kb.last_updated = now;
        self.save_knowledge(&kb)
    }

    /// Verifica si el usuario conoce un tema (nivel > 0.3).
    pub fn knows_topic(&self, username: &str, topic: &str) -> bool {
        self.get_knowledge(username)
            .and_then(|kb| kb.known_topics.get(topic).map(|t| t.level))
            .unwrap_or(0.0) > 0.3
    }

    // =========================================================================
    // API de Proyectos de Estudio
    // =========================================================================

    pub fn create_study_project(
        &self, name: &str, description: &str, owner: &str,
    ) -> Result<StudyProject, String> {
        let id = format!("study_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

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
        let mut projects = self.projects.lock();
        let project = projects.get_mut(project_id)
            .ok_or_else(|| "Proyecto no encontrado.".to_string())?;

        if !project.members.contains(&username.to_string()) {
            project.members.push(username.to_string());
        }

        let project = project.clone();
        drop(projects);
        self.save_project(&project)
    }

    pub fn get_user_projects(&self, username: &str) -> Vec<StudyProject> {
        self.projects.lock()
            .values()
            .filter(|p| p.members.contains(&username.to_string()))
            .cloned()
            .collect()
    }

    // =========================================================================
    // Generación de System Prompt de Estudio
    // =========================================================================

    /// Genera el system prompt de estudio personalizado para un usuario.
    pub fn build_study_system_prompt(&self, username: &str, base_prompt: &str) -> String {
        let profile = self.get_or_create_profile(username);
        let kb = self.get_or_create_knowledge(username);

        let mut prompt = base_prompt.to_string();

        // Inyectar perfil del estudiante
        prompt.push_str("\n\n---\n## PERFIL DEL ESTUDIANTE\n\n");

        if let Some(age) = profile.age {
            prompt.push_str(&format!("- **Edad**: {} años\n", age));
        }
        if let Some(ref hc) = profile.high_capabilities {
            prompt.push_str(&format!("- **Altas capacidades**: {}\n", hc));
        }
        if !profile.neurological_conditions.is_empty() {
            prompt.push_str(&format!("- **Condiciones neurológicas**: {}\n", profile.neurological_conditions.join(", ")));
        }
        if !profile.favorite_games.is_empty() {
            prompt.push_str(&format!("- **Juegos favoritos**: {}\n", profile.favorite_games.join(", ")));
        }
        if !profile.favorite_youtubers.is_empty() {
            prompt.push_str(&format!("- **YouTubers favoritos**: {}\n", profile.favorite_youtubers.join(", ")));
        }
        if !profile.hobbies.is_empty() {
            prompt.push_str(&format!("- **Hobbies**: {}\n", profile.hobbies.join(", ")));
        }

        prompt.push_str(&format!("\n## FASE ACTUAL: {}\n\n", match profile.phase {
            StudyPhase::NotStarted => "🟢 INICIO — Comienza preguntando qué sabe el usuario del tema.",
            StudyPhase::Exploration => "🔍 EXPLORACIÓN — Construye el perfil. Pregunta edad, condiciones, intereses si no se saben. Prueba métodos de enseñanza variados.",
            StudyPhase::Exploitation => "🎯 EXPLOTACIÓN — Usa el método optimizado. Mide rendimiento y ajusta.",
        }));

        // Inyectar estilos de aprendizaje
        if !profile.learning_style_summary.is_empty() {
            prompt.push_str(&format!("## RESUMEN DE ESTILO DE APRENDIZAJE\n{}\n\n", profile.learning_style_summary));
        }

        // Inyectar conocimientos conocidos
        if !kb.known_topics.is_empty() {
            prompt.push_str("## TEMAS CONOCIDOS\n");
            for (topic, prof) in &kb.known_topics {
                let emoji = if prof.level > 0.7 { "🟢" } else if prof.level > 0.3 { "🟡" } else { "🔴" };
                prompt.push_str(&format!("- {} **{}**: nivel {:.0}%\n", emoji, topic, prof.level * 100.0));
            }
            prompt.push('\n');
        }

        // Inyectar reglas de enseñanza
        prompt.push_str("\
## REGLAS DE ENSEÑANZA

1. **NUNCA hagas el código por el alumno.** Explica, guía, da pistas, pero el código lo escribe él/ella.
2. **Método socrático SOLO cuando detectes dificultad real.** No lo uses como default.
3. **Usa analogías con sus juegos/hobbies favoritos** para explicar conceptos.
4. **Forja autonomía.** Tu meta es que el alumno deje de necesitarte.
5. **Si no sabe algo, primero pregunta qué sabe del tema.** No asumas conocimiento previo.
6. **Adapta el ritmo.** Si ves engagement bajo (< 0.3), cambia de enfoque.
7. **Registra mentalmente qué funciona y qué no.** Actualiza el perfil de aprendizaje.
8. **Pregunta por edad, condiciones neurológicas e intereses** si aún no se saben (fase exploración).
9. **Para niños pequeños (< 12 años)**, usa lenguaje simple, muchas analogías visuales.
10. **Para adultos**, enfócate en aplicaciones prácticas y patrones de diseño.
");

        prompt
    }

    // =========================================================================
    // Hipótesis de enseñanza
    // =========================================================================

    pub fn record_hypothesis_start(&self, username: &str, method: &str, basis: &str, analogies: Vec<String>) -> Result<(), String> {
        let mut profile = self.get_or_create_profile(username);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

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
        &self, username: &str, conclusion: &str, metrics: HypothesisMetrics,
    ) -> Result<(), String> {
        let mut profile = self.get_or_create_profile(username);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        if let Some(hyp) = profile.hypothesis_history.last_mut() {
            hyp.ended_at = Some(now);
            hyp.metrics = metrics;
            hyp.conclusion = Some(conclusion.to_string());
        }

        profile.last_updated = now;

        // Si encontramos un método efectivo y estamos en exploración, pasar a explotación
        if conclusion.contains("efectivo") && profile.phase == StudyPhase::Exploration {
            let effective_count = profile.hypothesis_history.iter()
                .filter(|h| h.conclusion.as_ref().map(|c| c.contains("efectivo")).unwrap_or(false))
                .count();

            if effective_count >= 3 {
                profile.phase = StudyPhase::Exploitation;
                profile.exploitation_started_at = Some(now);
            }
        }

        self.save_profile(&profile)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_engine() -> StudyEngine {
        let tmp = std::env::temp_dir().join("iaf_test_study");
        let _ = std::fs::create_dir_all(&tmp);
        StudyEngine::new(tmp)
    }

    #[test]
    fn test_profile_creation_and_persistence() {
        let engine = test_engine();
        let mut profile = engine.get_or_create_profile("test_student");

        profile.age = Some(15);
        profile.favorite_games = vec!["Minecraft".into(), "Roblox".into()];
        profile.hobbies = vec!["dibujar".into()];
        engine.save_profile(&profile).unwrap();

        let loaded = engine.get_profile("test_student").unwrap();
        assert_eq!(loaded.age, Some(15));
        assert_eq!(loaded.favorite_games.len(), 2);
    }

    #[test]
    fn test_knowledge_tracking() {
        let engine = test_engine();

        engine.record_knowledge_demonstration(
            "student1", "variables_python", "x = 5", true,
        ).unwrap();

        engine.record_knowledge_demonstration(
            "student1", "loops_python", "for i in range(10)", false,
        ).unwrap();

        assert!(engine.knows_topic("student1", "variables_python"));
        assert!(!engine.knows_topic("student1", "loops_python")); // No explícito, nivel bajo

        let kb = engine.get_knowledge("student1").unwrap();
        assert_eq!(kb.known_topics.len(), 2);
    }

    #[test]
    fn test_study_project_creation() {
        let engine = test_engine();
        let proj = engine.create_study_project(
            "Python Básico", "Aprender Python desde cero", "profesor",
        ).unwrap();

        assert_eq!(proj.members, vec!["profesor".to_string()]);

        engine.add_member_to_project(&proj.id, "alumno1").unwrap();
        let projects = engine.get_user_projects("alumno1");
        assert_eq!(projects.len(), 1);
    }

    #[test]
    fn test_engagement_calculation() {
        let engine = test_engine();
        let mut profile = engine.get_or_create_profile("engaged_user");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        // Simular respuestas rápidas
        for i in 0..10 {
            profile.message_timestamps.push(MessageTimestamp {
                hour: 12,
                minute: i,
                day_of_week: 1,
                unix_timestamp: now - (10 - i) as u64 * 20, // 20s entre mensajes
                is_user_message: true,
            });
        }

        engine.save_profile(&profile).unwrap();
        let engagement = engine.calculate_engagement("engaged_user");
        assert!(engagement > 0.8, "Engagement should be high with fast responses, got {}", engagement);
    }

    #[test]
    fn test_disengagement_detection() {
        let engine = test_engine();
        let mut profile = engine.get_or_create_profile("bored_user");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        // Último mensaje fue hace 20 minutos
        profile.message_timestamps.push(MessageTimestamp {
            hour: 12,
            minute: 0,
            day_of_week: 1,
            unix_timestamp: now - 1200,
            is_user_message: true,
        });

        engine.save_profile(&profile).unwrap();
        assert!(engine.detect_disengagement("bored_user"));
    }

    #[test]
    fn test_hypothesis_flow_to_exploitation() {
        let engine = test_engine();
        let username = "hypothesis_test";

        engine.record_hypothesis_start(username, "visual_analogies", "dual_coding_theory", vec!["Minecraft".into()]).unwrap();
        engine.record_hypothesis_end(username, "muy efectivo", HypothesisMetrics {
            correct_answer_rate: 0.9,
            engagement_score: 0.85,
            ..Default::default()
        }).unwrap();

        engine.record_hypothesis_start(username, "project_based", "constructivism", vec![]).unwrap();
        engine.record_hypothesis_end(username, "efectivo", HypothesisMetrics {
            correct_answer_rate: 0.85,
            engagement_score: 0.8,
            ..Default::default()
        }).unwrap();

        engine.record_hypothesis_start(username, "pair_programming", "social_learning", vec![]).unwrap();
        engine.record_hypothesis_end(username, "efectivo", HypothesisMetrics {
            correct_answer_rate: 0.88,
            engagement_score: 0.9,
            ..Default::default()
        }).unwrap();

        // Should have transitioned to Exploitation
        let profile = engine.get_profile(username).unwrap();
        assert_eq!(profile.phase, StudyPhase::Exploitation);
    }
}
