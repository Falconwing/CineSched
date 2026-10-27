//! Report-specific vector PDF layouts shared by the desktop exporters.

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;

use chrono::{Datelike, Duration, Months, NaiveDate, Weekday};

use crate::localization::{format_long_date, translate, Language};
use crate::model::{Project, Scene, ShootDay};

const LANDSCAPE: (usize, usize) = (792, 612);
const PORTRAIT: (usize, usize) = (612, 792);
const NAVY: Color = Color(0.13, 0.19, 0.28);
const SLATE: Color = Color(0.22, 0.29, 0.38);
const INK: Color = Color(0.12, 0.15, 0.19);
const MID: Color = Color(0.42, 0.46, 0.51);
const PALE: Color = Color(0.92, 0.94, 0.96);
const GRID: Color = Color(0.72, 0.75, 0.78);
const WHITE: Color = Color(1.0, 1.0, 1.0);

#[derive(Clone, Copy)]
struct Color(f32, f32, f32);

struct CellStyle {
    fill: Color,
    foreground: Color,
    bold: bool,
    font_size: usize,
}

struct PdfDocument {
    width: usize,
    height: usize,
    language: Language,
    pages: Vec<Vec<String>>,
}

impl PdfDocument {
    fn new(width: usize, height: usize, language: Language) -> Self {
        Self {
            width,
            height,
            language,
            pages: vec![Vec::new()],
        }
    }

    fn page(&mut self) {
        self.pages.push(Vec::new());
    }

    fn text(&mut self, x: usize, top: usize, size: usize, text: &str, color: Color, bold: bool) {
        let y = self.height.saturating_sub(top + size);
        let font = if bold { "F2" } else { "F1" };
        let command = format!(
            "{} {} {} rg BT /{font} {size} Tf 1 0 0 1 {x} {y} Tm ({}) Tj ET",
            color.0,
            color.1,
            color.2,
            pdf_escape(text)
        );
        self.current().push(command);
    }

    fn rect(&mut self, x: usize, top: usize, width: usize, height: usize, fill: Color) {
        let y = self.height.saturating_sub(top + height);
        self.current().push(format!(
            "{} {} {} rg {x} {y} {width} {height} re f",
            fill.0, fill.1, fill.2
        ));
    }

    fn stroked_rect(
        &mut self,
        x: usize,
        top: usize,
        width: usize,
        height: usize,
        fill: Color,
        stroke: Color,
    ) {
        let y = self.height.saturating_sub(top + height);
        self.current().push(format!(
            "{} {} {} rg {} {} {} RG 0.6 w {x} {y} {width} {height} re B",
            fill.0, fill.1, fill.2, stroke.0, stroke.1, stroke.2
        ));
    }

    fn line(&mut self, x1: usize, top1: usize, x2: usize, top2: usize, color: Color) {
        let y1 = self.height.saturating_sub(top1);
        let y2 = self.height.saturating_sub(top2);
        self.current().push(format!(
            "{} {} {} RG 0.7 w {x1} {y1} m {x2} {y2} l S",
            color.0, color.1, color.2
        ));
    }

    fn current(&mut self) -> &mut Vec<String> {
        self.pages.last_mut().expect("PDF has an initial page")
    }

    fn write(mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let page_count = self.pages.len();
        let mut objects = vec![String::new(); 4 + page_count * 2];
        let page_ids: Vec<usize> = (0..page_count).map(|index| 5 + index * 2).collect();
        objects[0] = "<< /Type /Catalog /Pages 2 0 R >>".to_owned();
        objects[1] = format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            page_ids
                .iter()
                .map(|id| format!("{id} 0 R"))
                .collect::<Vec<_>>()
                .join(" "),
            page_count
        );
        objects[2] =
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_owned();
        objects[3] = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_owned();

        for (page_index, (page_id, commands)) in page_ids
            .iter()
            .copied()
            .zip(self.pages.iter_mut())
            .enumerate()
        {
            let content_id = page_id + 1;
            let footer = format!(
                "0.42 0.46 0.51 rg BT /F1 8 Tf 1 0 0 1 {} 18 Tm ({} {}) Tj ET",
                self.width.saturating_sub(74),
                pdf_escape(&translate(self.language, "Page")),
                page_index + 1
            );
            commands.push(footer);
            let content = commands.join("\n");
            objects[page_id - 1] = format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {content_id} 0 R >>",
                self.width, self.height
            );
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
                .expect("formatting into a string cannot fail");
        }
        let xref_offset = document.len();
        writeln!(document, "xref\n0 {}\n0000000000 65535 f ", offsets.len())
            .expect("formatting into a string cannot fail");
        for offset in offsets.iter().skip(1) {
            writeln!(document, "{offset:010} 00000 n ")
                .expect("formatting into a string cannot fail");
        }
        write!(
            document,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            offsets.len()
        )
        .expect("formatting into a string cannot fail");
        fs::write(path, document.as_bytes())
    }
}

/// Creates the landscape monthly grid used by the schedule report.
pub(crate) fn schedule(
    project: &Project,
    language: Language,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let dates: Vec<NaiveDate> = project
        .shoot_days
        .iter()
        .map(|day| day.date.date_naive())
        .collect();
    let first = dates.iter().min().copied();
    let last = dates.iter().max().copied();
    let start = first.map_or_else(
        || NaiveDate::from_ymd_opt(2025, 1, 1).expect("fixed date is valid"),
        |date| date - Duration::days(i64::from(date.weekday().num_days_from_sunday())),
    );
    let end = last.map_or(start, |date| {
        date + Duration::days(i64::from(6 - date.weekday().num_days_from_sunday()))
    });
    let weeks = calendar_weeks(start, end);
    let day_lookup = project
        .shoot_days
        .iter()
        .map(|day| (day.date.date_naive(), day))
        .collect::<std::collections::BTreeMap<_, _>>();
    let day_numbers = production_day_numbers(&project.shoot_days);
    let mut pdf = PdfDocument::new(LANDSCAPE.0, LANDSCAPE.1, language);
    let chunks = weeks.chunks(5);
    let page_total = chunks.len().max(1);
    for (page_index, week_chunk) in chunks.enumerate() {
        if page_index > 0 {
            pdf.page();
        }
        draw_report_header(
            &mut pdf,
            &project.project_title,
            &translate(language, "SHOOTING SCHEDULE"),
            language,
            page_index + 1,
            page_total,
            false,
        );
        let x0 = 34;
        let y0 = 104;
        let grid_width = 724;
        let cell_width = grid_width / 7;
        let row_height = 88;
        for (column, weekday) in weekday_headers(language).iter().enumerate() {
            pdf.rect(x0 + column * cell_width, y0, cell_width, 20, NAVY);
            pdf.text(
                x0 + column * cell_width + 5,
                y0 + 5,
                9,
                weekday,
                WHITE,
                true,
            );
        }
        for (row, week) in week_chunk.iter().enumerate() {
            let top = y0 + 20 + row * row_height;
            for (column, date) in week.iter().enumerate() {
                let x = x0 + column * cell_width;
                let day = day_lookup.get(date).copied();
                let fill = if day.is_some_and(|day| day.is_blackout) {
                    PALE
                } else {
                    WHITE
                };
                pdf.stroked_rect(x, top, cell_width, row_height, fill, GRID);
                pdf.text(x + 5, top + 5, 9, &date.day().to_string(), INK, true);
                if date.month() != week[3].month() {
                    pdf.text(
                        x + 21,
                        top + 5,
                        7,
                        &short_month(*date, language),
                        MID,
                        false,
                    );
                }
                if let Some(day) = day {
                    if let Some(day_number) = day_numbers.get(&day.id) {
                        pdf.text(
                            x + cell_width - 35,
                            top + 5,
                            7,
                            &format!("Day {day_number}"),
                            MID,
                            false,
                        );
                    }
                    let mut line_top = top + 19;
                    for scene in day
                        .scenes
                        .iter()
                        .filter(|scene| !scene.is_calendar_event)
                        .take(5)
                    {
                        let strip = scene_color(scene);
                        pdf.rect(x + 4, line_top, cell_width - 8, 11, strip);
                        pdf.text(
                            x + 7,
                            line_top + 2,
                            6,
                            &fit_text(&scene.display_title(), 22),
                            text_color(strip),
                            true,
                        );
                        line_top += 13;
                    }
                    let hidden = day
                        .scenes
                        .iter()
                        .filter(|scene| !scene.is_calendar_event)
                        .count()
                        .saturating_sub(5);
                    if hidden > 0 {
                        pdf.text(
                            x + 6,
                            line_top + 1,
                            7,
                            &format!("+{hidden} more"),
                            MID,
                            false,
                        );
                    }
                    let summary = format!(
                        "{} / {} pg / {}m",
                        day.scenes
                            .iter()
                            .filter(|scene| !scene.is_calendar_event)
                            .count(),
                        format_eighths(day.total_pages_eighths()),
                        day.total_estimated_minutes()
                    );
                    pdf.text(x + 5, top + row_height - 13, 6, &summary, MID, false);
                }
            }
        }
    }
    pdf.write(path)
}

/// Creates a landscape monthly calendar PDF with colored scene strips.
pub(crate) fn month_calendar(
    project: &Project,
    year: i32,
    month: u32,
    language: Language,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let first = NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "calendar year or month is outside the supported range",
        )
    })?;
    let next_month = first.checked_add_months(Months::new(1)).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "calendar year or month is outside the supported range",
        )
    })?;
    let last = next_month
        .checked_sub_signed(Duration::days(1))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "calendar year or month is outside the supported range",
            )
        })?;
    let start = first - Duration::days(i64::from(first.weekday().num_days_from_sunday()));
    let end = last + Duration::days(i64::from(6 - last.weekday().num_days_from_sunday()));
    let weeks = calendar_weeks(start, end);
    let day_lookup = project
        .shoot_days
        .iter()
        .map(|day| (day.date.date_naive(), day))
        .collect::<std::collections::BTreeMap<_, _>>();
    let day_numbers = production_day_numbers(&project.shoot_days);
    let mut pdf = PdfDocument::new(LANDSCAPE.0, LANDSCAPE.1, language);
    for (page_index, week_chunk) in weeks.chunks(5).enumerate() {
        if page_index > 0 {
            pdf.page();
        }
        let title = format!(
            "{} - {}",
            translate(language, "PRODUCTION CALENDAR"),
            format_month(year, month, language)
        );
        draw_report_header(
            &mut pdf,
            &project.project_title,
            &title,
            language,
            page_index + 1,
            weeks.len().div_ceil(5),
            false,
        );
        let x0 = 34;
        let y0 = 104;
        let grid_width = 724;
        let cell_width = grid_width / 7;
        let row_height = 88;
        for (column, weekday) in weekday_headers(language).iter().enumerate() {
            pdf.rect(x0 + column * cell_width, y0, cell_width, 20, NAVY);
            pdf.text(
                x0 + column * cell_width + 5,
                y0 + 5,
                9,
                weekday,
                WHITE,
                true,
            );
        }
        for (row, week) in week_chunk.iter().enumerate() {
            let top = y0 + 20 + row * row_height;
            for (column, date) in week.iter().enumerate() {
                let x = x0 + column * cell_width;
                let day = day_lookup.get(date).copied();
                let fill = if date.month() != month {
                    Color(0.96, 0.96, 0.96)
                } else if day.is_some_and(|day| day.is_blackout) {
                    PALE
                } else {
                    WHITE
                };
                pdf.stroked_rect(x, top, cell_width, row_height, fill, GRID);
                pdf.text(x + 5, top + 5, 9, &date.day().to_string(), INK, true);
                if let Some(day) = day {
                    if let Some(number) = day_numbers.get(&day.id) {
                        pdf.text(
                            x + cell_width - 35,
                            top + 5,
                            7,
                            &format!("{} {number}", translate(language, "Day")),
                            MID,
                            false,
                        );
                    }
                    let mut line_top = top + 20;
                    for scene in day
                        .scenes
                        .iter()
                        .filter(|scene| !scene.is_calendar_event)
                        .take(4)
                    {
                        let strip = scene_color(scene);
                        pdf.rect(x + 4, line_top, cell_width - 8, 12, strip);
                        pdf.text(
                            x + 7,
                            line_top + 2,
                            6,
                            &fit_text(&scene.display_title(), 21),
                            text_color(strip),
                            true,
                        );
                        line_top += 14;
                    }
                    let hidden = day
                        .scenes
                        .iter()
                        .filter(|scene| !scene.is_calendar_event)
                        .count()
                        .saturating_sub(4);
                    if hidden > 0 {
                        pdf.text(
                            x + 6,
                            line_top + 1,
                            7,
                            &format!("+{hidden} {}", translate(language, "more")),
                            MID,
                            false,
                        );
                    }
                    pdf.text(
                        x + 5,
                        top + row_height - 13,
                        6,
                        &format!(
                            "{} {} / {} {}",
                            day.scenes
                                .iter()
                                .filter(|scene| !scene.is_calendar_event)
                                .count(),
                            translate(language, "scenes"),
                            format_eighths(day.total_pages_eighths()),
                            translate(language, "pg")
                        ),
                        MID,
                        false,
                    );
                }
            }
        }
    }
    pdf.write(path)
}

/// Creates a portrait stripboard with individual color strips and end-of-day bars.
pub(crate) fn stripboard(
    project: &Project,
    language: Language,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let mut pdf = PdfDocument::new(PORTRAIT.0, PORTRAIT.1, language);
    draw_report_header(
        &mut pdf,
        &project.project_title,
        &translate(language, "STRIPBOARD"),
        language,
        1,
        1,
        true,
    );
    let mut top = 104;
    let days = project
        .shoot_days
        .iter()
        .filter(|day| !day.scenes.is_empty())
        .collect::<Vec<_>>();
    let day_numbers = production_day_numbers(&project.shoot_days);
    for day in days {
        if top + 66 > PORTRAIT.1 - 42 {
            pdf.page();
            top = 40;
        }
        let day_number = day_numbers.get(&day.id).copied().unwrap_or(0);
        pdf.rect(40, top, 532, 21, PALE);
        pdf.text(
            47,
            top + 5,
            10,
            &format!("{}  {}", translate(language, "Day"), day_number),
            NAVY,
            true,
        );
        pdf.text(
            145,
            top + 5,
            9,
            &format_long_date(day.date.date_naive(), language),
            INK,
            true,
        );
        pdf.text(
            475,
            top + 5,
            7,
            &format!("{} {}", day.scenes.len(), translate(language, "scenes")),
            MID,
            false,
        );
        top += 24;
        for scene in &day.scenes {
            let scene_height = if scene.summary.trim().is_empty() {
                20
            } else {
                28
            };
            if top + scene_height + 24 > PORTRAIT.1 - 38 {
                pdf.page();
                top = 40;
            }
            let fill = scene_color(scene);
            pdf.stroked_rect(40, top, 532, scene_height, fill, Color(0.2, 0.2, 0.2));
            let title = scene.display_title();
            pdf.text(
                47,
                top + 4,
                9,
                &fit_text(&title, 72),
                text_color(fill),
                true,
            );
            pdf.text(
                430,
                top + 5,
                7,
                &format!(
                    "{} {}",
                    format_eighths(scene.duration),
                    translate(language, "pg")
                ),
                text_color(fill),
                false,
            );
            pdf.text(
                490,
                top + 5,
                7,
                &format!("{} min", scene.estimated_time),
                text_color(fill),
                false,
            );
            if !scene.summary.trim().is_empty() {
                pdf.text(
                    47,
                    top + 16,
                    7,
                    &fit_text(&scene.summary, 96),
                    text_color(fill),
                    false,
                );
            }
            top += scene_height + 2;
        }
        let eod = format!(
            "-- {} {} -- {} pg -- {} {}",
            translate(language, "END OF DAY"),
            day_number,
            format_eighths(day.total_pages_eighths()),
            day.total_estimated_minutes(),
            translate(language, "min")
        );
        pdf.rect(40, top, 532, 21, NAVY);
        pdf.text(48, top + 5, 8, &eod, WHITE, true);
        top += 32;
    }
    pdf.write(path)
}

/// Creates a portrait one-line schedule with calculated time ranges.
pub(crate) fn timeline(
    project: &Project,
    language: Language,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let mut pdf = PdfDocument::new(PORTRAIT.0, PORTRAIT.1, language);
    draw_report_header(
        &mut pdf,
        &project.project_title,
        &translate(language, "ONE-LINE SHOOTING SCHEDULE"),
        language,
        1,
        1,
        true,
    );
    let mut top = 104;
    for day in &project.shoot_days {
        if top + 45 > PORTRAIT.1 - 40 {
            pdf.page();
            top = 40;
        }
        pdf.rect(36, top, 540, 22, NAVY);
        pdf.text(
            44,
            top + 5,
            9,
            &format_long_date(day.date.date_naive(), language),
            WHITE,
            true,
        );
        pdf.text(
            450,
            top + 5,
            8,
            &format!(
                "{} {}",
                translate(language, "CALL"),
                day.call_sheet.general_call_time
            ),
            WHITE,
            true,
        );
        top += 26;
        let mut minutes = parse_clock(&day.call_sheet.general_call_time).unwrap_or(7 * 60);
        for scene in day.scenes.iter().filter(|scene| !scene.is_calendar_event) {
            if !scene.custom_start_time.trim().is_empty() {
                if let Some(custom) = parse_clock(&scene.custom_start_time) {
                    minutes = custom;
                }
            }
            let duration = scene.estimated_time.max(0);
            let start = clock_label(minutes);
            minutes += duration;
            let end = clock_label(minutes);
            if top + 30 > PORTRAIT.1 - 40 {
                pdf.page();
                top = 40;
            }
            let fill = if scene.is_banner { PALE } else { WHITE };
            pdf.stroked_rect(36, top, 540, 28, fill, GRID);
            pdf.text(42, top + 8, 7, &format!("{start} - {end}"), MID, true);
            pdf.text(
                118,
                top + 7,
                8,
                &fit_text(&scene.display_title(), 57),
                INK,
                true,
            );
            pdf.text(
                408,
                top + 8,
                7,
                &fit_text(&scene.cast.join(", "), 22),
                MID,
                false,
            );
            pdf.text(
                528,
                top + 8,
                7,
                &format!(
                    "{} {}",
                    format_eighths(scene.duration),
                    translate(language, "pg")
                ),
                MID,
                false,
            );
            top += 28;
        }
        pdf.text(
            44,
            top + 2,
            8,
            &format!(
                "{} - {}",
                translate(language, "END OF DAY"),
                clock_label(minutes)
            ),
            NAVY,
            true,
        );
        top += 22;
    }
    pdf.write(path)
}

/// Creates an industry-style call sheet with milestone panels and production tables.
pub(crate) fn call_sheet(
    project: &Project,
    day_id: uuid::Uuid,
    language: Language,
    path: impl AsRef<Path>,
) -> Result<(), CallSheetPdfError> {
    let day = project
        .shoot_days
        .iter()
        .find(|day| day.id == day_id)
        .ok_or(CallSheetPdfError::MissingDay)?;
    let production = project.production_info.as_ref();
    let mut pdf = PdfDocument::new(PORTRAIT.0, PORTRAIT.1, language);
    let date = format_long_date(day.date.date_naive(), language);
    let day_number = production_day_numbers(&project.shoot_days)
        .get(&day.id)
        .copied()
        .unwrap_or(1);
    pdf.stroked_rect(28, 28, 556, 24, PALE, GRID);
    pdf.text(
        34,
        34,
        12,
        &format!(
            "{} #{} - {}",
            translate(language, "CALL SHEET"),
            day_number,
            date
        ),
        INK,
        true,
    );
    let mut top = 54;
    pdf.stroked_rect(28, top, 556, 68, Color(0.94, 0.95, 0.97), GRID);
    pdf.text(
        28,
        top + 7,
        9,
        &translate(language, "GENERAL CALL").to_uppercase(),
        MID,
        true,
    );
    pdf.text(
        28,
        top + 19,
        24,
        empty_default(&day.call_sheet.general_call_time, "07:30 AM"),
        NAVY,
        true,
    );
    if !day.call_sheet.quote_of_the_day.trim().is_empty() {
        pdf.text(
            42,
            top + 49,
            8,
            &fit_text(
                &format!(
                    "{}: {}",
                    translate(language, "QUOTE OF THE DAY"),
                    day.call_sheet.quote_of_the_day
                ),
                100,
            ),
            INK,
            false,
        );
    }
    if !day.call_sheet.work_day_schedule.trim().is_empty() {
        pdf.text(
            340,
            top + 14,
            8,
            &fit_text(
                &format!(
                    "{}: {}",
                    translate(language, "SCHEDULE"),
                    day.call_sheet.work_day_schedule
                ),
                58,
            ),
            INK,
            false,
        );
    }
    top += 74;

    let info_width = 556 / 3;
    let titles = ["CONTACTS", "MILESTONES", "WEATHER"];
    for (index, title) in titles.iter().enumerate() {
        let x = 28 + index * info_width;
        pdf.stroked_rect(x, top, info_width, 82, WHITE, GRID);
        pdf.rect(x, top, info_width, 16, PALE);
        pdf.text(x + 5, top + 4, 8, &translate(language, title), NAVY, true);
    }
    if let Some(production) = production {
        let contact_rows = [
            (
                translate(language, "PRODUCER"),
                format_contact(&production.producer_name, &production.producer_phone),
            ),
            (
                translate(language, "1ST AD"),
                format_contact(&production.ad_name, &production.ad_phone),
            ),
        ];
        for (index, (label, value)) in contact_rows.iter().enumerate() {
            pdf.text(34, top + 23 + index * 22, 7, label, MID, true);
            pdf.text(
                34,
                top + 33 + index * 22,
                7,
                &fit_text(value, 34),
                INK,
                false,
            );
        }
    } else if !day.call_sheet.prod_manager_contact.is_empty() {
        pdf.text(
            34,
            top + 24,
            7,
            &fit_text(&day.call_sheet.prod_manager_contact, 35),
            INK,
            true,
        );
    }
    let milestones = [
        ("READY TO SHOOT", &day.call_sheet.ready_to_shoot_time),
        ("LUNCH", &day.call_sheet.lunch_time),
        ("SNACK", &day.call_sheet.snack_time),
        ("WRAP", &day.call_sheet.wrap_time),
    ];
    for (index, (label, value)) in milestones.iter().enumerate() {
        let y = top + 23 + index * 14;
        pdf.text(221, y, 7, &translate(language, label), MID, true);
        pdf.text(300, y, 7, value, INK, true);
    }
    let weather_rows = [
        ("TEMP", &day.call_sheet.weather_temp),
        ("SKY", &day.call_sheet.weather_condition),
        ("WIND", &day.call_sheet.weather_precip_wind),
        ("SUN", &day.call_sheet.sun_times),
    ];
    for (index, (label, value)) in weather_rows.iter().enumerate() {
        let y = top + 23 + index * 14;
        pdf.text(408, y, 7, &translate(language, label), MID, true);
        pdf.text(442, y, 7, &fit_text(value, 22), INK, false);
    }
    top += 88;

    draw_labeled_bar(
        &mut pdf,
        top,
        "BASECAMP",
        &day.call_sheet.basecamp_location,
        language,
    );
    top += 22;
    draw_labeled_bar(
        &mut pdf,
        top,
        "NEAREST HOSPITAL",
        &day.call_sheet.nearest_hospital,
        language,
    );
    top += 26;
    if !day.call_sheet.locations.is_empty() {
        top = section_header(&mut pdf, top, "LOCATIONS", language);
        for location in &day.call_sheet.locations {
            top = ensure_call_sheet_room(&mut pdf, top, 18);
            let location_text = format!("{} - {}", location.name, location.address);
            draw_table_row(&mut pdf, top, &[&location_text], &[556]);
            top += 18;
        }
    }
    top = section_header(&mut pdf, top, "SCENES", language);
    draw_table_header(
        &mut pdf,
        top,
        &["SCENE", "SET / DESCRIPTION", "CAST", "PAGES", "LOCATION"],
        &[72, 172, 112, 55, 145],
        language,
    );
    top += 18;
    let mut current_minutes = parse_clock(&day.call_sheet.general_call_time).unwrap_or(7 * 60 + 30);
    for scene in day.scenes.iter().filter(|scene| !scene.is_calendar_event) {
        top = ensure_call_sheet_room(&mut pdf, top, 28);
        if let Some(custom) = parse_clock(&scene.custom_start_time) {
            current_minutes = custom;
        }
        let duration = scene
            .estimated_time
            .max(if scene.is_banner { 30 } else { 15 });
        let time_range = format!(
            "{} - {}",
            clock_label(current_minutes),
            clock_label(current_minutes + duration)
        );
        current_minutes += duration;
        let scene_number = if scene.scene_number.is_empty() {
            scene.display_title()
        } else {
            scene.scene_number.clone()
        };
        let location = if scene.real_location.is_empty() {
            scene.location_address.clone()
        } else {
            scene.real_location.clone()
        };
        let desc = if scene.summary.is_empty() {
            scene.title.clone()
        } else {
            scene.summary.clone()
        };
        draw_table_row(
            &mut pdf,
            top,
            &[
                &scene_number,
                &format!("{} / {}", desc, time_range),
                &scene.cast.join(", "),
                &format_eighths(scene.duration),
                &location,
            ],
            &[72, 172, 112, 55, 145],
        );
        top += 28;
    }
    if !day.call_sheet.cast_call_entries.is_empty() {
        top = section_header(&mut pdf, top + 4, "CAST CALLS", language);
        draw_table_header(
            &mut pdf,
            top,
            &[
                "#",
                "ACTOR / CHARACTER",
                "SCENES",
                "PICKUP",
                "HMU",
                "SET",
                "WRAP",
                "LOC",
            ],
            &[24, 150, 88, 58, 60, 58, 58, 40],
            language,
        );
        top += 18;
        for (index, entry) in day.call_sheet.cast_call_entries.iter().enumerate() {
            top = ensure_call_sheet_room(&mut pdf, top, 22);
            let actor = format!("{} / {}", entry.actor_name, entry.character_name);
            draw_table_row(
                &mut pdf,
                top,
                &[
                    &format!("{}", index + 1),
                    &actor,
                    &entry.scene_numbers,
                    &entry.pickup_time,
                    &entry.hmu_wardrobe_time,
                    &entry.on_set_time,
                    &entry.wrap_time,
                    &entry.location_index,
                ],
                &[24, 150, 88, 58, 60, 58, 58, 40],
            );
            top += 22;
            if !entry.ecdt.trim().is_empty()
                || !entry.hmu_wardrobe_time.trim().is_empty()
                || !entry.on_set_time.trim().is_empty()
                || !entry.wrap_time.trim().is_empty()
            {
                top = ensure_call_sheet_room(&mut pdf, top, 15);
                let details = format!(
                    "{} / HMU-Wardrobe {} / On Set {} / Wrap {} / Location {}",
                    entry.ecdt,
                    entry.hmu_wardrobe_time,
                    entry.on_set_time,
                    entry.wrap_time,
                    entry.location_index
                );
                pdf.text(64, top + 3, 6, &fit_text(&details, 92), MID, false);
                top += 14;
            }
        }
    }
    if !day.call_sheet.crew_call_entries.is_empty() {
        top = section_header(&mut pdf, top + 4, "CREW CALLS", language);
        draw_table_header(
            &mut pdf,
            top,
            &["ROLE", "NAME", "CALL", "PHONE"],
            &[190, 160, 100, 146],
            language,
        );
        top += 18;
        for entry in &day.call_sheet.crew_call_entries {
            top = ensure_call_sheet_room(&mut pdf, top, 18);
            draw_table_row(
                &mut pdf,
                top,
                &[&entry.role, &entry.name, &entry.call_time, &entry.phone],
                &[190, 160, 100, 146],
            );
            top += 18;
        }
    }
    if !day.call_sheet.production_notes.is_empty() || !day.call_sheet.notes.trim().is_empty() {
        top = section_header(&mut pdf, top + 4, "PRODUCTION NOTES", language);
        for note in day
            .call_sheet
            .production_notes
            .iter()
            .chain(std::iter::once(&day.call_sheet.notes))
            .filter(|note| !note.trim().is_empty())
        {
            top = ensure_call_sheet_room(&mut pdf, top, 22);
            pdf.text(
                34,
                top + 5,
                8,
                &fit_text(&format!("- {note}"), 100),
                INK,
                false,
            );
            top += 18;
        }
    }
    pdf.write(path).map_err(CallSheetPdfError::Io)
}

#[derive(Debug)]
pub(crate) enum CallSheetPdfError {
    MissingDay,
    Io(std::io::Error),
}

/// Creates one portrait, color-coded AD breakdown sheet for each scene.
pub(crate) fn breakdowns(
    project: &Project,
    language: Language,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let mut scenes = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for scene in project
        .all_scenes
        .iter()
        .chain(project.shoot_days.iter().flat_map(|day| day.scenes.iter()))
    {
        if !scene.is_calendar_event && seen.insert(scene.id) {
            scenes.push(scene);
        }
    }
    scenes.sort_by(|left, right| {
        scene_order(left)
            .cmp(&scene_order(right))
            .then(left.title.cmp(&right.title))
    });
    let mut pdf = PdfDocument::new(PORTRAIT.0, PORTRAIT.1, language);
    if scenes.is_empty() {
        draw_report_header(
            &mut pdf,
            &project.project_title,
            &translate(language, "SCENE BREAKDOWNS"),
            language,
            1,
            1,
            true,
        );
        pdf.write(path)?;
        return Ok(());
    }
    for (index, scene) in scenes.iter().enumerate() {
        if index > 0 {
            pdf.page();
        }
        pdf.text(
            36,
            34,
            17,
            &translate(language, "BREAKDOWN SHEET"),
            INK,
            true,
        );
        let top = 62;
        let width = 540;
        let rows = breakdown_rows(scene, language);
        let row_count = rows.len();
        let row_height = 82;
        for (row_index, row) in rows.iter().enumerate() {
            let y = top + row_index * row_height;
            let col_count = row.len();
            let cell_width = width / col_count;
            for (col_index, (label, value, color)) in row.iter().enumerate() {
                let x = 36 + col_index * cell_width;
                let actual_width = if col_index + 1 == col_count {
                    width - cell_width * col_index
                } else {
                    cell_width
                };
                pdf.stroked_rect(x, y, actual_width, row_height, WHITE, GRID);
                pdf.rect(x, y, actual_width, 18, PALE);
                pdf.text(
                    x + 5,
                    y + 5,
                    8,
                    &translate(language, label).to_uppercase(),
                    *color,
                    true,
                );
                let lines = wrap_text(value, actual_width.saturating_sub(12), 8)
                    .into_iter()
                    .take((row_height - 24) / 10)
                    .collect::<Vec<_>>();
                for (line_index, line) in lines.iter().enumerate() {
                    pdf.text(x + 6, y + 23 + line_index * 10, 8, line, INK, false);
                }
            }
        }
        let footer_top = top + row_count * row_height + 12;
        pdf.text(
            36,
            footer_top,
            8,
            &format!(
                "{} {} / {}: {}",
                translate(language, "BREAKDOWN SHEET"),
                index + 1,
                scenes.len(),
                scene.display_title()
            ),
            MID,
            false,
        );
    }
    pdf.write(path)
}

/// Creates a landscape paginated cast days-out-of-days matrix.
pub(crate) fn days_out_of_days(
    project: &Project,
    include_hold: bool,
    language: Language,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let days = project
        .shoot_days
        .iter()
        .filter(|day| day.scenes.iter().any(|scene| !scene.is_calendar_event))
        .collect::<Vec<_>>();
    let mut characters = project
        .production_info
        .as_ref()
        .map(|production| {
            production
                .cast_list
                .iter()
                .map(|cast| cast.character_name.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if characters.is_empty() {
        characters = days.iter().flat_map(|day| day.all_cast()).collect();
        characters.sort();
        characters.dedup();
    }
    let mut pdf = PdfDocument::new(LANDSCAPE.0, LANDSCAPE.1, language);
    let name_width = 138;
    let summary_width = 32;
    let summary_count = if include_hold { 3 } else { 2 };
    let fixed = name_width + summary_width * summary_count;
    let days_per_page = ((LANDSCAPE.0 - 72 - fixed) / 24).max(1);
    let rows_per_page = 21;
    let day_chunks = if days.is_empty() {
        vec![(0, 0)]
    } else {
        (0..days.len())
            .step_by(days_per_page)
            .map(|start| (start, (start + days_per_page).min(days.len())))
            .collect()
    };
    let row_chunks = if characters.is_empty() {
        vec![(0, 0)]
    } else {
        (0..characters.len())
            .step_by(rows_per_page)
            .map(|start| (start, (start + rows_per_page).min(characters.len())))
            .collect()
    };
    let page_total = day_chunks.len() * row_chunks.len();
    let mut page_index = 0;
    for (day_start, day_end) in day_chunks {
        for (row_start, row_end) in &row_chunks {
            if page_index > 0 {
                pdf.page();
            }
            page_index += 1;
            draw_report_header(
                &mut pdf,
                &project.project_title,
                &translate(language, "DAYS OUT OF DAYS"),
                language,
                page_index,
                page_total,
                false,
            );
            let chunk_days = &days[day_start..day_end];
            let chunk_chars = &characters[*row_start..*row_end];
            let col_day = (LANDSCAPE.0 - 72 - fixed) / chunk_days.len().max(1);
            let mut x = 36;
            let top = 100;
            let labels = if include_hold {
                vec!["CHARACTER", "TOT", "WRK", "HLD"]
            } else {
                vec!["CHARACTER", "TOT", "WRK"]
            };
            let widths = std::iter::once(name_width)
                .chain(std::iter::repeat_n(summary_width, summary_count))
                .collect::<Vec<_>>();
            for (label, width) in labels.iter().zip(widths) {
                draw_cell(
                    &mut pdf,
                    x,
                    top,
                    width,
                    34,
                    label,
                    CellStyle {
                        fill: PALE,
                        foreground: NAVY,
                        bold: true,
                        font_size: 7,
                    },
                );
                x += width;
            }
            let mut previous_month = 0;
            for day in chunk_days {
                let date = day.date.date_naive();
                let header = format!(
                    "{}\n{}\n{}",
                    if previous_month != date.month() {
                        short_month(date, language)
                    } else {
                        String::new()
                    },
                    short_weekday(date.weekday(), language),
                    date.day()
                );
                previous_month = date.month();
                draw_cell(
                    &mut pdf,
                    x,
                    top,
                    col_day,
                    34,
                    &header,
                    CellStyle {
                        fill: if day.is_blackout { PALE } else { WHITE },
                        foreground: INK,
                        bold: true,
                        font_size: 6,
                    },
                );
                x += col_day;
            }
            for (row_index, character) in chunk_chars.iter().enumerate() {
                let y = top + 34 + row_index * 20;
                let appearances = days
                    .iter()
                    .enumerate()
                    .filter(|(_, day)| {
                        day.scenes.iter().any(|scene| {
                            !scene.is_calendar_event
                                && scene
                                    .cast
                                    .iter()
                                    .any(|name| name.eq_ignore_ascii_case(character))
                        })
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                let first = appearances.first().copied();
                let last = appearances.last().copied();
                let mut x = 36;
                draw_cell(
                    &mut pdf,
                    x,
                    y,
                    name_width,
                    20,
                    character,
                    CellStyle {
                        fill: WHITE,
                        foreground: INK,
                        bold: false,
                        font_size: 8,
                    },
                );
                x += name_width;
                let statuses = chunk_days
                    .iter()
                    .enumerate()
                    .map(|(local_index, day)| {
                        let absolute_index = day_start + local_index;
                        let works = day.scenes.iter().any(|scene| {
                            !scene.is_calendar_event
                                && scene
                                    .cast
                                    .iter()
                                    .any(|name| name.eq_ignore_ascii_case(character))
                        });
                        let code = if works {
                            match (first == Some(absolute_index), last == Some(absolute_index)) {
                                (true, true) => "SWF",
                                (true, false) => "SW",
                                (false, true) => "WF",
                                (false, false) => "W",
                            }
                        } else if include_hold
                            && first.is_some_and(|first| absolute_index > first)
                            && last.is_some_and(|last| absolute_index < last)
                            && !day.scenes.is_empty()
                        {
                            "H"
                        } else if project
                            .production_info
                            .as_ref()
                            .and_then(|production| {
                                production.cast_list.iter().find(|member| {
                                    member.character_name.eq_ignore_ascii_case(character)
                                })
                            })
                            .is_some_and(|member| {
                                member
                                    .unavailable_ranges
                                    .iter()
                                    .any(|range| range.contains(day.date))
                            })
                        {
                            "X"
                        } else {
                            ""
                        };
                        code.to_owned()
                    })
                    .collect::<Vec<_>>();
                let total_span = first.zip(last).map_or(0, |(first, last)| last - first + 1);
                draw_cell(
                    &mut pdf,
                    x,
                    y,
                    summary_width,
                    20,
                    &total_span.to_string(),
                    CellStyle {
                        fill: WHITE,
                        foreground: INK,
                        bold: true,
                        font_size: 8,
                    },
                );
                x += summary_width;
                let work_count = appearances.len().to_string();
                draw_cell(
                    &mut pdf,
                    x,
                    y,
                    summary_width,
                    20,
                    &work_count,
                    CellStyle {
                        fill: WHITE,
                        foreground: INK,
                        bold: false,
                        font_size: 8,
                    },
                );
                x += summary_width;
                if include_hold {
                    let hold_count = statuses
                        .iter()
                        .filter(|code| code.as_str() == "H")
                        .count()
                        .to_string();
                    draw_cell(
                        &mut pdf,
                        x,
                        y,
                        summary_width,
                        20,
                        &hold_count,
                        CellStyle {
                            fill: WHITE,
                            foreground: INK,
                            bold: false,
                            font_size: 8,
                        },
                    );
                    x += summary_width;
                }
                for code in statuses {
                    let (fill, text) = match code.as_str() {
                        "SW" | "WF" | "SWF" => (Color(0.16, 0.39, 0.68), WHITE),
                        "W" => (PALE, INK),
                        "H" => (Color(0.98, 0.91, 0.63), INK),
                        "X" => (Color(0.72, 0.22, 0.22), WHITE),
                        _ => (WHITE, INK),
                    };
                    draw_cell(
                        &mut pdf,
                        x,
                        y,
                        col_day,
                        20,
                        &code,
                        CellStyle {
                            fill,
                            foreground: text,
                            bold: true,
                            font_size: 6,
                        },
                    );
                    x += col_day;
                }
            }
            let legend_top = 568;
            let legend = if include_hold {
                if language == Language::Spanish {
                    "SW = Inicio de trabajo   W = Trabajo   H = Espera   WF = Fin de trabajo   SWF = Inicio / trabajo / fin   X = No disponible"
                } else {
                    "SW = Start Work   W = Work   H = Hold   WF = Work Finish   SWF = Start / Work / Finish   X = Unavailable"
                }
            } else {
                if language == Language::Spanish {
                    "SW = Inicio de trabajo   W = Trabajo   WF = Fin de trabajo   SWF = Inicio / trabajo / fin   X = No disponible"
                } else {
                    "SW = Start Work   W = Work   WF = Work Finish   SWF = Start / Work / Finish   X = Unavailable"
                }
            };
            pdf.text(36, legend_top, 7, &translate(language, legend), MID, false);
        }
    }
    pdf.write(path)
}

fn draw_report_header(
    pdf: &mut PdfDocument,
    project_title: &str,
    subtitle: &str,
    language: Language,
    page: usize,
    pages: usize,
    portrait: bool,
) {
    let margin = if portrait { 36 } else { 34 };
    pdf.text(
        margin,
        28,
        16,
        empty_default(project_title, "Untitled Movie"),
        NAVY,
        true,
    );
    pdf.text(margin, 52, 9, subtitle, MID, false);
    pdf.text(
        pdf.width.saturating_sub(110),
        52,
        8,
        &format!("{} {page}/{pages}", translate(language, "PAGE")),
        MID,
        false,
    );
    pdf.line(margin, 72, pdf.width - margin, 72, GRID);
}

fn calendar_weeks(start: NaiveDate, end: NaiveDate) -> Vec<Vec<NaiveDate>> {
    let mut weeks = Vec::new();
    let mut date = start;
    while date <= end {
        weeks.push((0..7).map(|offset| date + Duration::days(offset)).collect());
        date += Duration::days(7);
    }
    weeks
}

fn weekday_headers(language: Language) -> [&'static str; 7] {
    match language {
        Language::English => [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ],
        Language::Spanish => [
            "Domingo",
            "Lunes",
            "Martes",
            "Miércoles",
            "Jueves",
            "Viernes",
            "Sábado",
        ],
    }
}

fn short_weekday(day: Weekday, language: Language) -> &'static str {
    match (language, day) {
        (Language::English, Weekday::Mon) => "M",
        (Language::English, Weekday::Tue) => "T",
        (Language::English, Weekday::Wed) => "W",
        (Language::English, Weekday::Thu) => "T",
        (Language::English, Weekday::Fri) => "F",
        (Language::English, Weekday::Sat) => "S",
        (Language::English, Weekday::Sun) => "S",
        (Language::Spanish, Weekday::Mon) => "L",
        (Language::Spanish, Weekday::Tue) => "M",
        (Language::Spanish, Weekday::Wed) => "X",
        (Language::Spanish, Weekday::Thu) => "J",
        (Language::Spanish, Weekday::Fri) => "V",
        (Language::Spanish, Weekday::Sat) => "S",
        (Language::Spanish, Weekday::Sun) => "D",
    }
}

fn short_month(date: NaiveDate, language: Language) -> String {
    match language {
        Language::English => date.format("%b").to_string(),
        Language::Spanish => match date.month() {
            1 => "ene",
            2 => "feb",
            3 => "mar",
            4 => "abr",
            5 => "may",
            6 => "jun",
            7 => "jul",
            8 => "ago",
            9 => "sep",
            10 => "oct",
            11 => "nov",
            _ => "dic",
        }
        .to_owned(),
    }
}

fn format_month(year: i32, month: u32, language: Language) -> String {
    let date = NaiveDate::from_ymd_opt(year, month.clamp(1, 12), 1).expect("month is clamped");
    match language {
        Language::English => date.format("%B %Y").to_string(),
        Language::Spanish => format!("{} {}", short_month(date, language), year),
    }
}

fn scene_color(scene: &Scene) -> Color {
    if scene.is_banner && !scene.banner_color_hex.is_empty() {
        return parse_hex_color(&scene.banner_color_hex).unwrap_or(SLATE);
    }
    if scene.is_banner {
        return SLATE;
    }
    match scene.day_night_type.to_ascii_uppercase().as_str() {
        "NIGHT" => Color(0.19, 0.24, 0.48),
        "DAWN" | "DUSK" => Color(0.58, 0.35, 0.61),
        "AFTERNOON" => Color(0.88, 0.64, 0.20),
        _ if scene.title.to_ascii_uppercase().starts_with("EXT") => Color(0.24, 0.53, 0.39),
        _ => Color(0.32, 0.53, 0.76),
    }
}

fn parse_hex_color(value: &str) -> Option<Color> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some(Color(
        ((value >> 16) & 255) as f32 / 255.0,
        ((value >> 8) & 255) as f32 / 255.0,
        (value & 255) as f32 / 255.0,
    ))
}

fn text_color(background: Color) -> Color {
    if background.0 * 0.299 + background.1 * 0.587 + background.2 * 0.114 > 0.62 {
        INK
    } else {
        WHITE
    }
}

fn production_day_numbers(days: &[ShootDay]) -> std::collections::HashMap<uuid::Uuid, usize> {
    let mut days = days.iter().collect::<Vec<_>>();
    days.sort_by_key(|day| day.date);
    let mut result = std::collections::HashMap::new();
    let mut number = 0;
    for day in days {
        if day.scenes.iter().any(|scene| !scene.is_calendar_event) {
            number += 1;
            result.insert(day.id, number);
        }
    }
    result
}

fn scene_order(scene: &Scene) -> (u32, String) {
    let numeric: String = scene
        .scene_number
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let suffix: String = scene
        .scene_number
        .chars()
        .skip(numeric.len())
        .take_while(char::is_ascii_alphabetic)
        .collect();
    (
        numeric.parse().unwrap_or(u32::MAX),
        suffix.to_ascii_uppercase(),
    )
}

fn breakdown_rows(scene: &Scene, language: Language) -> Vec<Vec<(&'static str, String, Color)>> {
    let heading = scene.title.split_whitespace().collect::<Vec<_>>();
    let int_ext = heading
        .iter()
        .find(|word| {
            word.trim_end_matches('.').eq_ignore_ascii_case("INT")
                || word.trim_end_matches('.').eq_ignore_ascii_case("EXT")
        })
        .copied()
        .unwrap_or("");
    let location = scene.real_location.clone();
    vec![
        vec![
            ("BREAKDOWN SHEET #", "1".to_owned(), INK),
            ("PROJECT", scene.display_title(), INK),
            ("SCENE #", scene.scene_number.clone(), INK),
        ],
        vec![
            ("INT / EXT", int_ext.to_owned(), INK),
            ("SETTING", scene.title.clone(), INK),
            ("LOCATION", location, INK),
        ],
        vec![
            ("DESCRIPTION", scene.summary.clone(), INK),
            (
                "DAY / NIGHT",
                translate(language, &scene.day_night_type),
                INK,
            ),
            ("SCRIPT PAGES", format_eighths(scene.duration), INK),
        ],
        vec![
            ("CAST", bullet_list(&scene.cast), Color(0.88, 0.12, 0.12)),
            (
                "EXTRAS / BACKGROUND",
                bullet_list(&scene.extras),
                Color(0.0, 0.65, 0.15),
            ),
            (
                "WARDROBE",
                bullet_list(&scene.wardrobe),
                Color(0.58, 0.28, 0.08),
            ),
        ],
        vec![
            (
                "HAIR & MAKEUP",
                bullet_list(&scene.makeup_hair),
                Color(0.05, 0.40, 0.90),
            ),
            ("PROPS", bullet_list(&scene.props), Color(0.55, 0.15, 0.75)),
            (
                "SET DRESSING",
                bullet_list(&scene.set_dressing),
                Color(0.80, 0.55, 0.0),
            ),
        ],
        vec![
            (
                "VEHICLES",
                bullet_list(&scene.vehicles),
                Color(0.48, 0.12, 0.52),
            ),
            (
                "SPECIAL EQUIPMENT",
                bullet_list(&scene.special_equipment),
                Color(0.27, 0.35, 0.39),
            ),
            (
                "STUNTS",
                bullet_list(&scene.stunts),
                Color(0.78, 0.16, 0.16),
            ),
        ],
        vec![
            ("SFX", bullet_list(&scene.sfx), Color(0.0, 0.55, 0.65)),
            ("VFX", bullet_list(&scene.vfx), Color(0.0, 0.55, 0.65)),
        ],
        vec![("NOTES", scene.breakdown_notes.clone(), INK)],
    ]
}

fn bullet_list(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn draw_cell(
    pdf: &mut PdfDocument,
    x: usize,
    top: usize,
    width: usize,
    height: usize,
    value: &str,
    style: CellStyle,
) {
    pdf.stroked_rect(x, top, width, height, style.fill, GRID);
    let lines = value.split('\n').collect::<Vec<_>>();
    let line_height = style.font_size + 2;
    let line_count = (height.saturating_sub(4) / line_height).max(1);
    for (index, line) in lines.iter().take(line_count).enumerate() {
        let centered_x = x + 4;
        pdf.text(
            centered_x,
            top + 3 + index * line_height,
            style.font_size,
            &fit_text(line, width.saturating_sub(8) * 2 / style.font_size.max(1)),
            style.foreground,
            style.bold,
        );
    }
}

fn section_header(pdf: &mut PdfDocument, top: usize, label: &str, language: Language) -> usize {
    pdf.rect(28, top, 556, 17, NAVY);
    pdf.text(34, top + 4, 8, &translate(language, label), WHITE, true);
    top + 18
}

fn draw_labeled_bar(
    pdf: &mut PdfDocument,
    top: usize,
    label: &str,
    value: &str,
    language: Language,
) {
    pdf.stroked_rect(28, top, 556, 19, WHITE, GRID);
    pdf.text(
        34,
        top + 5,
        8,
        &format!("{}: {}", translate(language, label), value),
        INK,
        true,
    );
}

fn draw_table_header(
    pdf: &mut PdfDocument,
    top: usize,
    labels: &[&str],
    widths: &[usize],
    language: Language,
) {
    let mut x = 28;
    for (label, width) in labels.iter().zip(widths) {
        draw_cell(
            pdf,
            x,
            top,
            *width,
            18,
            &translate(language, label),
            CellStyle {
                fill: PALE,
                foreground: NAVY,
                bold: true,
                font_size: 7,
            },
        );
        x += width;
    }
}

fn draw_table_row(pdf: &mut PdfDocument, top: usize, values: &[&str], widths: &[usize]) {
    let mut x = 28;
    for (value, width) in values.iter().zip(widths) {
        draw_cell(
            pdf,
            x,
            top,
            *width,
            20,
            value,
            CellStyle {
                fill: WHITE,
                foreground: INK,
                bold: false,
                font_size: 7,
            },
        );
        x += width;
    }
}

fn ensure_call_sheet_room(pdf: &mut PdfDocument, top: usize, height: usize) -> usize {
    if top + height > PORTRAIT.1 - 36 {
        pdf.page();
        let continuation = format!(
            "{} - {}",
            translate(pdf.language, "CALL SHEET"),
            translate(pdf.language, "CONTINUED")
        );
        pdf.text(36, 16, 8, &continuation, MID, true);
        pdf.line(36, 32, PORTRAIT.0 - 36, 32, GRID);
        42
    } else {
        top
    }
}

fn format_contact(name: &str, phone: &str) -> String {
    match (name.trim().is_empty(), phone.trim().is_empty()) {
        (true, true) => String::new(),
        (true, false) => phone.to_owned(),
        (false, true) => name.to_owned(),
        (false, false) => format!("{name} - {phone}"),
    }
}

fn empty_default<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

fn wrap_text(text: &str, width: usize, size: usize) -> Vec<String> {
    let limit = (width / size.max(1)).max(1);
    let mut lines = Vec::new();
    for source_line in text.lines() {
        let mut current = String::new();
        for word in source_line.split_whitespace() {
            if !current.is_empty() && current.chars().count() + word.chars().count() + 1 > limit {
                lines.push(current);
                current = String::new();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        lines.push(current);
    }
    lines
}

fn fit_text(value: &str, max_chars: usize) -> String {
    let clean = value.replace(['\n', '\r'], " ");
    if clean.chars().count() <= max_chars {
        return clean;
    }
    let mut truncated = clean
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

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

fn parse_clock(value: &str) -> Option<i32> {
    let upper = value.trim().to_ascii_uppercase();
    let pm = upper.ends_with("PM");
    let am = upper.ends_with("AM");
    let time = upper.trim_end_matches("AM").trim_end_matches("PM").trim();
    let (hour, minute) = time.split_once(':')?;
    let mut hour: i32 = hour.parse().ok()?;
    let minute: i32 = minute.parse().ok()?;
    if !(0..=59).contains(&minute) || !(1..=12).contains(&hour) {
        return None;
    }
    if pm && hour < 12 {
        hour += 12;
    } else if am && hour == 12 {
        hour = 0;
    }
    Some(hour * 60 + minute)
}

fn clock_label(minutes: i32) -> String {
    let minutes = minutes.rem_euclid(24 * 60);
    let hour24 = minutes / 60;
    let hour12 = match hour24 % 12 {
        0 => 12,
        hour => hour,
    };
    format!(
        "{hour12:02}:{:02} {}",
        minutes % 60,
        if hour24 >= 12 { "PM" } else { "AM" }
    )
}

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
