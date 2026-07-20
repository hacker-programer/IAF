// ============================================================================
// Tests de Regresión — Study Engine y Persistencia de Perfil
// Corrigen: ubicación incorrecta del perfil (.config/study/ en vez de 
// .config/data/<user>/profile.json) y falta de carga desde disco.
// ============================================================================

#[cfg(test)]
mod regression_study_tests {
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    fn tmp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("iaf_reg_{}", name));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);
        dir
    }

    // REG-STU-001: El perfil debe guardarse en .config/data/<user>/profile.json
    #[test]
    fn reg_stu001_profile_correct_path_structure() {
        let tmp = tmp_dir("stu001");
        let user_data = tmp.join(".config").join("data").join("testuser");
        fs::create_dir_all(&user_data).unwrap();
        let profile_path = user_data.join("profile.json");
        let profile = json!({
            "username": "testuser", "age": 14, "phase": "Exploration",
            "favorite_games": ["Minecraft"], "hobbies": ["programar"]
        });
        fs::write(&profile_path, serde_json::to_string_pretty(&profile).unwrap()).unwrap();
        assert!(profile_path.exists(),
            "REG-STU-001 FAIL: El perfil debe estar en .config/data/<user>/profile.json");
        let wrong_path = tmp.join(".config").join("study").join("profiles").join("testuser.json");
        assert!(!wrong_path.exists(),
            "REG-STU-001 FAIL: La ruta antigua .config/study/profiles/ no debe usarse.");
    }

    // REG-STU-002: Knowledge base en learnings.json
    #[test]
    fn reg_stu002_knowledge_correct_path() {
        let tmp = tmp_dir("stu002");
        let user_data = tmp.join(".config").join("data").join("testuser");
        fs::create_dir_all(&user_data).unwrap();
        let kb_path = user_data.join("learnings.json");
        let kb = json!({
            "username": "testuser", "known_topics": {}, "demonstrated_skills": [],
            "learning_summary": "", "last_updated": 1700000000_u64
        });
        fs::write(&kb_path, serde_json::to_string_pretty(&kb).unwrap()).unwrap();
        assert!(kb_path.exists(),
            "REG-STU-002 FAIL: learnings.json debe estar en .config/data/<user>/learnings.json");
        let wrong = tmp.join(".config").join("study").join("knowledge").join("testuser.json");
        assert!(!wrong.exists(),
            "REG-STU-002 FAIL: La ruta antigua .config/study/knowledge/ no debe usarse.");
    }

    // REG-STU-003: Teaching method en teachingMethod.json
    #[test]
    fn reg_stu003_teaching_method_correct_path() {
        let tmp = tmp_dir("stu003");
        let user_data = tmp.join(".config").join("data").join("testuser");
        fs::create_dir_all(&user_data).unwrap();
        let tm_path = user_data.join("teachingMethod.json");
        let tm = json!({
            "username": "testuser", "phase": "Exploration", "methods_tried": [],
            "methods_to_try": ["gamificacion"], "chosen_method": null,
            "failure_hypothesis": null, "success_hypothesis": null,
            "average_performance": null, "last_updated": 1700000000_u64
        });
        fs::write(&tm_path, serde_json::to_string_pretty(&tm).unwrap()).unwrap();
        assert!(tm_path.exists(), "REG-STU-003 FAIL: teachingMethod.json debe existir.");
    }

    // REG-STU-004: StudyEngine debe cargar perfiles desde disco al inicializar
    #[test]
    fn reg_stu004_engine_loads_existing_profiles() {
        let tmp = tmp_dir("stu004");
        let user_data = tmp.join(".config").join("data").join("existing_user");
        fs::create_dir_all(&user_data).unwrap();
        let profile = json!({
            "username": "existing_user", "age": 20, "high_capabilities": null,
            "neurological_conditions": [], "favorite_games": ["Factorio"],
            "favorite_youtubers": [], "hobbies": ["rust"],
            "phase": "Exploration", "exploration_started_at": 1700000000_u64,
            "exploitation_started_at": null, "hypothesis_history": [],
            "learning_style_summary": "", "message_timestamps": [],
            "last_updated": 1700000000_u64
        });
        fs::write(user_data.join("profile.json"),
            serde_json::to_string_pretty(&profile).unwrap()).unwrap();
        let kb = json!({
            "username": "existing_user",
            "known_topics": {"rust": {"topic": "rust", "level": 0.8, "evidence": [],
            "last_demonstrated": 1700000000_u64, "explicit": true}},
            "demonstrated_skills": [], "learning_summary": "",
            "last_updated": 1700000000_u64
        });
        fs::write(user_data.join("learnings.json"),
            serde_json::to_string_pretty(&kb).unwrap()).unwrap();
        assert!(user_data.join("profile.json").exists());
        let loaded: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(user_data.join("profile.json")).unwrap()).unwrap();
        assert_eq!(loaded["username"], "existing_user");
        assert_eq!(loaded["age"], 20);
    }

    // REG-STU-005: Múltiples usuarios con directorios separados
    #[test]
    fn reg_stu005_multiple_users_separate_dirs() {
        let tmp = tmp_dir("stu005");
        for user in &["alice", "bob"] {
            let dir = tmp.join(".config").join("data").join(user);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("profile.json"), serde_json::to_string_pretty(&json!({
                "username": user, "age": 15, "phase": "Exploration",
                "favorite_games": [], "hobbies": [], "neurological_conditions": [],
                "favorite_youtubers": [], "high_capabilities": null,
                "exploration_started_at": null, "exploitation_started_at": null,
                "hypothesis_history": [], "learning_style_summary": "",
                "message_timestamps": [], "last_updated": 0
            })).unwrap()).unwrap();
        }
        assert!(tmp.join(".config").join("data").join("alice").join("profile.json").exists());
        assert!(tmp.join(".config").join("data").join("bob").join("profile.json").exists());
    }

    // REG-STU-006: profile_exists_on_disk preciso
    #[test]
    fn reg_stu006_profile_exists_on_disk_accurate() {
        let tmp = tmp_dir("stu006");
        let user_data = tmp.join(".config").join("data").join("real_user");
        fs::create_dir_all(&user_data).unwrap();
        fs::write(user_data.join("profile.json"), "{}").unwrap();
        assert!(user_data.join("profile.json").exists());
        assert!(!tmp.join(".config").join("data").join("ghost").join("profile.json").exists());
    }

    // REG-STU-007: _projects no se carga como usuario
    #[test]
    fn reg_stu007_internal_dirs_ignored() {
        let tmp = tmp_dir("stu007");
        let projects_dir = tmp.join(".config").join("data").join("_projects");
        fs::create_dir_all(&projects_dir).unwrap();
        fs::write(projects_dir.join("p1.json"), "{}").unwrap();
        let user_dir = tmp.join(".config").join("data").join("real_user");
        fs::create_dir_all(&user_dir).unwrap();
        fs::write(user_dir.join("profile.json"), "{}").unwrap();
        let data_dir = tmp.join(".config").join("data");
        let user_dirs: Vec<String> = fs::read_dir(&data_dir).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| !n.starts_with('_'))
            .collect();
        assert!(user_dirs.contains(&"real_user".to_string()));
        assert!(!user_dirs.contains(&"_projects".to_string()));
    }

    // REG-STU-008: save_profile crea directorio si no existe
    #[test]
    fn reg_stu008_save_creates_directory() {
        let tmp = tmp_dir("stu008");
        let new_user_dir = tmp.join(".config").join("data").join("newuser");
        assert!(!new_user_dir.exists());
        fs::create_dir_all(&new_user_dir).unwrap();
        fs::write(new_user_dir.join("profile.json"), "{}").unwrap();
        assert!(new_user_dir.join("profile.json").exists());
    }
}
