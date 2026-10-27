//! Cross-platform application core for CineSched.
//!
//! The Rust migration begins with a JSON-compatible scheduling core and a
//! working desktop shell. Existing project files remain readable, while
//! unmodeled properties are retained when a project is loaded and saved.

pub mod model;
pub mod parsing;
pub mod reports;
pub mod script_import;

use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::model::Project;

/// Errors returned by project file operations.
#[derive(Debug, Error)]
pub enum ProjectFileError {
    /// The project file could not be read from disk.
    #[error("could not read project file: {0}")]
    Read(#[from] std::io::Error),
    /// The project file did not contain valid CineSched JSON.
    #[error("could not parse project JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// Loads a project from a path without changing its stored content.
///
/// The returned project keeps unknown JSON properties so newer project
/// features are not silently discarded by this migration stage.
pub fn load_project(path: impl AsRef<Path>) -> Result<Project, ProjectFileError> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Writes a project as pretty-printed JSON to the requested path.
///
/// The destination is written directly so saving an existing file works the
/// same way on Windows and macOS.
pub fn save_project(path: impl AsRef<Path>, project: &Project) -> Result<(), ProjectFileError> {
    let path = path.as_ref();
    let bytes = serde_json::to_vec_pretty(project)?;
    fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use crate::model::{Project, Scene, ShootDay};
    use crate::{load_project, save_project};

    /// Confirms old-style project dates and incomplete records can be read.
    #[test]
    fn reads_legacy_project_shape_and_retains_unknown_fields() {
        let json = r#"{
            "allScenes": [{
                "id": "00000000-0000-0000-0000-000000000001",
                "title": "INT. STAGE - DAY",
                "duration": 8,
                "estimatedTime": 120,
                "dayNightType": "DAY",
                "cast": "Alex, Sam",
                "legacyTag": "kept"
            }],
            "shootDays": [],
            "legacyProjectValue": 42
        }"#;

        let project: Project = serde_json::from_str(json).expect("legacy project parses");

        assert_eq!(project.project_title, "Untitled Movie");
        assert_eq!(project.all_scenes[0].cast, ["Alex", "Sam"]);
        assert_eq!(
            project.all_scenes[0].extra["legacyTag"],
            serde_json::json!("kept")
        );
        assert_eq!(project.extra["legacyProjectValue"], serde_json::json!(42));
    }

    /// Confirms Swift-compatible UTC date strings remain parseable and writable.
    #[test]
    fn swift_dates_round_trip_with_numeric_utc_offset() {
        let project = Project {
            created_date: Utc
                .with_ymd_and_hms(2025, 1, 2, 3, 4, 5)
                .single()
                .expect("test timestamp is valid"),
            ..Project::default()
        };

        let encoded = serde_json::to_string(&project).expect("project serializes");
        assert!(encoded.contains("2025-01-02T03:04:05+0000"));
        let decoded: Project = serde_json::from_str(&encoded).expect("project deserializes");
        assert_eq!(decoded.created_date, project.created_date);
    }

    /// Confirms scene movement preserves stable scene identifiers and day data.
    #[test]
    fn scheduling_moves_scene_from_boneyard_to_a_day() {
        let scene = Scene::new("EXT. PARK - DAY");
        let scene_id = scene.id;
        let day = ShootDay::new(Utc::now());
        let mut project = Project {
            all_scenes: vec![scene],
            shoot_days: vec![day],
            ..Project::default()
        };

        assert!(project.schedule_scene(scene_id, project.shoot_days[0].id));
        assert!(project.all_scenes.is_empty());
        assert_eq!(project.shoot_days[0].scenes[0].id, scene_id);
    }

    /// Confirms saving and reloading preserves project data through filesystem I/O.
    #[test]
    fn project_file_round_trip() {
        let path = std::env::temp_dir().join(format!("cinesched-{}.json", uuid::Uuid::new_v4()));
        let project = Project {
            project_title: "Round trip".to_owned(),
            all_scenes: vec![Scene::new("INT. STAGE - NIGHT")],
            ..Project::default()
        };

        save_project(&path, &project).expect("project saves");
        let loaded_result = load_project(&path);
        std::fs::remove_file(&path).expect("test file is removed");
        let loaded = loaded_result.expect("project loads");

        assert_eq!(loaded.project_title, project.project_title);
        assert_eq!(loaded.all_scenes[0].title, project.all_scenes[0].title);
    }
}
