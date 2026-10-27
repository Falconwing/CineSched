//! Portable printable schedule, call-sheet, breakdown, and DOOD reports.

use std::path::Path;

use thiserror::Error;
use uuid::Uuid;

use crate::localization::Language;
use crate::model::Project;

#[cfg(test)]
use crate::parsing::parse_time_to_minutes;
#[cfg(test)]
use std::fmt::Write as FmtWrite;
#[cfg(test)]
use std::fs;

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
    export_schedule_pdf_localized(project, Language::English, destination)
}

/// Writes the vector schedule layout with localized report labels.
pub fn export_schedule_pdf_localized(
    project: &Project,
    language: Language,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    crate::vector_pdf::schedule(project, language, destination)?;
    Ok(())
}

/// Writes a calendar-style month summary PDF.
pub fn export_month_calendar_pdf(
    project: &Project,
    year: i32,
    month: u32,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    export_month_calendar_pdf_localized(project, year, month, Language::English, destination)
}

/// Writes a localized, landscape monthly calendar with vector scene strips.
pub fn export_month_calendar_pdf_localized(
    project: &Project,
    year: i32,
    month: u32,
    language: Language,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    crate::vector_pdf::month_calendar(project, year, month, language, destination)?;
    Ok(())
}

/// Writes a stripboard-oriented PDF with one row per scheduled scene.
pub fn export_stripboard_pdf(
    project: &Project,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    export_stripboard_pdf_localized(project, Language::English, destination)
}

/// Writes the color-coded stripboard layout using the selected report language.
pub fn export_stripboard_pdf_localized(
    project: &Project,
    language: Language,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    crate::vector_pdf::stripboard(project, language, destination)?;
    Ok(())
}

/// Writes a dynamic one-line schedule PDF using call time and scene estimates.
pub fn export_timeline_pdf(
    project: &Project,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    export_timeline_pdf_localized(project, Language::English, destination)
}

/// Writes the time-cascading one-line schedule in the requested language.
pub fn export_timeline_pdf_localized(
    project: &Project,
    language: Language,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    crate::vector_pdf::timeline(project, language, destination)?;
    Ok(())
}

/// Writes a US Letter call sheet for one production date.
pub fn export_call_sheet_pdf(
    project: &Project,
    day_id: Uuid,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    export_call_sheet_pdf_localized(project, day_id, Language::English, destination)
}

/// Writes the portrait call-sheet design with translated section and table labels.
pub fn export_call_sheet_pdf_localized(
    project: &Project,
    day_id: Uuid,
    language: Language,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    crate::vector_pdf::call_sheet(project, day_id, language, destination).map_err(|error| {
        match error {
            crate::vector_pdf::CallSheetPdfError::MissingDay => ReportError::MissingDay,
            crate::vector_pdf::CallSheetPdfError::Io(error) => ReportError::Io(error),
        }
    })
}

/// Writes scene breakdown details for every scheduled and unscheduled scene.
pub fn export_breakdowns_pdf(
    project: &Project,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    export_breakdowns_pdf_localized(project, Language::English, destination)
}

/// Writes color-coded breakdown forms with localized department headings.
pub fn export_breakdowns_pdf_localized(
    project: &Project,
    language: Language,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    crate::vector_pdf::breakdowns(project, language, destination)?;
    Ok(())
}

/// Writes the cast work/break status report for all production days.
pub fn export_days_out_of_days_pdf(
    project: &Project,
    include_hold: bool,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    export_days_out_of_days_pdf_localized(project, include_hold, Language::English, destination)
}

/// Writes the color-coded DOOD matrix, including translated headings and legend.
pub fn export_days_out_of_days_pdf_localized(
    project: &Project,
    include_hold: bool,
    language: Language,
    destination: impl AsRef<Path>,
) -> Result<(), ReportError> {
    crate::vector_pdf::days_out_of_days(project, include_hold, language, destination)?;
    Ok(())
}

/// Creates a landscape letter PDF with repeated headers, wrapped text, and page numbers.
#[cfg(test)]
fn write_pdf(destination: impl AsRef<Path>, lines: &[String]) -> Result<(), ReportError> {
    write_pdf_sized(destination, lines, 792, 612)
}

/// Creates a paginated letter PDF while keeping report-specific data in readable rows.
#[cfg(test)]
fn write_pdf_sized(
    destination: impl AsRef<Path>,
    lines: &[String],
    page_width: usize,
    page_height: usize,
) -> Result<(), ReportError> {
    let title = lines.first().map_or("CineSched", String::as_str);
    let subtitle = lines.get(1).map_or("", String::as_str);
    let body = lines.get(2..).unwrap_or_default();
    let character_limit = if page_width > page_height { 112 } else { 82 };
    let wrapped_lines: Vec<String> = body
        .iter()
        .flat_map(|line| wrap_report_line(line, character_limit))
        .collect();
    let line_height = 13;
    let content_top = page_height - 90;
    let content_bottom = 42;
    let page_lines = ((content_top - content_bottom) / line_height).max(1);
    let page_count = wrapped_lines.len().max(1).div_ceil(page_lines);
    let mut objects = vec![String::new(); 4 + page_count * 2];
    let page_object_ids: Vec<usize> = (0..page_count).map(|index| 5 + index * 2).collect();
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
    objects[2] =
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned();
    objects[3] =
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .to_owned();

    for (page_index, page_id) in page_object_ids.iter().copied().enumerate() {
        let content_id = page_id + 1;
        objects[page_id - 1] = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_width} {page_height}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {content_id} 0 R >>"
        );
        let start = page_index * page_lines;
        let end = (start + page_lines).min(wrapped_lines.len());
        let mut content = String::new();
        let title_y = page_height - 38;
        let subtitle_y = page_height - 59;
        writeln!(
            content,
            "BT /F2 16 Tf 40 {title_y} Td ({}) Tj ET",
            pdf_escape(title)
        )
        .expect("writing to string cannot fail");
        writeln!(
            content,
            "BT /F1 9 Tf 40 {subtitle_y} Td ({}) Tj ET",
            pdf_escape(subtitle)
        )
        .expect("writing to string cannot fail");
        writeln!(
            content,
            "0.7 w 40 {} m {} {} l S",
            page_height - 70,
            page_width - 40,
            page_height - 70
        )
        .expect("writing to string cannot fail");
        for (line_index, line) in wrapped_lines[start..end].iter().enumerate() {
            let y = content_top - line_index * line_height;
            let font = if is_report_heading(line) { "F2" } else { "F1" };
            let x = if line.starts_with("  ") { 52 } else { 40 };
            let size = 9;
            let printable = pdf_escape(line);
            writeln!(
                content,
                "BT /{font} {size} Tf {x} {y} Td ({printable}) Tj ET"
            )
            .expect("writing to string cannot fail");
        }
        let footer_y = 25;
        writeln!(
            content,
            "BT /F1 8 Tf {} {footer_y} Td (Page {} of {}) Tj ET",
            page_width - 85,
            page_index + 1,
            page_count
        )
        .expect("writing to string cannot fail");
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

/// Wraps a report row to the selected page width without splitting words.
#[cfg(test)]
fn wrap_report_line(line: &str, character_limit: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let indentation = line
        .chars()
        .take_while(|character| character.is_whitespace())
        .count();
    let prefix = " ".repeat(indentation.min(character_limit.saturating_sub(1)));
    let mut output = Vec::new();
    let mut current = prefix.clone();
    for word in line.split_whitespace() {
        let separator = usize::from(current.trim() != "");
        if current.chars().count() + separator + word.chars().count() > character_limit
            && !current.trim().is_empty()
        {
            output.push(current);
            current = prefix.clone();
        }
        if current.trim().is_empty() {
            current.push_str(word);
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }
    if !current.trim().is_empty() {
        output.push(current);
    }
    output
}

/// Recognizes standalone uppercase section labels for bold PDF rendering.
#[cfg(test)]
fn is_report_heading(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && !trimmed.contains(':')
        && trimmed
            .chars()
            .any(|character| character.is_ascii_alphabetic())
        && trimmed
            .chars()
            .filter(|character| character.is_ascii_alphabetic())
            .all(|character| character.is_ascii_uppercase())
}

/// Escapes a report line using WinAnsi bytes supported by standard PDF fonts.
#[cfg(test)]
fn pdf_escape(value: &str) -> String {
    value
        .chars()
        .map(|character| match win_ansi_byte(character) {
            b'(' => "\\(".to_owned(),
            b')' => "\\)".to_owned(),
            b'\\' => "\\\\".to_owned(),
            byte if (32..127).contains(&byte) => char::from(byte).to_string(),
            byte => format!("\\{byte:03o}"),
        })
        .collect()
}

/// Maps Latin-1 and common typographic characters to standard WinAnsi encoding.
#[cfg(test)]
fn win_ansi_byte(character: char) -> u8 {
    let code = character as u32;
    if (0x20..=0x7e).contains(&code) || (0xa0..=0xff).contains(&code) {
        return code as u8;
    }
    match character {
        '€' => 0x80,
        '‚' => 0x82,
        'ƒ' => 0x83,
        '„' => 0x84,
        '…' => 0x85,
        '†' => 0x86,
        '‡' => 0x87,
        'ˆ' => 0x88,
        '‰' => 0x89,
        'Š' => 0x8a,
        '‹' => 0x8b,
        'Œ' => 0x8c,
        'Ž' => 0x8e,
        '\u{2018}' => 0x91,
        '\u{2019}' => 0x92,
        '\u{201C}' => 0x93,
        '\u{201D}' => 0x94,
        '•' => 0x95,
        '\u{2013}' => 0x96,
        '\u{2014}' => 0x97,
        '˜' => 0x98,
        '™' => 0x99,
        'š' => 0x9a,
        '›' => 0x9b,
        'œ' => 0x9c,
        'ž' => 0x9e,
        'Ÿ' => 0x9f,
        _ => b'?',
    }
}

/// Parses the 12-hour call-time strings stored in project files.
#[cfg(test)]
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

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::{Datelike, Duration, Utc};
    use uuid::Uuid;

    use crate::localization::Language;
    use crate::model::{Project, Scene, ShootDay};

    use super::{
        export_call_sheet_pdf, export_days_out_of_days_pdf, export_month_calendar_pdf,
        export_month_calendar_pdf_localized, export_schedule_pdf, export_stripboard_pdf,
        export_stripboard_pdf_localized, export_timeline_pdf, parse_call_time, pdf_escape,
        write_pdf, ReportError,
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
        let report = String::from_utf8_lossy(&bytes);
        assert!(report.contains("INT. STAGE - DAY"));
        assert!(report.contains("/MediaBox [0 0 792 612]"));
    }

    /// Confirms generated reports paginate, repeat page metadata, and encode Spanish text.
    #[test]
    fn report_pdf_wraps_and_paginates_with_winansi_text() {
        let path = std::env::temp_dir().join(format!("cinesched-{}.pdf", Uuid::new_v4()));
        let mut lines = vec!["CineSched".to_owned(), "Production Report".to_owned()];
        lines.extend((0..80).map(|index| format!("Production row number {index}")));

        write_pdf(&path, &lines).expect("multi-page report is written");
        let report = fs::read_to_string(&path).expect("PDF syntax is readable");
        fs::remove_file(path).expect("test PDF is removed");

        assert!(report.contains("/Count 3"));
        assert!(report.contains("(Page 3 of 3)"));
        assert_eq!(pdf_escape("Español ¡Sí!"), "Espa\\361ol \\241S\\355!");
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

        assert!(report.contains("/MediaBox [0 0 612 792]"));
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

    /// Confirms localized report labels are rendered into the actual PDF content stream.
    #[test]
    fn exports_spanish_vector_report_headings() {
        let project = Project {
            project_title: "Prueba de producción".to_owned(),
            ..Project::default()
        };
        let path = std::env::temp_dir().join(format!("cinesched-{}.pdf", Uuid::new_v4()));

        export_stripboard_pdf_localized(&project, Language::Spanish, &path)
            .expect("localized stripboard PDF is written");
        let report = fs::read_to_string(&path).expect("PDF syntax is readable");
        fs::remove_file(path).expect("test PDF is removed");

        assert!(report.contains("TIRA DE RODAJE"));
        assert!(report.contains("P\\301GINA 1/1"));
    }

    /// Rejects invalid calendar input instead of silently exporting a different month.
    #[test]
    fn rejects_invalid_calendar_month() {
        let project = Project::default();
        let path = std::env::temp_dir().join(format!("cinesched-{}.pdf", Uuid::new_v4()));

        let error =
            export_month_calendar_pdf_localized(&project, 2025, 13, Language::Spanish, &path)
                .expect_err("month 13 is not a valid calendar month");

        assert!(matches!(
            error,
            ReportError::Io(ref io_error)
                if io_error.kind() == std::io::ErrorKind::InvalidInput
        ));
        assert!(!path.exists());
    }

    /// Confirms call-sheet continuation pages retain their identity and do not lose scenes.
    #[test]
    fn paginates_long_call_sheets_with_continuation_heading() {
        let mut day = ShootDay::new(Utc::now());
        for number in 1..=40 {
            let mut scene = Scene::new(format!("INT. SET {number} - DAY"));
            scene.scene_number = number.to_string();
            day.scenes.push(scene);
        }
        let day_id = day.id;
        let project = Project {
            shoot_days: vec![day],
            ..Project::default()
        };
        let path = std::env::temp_dir().join(format!("cinesched-{}.pdf", Uuid::new_v4()));

        export_call_sheet_pdf(&project, day_id, &path).expect("multi-page call sheet is written");
        let report = fs::read_to_string(&path).expect("PDF syntax is readable");
        fs::remove_file(path).expect("test PDF is removed");

        assert!(report.contains("/Count 2"));
        assert!(report.contains("CALL SHEET - CONTINUED"));
        assert!(report.contains("(40)"));
    }

    /// Keeps the DOOD legend clear of the last character row on multi-page matrices.
    #[test]
    fn paginates_large_dood_cast_lists_without_overlapping_legend() {
        let mut day = ShootDay::new(Utc::now());
        let mut scene = Scene::new("INT. STAGE - DAY");
        scene.cast = (1..=23).map(|index| format!("CAST {index:02}")).collect();
        day.scenes.push(scene);
        let project = Project {
            shoot_days: vec![day],
            ..Project::default()
        };
        let path = std::env::temp_dir().join(format!("cinesched-{}.pdf", Uuid::new_v4()));

        export_days_out_of_days_pdf(&project, true, &path).expect("multi-page DOOD PDF is written");
        let report = fs::read_to_string(&path).expect("PDF syntax is readable");
        fs::remove_file(path).expect("test PDF is removed");

        assert!(report.contains("/Count 2"));
        assert!(report.contains("(CAST 23)"));
        assert!(report.contains("SW = Start Work"));
    }
}
