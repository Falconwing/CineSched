//! Portable printable schedule, call-sheet, breakdown, and DOOD reports.

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;

use chrono::Datelike;
use thiserror::Error;
use uuid::Uuid;

use crate::model::{Project, Scene, ShootDay};
use crate::parsing::parse_time_to_minutes;

/// Errors encountered while creating a production report.
#[derive(Debug, Error)]
pub enum ReportError {
    /// The report could not be written to the selected destination.
    #[error("could not write report: {0}")]
    Io(#[from] std::io::Error),
    /// The requested shooting day does not exist in this project.
    #[error("the selected shooting day does not exist")]
    MissingDay,
}

/// Writes a landscape PDF shooting schedule.
pub fn export_schedule_pdf(
    project: &Project,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    let mut lines = vec![
        project.project_title.clone(),
        "SHOOTING SCHEDULE".to_owned(),
    ];
    for day in &project.shoot_days {
        lines.push(format!(
            "{}  |  {} scenes  |  {} pages  |  {} min",
            day.date.format("%a %b %d, %Y"),
            day.scenes
                .iter()
                .filter(|scene| !scene.is_calendar_event)
                .count(),
            format_eighths(day.total_pages_eighths()),
            day.total_estimated_minutes()
        ));
        append_scene_lines(&mut lines, &day.scenes);
    }
    write_pdf(destination, &lines)
}

/// Writes a calendar-style month summary PDF.
pub fn export_month_calendar_pdf(
    project: &Project,
    year: i32,
    month: u32,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    let mut lines = vec![
        project.project_title.clone(),
        format!("PRODUCTION CALENDAR - {year:04}-{month:02}"),
        "Sunday | Monday | Tuesday | Wednesday | Thursday | Friday | Saturday".to_owned(),
    ];
    let days: Vec<&ShootDay> = project
        .shoot_days
        .iter()
        .filter(|day| day.date.year() == year && day.date.month() == month)
        .collect();
    for day in days {
        lines.push(format!(
            "{}  |  {} scenes  |  {} pages  |  {}",
            day.date.format("%a %b %d"),
            day.scenes
                .iter()
                .filter(|scene| !scene.is_calendar_event)
                .count(),
            format_eighths(day.total_pages_eighths()),
            day.call_sheet.general_call_time
        ));
        append_scene_lines(&mut lines, &day.scenes);
    }
    write_pdf(destination, &lines)
}

/// Writes a stripboard-oriented PDF with one row per scheduled scene.
pub fn export_stripboard_pdf(
    project: &Project,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    let mut lines = vec![
        project.project_title.clone(),
        "STRIPBOARD".to_owned(),
        "Shoot Day | Scene | Type | Pages | Cast | Estimate".to_owned(),
    ];
    for day in &project.shoot_days {
        for scene in &day.scenes {
            lines.push(format!(
                "{} | {} | {} | {} | {} | {} min{}",
                day.date.format("%Y-%m-%d"),
                scene.display_title(),
                scene.day_night_type,
                format_eighths(scene.duration),
                scene.cast.join(", "),
                scene.estimated_time,
                if scene.is_completed { " | DONE" } else { "" }
            ));
        }
    }
    write_pdf(destination, &lines)
}

/// Writes a dynamic one-line schedule PDF using call time and scene estimates.
pub fn export_timeline_pdf(
    project: &Project,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    let mut lines = vec![
        project.project_title.clone(),
        "ONE-LINE SHOOTING SCHEDULE".to_owned(),
    ];
    for day in &project.shoot_days {
        let mut current_minutes =
            parse_call_time(&day.call_sheet.general_call_time).unwrap_or(7 * 60);
        lines.push(format!(
            "{} - CALL {}",
            day.date.format("%A, %B %d, %Y"),
            clock_label(current_minutes)
        ));
        for scene in &day.scenes {
            if !scene.custom_start_time.is_empty() {
                if let Some(time) = parse_call_time(&scene.custom_start_time) {
                    current_minutes = time;
                }
            }
            let start = current_minutes;
            current_minutes += scene.estimated_time.max(0);
            lines.push(format!(
                "{} | {} | {} | {} | {}",
                clock_label(start),
                scene.display_title(),
                scene.cast.join(", "),
                format_eighths(scene.duration),
                clock_label(current_minutes)
            ));
        }
        lines.push(format!("END OF DAY - {}", clock_label(current_minutes)));
    }
    write_pdf(destination, &lines)
}

/// Writes a US Letter call sheet for one production date.
pub fn export_call_sheet_pdf(
    project: &Project,
    day_id: Uuid,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    let day = project
        .shoot_days
        .iter()
        .find(|day| day.id == day_id)
        .ok_or(ReportError::MissingDay)?;
    let call_sheet = &day.call_sheet;
    let mut lines = vec![
        project.project_title.clone(),
        format!("CALL SHEET - {}", day.date.format("%A, %B %d, %Y")),
        format!("General Call: {}", call_sheet.general_call_time),
        format!("Workday Schedule: {}", call_sheet.work_day_schedule),
        format!("Quote of the Day: {}", call_sheet.quote_of_the_day),
        format!("Ready to Shoot: {}", call_sheet.ready_to_shoot_time),
        format!("Lunch: {}", call_sheet.lunch_time),
        format!("Snack: {}", call_sheet.snack_time),
        format!("Dinner: {}", call_sheet.dinner_time),
        format!("Wrap: {}", call_sheet.wrap_time),
        format!("Production Manager: {}", call_sheet.prod_manager_contact),
        format!("Assistant Director: {}", call_sheet.ad_contact),
        format!("Weather: {}", call_sheet.weather_condition),
        format!("Temperature: {}", call_sheet.weather_temp),
        format!("Precipitation / Wind: {}", call_sheet.weather_precip_wind),
        format!("Sunrise / Sunset: {}", call_sheet.sun_times),
        format!("Basecamp: {}", call_sheet.basecamp_location),
        format!("Nearest Hospital: {}", call_sheet.nearest_hospital),
        "LOCATIONS".to_owned(),
    ];
    lines.extend(
        call_sheet
            .locations
            .iter()
            .map(|location| format!("{} - {}", location.name, location.address)),
    );
    lines.push("CAST CALLS".to_owned());
    lines.extend(call_sheet.cast_call_entries.iter().map(|entry| {
        format!(
            "{} / {} / Call {} / Scenes {}",
            entry.actor_name, entry.character_name, entry.pickup_time, entry.scene_numbers
        )
    }));
    lines.extend(call_sheet.cast_call_entries.iter().map(|entry| {
        format!(
            "  ECDT {} / HMU-Wardrobe {} / On Set {} / Wrap {} / Location {}",
            entry.ecdt,
            entry.hmu_wardrobe_time,
            entry.on_set_time,
            entry.wrap_time,
            entry.location_index
        )
    }));
    lines.push("CREW CALLS".to_owned());
    lines.extend(call_sheet.crew_call_entries.iter().map(|entry| {
        format!(
            "{} - {} / Call {} / {}",
            entry.name, entry.role, entry.call_time, entry.phone
        )
    }));
    lines.extend(
        call_sheet
            .production_notes
            .iter()
            .map(|note| format!("Production Note: {note}")),
    );
    lines.push(format!("Notes: {}", call_sheet.notes));
    append_scene_lines(&mut lines, &day.scenes);
    write_pdf(destination, &lines)
}

/// Writes scene breakdown details for every scheduled and unscheduled scene.
pub fn export_breakdowns_pdf(
    project: &Project,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    let mut scenes: Vec<&Scene> = project
        .all_scenes
        .iter()
        .chain(project.shoot_days.iter().flat_map(|day| day.scenes.iter()))
        .filter(|scene| !scene.is_calendar_event)
        .collect();
    scenes.sort_by(|left, right| {
        scene_order(left)
            .cmp(&scene_order(right))
            .then_with(|| left.title.cmp(&right.title))
    });
    let mut lines = vec![project.project_title.clone(), "SCENE BREAKDOWNS".to_owned()];
    for scene in scenes {
        lines.push(format!(
            "{} - {}",
            scene.display_title(),
            scene.day_night_type
        ));
        lines.push(format!("Location: {}", scene.real_location));
        lines.push(format!("Address: {}", scene.location_address));
        lines.push(format!("Cast: {}", scene.cast.join(", ")));
        lines.push(format!("Summary: {}", scene.summary));
        for (label, items) in [
            ("Extras", &scene.extras),
            ("Props", &scene.props),
            ("Set Dressing", &scene.set_dressing),
            ("Wardrobe", &scene.wardrobe),
            ("Makeup/Hair", &scene.makeup_hair),
            ("Vehicles", &scene.vehicles),
            ("Special Equipment", &scene.special_equipment),
            ("Stunts", &scene.stunts),
            ("SFX", &scene.sfx),
            ("VFX", &scene.vfx),
        ] {
            if !items.is_empty() {
                lines.push(format!("{label}: {}", items.join(", ")));
            }
        }
        lines.push(format!("Notes: {}", scene.breakdown_notes));
        lines.push(String::new());
    }
    write_pdf(destination, &lines)
}

/// Writes the cast work/break status report for all production days.
pub fn export_days_out_of_days_pdf(
    project: &Project,
    include_hold: bool,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    let days: Vec<&ShootDay> = project
        .shoot_days
        .iter()
        .filter(|day| day.scenes.iter().any(|scene| !scene.is_calendar_event))
        .collect();
    let mut characters: Vec<String> = project
        .production_info
        .as_ref()
        .map(|production| {
            production
                .cast_list
                .iter()
                .map(|member| member.character_name.clone())
                .collect()
        })
        .unwrap_or_default();
    if characters.is_empty() {
        characters = days.iter().flat_map(|day| day.all_cast()).collect();
        characters.sort();
        characters.dedup();
    }
    let mut lines = vec![project.project_title.clone(), "DAYS OUT OF DAYS".to_owned()];
    lines.push(format!(
        "Character | {}",
        days.iter()
            .map(|day| day.date.format("%m/%d").to_string())
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    for character in characters {
        let appearance_indices: Vec<usize> = days
            .iter()
            .enumerate()
            .filter(|(_, day)| {
                day.scenes.iter().any(|scene| {
                    !scene.is_calendar_event
                        && scene
                            .cast
                            .iter()
                            .any(|name| name.eq_ignore_ascii_case(&character))
                })
            })
            .map(|(index, _)| index)
            .collect();
        let first = appearance_indices.first().copied();
        let last = appearance_indices.last().copied();
        let cells = days
            .iter()
            .enumerate()
            .map(|(index, day)| {
                let works = day.scenes.iter().any(|scene| {
                    !scene.is_calendar_event
                        && scene
                            .cast
                            .iter()
                            .any(|name| name.eq_ignore_ascii_case(&character))
                });
                if works {
                    match (first == Some(index), last == Some(index)) {
                        (true, true) => "SWF",
                        (true, false) => "SW",
                        (false, true) => "WF",
                        (false, false) => "W",
                    }
                } else if include_hold
                    && first.is_some_and(|first| index > first)
                    && last.is_some_and(|last| index < last)
                {
                    "H"
                } else {
                    "X"
                }
            })
            .collect::<Vec<_>>();
        lines.push(format!("{character} | {}", cells.join(" | ")));
    }
    write_pdf(destination, &lines)
}

/// Creates a minimal standards-compliant PDF using its built-in Helvetica font.
fn write_pdf(destination: impl AsRef<Path>, lines: &[String]) -> Result<(), ReportError> {
    let page_lines = 44;
    let page_count = lines.len().max(1).div_ceil(page_lines);
    let mut objects = vec![String::new(); 3 + page_count * 2];
    let page_object_ids: Vec<usize> = (0..page_count).map(|index| 4 + index * 2).collect();
    objects[0] = "<< /Type /Catalog /Pages 2 0 R >>".to_owned();
    objects[1] = format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        page_object_ids
            .iter()
            .map(|id| format!("{id} 0 R"))
            .collect::<Vec<_>>()
            .join(" "),
        page_count
    );
    objects[2] = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned();

    for (page_index, page_id) in page_object_ids.iter().copied().enumerate() {
        let content_id = page_id + 1;
        objects[page_id - 1] = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 792 612] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>"
        );
        let start = page_index * page_lines;
        let end = (start + page_lines).min(lines.len());
        let mut content = String::from("BT /F1 10 Tf 40 572 Td 14 TL\n");
        for line in &lines[start..end] {
            let printable = pdf_escape(line);
            writeln!(content, "({printable}) Tj T*").expect("writing to string cannot fail");
        }
        content.push_str("ET");
        objects[content_id - 1] = format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            content
        );
    }

    let mut document = String::from("%PDF-1.4\n");
    let mut offsets = vec![0usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(document.len());
        writeln!(document, "{} 0 obj\n{}\nendobj", index + 1, object)
            .expect("writing to string cannot fail");
    }
    let xref_offset = document.len();
    writeln!(document, "xref\n0 {}\n0000000000 65535 f ", offsets.len())
        .expect("writing to string cannot fail");
    for offset in offsets.iter().skip(1) {
        writeln!(document, "{offset:010} 00000 n ").expect("writing to string cannot fail");
    }
    write!(
        document,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
        offsets.len()
    )
    .expect("writing to string cannot fail");
    fs::write(destination, document.as_bytes())?;
    Ok(())
}

/// Escapes a report line for PDF's literal-string syntax and basic font.
fn pdf_escape(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '(' => "\\(".to_owned(),
            ')' => "\\)".to_owned(),
            '\\' => "\\\\".to_owned(),
            '\n' | '\r' => " ".to_owned(),
            character if character.is_ascii() => character.to_string(),
            _ => "?".to_owned(),
        })
        .collect()
}

/// Orders numbered scenes by numeric and alphabetic scene-number components.
fn scene_order(scene: &Scene) -> (u32, String) {
    let number = scene.scene_number.trim();
    let numeric: String = number.chars().take_while(char::is_ascii_digit).collect();
    let suffix: String = number
        .chars()
        .skip(numeric.len())
        .take_while(char::is_ascii_alphabetic)
        .collect();
    (
        numeric.parse().unwrap_or(u32::MAX),
        suffix.to_ascii_uppercase(),
    )
}

/// Adds scene headings and their cast to a report.
fn append_scene_lines(lines: &mut Vec<String>, scenes: &[Scene]) {
    for scene in scenes.iter().filter(|scene| !scene.is_calendar_event) {
        lines.push(format!(
            "  {}  {}  {}  {}",
            scene.display_title(),
            scene.day_night_type,
            format_eighths(scene.duration),
            scene.cast.join(", ")
        ));
    }
}

/// Formats eighths of a screenplay page.
fn format_eighths(eighths: i32) -> String {
    let pages = eighths / 8;
    let remainder = eighths % 8;
    match (pages, remainder) {
        (0, 0) => "0".to_owned(),
        (0, fraction) => format!("{fraction}/8"),
        (whole, 0) => whole.to_string(),
        (whole, fraction) => format!("{whole} {fraction}/8"),
    }
}

/// Parses the 12-hour call-time strings stored in project files.
fn parse_call_time(value: &str) -> Option<i32> {
    let upper = value.trim().to_ascii_uppercase();
    let is_pm = upper.ends_with("PM");
    let is_am = upper.ends_with("AM");
    let time = upper.trim_end_matches("AM").trim_end_matches("PM").trim();
    let mut minutes = parse_time_to_minutes(time)?;
    if is_pm && minutes / 60 < 12 {
        minutes += 12 * 60;
    } else if is_am && minutes / 60 == 12 {
        minutes -= 12 * 60;
    }
    Some(minutes)
}

/// Formats minutes after midnight into a conventional 12-hour call time.
fn clock_label(minutes: i32) -> String {
    let wrapped = minutes.rem_euclid(24 * 60);
    let hour = wrapped / 60;
    let minute = wrapped % 60;
    let marker = if hour >= 12 { "PM" } else { "AM" };
    let hour = match hour % 12 {
        0 => 12,
        value => value,
    };
    format!("{hour:02}:{minute:02} {marker}")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::{Datelike, Duration, Utc};
    use uuid::Uuid;

    use crate::model::{Project, Scene, ShootDay};

    use super::{
        export_call_sheet_pdf, export_days_out_of_days_pdf, export_month_calendar_pdf,
        export_schedule_pdf, export_stripboard_pdf, export_timeline_pdf, parse_call_time,
    };

    /// Confirms schedule reports are emitted as valid multi-object PDF documents.
    #[test]
    fn writes_schedule_pdf() {
        let mut day = ShootDay::new(Utc::now());
        let mut scene = Scene::new("INT. STAGE - DAY");
        scene.scene_number = "1".to_owned();
        scene.duration = 8;
        scene.cast = vec!["MAYA".to_owned()];
        day.scenes.push(scene);
        let project = Project {
            project_title: "PDF Test".to_owned(),
            shoot_days: vec![day],
            ..Project::default()
        };
        let path = std::env::temp_dir().join(format!("cinesched-{}.pdf", Uuid::new_v4()));

        export_schedule_pdf(&project, &path).expect("schedule PDF is written");
        let bytes = fs::read(&path).expect("PDF exists");
        fs::remove_file(path).expect("test PDF is removed");

        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(String::from_utf8_lossy(&bytes).contains("INT. STAGE - DAY"));
    }

    /// Confirms call-sheet exports include the expanded schedule, weather, and cast details.
    #[test]
    fn exports_expanded_call_sheet_details() {
        use crate::model::{CastCallEntry, CrewCallEntry, Location};

        let mut day = ShootDay::new(Utc::now());
        day.call_sheet.work_day_schedule = "07:00 AM to 06:00 PM".to_owned();
        day.call_sheet.quote_of_the_day = "Stay on schedule".to_owned();
        day.call_sheet.prod_manager_contact = "555-0100".to_owned();
        day.call_sheet.weather_precip_wind = "Rain 10%, wind 5 mph".to_owned();
        day.call_sheet.sun_times = "Sunrise 06:30 AM".to_owned();
        day.call_sheet.production_notes = vec!["Safety meeting at basecamp".to_owned()];
        day.call_sheet.locations.push(Location {
            name: "Stage 4".to_owned(),
            address: "100 Studio Road".to_owned(),
            ..Location::default()
        });
        day.call_sheet.cast_call_entries.push(CastCallEntry {
            actor_name: "Alex".to_owned(),
            character_name: "MAYA".to_owned(),
            scene_numbers: "1A, 4".to_owned(),
            ecdt: "ET".to_owned(),
            hmu_wardrobe_time: "07:15 AM".to_owned(),
            on_set_time: "08:00 AM".to_owned(),
            wrap_time: "05:00 PM".to_owned(),
            location_index: "2".to_owned(),
            ..CastCallEntry::default()
        });
        day.call_sheet.crew_call_entries.push(CrewCallEntry {
            name: "Sam".to_owned(),
            role: "Camera".to_owned(),
            call_time: "06:45 AM".to_owned(),
            phone: "555-0101".to_owned(),
            ..CrewCallEntry::default()
        });
        let day_id = day.id;
        let project = Project {
            project_title: "Call Sheet Test".to_owned(),
            shoot_days: vec![day],
            ..Project::default()
        };
        let path = std::env::temp_dir().join(format!("cinesched-{}.pdf", Uuid::new_v4()));

        export_call_sheet_pdf(&project, day_id, &path).expect("call sheet PDF is written");
        let report = fs::read_to_string(&path).expect("PDF is readable");
        fs::remove_file(path).expect("test PDF is removed");

        for expected in [
            "07:00 AM to 06:00 PM",
            "Stay on schedule",
            "555-0100",
            "Rain 10%, wind 5 mph",
            "Sunrise 06:30 AM",
            "Stage 4 - 100 Studio Road",
            "ET / HMU-Wardrobe 07:15 AM / On Set 08:00 AM / Wrap 05:00 PM / Location 2",
            "Safety meeting at basecamp",
        ] {
            assert!(
                report.contains(expected),
                "missing call-sheet text: {expected}"
            );
        }
    }

    /// Confirms DOOD reports distinguish work, hold, and finish days.
    #[test]
    fn exports_days_out_of_days_pdf() {
        let mut first = ShootDay::new(Utc::now());
        let mut scene = Scene::new("INT. STAGE - DAY");
        scene.cast = vec!["MAYA".to_owned()];
        first.scenes.push(scene.clone());
        let second = ShootDay::new(first.date + Duration::days(1));
        let mut third = ShootDay::new(first.date + Duration::days(2));
        third.scenes.push(scene);
        let project = Project {
            shoot_days: vec![first, second, third],
            ..Project::default()
        };
        let path = std::env::temp_dir().join(format!("cinesched-{}.pdf", Uuid::new_v4()));

        export_days_out_of_days_pdf(&project, true, &path)
            .expect("days-out-of-days PDF is written");
        let report = fs::read_to_string(&path).expect("PDF is readable");
        fs::remove_file(path).expect("test PDF is removed");

        assert!(report.contains("SW"));
        assert!(report.contains("H"));
        assert!(report.contains("WF"));
    }

    /// Confirms schedule and monthly exporters create PDFs from real project days.
    #[test]
    fn exports_calendar_stripboard_and_timeline_pdfs() {
        let mut day = ShootDay::new(Utc::now());
        day.call_sheet.general_call_time = "07:00 AM".to_owned();
        let mut scene = Scene::new("EXT. PARK - NIGHT");
        scene.duration = 4;
        scene.estimated_time = 75;
        scene.custom_start_time = "08:00 AM".to_owned();
        day.scenes.push(scene);
        let project = Project {
            shoot_days: vec![day],
            ..Project::default()
        };
        let month_path = std::env::temp_dir().join(format!("cinesched-{}.pdf", Uuid::new_v4()));
        let stripboard_path =
            std::env::temp_dir().join(format!("cinesched-{}.pdf", Uuid::new_v4()));
        let timeline_path = std::env::temp_dir().join(format!("cinesched-{}.pdf", Uuid::new_v4()));

        export_month_calendar_pdf(
            &project,
            project.shoot_days[0].date.year(),
            project.shoot_days[0].date.month(),
            &month_path,
        )
        .expect("calendar report is written");
        export_stripboard_pdf(&project, &stripboard_path).expect("stripboard report is written");
        export_timeline_pdf(&project, &timeline_path).expect("timeline report is written");
        let timeline = fs::read_to_string(&timeline_path).expect("timeline PDF is readable");
        for path in [month_path, stripboard_path, timeline_path] {
            assert!(fs::read(&path)
                .expect("PDF exists")
                .starts_with(b"%PDF-1.4"));
            fs::remove_file(&path).expect("test PDF is removed");
        }

        assert!(timeline.contains("08:00 AM"));
        assert!(timeline.contains("09:15 AM"));
        assert_eq!(parse_call_time("01:30 PM"), Some(810));
        assert_eq!(parse_call_time("12:15 AM"), Some(15));
    }
}
