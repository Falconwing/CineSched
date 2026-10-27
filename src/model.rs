//! JSON-compatible production schedule data types.
//!
//! The `extra` maps retain fields that are not yet represented in the Rust
//! interface, protecting call-sheet and breakdown data during migration.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Additional JSON properties retained from Swift project files.
pub type ExtraFields = BTreeMap<String, Value>;

/// A physical production location in a call sheet or location roster.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    /// Stable identifier for production-location references.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// Location or basecamp name.
    #[serde(default)]
    pub name: String,
    /// Street address or production directions.
    #[serde(default)]
    pub address: String,
    /// Fields introduced by newer clients.
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// A date interval when a cast member is unavailable.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateRange {
    /// Stable identifier for this availability record.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// First unavailable date, inclusive.
    #[serde(with = "swift_date")]
    pub start: DateTime<Utc>,
    /// Last unavailable date, inclusive.
    #[serde(with = "swift_date")]
    pub end: DateTime<Utc>,
}

impl DateRange {
    /// Creates a date range and normalizes its endpoints into ascending order.
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            start: start.min(end),
            end: start.max(end),
        }
    }

    /// Returns whether a timestamp falls in this inclusive date range.
    pub fn contains(&self, date: DateTime<Utc>) -> bool {
        let day = date.date_naive();
        let start = self.start.min(self.end).date_naive();
        let end = self.start.max(self.end).date_naive();
        day >= start && day <= end
    }
}

/// A cast member and the character they portray.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastMember {
    /// Stable identifier used by call-sheet overrides.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// Actor's display name.
    #[serde(default)]
    pub actor_name: String,
    /// Character name used in scene cast lists.
    #[serde(default)]
    pub character_name: String,
    /// Dates when this cast member cannot work.
    #[serde(default)]
    pub unavailable_ranges: Vec<DateRange>,
    /// Fields introduced by newer clients.
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl CastMember {
    /// Formats the actor and character in the established call-sheet style.
    pub fn display_string(&self) -> String {
        if self.actor_name.trim().is_empty() {
            self.character_name.clone()
        } else {
            format!("{} - {}", self.actor_name, self.character_name)
        }
    }
}

/// A crew member in the production roster.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrewMember {
    /// Stable identifier for call-sheet overrides.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// Crew member's name.
    #[serde(default)]
    pub name: String,
    /// Production role.
    #[serde(default)]
    pub role: String,
    /// Crew contact number.
    #[serde(default)]
    pub phone: String,
    /// Whether this crew member is included on every shoot day by default.
    #[serde(default)]
    pub is_daily_default: bool,
    /// Fields introduced by newer clients.
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl CrewMember {
    /// Formats the crew member with their role when one is present.
    pub fn display_string(&self) -> String {
        if self.role.trim().is_empty() {
            self.name.clone()
        } else {
            format!("{} - {}", self.name, self.role)
        }
    }
}

/// A cast call-sheet row.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastCallEntry {
    /// Stable identifier for the row.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// Character name.
    #[serde(default)]
    pub character_name: String,
    /// Actor name.
    #[serde(default)]
    pub actor_name: String,
    /// Script scene numbers for the cast member.
    #[serde(default)]
    pub scene_numbers: String,
    /// Employment category such as `E`, `ET`, or `W`.
    #[serde(default = "default_ecdt")]
    pub ecdt: String,
    /// Pickup time.
    #[serde(default)]
    pub pickup_time: String,
    /// Hair and makeup / wardrobe time.
    #[serde(default)]
    pub hmu_wardrobe_time: String,
    /// On-set time.
    #[serde(default)]
    pub on_set_time: String,
    /// Estimated wrap time.
    #[serde(default)]
    pub wrap_time: String,
    /// Call-sheet location number.
    #[serde(default = "default_location_index")]
    pub location_index: String,
    /// Fields introduced by newer clients.
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// A crew call-sheet row.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrewCallEntry {
    /// Stable identifier for the row.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// Production role.
    #[serde(default)]
    pub role: String,
    /// Crew member's name.
    #[serde(default)]
    pub name: String,
    /// Crew call time.
    #[serde(default)]
    pub call_time: String,
    /// Crew member's phone number.
    #[serde(default)]
    pub phone: String,
    /// Fields introduced by newer clients.
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Per-day call sheet and production milestones.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallSheet {
    /// General crew call.
    #[serde(default)]
    pub general_call_time: String,
    /// Optional work-day description.
    #[serde(default)]
    pub work_day_schedule: String,
    /// Ready-to-shoot time.
    #[serde(default)]
    pub ready_to_shoot_time: String,
    /// Lunch time.
    #[serde(default)]
    pub lunch_time: String,
    /// Snack time.
    #[serde(default)]
    pub snack_time: String,
    /// Dinner time.
    #[serde(default)]
    pub dinner_time: String,
    /// Estimated wrap time.
    #[serde(default)]
    pub wrap_time: String,
    /// Optional quote printed on the call sheet.
    #[serde(default)]
    pub quote_of_the_day: String,
    /// Production manager contact override.
    #[serde(default)]
    pub prod_manager_contact: String,
    /// Assistant director contact override.
    #[serde(default)]
    pub ad_contact: String,
    /// Weather temperature.
    #[serde(default)]
    pub weather_temp: String,
    /// Weather summary.
    #[serde(default)]
    pub weather_condition: String,
    /// Precipitation and wind forecast.
    #[serde(default)]
    pub weather_precip_wind: String,
    /// Sunrise and sunset details.
    #[serde(default)]
    pub sun_times: String,
    /// Basecamp location.
    #[serde(default)]
    pub basecamp_location: String,
    /// Nearest hospital or emergency facility.
    #[serde(default)]
    pub nearest_hospital: String,
    /// Per-cast call-sheet rows.
    #[serde(default)]
    pub cast_call_entries: Vec<CastCallEntry>,
    /// Per-crew call-sheet rows.
    #[serde(default)]
    pub crew_call_entries: Vec<CrewCallEntry>,
    /// Production-wide notices shown for the day.
    #[serde(default)]
    pub production_notes: Vec<String>,
    /// Shooting and basecamp locations.
    #[serde(default)]
    pub locations: Vec<Location>,
    /// Optional character-name override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast_override: Option<Vec<String>>,
    /// Legacy text-based crew override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crew_override: Option<Vec<String>>,
    /// Crew roster identifiers selected for the day.
    #[serde(
        rename = "crewIDOverride",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub crew_id_override: Option<Vec<Uuid>>,
    /// One-off crew not in the production roster.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crew_one_offs: Option<Vec<String>>,
    /// General call-sheet notes.
    #[serde(default)]
    pub notes: String,
    /// Fields introduced by newer clients.
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// The saved baseline for cast availability and production days.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleLock {
    /// Timestamp at which the production schedule was locked.
    #[serde(with = "swift_date")]
    pub locked_at: DateTime<Utc>,
    /// Character names mapped to their locked work dates.
    #[serde(default)]
    pub working_days: BTreeMap<String, Vec<DateTime<Utc>>>,
    /// Fields introduced by newer clients.
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Production-wide contacts, rosters, and schedule lock.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionInfo {
    /// Production company name.
    #[serde(default)]
    pub company_name: String,
    /// Director name.
    #[serde(default)]
    pub director_name: String,
    /// Director contact number.
    #[serde(default)]
    pub director_phone: String,
    /// Producer name.
    #[serde(default)]
    pub producer_name: String,
    /// Producer contact number.
    #[serde(default)]
    pub producer_phone: String,
    /// Assistant director name.
    #[serde(default)]
    pub ad_name: String,
    /// Assistant director contact number.
    #[serde(default)]
    pub ad_phone: String,
    /// Primary production contact number.
    #[serde(default)]
    pub contact_number: String,
    /// Default daily lunch time.
    #[serde(default = "default_lunch_time")]
    pub default_lunch_time: String,
    /// Production crew roster.
    #[serde(default)]
    pub crew: Vec<CrewMember>,
    /// Cast and character roster.
    #[serde(default)]
    pub cast_list: Vec<CastMember>,
    /// Reusable production locations.
    #[serde(default)]
    pub location_roster: Vec<Location>,
    /// Optional baseline snapshot for schedule-lock reports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_lock: Option<ScheduleLock>,
    /// Fields introduced by newer clients.
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// A script scene or banner in the production schedule.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    /// Stable identifier used by schedule operations.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// Scene heading or banner label shown in the interface.
    pub title: String,
    /// Script scene number, when available.
    #[serde(default)]
    pub scene_number: String,
    /// Script page length in eighths.
    #[serde(default)]
    pub duration: i32,
    /// Estimated shoot duration in minutes.
    #[serde(default)]
    pub estimated_time: i32,
    /// Existing CineSched time-of-day code, such as `DAY` or `NIGHT`.
    #[serde(default = "default_day_night_type")]
    pub day_night_type: String,
    /// Cast or character names associated with this scene.
    #[serde(default, deserialize_with = "deserialize_cast")]
    pub cast: Vec<String>,
    /// Scene summary or banner note.
    #[serde(default)]
    pub summary: String,
    /// Physical production location name, if different from the scene slugline.
    #[serde(default)]
    pub real_location: String,
    /// Physical production location address.
    #[serde(default)]
    pub location_address: String,
    /// Breakdown categories used by production departments.
    #[serde(default)]
    pub extras: Vec<String>,
    /// Breakdown properties and set-dressing items.
    #[serde(default)]
    pub props: Vec<String>,
    /// Set-dressing items attached to the scene.
    #[serde(default)]
    pub set_dressing: Vec<String>,
    /// Wardrobe requirements attached to the scene.
    #[serde(default)]
    pub wardrobe: Vec<String>,
    /// Makeup and hair requirements attached to the scene.
    #[serde(default)]
    pub makeup_hair: Vec<String>,
    /// Vehicle requirements attached to the scene.
    #[serde(default)]
    pub vehicles: Vec<String>,
    /// Special equipment requirements attached to the scene.
    #[serde(default)]
    pub special_equipment: Vec<String>,
    /// Stunts attached to the scene.
    #[serde(default)]
    pub stunts: Vec<String>,
    /// Special effects attached to the scene.
    #[serde(default)]
    pub sfx: Vec<String>,
    /// Visual effects attached to the scene.
    #[serde(default)]
    pub vfx: Vec<String>,
    /// Production-specific breakdown notes.
    #[serde(default)]
    pub breakdown_notes: String,
    /// True for banners and non-scene strips.
    #[serde(default)]
    pub is_banner: bool,
    /// Optional banner classification used by auto-meals and company moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_type: Option<String>,
    /// Banner display title.
    #[serde(default)]
    pub banner_title: String,
    /// Banner secondary note.
    #[serde(default)]
    pub banner_note: String,
    /// Optional user-selected banner color as an RGB hex string.
    #[serde(default)]
    pub banner_color_hex: String,
    /// Marks a meal banner generated from the production schedule.
    #[serde(default)]
    pub is_auto_meal: bool,
    /// Optional meal milestone classification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meal_kind: Option<String>,
    /// True for calendar events that should not count as script scenes.
    #[serde(default)]
    pub is_calendar_event: bool,
    /// Optional manually assigned start time.
    #[serde(default)]
    pub custom_start_time: String,
    /// True after a scene has been completed.
    #[serde(default)]
    pub is_completed: bool,
    /// Data fields not yet surfaced by the Rust interface.
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl Scene {
    /// Creates a new unscheduled scene with default production metadata.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            scene_number: String::new(),
            duration: 0,
            estimated_time: 0,
            day_night_type: default_day_night_type(),
            cast: Vec::new(),
            summary: String::new(),
            real_location: String::new(),
            location_address: String::new(),
            extras: Vec::new(),
            props: Vec::new(),
            set_dressing: Vec::new(),
            wardrobe: Vec::new(),
            makeup_hair: Vec::new(),
            vehicles: Vec::new(),
            special_equipment: Vec::new(),
            stunts: Vec::new(),
            sfx: Vec::new(),
            vfx: Vec::new(),
            breakdown_notes: String::new(),
            is_banner: false,
            banner_type: None,
            banner_title: String::new(),
            banner_note: String::new(),
            banner_color_hex: String::new(),
            is_auto_meal: false,
            meal_kind: None,
            is_calendar_event: false,
            custom_start_time: String::new(),
            is_completed: false,
            extra: ExtraFields::new(),
        }
    }

    /// Returns the scene number followed by its heading when a number exists.
    pub fn display_title(&self) -> String {
        let number = self.scene_number.trim();
        if number.is_empty() {
            self.title.clone()
        } else {
            format!("{number}. {}", self.title)
        }
    }

    /// Creates a production banner such as a company move or custom notice.
    pub fn create_banner(
        banner_type: impl Into<String>,
        title: impl Into<String>,
        note: impl Into<String>,
        estimated_time: &str,
    ) -> Self {
        let title = title.into();
        let note = note.into();
        let mut scene = Self::new(title.clone());
        scene.estimated_time = crate::parsing::parse_time_to_minutes(estimated_time).unwrap_or(0);
        scene.day_night_type = "CUSTOM".to_owned();
        scene.summary = note.clone();
        scene.is_banner = true;
        scene.banner_type = Some(banner_type.into());
        scene.banner_title = title;
        scene.banner_note = note;
        scene.banner_color_hex = "8B5CF6".to_owned();
        scene
    }

    /// Creates a generated meal or production milestone strip.
    pub fn create_auto_meal(kind: &str, time: &str) -> Self {
        let (title, estimated_time, banner_type, color) = match kind {
            "General Call" => ("GENERAL CALL", 0, "Notice", "1E3A8A"),
            "Ready to Shoot" => ("READY TO SHOOT", 0, "Notice", "064E3B"),
            "Lunch" => ("LUNCH", 60, "Meal Break", "18181B"),
            "Snack" => ("SNACK", 15, "Meal Break", "18181B"),
            "Dinner" => ("DINNER", 60, "Meal Break", "18181B"),
            "Wrap" => ("WRAP", 0, "Notice", "991B1B"),
            _ => ("NOTICE", 0, "Notice", "8B5CF6"),
        };
        let time = time.trim();
        let title = if time.is_empty() {
            title.to_owned()
        } else {
            format!("{title} ({time})")
        };
        let mut scene = Self::create_banner(banner_type, title, time, "0");
        scene.estimated_time = estimated_time;
        scene.banner_color_hex = color.to_owned();
        scene.is_auto_meal = true;
        scene.meal_kind = Some(kind.to_owned());
        scene.custom_start_time = time.to_owned();
        scene
    }

    /// Creates an independent agenda event that is excluded from production totals.
    pub fn create_calendar_event(title: impl Into<String>, time: impl Into<String>) -> Self {
        let title = title.into();
        let time = time.into();
        let mut scene = Self::create_banner("Notice", title.clone(), time.clone(), "0");
        scene.is_calendar_event = true;
        scene.banner_title = title;
        scene.banner_color_hex = "6366F1".to_owned();
        scene.custom_start_time = time;
        scene.estimated_time = 0;
        scene
    }
}

/// A calendar date and its assigned scenes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShootDay {
    /// Stable identifier used by schedule operations.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// Date associated with this shoot day.
    #[serde(with = "swift_date")]
    pub date: DateTime<Utc>,
    /// Scenes assigned to this date.
    #[serde(default)]
    pub scenes: Vec<Scene>,
    /// The production call sheet associated with this shooting date.
    #[serde(default)]
    pub call_sheet: CallSheet,
    /// True when this date is marked unavailable for shooting.
    #[serde(default)]
    pub is_blackout: bool,
    /// Data fields not yet surfaced by the Rust interface.
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl ShootDay {
    /// Creates a shoot day with an empty call sheet and no assigned scenes.
    pub fn new(date: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            date,
            scenes: Vec::new(),
            call_sheet: CallSheet::default(),
            is_blackout: false,
            extra: ExtraFields::new(),
        }
    }

    /// Returns the sum of page eighths for actual script scenes on the day.
    pub fn total_pages_eighths(&self) -> i32 {
        self.scenes
            .iter()
            .filter(|scene| !scene.is_calendar_event)
            .map(|scene| scene.duration)
            .sum()
    }

    /// Returns the sum of estimated minutes for scenes and schedule banners.
    pub fn total_estimated_minutes(&self) -> i32 {
        self.scenes.iter().map(|scene| scene.estimated_time).sum()
    }

    /// Returns unique cast names appearing on the day in sorted order.
    pub fn all_cast(&self) -> Vec<String> {
        self.scenes
            .iter()
            .flat_map(|scene| scene.cast.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

/// The persisted project root used by both the Rust and Swift applications.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Unscheduled scenes shown in the Boneyard.
    #[serde(default)]
    pub all_scenes: Vec<Scene>,
    /// Calendar schedule in production date order.
    #[serde(default)]
    pub shoot_days: Vec<ShootDay>,
    /// Project name.
    #[serde(default = "default_project_title")]
    pub project_title: String,
    /// Creation timestamp.
    #[serde(default = "Utc::now", with = "swift_date")]
    pub created_date: DateTime<Utc>,
    /// Legacy schedule-shift preference, retained for compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_shift_mode_enabled: Option<bool>,
    /// Production contacts, cast, crew, locations, and schedule lock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_info: Option<ProductionInfo>,
    /// Data fields not yet surfaced by the Rust interface.
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            all_scenes: Vec::new(),
            shoot_days: Vec::new(),
            project_title: default_project_title(),
            created_date: Utc::now(),
            is_shift_mode_enabled: Some(false),
            production_info: None,
            extra: ExtraFields::new(),
        }
    }
}

impl Project {
    /// Moves an unscheduled scene into the selected day.
    ///
    /// Returns `false` when either the scene or destination day is missing.
    pub fn schedule_scene(&mut self, scene_id: Uuid, day_id: Uuid) -> bool {
        let Some(scene_index) = self
            .all_scenes
            .iter()
            .position(|scene| scene.id == scene_id)
        else {
            return false;
        };
        let Some(day) = self.shoot_days.iter_mut().find(|day| day.id == day_id) else {
            return false;
        };
        day.scenes.push(self.all_scenes.remove(scene_index));
        true
    }

    /// Moves a scheduled scene back into the unscheduled Boneyard.
    ///
    /// Returns `false` when the scene is not assigned to a shoot day.
    pub fn unschedule_scene(&mut self, scene_id: Uuid) -> bool {
        for day in &mut self.shoot_days {
            if let Some(scene_index) = day.scenes.iter().position(|scene| scene.id == scene_id) {
                self.all_scenes.push(day.scenes.remove(scene_index));
                return true;
            }
        }
        false
    }

    /// Finds a scene by identifier in the Boneyard or any scheduled day.
    pub fn scene(&self, scene_id: Uuid) -> Option<&Scene> {
        self.all_scenes
            .iter()
            .chain(self.shoot_days.iter().flat_map(|day| day.scenes.iter()))
            .find(|scene| scene.id == scene_id)
    }

    /// Finds a mutable scene by identifier in the Boneyard or schedule.
    pub fn scene_mut(&mut self, scene_id: Uuid) -> Option<&mut Scene> {
        if let Some(index) = self
            .all_scenes
            .iter()
            .position(|scene| scene.id == scene_id)
        {
            return self.all_scenes.get_mut(index);
        }
        self.shoot_days
            .iter_mut()
            .flat_map(|day| day.scenes.iter_mut())
            .find(|scene| scene.id == scene_id)
    }

    /// Moves a scene to a destination day, swapping with its previous schedule slot.
    pub fn move_scene(&mut self, scene_id: Uuid, destination_day_id: Uuid) -> bool {
        let Some(destination_index) = self.day_index(destination_day_id) else {
            return false;
        };
        let previous_index = self
            .shoot_days
            .iter()
            .position(|day| day.scenes.iter().any(|item| item.id == scene_id));
        let Some(scene) = self.take_scene(scene_id) else {
            return false;
        };
        if let Some(previous_index) = previous_index {
            if previous_index != destination_index
                && !self.shoot_days[destination_index].scenes.is_empty()
            {
                let displaced = self.shoot_days[destination_index].scenes.remove(0);
                self.shoot_days[previous_index].scenes.push(displaced);
            }
        }
        self.shoot_days[destination_index].scenes.push(scene);
        true
    }

    /// Swaps the scheduled content and call sheets assigned to two calendar dates.
    ///
    /// Shoot-day identifiers, dates, blackout state, and unknown day metadata stay
    /// with their calendar dates. Returns `false` when either day is missing or
    /// both identifiers refer to the same day.
    pub fn swap_day_contents(&mut self, first_day_id: Uuid, second_day_id: Uuid) -> bool {
        let Some(first_index) = self.day_index(first_day_id) else {
            return false;
        };
        let Some(second_index) = self.day_index(second_day_id) else {
            return false;
        };
        if first_index == second_index {
            return false;
        }

        let (first_day, second_day) = if first_index < second_index {
            let (before_second, from_second) = self.shoot_days.split_at_mut(second_index);
            (&mut before_second[first_index], &mut from_second[0])
        } else {
            let (before_first, from_first) = self.shoot_days.split_at_mut(first_index);
            (&mut from_first[0], &mut before_first[second_index])
        };
        std::mem::swap(&mut first_day.scenes, &mut second_day.scenes);
        std::mem::swap(&mut first_day.call_sheet, &mut second_day.call_sheet);
        true
    }

    /// Removes and returns a scene from either schedule location.
    pub fn take_scene(&mut self, scene_id: Uuid) -> Option<Scene> {
        if let Some(index) = self
            .all_scenes
            .iter()
            .position(|scene| scene.id == scene_id)
        {
            return Some(self.all_scenes.remove(index));
        }
        for day in &mut self.shoot_days {
            if let Some(index) = day.scenes.iter().position(|scene| scene.id == scene_id) {
                return Some(day.scenes.remove(index));
            }
        }
        None
    }

    fn day_index(&self, day_id: Uuid) -> Option<usize> {
        self.shoot_days.iter().position(|day| day.id == day_id)
    }

    /// Finds scenes scheduled while their matched cast member is unavailable.
    pub fn scan_conflicts(&self) -> Vec<ScheduleConflict> {
        let Some(production) = self.production_info.as_ref() else {
            return Vec::new();
        };
        let mut conflicts = Vec::new();
        for day in &self.shoot_days {
            for scene in &day.scenes {
                for character in &scene.cast {
                    let Some(member) = production.cast_list.iter().find(|member| {
                        member
                            .character_name
                            .trim()
                            .eq_ignore_ascii_case(character.trim())
                    }) else {
                        continue;
                    };
                    if member
                        .unavailable_ranges
                        .iter()
                        .any(|range| range.contains(day.date))
                    {
                        conflicts.push(ScheduleConflict {
                            date: day.date,
                            scene_id: scene.id,
                            scene_title: scene.display_title(),
                            character: character.clone(),
                            actor_display_name: member.display_string(),
                        });
                    }
                }
            }
        }
        conflicts.sort_by_key(|conflict| conflict.date);
        conflicts
    }

    /// Compares the current cast work dates to the active schedule-lock snapshot.
    pub fn schedule_lock_changes(&self) -> Vec<ScheduleLockChange> {
        let Some(lock) = self
            .production_info
            .as_ref()
            .and_then(|production| production.schedule_lock.as_ref())
        else {
            return Vec::new();
        };
        let mut current: BTreeMap<String, std::collections::BTreeSet<_>> = BTreeMap::new();
        for day in &self.shoot_days {
            for scene in &day.scenes {
                for character in &scene.cast {
                    let name = character.trim();
                    if !name.is_empty() {
                        current
                            .entry(name.to_owned())
                            .or_default()
                            .insert(day.date.date_naive());
                    }
                }
            }
        }
        let characters: std::collections::BTreeSet<_> = current
            .keys()
            .chain(lock.working_days.keys())
            .cloned()
            .collect();
        let mut changes = Vec::new();
        for character in characters {
            let locked: std::collections::BTreeSet<_> = lock
                .working_days
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(&character))
                .map(|(_, dates)| dates.iter().map(|date| date.date_naive()).collect())
                .unwrap_or_default();
            let actual = current.get(&character).cloned().unwrap_or_default();
            let added_days = actual
                .difference(&locked)
                .filter_map(|date| date.and_hms_opt(0, 0, 0))
                .map(|date| DateTime::from_naive_utc_and_offset(date, Utc))
                .collect::<Vec<_>>();
            let removed_days = locked
                .difference(&actual)
                .filter_map(|date| date.and_hms_opt(0, 0, 0))
                .map(|date| DateTime::from_naive_utc_and_offset(date, Utc))
                .collect::<Vec<_>>();
            if added_days.is_empty() && removed_days.is_empty() {
                continue;
            }
            let actor_display_name = self
                .production_info
                .as_ref()
                .and_then(|production| {
                    production.cast_list.iter().find(|member| {
                        member
                            .character_name
                            .trim()
                            .eq_ignore_ascii_case(&character)
                    })
                })
                .map(CastMember::display_string)
                .unwrap_or_else(|| character.clone());
            changes.push(ScheduleLockChange {
                character,
                actor_display_name,
                added_days,
                removed_days,
            });
        }
        changes.sort_by(|left, right| left.actor_display_name.cmp(&right.actor_display_name));
        changes
    }

    /// Locks each cast member's current set of scheduled work dates.
    pub fn lock_schedule(&mut self, locked_at: DateTime<Utc>) {
        let mut working_days: BTreeMap<String, Vec<DateTime<Utc>>> = BTreeMap::new();
        for day in &self.shoot_days {
            for scene in &day.scenes {
                if scene.is_calendar_event {
                    continue;
                }
                for character in &scene.cast {
                    let name = character.trim();
                    if !name.is_empty() {
                        let dates = working_days.entry(name.to_owned()).or_default();
                        if !dates
                            .iter()
                            .any(|date| date.date_naive() == day.date.date_naive())
                        {
                            dates.push(day.date);
                        }
                    }
                }
            }
        }
        for dates in working_days.values_mut() {
            dates.sort();
        }
        let production = self
            .production_info
            .get_or_insert_with(ProductionInfo::default);
        production.schedule_lock = Some(ScheduleLock {
            locked_at,
            working_days,
            extra: ExtraFields::new(),
        });
    }

    /// Removes the active schedule lock and its comparison baseline.
    pub fn unlock_schedule(&mut self) {
        if let Some(production) = self.production_info.as_mut() {
            production.schedule_lock = None;
        }
    }

    /// Returns scene identifiers whose non-empty scene numbers collide.
    pub fn duplicate_scene_number_ids(&self) -> std::collections::BTreeSet<Uuid> {
        let mut by_number: BTreeMap<String, Vec<Uuid>> = BTreeMap::new();
        for scene in self
            .all_scenes
            .iter()
            .chain(self.shoot_days.iter().flat_map(|day| day.scenes.iter()))
        {
            let number = scene.scene_number.trim();
            if number.is_empty() {
                continue;
            }
            let key = number
                .to_ascii_uppercase()
                .trim_start_matches('0')
                .to_owned();
            by_number.entry(key).or_default().push(scene.id);
        }
        by_number
            .into_values()
            .filter(|ids| ids.len() > 1)
            .flatten()
            .collect()
    }
}

/// One conflict between an unavailable cast member and a scheduled scene.
#[derive(Clone, Debug)]
pub struct ScheduleConflict {
    /// Date on which the actor is unavailable.
    pub date: DateTime<Utc>,
    /// Stable identifier of the affected scene.
    pub scene_id: Uuid,
    /// Heading of the affected scene.
    pub scene_title: String,
    /// Character whose actor is unavailable.
    pub character: String,
    /// Actor/character display name from production setup.
    pub actor_display_name: String,
}

/// One character's changed working dates since the schedule was locked.
#[derive(Clone, Debug)]
pub struct ScheduleLockChange {
    /// Character name from the scene cast lists.
    pub character: String,
    /// Actor and character display name.
    pub actor_display_name: String,
    /// Dates added since the schedule was locked.
    pub added_days: Vec<DateTime<Utc>>,
    /// Dates removed since the schedule was locked.
    pub removed_days: Vec<DateTime<Utc>>,
}

/// Deserializes both modern cast arrays and older comma-separated cast strings.
fn deserialize_cast<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Array(values) => values
            .into_iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| serde::de::Error::custom("cast array entries must be strings"))
            })
            .collect(),
        Value::String(value) => Ok(value
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect()),
        Value::Null => Ok(Vec::new()),
        _ => Err(serde::de::Error::custom(
            "cast must be an array of names or a comma-separated string",
        )),
    }
}

/// Produces the JSON object used for an empty Swift call sheet.
/// Provides the initial time-of-day value expected by Swift scene records.
fn default_day_night_type() -> String {
    "DAY".to_owned()
}

/// Provides the default project title for legacy documents without one.
fn default_project_title() -> String {
    "Untitled Movie".to_owned()
}

/// Provides the default production lunch time used by the Swift application.
fn default_lunch_time() -> String {
    "01:30 PM".to_owned()
}

/// Provides the default employment category used by call-sheet cast rows.
fn default_ecdt() -> String {
    "E".to_owned()
}

/// Provides the default first location number used by cast call-sheet rows.
fn default_location_index() -> String {
    "1".to_owned()
}

/// Encodes and decodes dates in the string form used by Swift's formatted encoder.
mod swift_date {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    /// Formats UTC dates with the numeric offset used by the Swift project writer.
    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.format("%Y-%m-%dT%H:%M:%S%z").to_string())
    }

    /// Accepts both numeric-offset Swift timestamps and RFC 3339 timestamps.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&value)
            .or_else(|_| DateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S%z"))
            .map(|date| date.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::Utc;

    use super::{Project, Scene, ShootDay, Uuid};

    /// Confirms current project dates use the application-local Swift format.
    #[test]
    fn date_serialization_uses_swift_offset_style() {
        let day = ShootDay::new(Utc::now());
        let json = serde_json::to_string(&day).expect("day serializes");
        assert!(json.contains("+0000"));
        assert!(serde_json::from_str::<ShootDay>(&json).is_ok());
    }

    /// Confirms unknown root and scene fields survive a decode/encode cycle.
    #[test]
    fn unmodeled_json_fields_are_preserved() {
        let raw = r#"{
            "projectTitle":"Kept",
            "customProject":"value",
            "allScenes":[{
                "id":"00000000-0000-0000-0000-000000000001",
                "title":"INT. HOUSE - DAY",
                "duration":8,
                "estimatedTime":90,
                "dayNightType":"DAY",
                "customScene":true
            }],
            "shootDays":[]
        }"#;

        let project: Project = serde_json::from_str(raw).expect("project deserializes");
        let encoded = serde_json::to_value(project).expect("project serializes");

        assert_eq!(encoded["customProject"], "value");
        assert_eq!(encoded["allScenes"][0]["customScene"], true);
    }

    /// Confirms the project can create and move a scene to the Boneyard.
    #[test]
    fn unscheduling_moves_scene_back_to_boneyard() {
        let mut scene = Scene::new("EXT. PARK - DAY");
        let id = scene.id;
        let mut day = ShootDay::new(Utc::now());
        day.scenes.push(scene.clone());
        scene.title = "Updated title".to_owned();
        let mut project = Project {
            all_scenes: Vec::new(),
            shoot_days: vec![day],
            ..Project::default()
        };

        assert!(project.unschedule_scene(id));
        assert_eq!(project.all_scenes[0].title, "EXT. PARK - DAY");
        assert!(project.shoot_days[0].scenes.is_empty());
    }

    /// Confirms schedule conflict matching is case-insensitive and date-inclusive.
    #[test]
    fn detects_unavailable_cast_on_shoot_day() {
        use chrono::{Duration, Utc};

        use super::{CastMember, DateRange, ProductionInfo};

        let date = Utc::now();
        let mut scene = Scene::new("INT. STAGE - DAY");
        scene.cast = vec!["MAYA".to_owned()];
        let mut day = ShootDay::new(date);
        day.scenes.push(scene);
        let mut project = Project {
            shoot_days: vec![day],
            production_info: Some(ProductionInfo {
                cast_list: vec![CastMember {
                    actor_name: "Alex".to_owned(),
                    character_name: "Maya".to_owned(),
                    unavailable_ranges: vec![DateRange::new(
                        date - Duration::days(1),
                        date + Duration::days(1),
                    )],
                    ..CastMember::default()
                }],
                ..ProductionInfo::default()
            }),
            ..Project::default()
        };

        assert_eq!(project.scan_conflicts().len(), 1);
        project.shoot_days[0].date += Duration::days(2);
        assert!(project.scan_conflicts().is_empty());
    }

    /// Confirms schedule-lock reports both added and removed character dates.
    #[test]
    fn reports_changes_from_schedule_lock() {
        use chrono::{Duration, Utc};

        use super::{ProductionInfo, ScheduleLock};

        let today = Utc::now();
        let mut working_scene = Scene::new("EXT. PARK - DAY");
        working_scene.cast = vec!["MAYA".to_owned()];
        let mut day = ShootDay::new(today);
        day.scenes.push(working_scene);
        let locked_day = today - Duration::days(1);
        let project = Project {
            shoot_days: vec![day],
            production_info: Some(ProductionInfo {
                schedule_lock: Some(ScheduleLock {
                    locked_at: today,
                    working_days: BTreeMap::from([("MAYA".to_owned(), vec![locked_day])]),
                    extra: BTreeMap::new(),
                }),
                ..ProductionInfo::default()
            }),
            ..Project::default()
        };

        let changes = project.schedule_lock_changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].added_days.len(), 1);
        assert_eq!(changes[0].removed_days.len(), 1);
    }

    /// Confirms moving a scheduled scene swaps the destination day's first scene.
    #[test]
    fn moving_scheduled_scene_swaps_destination_scene() {
        use chrono::{Duration, Utc};

        let mut first_day = ShootDay::new(Utc::now());
        let mut moving = Scene::new("SCENE TO MOVE");
        moving.scene_number = "1".to_owned();
        let moving_id = moving.id;
        first_day.scenes.push(moving);
        let mut second_day = ShootDay::new(first_day.date + Duration::days(1));
        let mut displaced = Scene::new("SCENE TO SWAP");
        displaced.scene_number = "2".to_owned();
        let displaced_id = displaced.id;
        second_day.scenes.push(displaced);
        let second_day_id = second_day.id;
        let mut project = Project {
            shoot_days: vec![first_day, second_day],
            ..Project::default()
        };

        assert!(project.move_scene(moving_id, second_day_id));
        assert_eq!(project.shoot_days[1].scenes[0].id, moving_id);
        assert_eq!(project.shoot_days[0].scenes[0].id, displaced_id);
    }

    /// Confirms whole-day swaps move scenes and call sheets but preserve date-specific state.
    #[test]
    fn swapping_day_contents_preserves_calendar_identity_and_blackouts() {
        use chrono::{Duration, Utc};

        let mut first_day = ShootDay::new(Utc::now());
        let first_day_id = first_day.id;
        first_day.is_blackout = true;
        first_day.call_sheet.notes = "First day notes".to_owned();
        let first_scene = Scene::new("FIRST DAY SCENE");
        let first_scene_id = first_scene.id;
        first_day.scenes.push(first_scene);

        let mut second_day = ShootDay::new(first_day.date + Duration::days(1));
        let second_day_id = second_day.id;
        second_day.call_sheet.notes = "Second day notes".to_owned();
        let second_scene = Scene::new("SECOND DAY SCENE");
        let second_scene_id = second_scene.id;
        second_day.scenes.push(second_scene);
        let second_date = second_day.date;

        let mut project = Project {
            shoot_days: vec![first_day, second_day],
            ..Project::default()
        };

        assert!(project.swap_day_contents(first_day_id, second_day_id));
        assert_eq!(project.shoot_days[0].id, first_day_id);
        assert_eq!(project.shoot_days[1].id, second_day_id);
        assert_eq!(project.shoot_days[1].date, second_date);
        assert!(project.shoot_days[0].is_blackout);
        assert!(!project.shoot_days[1].is_blackout);
        assert_eq!(project.shoot_days[0].scenes[0].id, second_scene_id);
        assert_eq!(project.shoot_days[1].scenes[0].id, first_scene_id);
        assert_eq!(project.shoot_days[0].call_sheet.notes, "Second day notes");
        assert_eq!(project.shoot_days[1].call_sheet.notes, "First day notes");
    }

    /// Confirms whole-day swaps reject invalid identifiers and same-day requests.
    #[test]
    fn swapping_day_contents_rejects_missing_or_identical_days() {
        let day = ShootDay::new(Utc::now());
        let day_id = day.id;
        let mut project = Project {
            shoot_days: vec![day],
            ..Project::default()
        };

        assert!(!project.swap_day_contents(day_id, day_id));
        assert!(!project.swap_day_contents(day_id, Uuid::new_v4()));
        assert!(!project.swap_day_contents(Uuid::new_v4(), day_id));
        assert_eq!(project.shoot_days.len(), 1);
    }

    /// Confirms typed call-sheet and production fields decode from the Swift schema.
    #[test]
    fn reads_swift_call_sheet_and_production_records() {
        let json = r#"{
            "allScenes":[],
            "shootDays":[{
                "id":"00000000-0000-0000-0000-000000000010",
                "date":"2025-01-02T03:04:05+0000",
                "scenes":[],
                "callSheet":{
                    "generalCallTime":"07:00 AM",
                    "locations":[{"id":"00000000-0000-0000-0000-000000000011","name":"Stage","address":"Main Road"}],
                    "castCallEntries":[{"id":"00000000-0000-0000-0000-000000000012","characterName":"MAYA","actorName":"Alex"}]
                }
            }],
            "productionInfo":{
                "companyName":"CineSched Pictures",
                "directorName":"Director",
                "crew":[],
                "castList":[{"id":"00000000-0000-0000-0000-000000000013","actorName":"Alex","characterName":"MAYA","unavailableRanges":[]}],
                "locationRoster":[]
            }
        }"#;

        let project: Project = serde_json::from_str(json).expect("Swift production data parses");
        let call_sheet = &project.shoot_days[0].call_sheet;
        assert_eq!(call_sheet.general_call_time, "07:00 AM");
        assert_eq!(call_sheet.locations[0].name, "Stage");
        assert_eq!(call_sheet.cast_call_entries[0].character_name, "MAYA");
        assert_eq!(
            project.production_info.as_ref().unwrap().company_name,
            "CineSched Pictures"
        );
    }

    /// Confirms generated meal strips retain their timing and banner metadata.
    #[test]
    fn generated_meal_strips_keep_milestone_values() {
        let lunch = Scene::create_auto_meal("Lunch", "01:30 PM");
        assert_eq!(lunch.banner_title, "LUNCH (01:30 PM)");
        assert_eq!(lunch.estimated_time, 60);
        assert_eq!(lunch.banner_type.as_deref(), Some("Meal Break"));
        assert!(lunch.is_auto_meal);

        let event = Scene::create_calendar_event("Rehearsal", "10:00 AM");
        assert!(event.is_calendar_event);
        assert!(!event.is_auto_meal);
        assert_eq!(event.estimated_time, 0);
    }
}
