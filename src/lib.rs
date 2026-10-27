//! Cross-platform application core for CineSched.
//!
//! The Rust migration begins with a JSON-compatible scheduling core and a
//! working desktop shell. Existing project files remain readable, while
//! unmodeled properties are retained when a project is loaded and saved.

pub mod localization;
pub mod model;
pub mod parsing;
pub mod reports;
pub mod script_import;
mod vector_pdf;

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

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

/// Errors encountered while saving or recovering an autosaved project.
#[derive(Debug, Error)]
pub enum AutosaveError {
    /// The project could not be encoded as JSON.
    #[error("could not serialize autosave: {0}")]
    Json(#[from] serde_json::Error),
    /// The autosave could not be read or safely written.
    #[error("could not access autosave: {0}")]
    Io(#[from] std::io::Error),
    /// Autosave files were present, but none contained a valid project.
    #[error("no valid autosave copy could be recovered")]
    NoValidCopy,
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

/// Writes a recoverable project snapshot using alternating autosave slots.
///
/// The older slot is retained while the new JSON is flushed to a temporary file.
/// A crash during replacement therefore leaves at least one previous snapshot.
pub fn save_autosave(base_path: impl AsRef<Path>, project: &Project) -> Result<(), AutosaveError> {
    let base_path = base_path.as_ref();
    let slots = autosave_slots(base_path);
    let newest_slot = slots
        .iter()
        .filter_map(|path| {
            fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
                .map(|modified| (path, modified))
        })
        .max_by_key(|(_, modified)| *modified)
        .map(|(path, _)| path);
    let destination = slots
        .iter()
        .find(|path| Some(*path) != newest_slot)
        .unwrap_or(&slots[0]);
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let temporary_path = destination.with_extension(format!("tmp-{}", std::process::id()));
    let bytes = serde_json::to_vec_pretty(project)?;
    let mut temporary_file = File::create(&temporary_path)?;
    temporary_file.write_all(&bytes)?;
    temporary_file.sync_all()?;
    drop(temporary_file);

    if destination.exists() {
        fs::remove_file(destination)?;
    }
    if let Err(error) = fs::rename(&temporary_path, destination) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error.into());
    }
    Ok(())
}

/// Loads the newest valid autosave copy, falling back to the older slot if needed.
///
/// Returns `Ok(None)` when no autosave files exist. If files do exist but both are
/// invalid, returns [`AutosaveError::NoValidCopy`] rather than hiding corruption.
pub fn load_latest_autosave(base_path: impl AsRef<Path>) -> Result<Option<Project>, AutosaveError> {
    let slots = autosave_slots(base_path.as_ref());
    let mut existing = Vec::new();
    for path in slots {
        match fs::metadata(&path) {
            Ok(metadata) => existing.push((path, metadata.modified()?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    if existing.is_empty() {
        return Ok(None);
    }

    existing.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));
    for (path, _) in &existing {
        let bytes = fs::read(path)?;
        if let Ok(project) = serde_json::from_slice(&bytes) {
            return Ok(Some(project));
        }
    }
    Err(AutosaveError::NoValidCopy)
}

fn autosave_slots(base_path: &Path) -> [PathBuf; 2] {
    [
        base_path.with_file_name("autosave-0.json"),
        base_path.with_file_name("autosave-1.json"),
    ]
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::{TimeZone, Utc};

    use crate::model::{Project, Scene, ShootDay};
    use crate::{load_latest_autosave, load_project, save_autosave, save_project, AutosaveError};

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

    /// Confirms autosave alternates slots and loads the newest valid project state.
    #[test]
    fn autosave_round_trip_and_corrupt_latest_slot_recovery() {
        let directory =
            std::env::temp_dir().join(format!("cinesched-autosave-{}", uuid::Uuid::new_v4()));
        let base_path = directory.join("autosave.json");
        let first = Project {
            project_title: "First snapshot".to_owned(),
            ..Project::default()
        };
        let latest = Project {
            project_title: "Latest snapshot".to_owned(),
            ..Project::default()
        };

        assert!(load_latest_autosave(&base_path)
            .expect("missing autosaves are not errors")
            .is_none());
        save_autosave(&base_path, &first).expect("first autosave succeeds");
        std::thread::sleep(std::time::Duration::from_millis(20));
        save_autosave(&base_path, &latest).expect("second autosave succeeds");

        assert_eq!(
            load_latest_autosave(&base_path)
                .expect("latest autosave loads")
                .expect("autosave exists")
                .project_title,
            "Latest snapshot"
        );
        fs::write(directory.join("autosave-1.json"), b"invalid json")
            .expect("newest slot is corrupted");
        assert_eq!(
            load_latest_autosave(&base_path)
                .expect("older autosave is used")
                .expect("backup autosave exists")
                .project_title,
            "First snapshot"
        );
        fs::remove_dir_all(directory).expect("autosave test directory is removed");
    }

    /// Confirms recovery reports corrupt autosave data instead of treating it as missing.
    #[test]
    fn autosave_recovery_reports_when_all_copies_are_invalid() {
        let directory =
            std::env::temp_dir().join(format!("cinesched-autosave-{}", uuid::Uuid::new_v4()));
        let base_path = directory.join("autosave.json");
        fs::create_dir_all(&directory).expect("test directory is created");
        fs::write(directory.join("autosave-0.json"), b"bad")
            .expect("first bad autosave is written");
        fs::write(directory.join("autosave-1.json"), b"also bad")
            .expect("second bad autosave is written");

        assert!(matches!(
            load_latest_autosave(base_path),
            Err(AutosaveError::NoValidCopy)
        ));
        fs::remove_dir_all(directory).expect("autosave test directory is removed");
    }
}
