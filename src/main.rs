//! Native egui desktop entry point for CineSched.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use eframe::egui;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use cinesched::localization::{
    format_long_date, format_month, translate, translate_status, Language,
};

use cinesched::model::{
    CastCallEntry, CastMember, CrewCallEntry, CrewMember, DateRange, Location, ProductionInfo,
    Project, Scene, ShootDay,
};
use cinesched::reports::{
    export_breakdowns_pdf_localized, export_call_sheet_pdf_localized,
    export_days_out_of_days_pdf_localized, export_month_calendar_pdf_localized,
    export_schedule_pdf_localized, export_stripboard_pdf_localized, export_timeline_pdf_localized,
};
use cinesched::script_import::import_script;
use cinesched::{load_latest_autosave, load_project, save_autosave, save_project};

/// Starts the cross-platform desktop application.
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "CineSched",
        options,
        Box::new(|creation_context| {
            let app: CineSchedApp = creation_context
                .storage
                .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
                .unwrap_or_default();
            let mut app = app;
            app.initialize_autosave();
            Ok(Box::new(app))
        }),
    )
}

/// Application state and the first Rust scheduling interface.
#[derive(Serialize, Deserialize)]
struct CineSchedApp {
    project: Project,
    file_path: Option<std::path::PathBuf>,
    #[serde(default)]
    recent_projects: Vec<std::path::PathBuf>,
    #[serde(skip)]
    autosave_path: Option<PathBuf>,
    #[serde(skip)]
    autosave_dirty: bool,
    #[serde(skip)]
    autosave_countdown: f32,
    #[serde(skip)]
    autosave_recovery_available: bool,
    #[serde(skip)]
    history_action_this_frame: bool,
    new_scene_title: String,
    status: String,
    month: NaiveDate,
    stripboard: bool,
    selected_scene: Option<Uuid>,
    selected_call_sheet: Option<Uuid>,
    show_production: bool,
    show_conflicts: bool,
    show_lock_report: bool,
    include_hold: bool,
    event_title: String,
    event_time: String,
    event_date: NaiveDate,
    banner_type: String,
    meal_kind: String,
    search_query: String,
    boneyard_sort: String,
    new_cast_actor: String,
    new_cast_character: String,
    new_crew_name: String,
    new_crew_role: String,
    availability_start: String,
    availability_end: String,
    #[serde(skip)]
    undo_stack: Vec<Project>,
    #[serde(skip)]
    redo_stack: Vec<Project>,
    #[serde(skip)]
    selected_boneyard_scenes: BTreeSet<Uuid>,
    #[serde(skip)]
    bulk_schedule_target: Option<Uuid>,
    #[serde(skip)]
    selected_day_for_move: Option<Uuid>,
    schedule_targets: BTreeMap<Uuid, Uuid>,
    dark_mode: bool,
    show_cast: bool,
    show_estimated_time: bool,
    #[serde(default)]
    language: Language,
}

/// Display-only values for a shooting day in the stripboard.
struct StripboardDay {
    day_id: Uuid,
    date: NaiveDate,
    scenes: Vec<(Uuid, String, bool)>,
    pages: i32,
    minutes: i32,
}

impl Default for CineSchedApp {
    fn default() -> Self {
        let today = Local::now().date_naive();
        let month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
            .expect("the current year and month are valid");
        Self {
            project: Project::default(),
            file_path: None,
            recent_projects: Vec::new(),
            autosave_path: None,
            autosave_dirty: false,
            autosave_countdown: 0.0,
            autosave_recovery_available: false,
            history_action_this_frame: false,
            new_scene_title: String::new(),
            status: String::new(),
            month,
            stripboard: false,
            selected_scene: None,
            selected_call_sheet: None,
            show_production: false,
            show_conflicts: false,
            show_lock_report: false,
            include_hold: true,
            event_title: String::new(),
            event_time: String::new(),
            event_date: today,
            banner_type: "Notice".to_owned(),
            meal_kind: "Lunch".to_owned(),
            search_query: String::new(),
            boneyard_sort: "Show Order".to_owned(),
            new_cast_actor: String::new(),
            new_cast_character: String::new(),
            new_crew_name: String::new(),
            new_crew_role: String::new(),
            availability_start: today.format("%Y-%m-%d").to_string(),
            availability_end: today.format("%Y-%m-%d").to_string(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            selected_boneyard_scenes: BTreeSet::new(),
            bulk_schedule_target: None,
            selected_day_for_move: None,
            schedule_targets: BTreeMap::new(),
            dark_mode: false,
            show_cast: false,
            show_estimated_time: false,
            language: Language::default(),
        }
    }
}

impl eframe::App for CineSchedApp {
    /// Draws the project controls, unscheduled scenes, and shooting calendar.
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        let project_before_frame = self.project.clone();
        self.history_action_this_frame = false;
        context.set_visuals(if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        egui::TopBottomPanel::top("main_menu").show(context, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(translate(self.language, "New")).clicked() {
                    self.record_undo();
                    self.project = Project::default();
                    self.file_path = None;
                    self.status = translate(self.language, "Created a new project");
                }
                if ui.button(translate(self.language, "Open...")).clicked() {
                    self.open_project();
                }
                ui.menu_button(translate(self.language, "Open Recent"), |ui| {
                    if self.recent_projects.is_empty() {
                        ui.label(translate(self.language, "No recent projects"));
                    } else {
                        for path in self.recent_projects.clone() {
                            let label = path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or_else(|| path.to_str().unwrap_or("Project"));
                            let response = ui.button(label);
                            if response.on_hover_text(path.display().to_string()).clicked() {
                                self.open_project_path(path);
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui
                            .button(translate(self.language, "Clear Recent Projects"))
                            .clicked()
                        {
                            self.recent_projects.clear();
                            ui.close_menu();
                        }
                    }
                });
                if ui.button(translate(self.language, "Save")).clicked() {
                    self.save_current_project();
                }
                if ui.button(translate(self.language, "Save As...")).clicked() {
                    self.save_project_as();
                }
                if ui
                    .button(translate(self.language, "Import Script..."))
                    .clicked()
                {
                    self.import_script_file();
                }
                if ui
                    .button(translate(
                        self.language,
                        if self.stripboard {
                            "Calendar"
                        } else {
                            "Stripboard"
                        },
                    ))
                    .clicked()
                {
                    self.stripboard = !self.stripboard;
                }
                if ui
                    .button(translate(self.language, "Production Setup"))
                    .clicked()
                {
                    self.show_production = true;
                }
                ui.checkbox(&mut self.dark_mode, translate(self.language, "Dark Mode"));
                ui.checkbox(&mut self.show_cast, translate(self.language, "Show Cast"));
                ui.checkbox(
                    &mut self.show_estimated_time,
                    translate(self.language, "Show Estimated Time"),
                );
                ui.checkbox(
                    &mut self.include_hold,
                    translate(self.language, "Include Holds in DOOD"),
                );
                ui.menu_button(translate(self.language, "Language"), |ui| {
                    for language in [Language::English, Language::Spanish] {
                        if ui
                            .selectable_label(self.language == language, language.display_name())
                            .clicked()
                        {
                            self.language = language;
                            ui.close_menu();
                        }
                    }
                });
                if ui
                    .button(translate(self.language, "Scan Conflicts"))
                    .clicked()
                {
                    self.show_conflicts = true;
                }
                if ui
                    .button(translate(self.language, "Check Duplicate Numbers"))
                    .clicked()
                {
                    let duplicates = self.project.duplicate_scene_number_ids();
                    self.set_status(if duplicates.is_empty() {
                        translate(self.language, "No duplicate scene numbers found")
                    } else {
                        format!("{} duplicate scene numbers found", duplicates.len())
                    });
                }
                if ui
                    .button(translate(self.language, "Lock Schedule"))
                    .clicked()
                {
                    self.record_undo();
                    self.project.lock_schedule(Utc::now());
                    self.status = translate(self.language, "Schedule locked");
                }
                if ui.button(translate(self.language, "Unlock")).clicked() {
                    self.record_undo();
                    self.project.unlock_schedule();
                    self.status = translate(self.language, "Schedule unlocked");
                }
                if ui.button(translate(self.language, "Lock Report")).clicked() {
                    self.show_lock_report = true;
                }
                if ui.button(translate(self.language, "Undo")).clicked() {
                    self.undo();
                }
                if ui.button(translate(self.language, "Redo")).clicked() {
                    self.redo();
                }
                if ui
                    .button(translate(self.language, "Export PDF..."))
                    .clicked()
                {
                    self.export_schedule();
                }
                if ui
                    .button(translate(self.language, "Breakdowns PDF..."))
                    .clicked()
                {
                    self.export_breakdowns();
                }
                if ui.button(translate(self.language, "DOOD PDF...")).clicked() {
                    self.export_dood();
                }
                if ui
                    .button(translate(self.language, "Month PDF..."))
                    .clicked()
                {
                    self.export_month();
                }
                if ui
                    .button(translate(self.language, "Stripboard PDF..."))
                    .clicked()
                {
                    self.export_stripboard();
                }
                if ui
                    .button(translate(self.language, "One-Line PDF..."))
                    .clicked()
                {
                    self.export_timeline();
                }
                ui.separator();
                ui.text_edit_singleline(&mut self.project.project_title);
            });
        });

        egui::TopBottomPanel::bottom("status").show(context, |ui| {
            ui.label(&self.status);
        });

        egui::CentralPanel::default().show(context, |ui| {
            ui.heading("CineSched");
            if self.autosave_recovery_available {
                ui.horizontal(|ui| {
                    ui.label(translate(
                        self.language,
                        "An autosaved project is available to recover.",
                    ));
                    if ui.button(translate(self.language, "Recover")).clicked() {
                        self.recover_autosave();
                    }
                    if ui.button(translate(self.language, "Discard")).clicked() {
                        self.discard_autosave();
                    }
                });
            }
            ui.label(format!(
                "{} unscheduled scenes | {} calendar days",
                self.project.all_scenes.len(),
                self.project.shoot_days.len()
            ));

            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.new_scene_title);
                if ui.button(translate(self.language, "Add Scene")).clicked()
                    && !self.new_scene_title.trim().is_empty()
                {
                    self.record_undo();
                    self.project
                        .all_scenes
                        .push(Scene::new(self.new_scene_title.trim()));
                    self.new_scene_title.clear();
                }
                if ui
                    .button(translate(self.language, "Add 30-Day Calendar"))
                    .clicked()
                {
                    self.add_calendar();
                }
            });

            ui.separator();
            egui::SidePanel::left("boneyard")
                .resizable(true)
                .default_width(260.0)
                .show_inside(ui, |ui| self.draw_boneyard(ui));
            egui::ScrollArea::vertical().show(ui, |ui| self.draw_schedule(ui));
        });

        self.draw_call_sheet(context);
        self.draw_production_setup(context);
        self.draw_conflict_report(context);
        self.draw_schedule_lock_report(context);
        self.draw_scene_editor(context);
        self.finish_project_frame(project_before_frame, context);
    }

    /// Persists project state and appearance preferences across application launches.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

impl CineSchedApp {
    /// Stores a user-facing message after translating its stable text and template.
    fn set_status(&mut self, message: impl AsRef<str>) {
        self.status = translate_status(self.language, message.as_ref());
    }

    /// Locates the app-data autosave and makes a valid previous copy recoverable.
    fn initialize_autosave(&mut self) {
        self.autosave_path =
            eframe::storage_dir("CineSched").map(|directory| directory.join("autosave.json"));
        let Some(path) = self.autosave_path.as_ref() else {
            self.set_status("Autosave is unavailable on this platform");
            return;
        };
        match load_latest_autosave(path) {
            Ok(project) => self.autosave_recovery_available = project.is_some(),
            Err(error) => {
                self.set_status(format!("Autosave recovery check failed: {error}"));
                self.autosave_recovery_available = true;
            }
        }
    }

    /// Applies the latest valid autosave and schedules it for another safe write.
    fn recover_autosave(&mut self) {
        let Some(path) = self.autosave_path.as_ref() else {
            self.set_status("Autosave recovery is unavailable");
            return;
        };
        match load_latest_autosave(path) {
            Ok(Some(project)) => {
                self.project = project;
                self.file_path = None;
                self.autosave_recovery_available = false;
                self.autosave_dirty = true;
                self.autosave_countdown = 1.5;
                self.set_status("Recovered the latest autosaved project");
            }
            Ok(None) => {
                self.autosave_recovery_available = false;
                self.set_status("No autosaved project is available");
            }
            Err(error) => {
                self.set_status(format!("Autosave recovery failed: {error}"));
            }
        }
    }

    /// Discards both autosave slots after the user declines recovery.
    fn discard_autosave(&mut self) {
        match self.clear_autosave_files() {
            Ok(()) => {
                self.autosave_recovery_available = false;
                self.autosave_dirty = false;
                self.set_status("Discarded the recovered project snapshot");
            }
            Err(error) => self.set_status(format!("Could not discard autosave: {error}")),
        }
    }

    /// Removes autosave snapshots after explicit save or loading another project.
    fn clear_autosave_files(&self) -> std::io::Result<()> {
        let Some(base_path) = self.autosave_path.as_ref() else {
            return Ok(());
        };
        for path in [
            base_path.with_file_name("autosave-0.json"),
            base_path.with_file_name("autosave-1.json"),
            base_path.with_file_name(format!("autosave-0.tmp-{}", std::process::id())),
            base_path.with_file_name(format!("autosave-1.tmp-{}", std::process::id())),
        ] {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// Captures every project change, updates undo history, and debounces autosave.
    fn finish_project_frame(&mut self, before: Project, context: &egui::Context) {
        let changed = match (
            serde_json::to_value(&before),
            serde_json::to_value(&self.project),
        ) {
            (Ok(before), Ok(after)) => before != after,
            (Err(error), _) | (_, Err(error)) => {
                self.set_status(format!("Could not compare project state: {error}"));
                false
            }
        };
        if changed {
            if !self.history_action_this_frame {
                let already_recorded = self
                    .undo_stack
                    .last()
                    .and_then(|snapshot| serde_json::to_value(snapshot).ok())
                    .is_some_and(|snapshot| {
                        serde_json::to_value(&before).is_ok_and(|previous| snapshot == previous)
                    });
                if !already_recorded {
                    self.undo_stack.push(before);
                    self.undo_stack.truncate(50);
                    self.redo_stack.clear();
                }
            }
            self.autosave_dirty = true;
            self.autosave_countdown = 1.5;
        }

        if !self.autosave_dirty {
            return;
        }
        context.request_repaint_after(std::time::Duration::from_millis(250));
        self.autosave_countdown -= context.input(|input| input.stable_dt);
        if self.autosave_countdown > 0.0 {
            return;
        }

        let Some(path) = self.autosave_path.as_ref() else {
            self.set_status("Project changed but autosave path is unavailable");
            self.autosave_countdown = 10.0;
            return;
        };
        match save_autosave(path, &self.project) {
            Ok(()) => {
                self.autosave_dirty = false;
            }
            Err(error) => {
                self.set_status(format!("Autosave failed: {error}"));
                self.autosave_countdown = 10.0;
            }
        }
    }

    /// Opens and decodes a project selected with the native file picker.
    fn open_project(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("CineSched project", &["json"])
            .pick_file()
        else {
            return;
        };
        self.open_project_path(path);
    }

    /// Loads a project path and records it for the Open Recent menu.
    fn open_project_path(&mut self, path: std::path::PathBuf) {
        match load_project(&path) {
            Ok(project) => {
                self.project = project;
                self.file_path = Some(path.clone());
                record_recent_project(&mut self.recent_projects, path.clone());
                self.autosave_dirty = false;
                self.autosave_recovery_available = false;
                self.set_status(match self.clear_autosave_files() {
                    Ok(()) => format!("Opened {}", path.display()),
                    Err(error) => format!(
                        "Opened {}, but could not clear old autosave: {error}",
                        path.display()
                    ),
                });
            }
            Err(error) => self.set_status(format!("Open failed: {error}")),
        }
    }

    /// Saves the current project to its previously selected path.
    fn save_current_project(&mut self) {
        if let Some(path) = self.file_path.clone() {
            match save_project(&path, &self.project) {
                Ok(()) => {
                    record_recent_project(&mut self.recent_projects, path.clone());
                    self.autosave_dirty = false;
                    self.set_status(match self.clear_autosave_files() {
                        Ok(()) => format!("Saved {}", path.display()),
                        Err(error) => format!(
                            "Saved {}, but could not clear autosave: {error}",
                            path.display()
                        ),
                    });
                }
                Err(error) => self.set_status(format!("Save failed: {error}")),
            }
        } else {
            self.save_project_as();
        }
    }

    /// Prompts for a project path and saves the current JSON document.
    fn save_project_as(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("CineSched project", &["json"])
            .set_file_name("CineSched.json")
            .save_file()
        else {
            return;
        };
        match save_project(&path, &self.project) {
            Ok(()) => {
                self.file_path = Some(path.clone());
                record_recent_project(&mut self.recent_projects, path.clone());
                self.autosave_dirty = false;
                self.set_status(match self.clear_autosave_files() {
                    Ok(()) => format!("Saved {}", path.display()),
                    Err(error) => format!(
                        "Saved {}, but could not clear autosave: {error}",
                        path.display()
                    ),
                });
            }
            Err(error) => self.set_status(format!("Save failed: {error}")),
        }
    }

    /// Imports a supported screenplay and adds its scenes to the Boneyard.
    fn import_script_file(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter(
                "Screenplays",
                &["fdx", "fountain", "fnt", "highland", "txt"],
            )
            .pick_file()
        else {
            return;
        };
        match import_script(&path) {
            Ok(scenes) => {
                self.record_undo();
                let count = scenes.len();
                self.project.all_scenes.extend(scenes);
                self.set_status(format!("Imported {count} scenes from {}", path.display()));
            }
            Err(error) => self.set_status(format!("Script import failed: {error}")),
        }
    }

    /// Stores the last project snapshot for the undo command.
    fn record_undo(&mut self) {
        self.undo_stack.push(self.project.clone());
        self.undo_stack.truncate(50);
        self.redo_stack.clear();
    }

    /// Restores the preceding project state.
    fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack
                .push(std::mem::replace(&mut self.project, previous));
            self.set_status("Undid the last change");
            self.history_action_this_frame = true;
        }
    }

    /// Reapplies the most recently undone project state.
    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack
                .push(std::mem::replace(&mut self.project, next));
            self.set_status("Redid the last change");
            self.history_action_this_frame = true;
        }
    }

    /// Adds a banner or scene event to a calendar date, creating the day if needed.
    fn add_scene_to_date(&mut self, scene: Scene, date: NaiveDate) {
        let Some(timestamp) = local_day(date) else {
            self.set_status("Could not interpret the selected calendar date");
            return;
        };
        self.record_undo();
        let day_index = self
            .project
            .shoot_days
            .iter()
            .position(|day| day.date.date_naive() == date)
            .unwrap_or_else(|| {
                self.project.shoot_days.push(ShootDay::new(timestamp));
                self.project.shoot_days.len() - 1
            });
        self.project.shoot_days[day_index].scenes.push(scene);
        self.project.shoot_days.sort_by_key(|day| day.date);
    }

    /// Exports the whole production schedule to a PDF selected by the user.
    fn export_schedule(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("Shooting Schedule.pdf")
            .save_file()
        else {
            return;
        };
        self.set_status(
            match export_schedule_pdf_localized(&self.project, self.language, &path) {
                Ok(()) => format!("Exported {}", path.display()),
                Err(error) => format!("Schedule export failed: {error}"),
            },
        );
    }

    /// Edits call-sheet milestones, weather, locations, and notes for one day.
    fn draw_call_sheet(&mut self, context: &egui::Context) {
        let Some(day_id) = self.selected_call_sheet else {
            return;
        };
        let project_before_edit = self.project.clone();
        let mut project_changed = false;
        let mut open = true;
        let mut export_requested = false;
        let production_cast = self
            .project
            .production_info
            .as_ref()
            .map(|production| production.cast_list.clone())
            .unwrap_or_default();
        let production_crew = self
            .project
            .production_info
            .as_ref()
            .map(|production| production.crew.clone())
            .unwrap_or_default();
        let location_roster = self
            .project
            .production_info
            .as_ref()
            .map(|production| production.location_roster.clone())
            .unwrap_or_default();
        egui::Window::new(translate(self.language, "Call Sheet"))
            .open(&mut open)
            .default_width(800.0)
            .default_height(700.0)
            .show(context, |ui| {
                if let Some(day) = self
                    .project
                    .shoot_days
                    .iter_mut()
                    .find(|day| day.id == day_id)
                {
                    ui.heading(format_long_date(day.date.date_naive(), self.language));
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.collapsing(translate(self.language, "General and Schedule"), |ui| {
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "General Call"),
                                &mut day.call_sheet.general_call_time,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Estimated Workday Schedule"),
                                &mut day.call_sheet.work_day_schedule,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Quote of the Day"),
                                &mut day.call_sheet.quote_of_the_day,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Ready to Shoot"),
                                &mut day.call_sheet.ready_to_shoot_time,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Lunch"),
                                &mut day.call_sheet.lunch_time,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Snack"),
                                &mut day.call_sheet.snack_time,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Dinner"),
                                &mut day.call_sheet.dinner_time,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Estimated Wrap"),
                                &mut day.call_sheet.wrap_time,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Production Manager Contact"),
                                &mut day.call_sheet.prod_manager_contact,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Assistant Director Contact"),
                                &mut day.call_sheet.ad_contact,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Nearest Hospital / Emergency Facility"),
                                &mut day.call_sheet.nearest_hospital,
                            );
                        });
                        ui.collapsing(translate(self.language, "Weather and Sun Times"), |ui| {
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Temperature"),
                                &mut day.call_sheet.weather_temp,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Sky Condition"),
                                &mut day.call_sheet.weather_condition,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Precipitation and Wind"),
                                &mut day.call_sheet.weather_precip_wind,
                            );
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Sunrise / Sunset"),
                                &mut day.call_sheet.sun_times,
                            );
                        });
                        ui.collapsing(translate(self.language, "Locations and Basecamp"), |ui| {
                            project_changed |= labeled_text(
                                ui,
                                &translate(self.language, "Basecamp"),
                                &mut day.call_sheet.basecamp_location,
                            );
                            if ui
                                .button(translate(self.language, "Add Location"))
                                .clicked()
                            {
                                day.call_sheet.locations.push(Default::default());
                                project_changed = true;
                            }
                            ui.menu_button(
                                translate(self.language, "Add from Location Roster"),
                                |ui| {
                                    for location in &location_roster {
                                        if ui.button(&location.name).clicked() {
                                            if !day.call_sheet.locations.iter().any(|existing| {
                                                existing.name.eq_ignore_ascii_case(&location.name)
                                            }) {
                                                day.call_sheet.locations.push(location.clone());
                                                project_changed = true;
                                            }
                                            ui.close_menu();
                                        }
                                    }
                                },
                            );
                            if ui
                                .button(translate(self.language, "Populate from Scene Locations"))
                                .clicked()
                            {
                                project_changed |=
                                    populate_call_sheet_locations(day, &location_roster);
                            }
                            let mut remove_location = None;
                            for (index, location) in day.call_sheet.locations.iter_mut().enumerate()
                            {
                                ui.horizontal(|ui| {
                                    ui.label(format!("LOC {}", index + 1));
                                    project_changed |=
                                        ui.text_edit_singleline(&mut location.name).changed();
                                    project_changed |=
                                        ui.text_edit_singleline(&mut location.address).changed();
                                    if ui
                                        .small_button(translate(self.language, "Remove"))
                                        .clicked()
                                    {
                                        remove_location = Some(index);
                                    }
                                });
                            }
                            if let Some(index) = remove_location {
                                day.call_sheet.locations.remove(index);
                                project_changed = true;
                            }
                        });
                        ui.collapsing(translate(self.language, "Cast Calls"), |ui| {
                            if ui
                                .button(translate(
                                    self.language,
                                    "Populate / Update Cast from Scenes",
                                ))
                                .clicked()
                            {
                                project_changed |= populate_cast_calls(day, &production_cast);
                            }
                            if ui
                                .button(translate(self.language, "Add Cast Row"))
                                .clicked()
                            {
                                day.call_sheet
                                    .cast_call_entries
                                    .push(CastCallEntry::default());
                                project_changed = true;
                            }
                            let mut remove_cast = None;
                            for (index, entry) in
                                day.call_sheet.cast_call_entries.iter_mut().enumerate()
                            {
                                ui.group(|ui| {
                                    ui.label(format!(
                                        "{} {}",
                                        translate(self.language, "Cast member"),
                                        index + 1
                                    ));
                                    ui.horizontal_wrapped(|ui| {
                                        project_changed |= labeled_text(
                                            ui,
                                            &translate(self.language, "Character"),
                                            &mut entry.character_name,
                                        );
                                        project_changed |= labeled_text(
                                            ui,
                                            &translate(self.language, "Actor"),
                                            &mut entry.actor_name,
                                        );
                                        project_changed |= labeled_text(
                                            ui,
                                            &translate(self.language, "Scenes"),
                                            &mut entry.scene_numbers,
                                        );
                                        project_changed |=
                                            labeled_text(ui, "ECDT", &mut entry.ecdt);
                                    });
                                    ui.horizontal_wrapped(|ui| {
                                        project_changed |= labeled_text(
                                            ui,
                                            &translate(self.language, "Pickup"),
                                            &mut entry.pickup_time,
                                        );
                                        project_changed |= labeled_text(
                                            ui,
                                            &translate(self.language, "HMU / Wardrobe"),
                                            &mut entry.hmu_wardrobe_time,
                                        );
                                        project_changed |= labeled_text(
                                            ui,
                                            &translate(self.language, "On Set"),
                                            &mut entry.on_set_time,
                                        );
                                        project_changed |= labeled_text(
                                            ui,
                                            &translate(self.language, "Wrap"),
                                            &mut entry.wrap_time,
                                        );
                                        project_changed |= labeled_text(
                                            ui,
                                            &translate(self.language, "Location #"),
                                            &mut entry.location_index,
                                        );
                                        if ui
                                            .small_button(translate(self.language, "Remove"))
                                            .clicked()
                                        {
                                            remove_cast = Some(index);
                                        }
                                    });
                                });
                            }
                            if let Some(index) = remove_cast {
                                day.call_sheet.cast_call_entries.remove(index);
                                project_changed = true;
                            }
                        });
                        ui.collapsing(translate(self.language, "Crew Calls"), |ui| {
                            if ui
                                .button(translate(self.language, "Load Crew from Production Setup"))
                                .clicked()
                            {
                                let general_call_time = day.call_sheet.general_call_time.clone();
                                project_changed |= populate_crew_calls(
                                    &production_crew,
                                    &mut day.call_sheet.crew_call_entries,
                                    &general_call_time,
                                );
                            }
                            if ui
                                .button(translate(self.language, "Add Crew Row"))
                                .clicked()
                            {
                                day.call_sheet
                                    .crew_call_entries
                                    .push(CrewCallEntry::default());
                                project_changed = true;
                            }
                            let mut remove_crew = None;
                            for (index, entry) in
                                day.call_sheet.crew_call_entries.iter_mut().enumerate()
                            {
                                ui.horizontal_wrapped(|ui| {
                                    project_changed |= labeled_text(
                                        ui,
                                        &translate(self.language, "Role"),
                                        &mut entry.role,
                                    );
                                    project_changed |= labeled_text(
                                        ui,
                                        &translate(self.language, "Name"),
                                        &mut entry.name,
                                    );
                                    project_changed |= labeled_text(
                                        ui,
                                        &translate(self.language, "Call Time"),
                                        &mut entry.call_time,
                                    );
                                    project_changed |= labeled_text(
                                        ui,
                                        &translate(self.language, "Phone"),
                                        &mut entry.phone,
                                    );
                                    if ui
                                        .small_button(translate(self.language, "Remove"))
                                        .clicked()
                                    {
                                        remove_crew = Some(index);
                                    }
                                });
                            }
                            if let Some(index) = remove_crew {
                                day.call_sheet.crew_call_entries.remove(index);
                                project_changed = true;
                            }
                        });
                        ui.collapsing(translate(self.language, "Production Notes"), |ui| {
                            ui.label(translate(self.language, "Call sheet notes"));
                            project_changed |=
                                ui.text_edit_multiline(&mut day.call_sheet.notes).changed();
                            ui.label(translate(self.language, "Production notes (one per line)"));
                            let mut production_notes = day.call_sheet.production_notes.join("\n");
                            if ui.text_edit_multiline(&mut production_notes).changed() {
                                day.call_sheet.production_notes = production_notes
                                    .lines()
                                    .map(str::trim)
                                    .filter(|line| !line.is_empty())
                                    .map(str::to_owned)
                                    .collect();
                                project_changed = true;
                            }
                        });
                    });
                    if ui
                        .button(translate(self.language, "Export Call Sheet PDF..."))
                        .clicked()
                    {
                        export_requested = true;
                    }
                }
            });
        if project_changed {
            self.undo_stack.push(project_before_edit);
            self.undo_stack.truncate(50);
            self.redo_stack.clear();
        }
        if !open {
            self.selected_call_sheet = None;
        }
        if export_requested {
            if let Some(path) = FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name("Call Sheet.pdf")
                .save_file()
            {
                self.set_status(
                    match export_call_sheet_pdf_localized(
                        &self.project,
                        day_id,
                        self.language,
                        path,
                    ) {
                        Ok(()) => "Call sheet PDF exported".to_owned(),
                        Err(error) => format!("Call sheet export failed: {error}"),
                    },
                );
            }
        }
    }

    /// Edits company contacts, cast availability, crew defaults, and locations.
    fn draw_production_setup(&mut self, context: &egui::Context) {
        if !self.show_production {
            return;
        }
        let previous_project = self.project.clone();
        egui::Window::new(translate(self.language, "Production Setup"))
            .open(&mut self.show_production)
            .default_width(600.0)
            .show(context, |ui| {
                let production = self
                    .project
                    .production_info
                    .get_or_insert_with(ProductionInfo::default);
                ui.horizontal(|ui| {
                    ui.label(translate(self.language, "Company"));
                    ui.text_edit_singleline(&mut production.company_name);
                });
                ui.horizontal(|ui| {
                    ui.label(translate(self.language, "Director"));
                    ui.text_edit_singleline(&mut production.director_name);
                    ui.text_edit_singleline(&mut production.director_phone);
                });
                ui.horizontal(|ui| {
                    ui.label(translate(self.language, "Producer"));
                    ui.text_edit_singleline(&mut production.producer_name);
                    ui.text_edit_singleline(&mut production.producer_phone);
                });
                ui.horizontal(|ui| {
                    ui.label(translate(self.language, "1st AD"));
                    ui.text_edit_singleline(&mut production.ad_name);
                    ui.text_edit_singleline(&mut production.ad_phone);
                });
                ui.horizontal(|ui| {
                    ui.label(translate(self.language, "Company Contact"));
                    ui.text_edit_singleline(&mut production.contact_number);
                    ui.label(translate(self.language, "Lunch"));
                    ui.text_edit_singleline(&mut production.default_lunch_time);
                });

                ui.separator();
                ui.heading(translate(self.language, "Cast and Character Roster"));
                ui.horizontal(|ui| {
                    ui.label(translate(
                        self.language,
                        "New unavailability range (YYYY-MM-DD)",
                    ));
                    ui.text_edit_singleline(&mut self.availability_start);
                    ui.text_edit_singleline(&mut self.availability_end);
                });
                for member in &mut production.cast_list {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut member.actor_name);
                        ui.text_edit_singleline(&mut member.character_name);
                        if ui
                            .small_button(translate(self.language, "Add range"))
                            .clicked()
                        {
                            let start =
                                NaiveDate::parse_from_str(&self.availability_start, "%Y-%m-%d");
                            let end = NaiveDate::parse_from_str(&self.availability_end, "%Y-%m-%d");
                            match (start, end) {
                                (Ok(start), Ok(end)) => {
                                    if let (Some(start), Some(end)) =
                                        (local_day(start), local_day(end))
                                    {
                                        self.undo_stack.push(previous_project.clone());
                                        self.redo_stack.clear();
                                        member.unavailable_ranges.push(DateRange::new(start, end));
                                    }
                                }
                                _ => {
                                    self.status =
                                        translate(self.language, "Enter dates using YYYY-MM-DD");
                                }
                            }
                        }
                    });
                    for range in &mut member.unavailable_ranges {
                        ui.horizontal(|ui| {
                            ui.label(translate(self.language, "Unavailable"));
                            let mut start = range.start.format("%Y-%m-%d").to_string();
                            if ui.text_edit_singleline(&mut start).changed() {
                                if let Ok(date) = NaiveDate::parse_from_str(&start, "%Y-%m-%d") {
                                    if let Some(timestamp) = local_day(date) {
                                        range.start = timestamp;
                                    }
                                }
                            }
                            ui.label(translate(self.language, "through"));
                            let mut end = range.end.format("%Y-%m-%d").to_string();
                            if ui.text_edit_singleline(&mut end).changed() {
                                if let Ok(date) = NaiveDate::parse_from_str(&end, "%Y-%m-%d") {
                                    if let Some(timestamp) = local_day(date) {
                                        range.end = timestamp;
                                    }
                                }
                            }
                        });
                    }
                }
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.new_cast_actor);
                    ui.text_edit_singleline(&mut self.new_cast_character);
                    if ui
                        .button(translate(self.language, "Add Cast Member"))
                        .clicked()
                        && !self.new_cast_character.trim().is_empty()
                    {
                        self.undo_stack.push(previous_project.clone());
                        self.redo_stack.clear();
                        production.cast_list.push(CastMember {
                            actor_name: self.new_cast_actor.trim().to_owned(),
                            character_name: self.new_cast_character.trim().to_owned(),
                            ..Default::default()
                        });
                        self.new_cast_actor.clear();
                        self.new_cast_character.clear();
                    }
                });

                ui.separator();
                ui.heading(translate(self.language, "Crew Roster"));
                for crew in &mut production.crew {
                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut crew.is_daily_default,
                            translate(self.language, "Daily"),
                        );
                        ui.text_edit_singleline(&mut crew.name);
                        ui.text_edit_singleline(&mut crew.role);
                        ui.text_edit_singleline(&mut crew.phone);
                    });
                }
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.new_crew_name);
                    ui.text_edit_singleline(&mut self.new_crew_role);
                    if ui
                        .button(translate(self.language, "Add Crew Member"))
                        .clicked()
                        && !self.new_crew_name.trim().is_empty()
                    {
                        self.undo_stack.push(previous_project.clone());
                        self.redo_stack.clear();
                        production.crew.push(CrewMember {
                            name: self.new_crew_name.trim().to_owned(),
                            role: self.new_crew_role.trim().to_owned(),
                            ..Default::default()
                        });
                        self.new_crew_name.clear();
                        self.new_crew_role.clear();
                    }
                });
            });
    }

    /// Displays actor-unavailability and schedule-lock reports.
    fn draw_conflict_report(&mut self, context: &egui::Context) {
        if !self.show_conflicts {
            return;
        }
        egui::Window::new(translate(self.language, "Cast Availability Conflicts"))
            .open(&mut self.show_conflicts)
            .show(context, |ui| {
                let conflicts = self.project.scan_conflicts();
                if conflicts.is_empty() {
                    ui.label(translate(
                        self.language,
                        "No cast availability conflicts found.",
                    ));
                }
                for conflict in conflicts {
                    ui.label(format!(
                        "{} - {} ({}) unavailable on {}",
                        conflict.scene_title,
                        conflict.character,
                        conflict.actor_display_name,
                        conflict.date.format("%Y-%m-%d")
                    ));
                }
            });
    }

    /// Displays the added and removed cast work days since the schedule lock.
    fn draw_schedule_lock_report(&mut self, context: &egui::Context) {
        if !self.show_lock_report {
            return;
        }
        egui::Window::new(translate(self.language, "Schedule Lock Report"))
            .open(&mut self.show_lock_report)
            .show(context, |ui| {
                let changes = self.project.schedule_lock_changes();
                if changes.is_empty() {
                    ui.label(translate(
                        self.language,
                        "No schedule changes since the lock.",
                    ));
                }
                for change in changes {
                    ui.heading(&change.actor_display_name);
                    ui.label(format!(
                        "{} {}",
                        translate(self.language, "Added:"),
                        format_dates(&change.added_days)
                    ));
                    ui.label(format!(
                        "{} {}",
                        translate(self.language, "Removed:"),
                        format_dates(&change.removed_days)
                    ));
                }
            });
    }

    /// Edits scene headings, timing, cast, locations, and breakdown department tags.
    fn draw_scene_editor(&mut self, context: &egui::Context) {
        let Some(scene_id) = self.selected_scene else {
            return;
        };
        let mut open = true;
        egui::Window::new(translate(self.language, "Edit Scene"))
            .open(&mut open)
            .default_width(560.0)
            .show(context, |ui| {
                if let Some(scene) = self.project.scene_mut(scene_id) {
                    ui.horizontal(|ui| {
                        ui.label(translate(self.language, "Scene Number"));
                        ui.text_edit_singleline(&mut scene.scene_number);
                        ui.label(translate(self.language, "Page Eighths"));
                        ui.add(egui::DragValue::new(&mut scene.duration).range(0..=999));
                    });
                    ui.label(translate(self.language, "Heading"));
                    ui.text_edit_singleline(&mut scene.title);
                    ui.horizontal(|ui| {
                        ui.label(translate(self.language, "Estimated Minutes"));
                        ui.add(egui::DragValue::new(&mut scene.estimated_time).range(0..=9999));
                        ui.label(translate(self.language, "Time of Day"));
                        egui::ComboBox::from_id_salt(("time-of-day", scene_id))
                            .selected_text(translate(self.language, &scene.day_night_type))
                            .show_ui(ui, |ui| {
                                for value in ["DAY", "NIGHT", "DAWN", "DUSK", "AFTERNOON", "CUSTOM"]
                                {
                                    ui.selectable_value(
                                        &mut scene.day_night_type,
                                        value.to_owned(),
                                        translate(self.language, value),
                                    );
                                }
                            });
                    });
                    ui.label(translate(self.language, "Cast (comma separated)"));
                    let mut cast = scene.cast.join(", ");
                    if ui.text_edit_singleline(&mut cast).changed() {
                        scene.cast = cast
                            .split(',')
                            .map(str::trim)
                            .filter(|name| !name.is_empty())
                            .map(str::to_owned)
                            .collect();
                    }
                    ui.label(translate(self.language, "Summary"));
                    ui.text_edit_multiline(&mut scene.summary);
                    ui.horizontal(|ui| {
                        ui.label(translate(self.language, "Real Location"));
                        ui.text_edit_singleline(&mut scene.real_location);
                    });
                    ui.horizontal(|ui| {
                        ui.label(translate(self.language, "Address"));
                        ui.text_edit_singleline(&mut scene.location_address);
                    });
                    ui.checkbox(
                        &mut scene.is_completed,
                        translate(self.language, "Completed"),
                    );
                    for (label, values) in [
                        ("Extras", &mut scene.extras),
                        ("Props", &mut scene.props),
                        ("Set Dressing", &mut scene.set_dressing),
                        ("Wardrobe", &mut scene.wardrobe),
                        ("Makeup/Hair", &mut scene.makeup_hair),
                        ("Vehicles", &mut scene.vehicles),
                        ("Special Equipment", &mut scene.special_equipment),
                        ("Stunts", &mut scene.stunts),
                        ("SFX", &mut scene.sfx),
                        ("VFX", &mut scene.vfx),
                    ] {
                        edit_tag_list(ui, label, values);
                    }
                    ui.label(translate(self.language, "Breakdown Notes"));
                    ui.text_edit_multiline(&mut scene.breakdown_notes);
                }
            });
        if !open {
            self.selected_scene = None;
        }
    }

    /// Exports production breakdown sheets to a PDF selected by the user.
    fn export_breakdowns(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("Scene Breakdowns.pdf")
            .save_file()
        else {
            return;
        };
        self.set_status(
            match export_breakdowns_pdf_localized(&self.project, self.language, &path) {
                Ok(()) => format!("Exported {}", path.display()),
                Err(error) => format!("Breakdown export failed: {error}"),
            },
        );
    }

    /// Exports a cast days-out-of-days matrix to a PDF selected by the user.
    fn export_dood(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("Days Out of Days.pdf")
            .save_file()
        else {
            return;
        };
        self.set_status(
            match export_days_out_of_days_pdf_localized(
                &self.project,
                self.include_hold,
                self.language,
                &path,
            ) {
                Ok(()) => format!("Exported {}", path.display()),
                Err(error) => format!("DOOD export failed: {error}"),
            },
        );
    }

    /// Exports the currently displayed calendar month.
    fn export_month(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("Production Calendar.pdf")
            .save_file()
        else {
            return;
        };
        self.set_status(
            match export_month_calendar_pdf_localized(
                &self.project,
                self.month.year(),
                self.month.month(),
                self.language,
                path,
            ) {
                Ok(()) => "Monthly calendar PDF exported".to_owned(),
                Err(error) => format!("Calendar export failed: {error}"),
            },
        );
    }

    /// Exports all scheduled scene strips as a stripboard PDF.
    fn export_stripboard(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("Stripboard.pdf")
            .save_file()
        else {
            return;
        };
        self.set_status(
            match export_stripboard_pdf_localized(&self.project, self.language, path) {
                Ok(()) => "Stripboard PDF exported".to_owned(),
                Err(error) => format!("Stripboard export failed: {error}"),
            },
        );
    }

    /// Exports a day timeline calculated from call times and scene estimates.
    fn export_timeline(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("One-Line Schedule.pdf")
            .save_file()
        else {
            return;
        };
        self.set_status(
            match export_timeline_pdf_localized(&self.project, self.language, path) {
                Ok(()) => "One-line schedule PDF exported".to_owned(),
                Err(error) => format!("One-line export failed: {error}"),
            },
        );
    }

    /// Adds a date range beginning today without replacing any existing days.
    fn add_calendar(&mut self) {
        let today = Local::now().date_naive();
        for offset in 0..30 {
            let date = today + Duration::days(offset);
            if self
                .project
                .shoot_days
                .iter()
                .any(|day| day.date.date_naive() == date)
            {
                continue;
            }
            if let Some(day) = local_day(date) {
                self.project.shoot_days.push(ShootDay::new(day));
            }
        }
        self.project.shoot_days.sort_by_key(|day| day.date);
        self.set_status("Added 30 calendar days");
    }

    /// Renders the list of unscheduled scenes with one-click scheduling actions.
    fn draw_boneyard(&mut self, ui: &mut egui::Ui) {
        ui.heading(translate(self.language, "Boneyard"));
        let (_, dropped_scene_ids) =
            ui.dnd_drop_zone::<Vec<Uuid>, _>(egui::Frame::group(ui.style()), |ui| {
                ui.label(translate(
                    self.language,
                    "Drop scheduled scenes here to return them to the Boneyard",
                ))
            });
        if let Some(scene_ids) = dropped_scene_ids {
            let scheduled_ids: Vec<Uuid> = scene_ids
                .iter()
                .copied()
                .filter(|scene_id| {
                    self.project
                        .shoot_days
                        .iter()
                        .any(|day| day.scenes.iter().any(|scene| scene.id == *scene_id))
                })
                .collect();
            if !scheduled_ids.is_empty() {
                self.record_undo();
                for scene_id in &scheduled_ids {
                    self.project.unschedule_scene(*scene_id);
                }
                self.set_status(format!(
                    "Returned {} scenes to the Boneyard",
                    scheduled_ids.len()
                ));
            }
        }
        ui.text_edit_singleline(&mut self.search_query);
        egui::ComboBox::from_id_salt("boneyard-sort")
            .selected_text(translate(self.language, &self.boneyard_sort))
            .show_ui(ui, |ui| {
                for sort in [
                    "Show Order",
                    "Default",
                    "Location",
                    "INT/EXT",
                    "Cast",
                    "Day/Night",
                ] {
                    ui.selectable_value(
                        &mut self.boneyard_sort,
                        sort.to_owned(),
                        translate(self.language, sort),
                    );
                }
            });
        if self.project.all_scenes.is_empty() {
            ui.label(translate(self.language, "No unscheduled scenes."));
            return;
        }
        let query = self.search_query.trim().to_ascii_lowercase();
        let mut scenes: Vec<&Scene> = self
            .project
            .all_scenes
            .iter()
            .filter(|scene| {
                query.is_empty()
                    || scene.display_title().to_ascii_lowercase().contains(&query)
                    || scene.summary.to_ascii_lowercase().contains(&query)
                    || scene
                        .cast
                        .iter()
                        .any(|character| character.to_ascii_lowercase().contains(&query))
            })
            .collect();
        match self.boneyard_sort.as_str() {
            "Location" => scenes.sort_by(|left, right| {
                left.real_location
                    .cmp(&right.real_location)
                    .then_with(|| left.title.cmp(&right.title))
            }),
            "INT/EXT" => scenes.sort_by(|left, right| {
                heading_int_ext(&left.title).cmp(heading_int_ext(&right.title))
            }),
            "Cast" => scenes.sort_by_key(|scene| scene.cast.join(", ")),
            "Day/Night" => {
                scenes.sort_by(|left, right| left.day_night_type.cmp(&right.day_night_type))
            }
            "Default" => scenes.sort_by_key(|scene| scene_order_key(scene)),
            _ => {}
        }
        let scenes: Vec<(Uuid, String)> = scenes
            .into_iter()
            .map(|scene| {
                (
                    scene.id,
                    scene_label(scene, self.show_cast, self.show_estimated_time),
                )
            })
            .collect();
        ui.horizontal(|ui| {
            ui.label(format!(
                "{} {}",
                self.selected_boneyard_scenes.len(),
                translate(self.language, "scenes selected")
            ));
            if ui
                .button(translate(self.language, "Select visible"))
                .clicked()
            {
                self.selected_boneyard_scenes
                    .extend(scenes.iter().map(|(scene_id, _)| *scene_id));
            }
            if ui
                .button(translate(self.language, "Clear selection"))
                .clicked()
            {
                self.selected_boneyard_scenes.clear();
            }
            if let Some(first_day) = self.project.shoot_days.first() {
                let mut day_id = self.bulk_schedule_target.unwrap_or(first_day.id);
                egui::ComboBox::from_id_salt("bulk-schedule-day")
                    .selected_text(
                        self.project
                            .shoot_days
                            .iter()
                            .find(|day| day.id == day_id)
                            .map(|day| day.date.format("%b %d").to_string())
                            .unwrap_or_else(|| translate(self.language, "Choose day")),
                    )
                    .show_ui(ui, |ui| {
                        for day in &self.project.shoot_days {
                            ui.selectable_value(
                                &mut day_id,
                                day.id,
                                day.date.format("%b %d").to_string(),
                            );
                        }
                    });
                self.bulk_schedule_target = Some(day_id);
                if ui
                    .add_enabled(
                        !self.selected_boneyard_scenes.is_empty(),
                        egui::Button::new(translate(self.language, "Schedule selected")),
                    )
                    .clicked()
                {
                    let scheduled = self.schedule_selected_scenes(day_id);
                    self.set_status(format!("Scheduled {scheduled} scenes"));
                }
            }
        });
        for (scene_id, title) in &scenes {
            let payload: Vec<Uuid> = if self.selected_boneyard_scenes.contains(scene_id) {
                scenes
                    .iter()
                    .filter(|(candidate_id, _)| {
                        self.selected_boneyard_scenes.contains(candidate_id)
                    })
                    .map(|(candidate_id, _)| *candidate_id)
                    .collect()
            } else {
                vec![*scene_id]
            };
            ui.dnd_drag_source(egui::Id::new(("boneyard-scene", scene_id)), payload, |ui| {
                ui.horizontal(|ui| {
                    let mut is_selected = self.selected_boneyard_scenes.contains(scene_id);
                    if ui.checkbox(&mut is_selected, "").changed() {
                        if is_selected {
                            self.selected_boneyard_scenes.insert(*scene_id);
                        } else {
                            self.selected_boneyard_scenes.remove(scene_id);
                        }
                    }
                    if ui.selectable_label(false, title).clicked() {
                        self.selected_scene = Some(*scene_id);
                    }
                    if !self.project.shoot_days.is_empty() {
                        let mut day_id = *self
                            .schedule_targets
                            .entry(*scene_id)
                            .or_insert(self.project.shoot_days[0].id);
                        egui::ComboBox::from_id_salt(("schedule-day", scene_id))
                            .selected_text(
                                self.project
                                    .shoot_days
                                    .iter()
                                    .find(|day| day.id == day_id)
                                    .map(|day| day.date.format("%b %d").to_string())
                                    .unwrap_or_else(|| "Choose day".to_owned()),
                            )
                            .show_ui(ui, |ui| {
                                for day in &self.project.shoot_days {
                                    ui.selectable_value(
                                        &mut day_id,
                                        day.id,
                                        day.date.format("%b %d").to_string(),
                                    );
                                }
                            });
                        self.schedule_targets.insert(*scene_id, day_id);
                        if ui.small_button("Add").clicked() {
                            self.record_undo();
                            self.project.schedule_scene(*scene_id, day_id);
                        }
                    }
                });
            });
        }
    }

    /// Schedules selected Boneyard scenes in their current display order.
    ///
    /// Stale selections are ignored. One undo snapshot covers the whole batch,
    /// and the selection is cleared after the scheduling attempt.
    fn schedule_selected_scenes(&mut self, day_id: Uuid) -> usize {
        let scene_ids: Vec<Uuid> = self
            .project
            .all_scenes
            .iter()
            .filter(|scene| self.selected_boneyard_scenes.contains(&scene.id))
            .map(|scene| scene.id)
            .collect();
        if scene_ids.is_empty() || !self.project.shoot_days.iter().any(|day| day.id == day_id) {
            return 0;
        }

        self.record_undo();
        let scheduled = scene_ids
            .into_iter()
            .filter(|scene_id| self.project.schedule_scene(*scene_id, day_id))
            .count();
        self.selected_boneyard_scenes.clear();
        scheduled
    }

    /// Moves a drag payload to a schedule position and records one undo transaction.
    fn accept_scene_drop(&mut self, scene_ids: &[Uuid], day_id: Uuid, position: usize) -> usize {
        let moving_ids: Vec<Uuid> = scene_ids
            .iter()
            .copied()
            .filter(|scene_id| self.project.scene(*scene_id).is_some())
            .collect();
        if moving_ids.is_empty() || !self.project.shoot_days.iter().any(|day| day.id == day_id) {
            return 0;
        }
        self.record_undo();
        let moved = self
            .project
            .move_scenes_to_day(&moving_ids, day_id, position);
        for scene_id in &moving_ids {
            self.selected_boneyard_scenes.remove(scene_id);
        }
        if moved > 0 {
            self.set_status(format!("Moved {moved} scenes to the selected day"));
        }
        moved
    }

    /// Renders calendar days and allows scenes to return to the Boneyard.
    fn draw_schedule(&mut self, ui: &mut egui::Ui) {
        if self.stripboard {
            self.draw_stripboard(ui);
        } else {
            self.draw_calendar(ui);
        }
    }

    /// Draws a month grid with one editable schedule cell per calendar day.
    fn draw_calendar(&mut self, ui: &mut egui::Ui) {
        ui.heading(translate(self.language, "Schedule"));
        ui.horizontal(|ui| {
            if ui.button("<").clicked() {
                self.month = previous_month(self.month);
            }
            ui.heading(format_month(self.month, self.language));
            if ui.button(">").clicked() {
                self.month = next_month(self.month);
            }
            if ui.button(translate(self.language, "Today")).clicked() {
                let today = Local::now().date_naive();
                self.month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                    .expect("the current year and month are valid");
            }
        });
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.event_title);
            ui.text_edit_singleline(&mut self.event_time);
            if ui
                .small_button(translate(self.language, "Previous Day"))
                .clicked()
            {
                self.event_date -= Duration::days(1);
            }
            ui.label(self.event_date.format("%Y-%m-%d").to_string());
            if ui
                .small_button(translate(self.language, "Next Day"))
                .clicked()
            {
                self.event_date += Duration::days(1);
            }
            if ui
                .button(translate(self.language, "Add Calendar Event"))
                .clicked()
                && !self.event_title.trim().is_empty()
            {
                self.add_scene_to_date(
                    Scene::create_calendar_event(self.event_title.trim(), self.event_time.trim()),
                    self.event_date,
                );
                self.event_title.clear();
                self.event_time.clear();
            }
            egui::ComboBox::from_id_salt("banner-kind")
                .selected_text(translate(self.language, &self.banner_type))
                .show_ui(ui, |ui| {
                    for kind in ["Company Move", "Meal Break", "Notice", "Custom Banner"] {
                        ui.selectable_value(
                            &mut self.banner_type,
                            kind.to_owned(),
                            translate(self.language, kind),
                        );
                    }
                });
            if ui.button(translate(self.language, "Add Banner")).clicked()
                && !self.event_title.trim().is_empty()
            {
                self.add_scene_to_date(
                    Scene::create_banner(
                        self.banner_type.clone(),
                        self.event_title.trim(),
                        self.event_time.trim(),
                        "0:30",
                    ),
                    self.event_date,
                );
                self.event_title.clear();
                self.event_time.clear();
            }
            egui::ComboBox::from_id_salt("meal-kind")
                .selected_text(translate(self.language, &self.meal_kind))
                .show_ui(ui, |ui| {
                    for kind in [
                        "General Call",
                        "Ready to Shoot",
                        "Lunch",
                        "Snack",
                        "Dinner",
                        "Wrap",
                    ] {
                        ui.selectable_value(
                            &mut self.meal_kind,
                            kind.to_owned(),
                            translate(self.language, kind),
                        );
                    }
                });
            if ui.button(translate(self.language, "Add Meal")).clicked() {
                self.add_scene_to_date(
                    Scene::create_auto_meal(&self.meal_kind, self.event_time.trim()),
                    self.event_date,
                );
            }
            if ui
                .button(translate(self.language, "Add 30-Day Calendar"))
                .clicked()
            {
                self.add_calendar();
            }
        });

        let first = self.month;
        let first_weekday = first.weekday().num_days_from_sunday() as usize;
        let days_in_month = next_month(first)
            .pred_opt()
            .expect("next month has a previous date")
            .day();
        let mut cell_dates = vec![None; first_weekday];
        cell_dates.extend(
            (1..=days_in_month)
                .filter_map(|day| NaiveDate::from_ymd_opt(first.year(), first.month(), day))
                .map(Some),
        );
        while cell_dates.len() % 7 != 0 {
            cell_dates.push(None);
        }
        for header in ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] {
            ui.label(translate(self.language, header));
        }
        for week in cell_dates.chunks(7) {
            ui.columns(7, |columns| {
                for (column, date) in columns.iter_mut().zip(week) {
                    if let Some(date) = date {
                        let scenes: Vec<(Uuid, String)> = self
                            .project
                            .shoot_days
                            .iter()
                            .find(|day| day.date.date_naive() == *date)
                            .map(|day| {
                                day.scenes
                                    .iter()
                                    .map(|scene| {
                                        (
                                            scene.id,
                                            scene_label(
                                                scene,
                                                self.show_cast,
                                                self.show_estimated_time,
                                            ),
                                        )
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        let existing_day_id = self
                            .project
                            .shoot_days
                            .iter()
                            .find(|day| day.date.date_naive() == *date)
                            .map(|day| day.id);
                        let (_, dropped_scene_ids) = column.dnd_drop_zone::<Vec<Uuid>, _>(
                            egui::Frame::group(column.style()),
                            |ui| {
                                ui.strong(date.day().to_string());
                                if let Some(day_id) = self
                                    .project
                                    .shoot_days
                                    .iter()
                                    .find(|day| day.date.date_naive() == *date)
                                    .map(|day| day.id)
                                {
                                    if ui.small_button("Call Sheet").clicked() {
                                        self.selected_call_sheet = Some(day_id);
                                    }
                                    if ui
                                        .small_button(
                                            if self.selected_day_for_move == Some(day_id) {
                                                "Cancel day move"
                                            } else {
                                                "Move day..."
                                            },
                                        )
                                        .clicked()
                                    {
                                        self.selected_day_for_move = (self.selected_day_for_move
                                            != Some(day_id))
                                        .then_some(day_id);
                                    }
                                    if let Some(source_day_id) = self
                                        .selected_day_for_move
                                        .filter(|source_day_id| *source_day_id != day_id)
                                    {
                                        if ui.small_button("Swap with selected day").clicked() {
                                            self.record_undo();
                                            if self.project.swap_day_contents(source_day_id, day_id)
                                            {
                                                self.selected_day_for_move = None;
                                                self.set_status(
                                                    "Swapped schedule and call sheet between days",
                                                );
                                            } else {
                                                self.set_status(
                                                    "Could not swap the selected shooting days",
                                                );
                                            }
                                        }
                                    }
                                    let is_blackout = self
                                        .project
                                        .shoot_days
                                        .iter()
                                        .find(|day| day.id == day_id)
                                        .is_some_and(|day| day.is_blackout);
                                    if ui
                                        .small_button(if is_blackout {
                                            "Clear Blackout"
                                        } else {
                                            "Blackout"
                                        })
                                        .clicked()
                                    {
                                        self.record_undo();
                                        if let Some(day) = self
                                            .project
                                            .shoot_days
                                            .iter_mut()
                                            .find(|day| day.id == day_id)
                                        {
                                            day.is_blackout = !day.is_blackout;
                                        }
                                    }
                                }
                                for (position, (scene_id, title)) in scenes.into_iter().enumerate()
                                {
                                    let (drop_zone, dropped_scene_ids) = ui
                                        .dnd_drop_zone::<Vec<Uuid>, _>(egui::Frame::NONE, |ui| {
                                            ui.small_button(title)
                                        });
                                    if let Some(scene_ids) = dropped_scene_ids {
                                        if let Some(day_id) = existing_day_id {
                                            self.accept_scene_drop(&scene_ids, day_id, position);
                                        }
                                    }
                                    if drop_zone.inner.clicked() {
                                        self.selected_scene = Some(scene_id);
                                    }
                                }
                                if self
                                    .project
                                    .shoot_days
                                    .iter()
                                    .all(|day| day.date.date_naive() != *date)
                                    && ui
                                        .small_button(translate(self.language, "Add Day"))
                                        .clicked()
                                {
                                    if let Some(timestamp) = local_day(*date) {
                                        self.record_undo();
                                        self.project.shoot_days.push(ShootDay::new(timestamp));
                                        self.project.shoot_days.sort_by_key(|day| day.date);
                                    }
                                }
                            },
                        );
                        if let Some(scene_ids) = dropped_scene_ids {
                            let (day_id, created_day) = match existing_day_id {
                                Some(day_id) => (Some(day_id), false),
                                None => {
                                    if let Some(timestamp) = local_day(*date) {
                                        self.record_undo();
                                        let day = ShootDay::new(timestamp);
                                        let day_id = day.id;
                                        self.project.shoot_days.push(day);
                                        self.project.shoot_days.sort_by_key(|day| day.date);
                                        (Some(day_id), true)
                                    } else {
                                        (None, false)
                                    }
                                }
                            };
                            if let Some(day_id) = day_id {
                                let position = self
                                    .project
                                    .shoot_days
                                    .iter()
                                    .find(|day| day.id == day_id)
                                    .map_or(0, |day| day.scenes.len());
                                if created_day {
                                    let moved = self
                                        .project
                                        .move_scenes_to_day(&scene_ids, day_id, position);
                                    self.set_status(format!(
                                        "Moved {moved} scenes to the selected day"
                                    ));
                                } else {
                                    self.accept_scene_drop(&scene_ids, day_id, position);
                                }
                                for scene_id in scene_ids.iter() {
                                    self.selected_boneyard_scenes.remove(scene_id);
                                }
                            }
                        }
                    }
                }
            });
        }
    }

    /// Draws the scene list in timeline/day order with completion and move controls.
    fn draw_stripboard(&mut self, ui: &mut egui::Ui) {
        ui.heading(translate(self.language, "Stripboard"));
        let days: Vec<StripboardDay> = self
            .project
            .shoot_days
            .iter()
            .map(|day| StripboardDay {
                day_id: day.id,
                date: day.date.date_naive(),
                scenes: day
                    .scenes
                    .iter()
                    .map(|scene| {
                        (
                            scene.id,
                            scene_label(scene, self.show_cast, self.show_estimated_time),
                            scene.is_completed,
                        )
                    })
                    .collect(),
                pages: day.total_pages_eighths(),
                minutes: day.total_estimated_minutes(),
            })
            .collect();
        for day in days {
            let (_, dropped_scene_ids) =
                ui.dnd_drop_zone::<Vec<Uuid>, _>(egui::Frame::group(ui.style()), |ui| {
                    ui.strong(format_long_date(day.date, self.language));
                    ui.label(format!(
                        "{} pages / {} min",
                        eighths_label(day.pages),
                        day.minutes
                    ));
                    for (scene_position, (scene_id, title, mut completed)) in
                        day.scenes.into_iter().enumerate()
                    {
                        let (_, scene_drop_payload) =
                            ui.dnd_drop_zone::<Vec<Uuid>, _>(egui::Frame::NONE, |ui| {
                                ui.dnd_drag_source(
                                    egui::Id::new(("scheduled-scene", scene_id)),
                                    vec![scene_id],
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            if ui.checkbox(&mut completed, "").changed() {
                                                self.record_undo();
                                                if let Some(scene) =
                                                    self.project.scene_mut(scene_id)
                                                {
                                                    scene.is_completed = completed;
                                                }
                                            }
                                            if ui.selectable_label(false, title).clicked() {
                                                self.selected_scene = Some(scene_id);
                                            }
                                            let destination_days: Vec<(Uuid, String)> = self
                                                .project
                                                .shoot_days
                                                .iter()
                                                .filter(|shoot_day| shoot_day.id != day.day_id)
                                                .map(|shoot_day| {
                                                    (
                                                        shoot_day.id,
                                                        shoot_day.date.format("%b %d").to_string(),
                                                    )
                                                })
                                                .collect();
                                            if let Some((first_destination, _)) =
                                                destination_days.first()
                                            {
                                                let mut destination_id = self
                                                    .schedule_targets
                                                    .get(&scene_id)
                                                    .copied()
                                                    .filter(|target| {
                                                        destination_days
                                                            .iter()
                                                            .any(|(day_id, _)| day_id == target)
                                                    })
                                                    .unwrap_or(*first_destination);
                                                egui::ComboBox::from_id_salt((
                                                    "move-scene-day",
                                                    scene_id,
                                                ))
                                                .selected_text(
                                                    destination_days
                                                        .iter()
                                                        .find(|(day_id, _)| {
                                                            *day_id == destination_id
                                                        })
                                                        .map(|(_, label)| label.as_str())
                                                        .unwrap_or("Choose day"),
                                                )
                                                .show_ui(ui, |ui| {
                                                    for (target_id, label) in &destination_days {
                                                        ui.selectable_value(
                                                            &mut destination_id,
                                                            *target_id,
                                                            label,
                                                        );
                                                    }
                                                });
                                                self.schedule_targets
                                                    .insert(scene_id, destination_id);
                                                if ui.small_button("Move day").clicked() {
                                                    self.record_undo();
                                                    if !self
                                                        .project
                                                        .move_scene(scene_id, destination_id)
                                                    {
                                                        self.set_status(
                                                            "Could not move scene to the selected day",
                                                        );
                                                    }
                                                }
                                            }
                                            if ui.small_button("Up").clicked() {
                                                let position = self
                                                    .project
                                                    .shoot_days
                                                    .iter()
                                                    .find(|shoot_day| shoot_day.id == day.day_id)
                                                    .and_then(|shoot_day| {
                                                        shoot_day
                                                            .scenes
                                                            .iter()
                                                            .position(|scene| scene.id == scene_id)
                                                    });
                                                if let Some(index) =
                                                    position.filter(|index| *index > 0)
                                                {
                                                    self.record_undo();
                                                    if let Some(day) =
                                                        self.project.shoot_days.iter_mut().find(
                                                            |shoot_day| shoot_day.id == day.day_id,
                                                        )
                                                    {
                                                        day.scenes.swap(index, index - 1);
                                                    }
                                                }
                                            }
                                            if ui.small_button("Down").clicked() {
                                                let position = self
                                                    .project
                                                    .shoot_days
                                                    .iter()
                                                    .find(|shoot_day| shoot_day.id == day.day_id)
                                                    .and_then(|shoot_day| {
                                                        shoot_day
                                                            .scenes
                                                            .iter()
                                                            .position(|scene| scene.id == scene_id)
                                                    });
                                                let can_move_down = position.is_some_and(|index| {
                                                    self.project
                                                        .shoot_days
                                                        .iter()
                                                        .find(|shoot_day| {
                                                            shoot_day.id == day.day_id
                                                        })
                                                        .is_some_and(|shoot_day| {
                                                            index + 1 < shoot_day.scenes.len()
                                                        })
                                                });
                                                if let Some(index) =
                                                    position.filter(|_| can_move_down)
                                                {
                                                    self.record_undo();
                                                    if let Some(day) =
                                                        self.project.shoot_days.iter_mut().find(
                                                            |shoot_day| shoot_day.id == day.day_id,
                                                        )
                                                    {
                                                        day.scenes.swap(index, index + 1);
                                                    }
                                                }
                                            }
                                            if ui.small_button("Unassign").clicked() {
                                                self.record_undo();
                                                self.project.unschedule_scene(scene_id);
                                            }
                                        })
                                    },
                                )
                            });
                        if let Some(scene_ids) = scene_drop_payload {
                            self.accept_scene_drop(&scene_ids, day.day_id, scene_position);
                        }
                    }
                });
            if let Some(scene_ids) = dropped_scene_ids {
                let position = self
                    .project
                    .shoot_days
                    .iter()
                    .find(|shoot_day| shoot_day.id == day.day_id)
                    .map_or(0, |shoot_day| shoot_day.scenes.len());
                self.accept_scene_drop(&scene_ids, day.day_id, position);
            }
        }
    }
}

/// Draws a labeled single-line text field and reports whether it changed.
fn labeled_text(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed = ui.text_edit_singleline(value).changed();
    });
    changed
}

/// Adds cast members from a shoot day's scenes and fills missing roster details.
fn populate_cast_calls(day: &mut ShootDay, cast_roster: &[CastMember]) -> bool {
    let on_set_time = if day.call_sheet.ready_to_shoot_time.is_empty() {
        day.call_sheet.general_call_time.clone()
    } else {
        day.call_sheet.ready_to_shoot_time.clone()
    };
    let wrap_time = day.call_sheet.dinner_time.clone();
    let cast_details: Vec<(String, String, String)> = day
        .all_cast()
        .into_iter()
        .map(|character| {
            let actor = cast_roster
                .iter()
                .find(|member| {
                    member
                        .character_name
                        .trim()
                        .eq_ignore_ascii_case(character.trim())
                })
                .map(|member| member.actor_name.clone())
                .unwrap_or_default();
            let scene_numbers = day
                .scenes
                .iter()
                .filter(|scene| {
                    scene.cast.iter().any(|scene_character| {
                        scene_character
                            .trim()
                            .eq_ignore_ascii_case(character.trim())
                    })
                })
                .map(|scene| scene.scene_number.trim())
                .filter(|number| !number.is_empty())
                .collect::<Vec<_>>()
                .join(", ");
            (character, actor, scene_numbers)
        })
        .collect();

    let mut changed = false;
    for (character_name, actor_name, scene_numbers) in cast_details {
        if let Some(entry) = day.call_sheet.cast_call_entries.iter_mut().find(|entry| {
            entry
                .character_name
                .trim()
                .eq_ignore_ascii_case(character_name.trim())
        }) {
            if entry.actor_name.trim().is_empty() && !actor_name.trim().is_empty() {
                entry.actor_name = actor_name;
                changed = true;
            }
            if entry.scene_numbers.trim().is_empty() && !scene_numbers.trim().is_empty() {
                entry.scene_numbers = scene_numbers;
                changed = true;
            }
            if entry.on_set_time.trim().is_empty() && !on_set_time.trim().is_empty() {
                entry.on_set_time.clone_from(&on_set_time);
                changed = true;
            }
            if entry.wrap_time.trim().is_empty() && !wrap_time.trim().is_empty() {
                entry.wrap_time.clone_from(&wrap_time);
                changed = true;
            }
        } else {
            day.call_sheet.cast_call_entries.push(CastCallEntry {
                character_name,
                actor_name,
                scene_numbers,
                on_set_time: on_set_time.clone(),
                wrap_time: wrap_time.clone(),
                ..CastCallEntry::default()
            });
            changed = true;
        }
    }
    changed
}

/// Adds missing production crew to a call sheet without duplicating existing rows.
fn populate_crew_calls(
    crew_roster: &[CrewMember],
    entries: &mut Vec<CrewCallEntry>,
    general_call_time: &str,
) -> bool {
    let call_time = if general_call_time.trim().is_empty() {
        "07:30 AM"
    } else {
        general_call_time
    };
    let mut changed = false;
    for member in crew_roster {
        let existing = entries.iter_mut().find(|entry| {
            entry.role.trim().eq_ignore_ascii_case(member.role.trim())
                && (entry.name.trim().is_empty()
                    || entry.name.trim().eq_ignore_ascii_case(member.name.trim()))
        });
        if let Some(entry) = existing {
            if entry.name.trim().is_empty() {
                entry.name.clone_from(&member.name);
                changed = true;
            }
            if entry.phone.trim().is_empty() && !member.phone.trim().is_empty() {
                entry.phone.clone_from(&member.phone);
                changed = true;
            }
            if entry.call_time.trim().is_empty() {
                entry.call_time = call_time.to_owned();
                changed = true;
            }
        } else {
            entries.push(CrewCallEntry {
                role: member.role.clone(),
                name: member.name.clone(),
                call_time: call_time.to_owned(),
                phone: member.phone.clone(),
                ..CrewCallEntry::default()
            });
            changed = true;
        }
    }
    changed
}

/// Adds unique scene locations, enriching each with its matching roster address.
fn populate_call_sheet_locations(day: &mut ShootDay, location_roster: &[Location]) -> bool {
    let mut scene_locations = Vec::new();
    for scene in &day.scenes {
        let name = scene.real_location.trim();
        let exists_on_call_sheet = day
            .call_sheet
            .locations
            .iter()
            .any(|location| location.name.trim().eq_ignore_ascii_case(name));
        let exists_in_candidates = scene_locations
            .iter()
            .any(|location: &Location| location.name.eq_ignore_ascii_case(name));
        if name.is_empty() || exists_on_call_sheet || exists_in_candidates {
            continue;
        }

        let address = location_roster
            .iter()
            .find(|location| location.name.trim().eq_ignore_ascii_case(name))
            .map(|location| location.address.clone())
            .filter(|address| !address.trim().is_empty())
            .unwrap_or_else(|| scene.location_address.clone());
        scene_locations.push(Location {
            name: name.to_owned(),
            address,
            ..Location::default()
        });
    }
    if scene_locations.is_empty() {
        return false;
    }

    day.call_sheet.locations.extend(scene_locations);
    true
}

/// Records a project path at the front of the bounded recent-project list.
fn record_recent_project(recent_projects: &mut Vec<std::path::PathBuf>, path: std::path::PathBuf) {
    recent_projects.retain(|recent| recent != &path);
    recent_projects.insert(0, path);
    recent_projects.truncate(10);
}

/// Converts a local calendar date into the UTC timestamp used for persistence.
fn local_day(date: NaiveDate) -> Option<chrono::DateTime<Utc>> {
    Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0)?)
        .single()
        .map(|date| date.with_timezone(&Utc))
}

/// Formats page eighths using the standard script page notation.
fn eighths_label(eighths: i32) -> String {
    let pages = eighths / 8;
    let remainder = eighths % 8;
    match (pages, remainder) {
        (0, 0) => "0".to_owned(),
        (0, fraction) => format!("{fraction}/8"),
        (whole, 0) => whole.to_string(),
        (whole, fraction) => format!("{whole} {fraction}/8"),
    }
}

/// Moves a calendar cursor to the first day of the previous month.
fn previous_month(month: NaiveDate) -> NaiveDate {
    let previous = month.pred_opt().expect("a valid month has a previous date");
    NaiveDate::from_ymd_opt(previous.year(), previous.month(), 1)
        .expect("the previous year and month are valid")
}

/// Moves a calendar cursor to the first day of the next month.
fn next_month(month: NaiveDate) -> NaiveDate {
    let days = match month.month() {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if month.with_day(29).is_some() => 29,
        2 => 28,
        _ => unreachable!("chrono dates always use a valid month"),
    };
    let first_of_next = month
        .checked_add_signed(Duration::days(days))
        .expect("the calendar cursor is within chrono's supported date range");
    NaiveDate::from_ymd_opt(first_of_next.year(), first_of_next.month(), 1)
        .expect("the next year and month are valid")
}

/// Edits a comma-separated production department tag list.
fn edit_tag_list(ui: &mut egui::Ui, label: &str, values: &mut Vec<String>) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut text = values.join(", ");
        if ui.text_edit_singleline(&mut text).changed() {
            *values = text
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect();
        }
    });
}

/// Formats report dates as a comma-separated list.
fn format_dates(dates: &[chrono::DateTime<Utc>]) -> String {
    if dates.is_empty() {
        "None".to_owned()
    } else {
        dates
            .iter()
            .map(|date| date.format("%Y-%m-%d").to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Formats a scene strip with the enabled cast and time-estimate details.
fn scene_label(scene: &Scene, show_cast: bool, show_estimated_time: bool) -> String {
    let mut label = scene.display_title();
    if show_cast && !scene.cast.is_empty() {
        label.push_str(&format!(" | {}", scene.cast.join(", ")));
    }
    if show_estimated_time {
        label.push_str(&format!(" | {} min", scene.estimated_time));
    }
    label
}

/// Sorts numbered scenes before unnumbered scenes in script order.
fn scene_order_key(scene: &Scene) -> (u32, String) {
    let number = scene.scene_number.trim();
    let digits: String = number.chars().take_while(char::is_ascii_digit).collect();
    let suffix: String = number
        .chars()
        .skip(digits.len())
        .take_while(char::is_ascii_alphabetic)
        .collect();
    (
        digits.parse().unwrap_or(u32::MAX),
        suffix.to_ascii_uppercase(),
    )
}

/// Returns the interior/exterior heading prefix for Boneyard grouping.
fn heading_int_ext(title: &str) -> &'static str {
    let title = title.trim_start().to_ascii_uppercase();
    if title.starts_with("INT./EXT") || title.starts_with("INT/EXT") {
        "INT/EXT"
    } else if title.starts_with("INT") {
        "INT"
    } else if title.starts_with("EXT") {
        "EXT"
    } else {
        "OTHER"
    }
}

#[cfg(test)]
mod tests {
    use cinesched::model::{
        CastCallEntry, CastMember, CrewCallEntry, CrewMember, Location, Scene, ShootDay,
    };

    use super::{
        eighths_label, heading_int_ext, next_month, populate_call_sheet_locations,
        populate_cast_calls, populate_crew_calls, previous_month, record_recent_project,
        scene_order_key, CineSchedApp, Language, Project, Uuid,
    };
    use chrono::NaiveDate;

    /// Confirms common page-length totals use screenplay notation.
    #[test]
    fn formats_page_eighths() {
        assert_eq!(eighths_label(0), "0");
        assert_eq!(eighths_label(3), "3/8");
        assert_eq!(eighths_label(8), "1");
        assert_eq!(eighths_label(15), "1 7/8");
    }

    /// Confirms month navigation handles year boundaries and leap years.
    #[test]
    fn calendar_month_navigation_handles_boundaries() {
        assert_eq!(
            next_month(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()),
            NaiveDate::from_ymd_opt(2024, 3, 1).unwrap()
        );
        assert_eq!(
            previous_month(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            NaiveDate::from_ymd_opt(2024, 12, 1).unwrap()
        );
    }

    /// Confirms project and view preferences survive eframe JSON persistence.
    #[test]
    fn app_state_persists_preferences_and_project() {
        let recent_path = std::path::PathBuf::from("C:\\Projects\\CineSched.json");
        let app = CineSchedApp {
            dark_mode: true,
            show_cast: true,
            show_estimated_time: true,
            language: Language::Spanish,
            recent_projects: vec![recent_path.clone()],
            project: Project {
                project_title: "Persistent Project".to_owned(),
                ..Project::default()
            },
            ..CineSchedApp::default()
        };

        let encoded = serde_json::to_string(&app).expect("app state serializes");
        let restored: CineSchedApp =
            serde_json::from_str(&encoded).expect("app state deserializes");

        assert!(restored.dark_mode);
        assert!(restored.show_cast);
        assert!(restored.show_estimated_time);
        assert_eq!(restored.language, Language::Spanish);
        assert_eq!(restored.project.project_title, "Persistent Project");
        assert_eq!(restored.recent_projects, [recent_path]);
        assert!(restored.undo_stack.is_empty());

        let mut previous_version: serde_json::Value =
            serde_json::from_str(&encoded).expect("app JSON parses");
        previous_version
            .as_object_mut()
            .expect("app state is an object")
            .remove("recent_projects");
        let restored_previous: CineSchedApp = serde_json::from_value(previous_version)
            .expect("app state without recent projects remains compatible");
        assert!(restored_previous.recent_projects.is_empty());
    }

    /// Confirms recent paths are de-duplicated, promoted, and capped at ten entries.
    #[test]
    fn recent_projects_promote_duplicates_and_keep_ten_paths() {
        let mut recent = Vec::new();
        for index in 0..10 {
            record_recent_project(
                &mut recent,
                std::path::PathBuf::from(format!("project-{index}.json")),
            );
        }
        let oldest = recent[9].clone();

        record_recent_project(&mut recent, oldest.clone());
        record_recent_project(&mut recent, std::path::PathBuf::from("new-project.json"));

        assert_eq!(recent.len(), 10);
        assert_eq!(recent[0], std::path::PathBuf::from("new-project.json"));
        assert_eq!(recent[1], oldest);
        assert_eq!(recent.iter().filter(|path| **path == oldest).count(), 1);
    }

    /// Confirms common heading and script-order groupings remain stable.
    #[test]
    fn scene_helpers_group_headings_and_order_numbers() {
        let mut scene = Scene::new("EXT. PARK - DAY");
        scene.scene_number = "12A".to_owned();
        assert_eq!(heading_int_ext(&scene.title), "EXT");
        assert_eq!(scene_order_key(&scene), (12, "A".to_owned()));
        assert_eq!(heading_int_ext("INT./EXT. CAR - NIGHT"), "INT/EXT");
    }

    /// Confirms multi-scene scheduling preserves Boneyard order and uses one undo step.
    #[test]
    fn bulk_scheduling_preserves_scene_order_and_records_one_undo_step() {
        let first = Scene::new("First scene");
        let second = Scene::new("Second scene");
        let first_id = first.id;
        let second_id = second.id;
        let day = ShootDay::new(chrono::Utc::now());
        let day_id = day.id;
        let mut app = CineSchedApp::default();
        app.project.all_scenes = vec![first, second];
        app.project.shoot_days.push(day);
        app.selected_boneyard_scenes.insert(second_id);
        app.selected_boneyard_scenes.insert(first_id);

        assert_eq!(app.schedule_selected_scenes(day_id), 2);

        let day = app.project.shoot_days.remove(0);
        assert_eq!(
            day.scenes.iter().map(|scene| scene.id).collect::<Vec<_>>(),
            vec![first_id, second_id]
        );
        assert!(app.project.all_scenes.is_empty());
        assert!(app.selected_boneyard_scenes.is_empty());
        assert_eq!(app.undo_stack.len(), 1);
    }

    /// Confirms bulk scheduling with an invalid destination leaves the selection intact.
    #[test]
    fn bulk_scheduling_rejects_missing_day_without_mutating_project() {
        let scene = Scene::new("Unscheduled scene");
        let scene_id = scene.id;
        let mut app = CineSchedApp::default();
        app.project.all_scenes.push(scene);
        app.selected_boneyard_scenes.insert(scene_id);

        assert_eq!(app.schedule_selected_scenes(Uuid::new_v4()), 0);

        assert_eq!(app.project.all_scenes.len(), 1);
        assert!(app.selected_boneyard_scenes.contains(&scene_id));
        assert!(app.undo_stack.is_empty());
    }

    /// Confirms call-sheet cast population derives scene and actor details without overwriting edits.
    #[test]
    fn cast_call_population_fills_roster_details_and_preserves_existing_values() {
        let mut day = ShootDay::new(chrono::Utc::now());
        day.call_sheet.general_call_time = "07:00 AM".to_owned();
        day.call_sheet.ready_to_shoot_time = "08:00 AM".to_owned();
        day.call_sheet.dinner_time = "06:00 PM".to_owned();
        let mut first_scene = Scene::new("INT. STUDIO - DAY");
        first_scene.scene_number = "3A".to_owned();
        first_scene.cast = vec!["MAYA".to_owned(), "LEO".to_owned()];
        let mut second_scene = Scene::new("EXT. PARK - NIGHT");
        second_scene.scene_number = "8".to_owned();
        second_scene.cast = vec!["maya".to_owned()];
        day.scenes = vec![first_scene, second_scene];
        day.call_sheet.cast_call_entries.push(CastCallEntry {
            character_name: "MAYA".to_owned(),
            actor_name: "Existing Actor".to_owned(),
            pickup_time: "06:30 AM".to_owned(),
            ..CastCallEntry::default()
        });
        let roster = vec![CastMember {
            actor_name: "Actor from Roster".to_owned(),
            character_name: "MAYA".to_owned(),
            ..CastMember::default()
        }];

        assert!(populate_cast_calls(&mut day, &roster));
        assert!(!populate_cast_calls(&mut day, &roster));

        assert_eq!(day.call_sheet.cast_call_entries.len(), 2);
        let maya = day
            .call_sheet
            .cast_call_entries
            .iter()
            .find(|entry| entry.character_name == "MAYA")
            .unwrap();
        assert_eq!(maya.actor_name, "Existing Actor");
        assert_eq!(maya.scene_numbers, "3A, 8");
        assert_eq!(maya.pickup_time, "06:30 AM");
        assert_eq!(maya.on_set_time, "08:00 AM");
        assert_eq!(maya.wrap_time, "06:00 PM");
        let leo = day
            .call_sheet
            .cast_call_entries
            .iter()
            .find(|entry| entry.character_name == "LEO")
            .unwrap();
        assert_eq!(leo.scene_numbers, "3A");
        assert_eq!(leo.on_set_time, "08:00 AM");
        assert_eq!(leo.wrap_time, "06:00 PM");
    }

    /// Confirms crew roster loading is case-insensitive, non-destructive, and repeatable.
    #[test]
    fn crew_call_population_avoids_duplicates_and_uses_general_call() {
        let roster = vec![
            CrewMember {
                name: "Alex".to_owned(),
                role: "Director".to_owned(),
                phone: "555-0101".to_owned(),
                ..CrewMember::default()
            },
            CrewMember {
                name: "Sam".to_owned(),
                role: "Camera".to_owned(),
                ..CrewMember::default()
            },
        ];
        let mut entries = vec![CrewCallEntry {
            name: "alex".to_owned(),
            role: "DIRECTOR".to_owned(),
            call_time: "06:00 AM".to_owned(),
            ..CrewCallEntry::default()
        }];

        assert!(populate_crew_calls(&roster, &mut entries, "07:00 AM"));
        assert!(!populate_crew_calls(&roster, &mut entries, "07:00 AM"));

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].call_time, "06:00 AM");
        assert_eq!(entries[0].phone, "555-0101");
        assert_eq!(entries[1].call_time, "07:00 AM");
        assert_eq!(entries[1].phone, "");
    }

    /// Confirms scene locations populate once, reuse roster addresses, and preserve manual rows.
    #[test]
    fn call_sheet_location_population_merges_scene_and_roster_data() {
        let mut day = ShootDay::new(chrono::Utc::now());
        let mut first = Scene::new("INT. STAGE - DAY");
        first.real_location = "Studio".to_owned();
        first.location_address = "Fallback address".to_owned();
        let mut second = Scene::new("EXT. PARK - DAY");
        second.real_location = "studio".to_owned();
        let mut third = Scene::new("EXT. STREET - NIGHT");
        third.real_location = "Street".to_owned();
        third.location_address = "Scene address".to_owned();
        day.scenes = vec![first, second, third];
        day.call_sheet.locations.push(Location {
            name: "Existing".to_owned(),
            address: "Manual address".to_owned(),
            ..Location::default()
        });
        let roster = vec![Location {
            name: "STUDIO".to_owned(),
            address: "Roster address".to_owned(),
            ..Location::default()
        }];

        assert!(populate_call_sheet_locations(&mut day, &roster));
        assert!(!populate_call_sheet_locations(&mut day, &roster));

        assert_eq!(day.call_sheet.locations.len(), 3);
        assert_eq!(day.call_sheet.locations[0].address, "Manual address");
        assert_eq!(day.call_sheet.locations[1].name, "Studio");
        assert_eq!(day.call_sheet.locations[1].address, "Roster address");
        assert_eq!(day.call_sheet.locations[2].address, "Scene address");
    }
}
