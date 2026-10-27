//! Importers for Final Draft, Fountain, and Highland screenplay files.

use std::collections::BTreeSet;
use std::io::{Cursor, Read};
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;
use thiserror::Error;
use zip::ZipArchive;

use crate::model::Scene;
use crate::parsing::parse_pages_to_eighths;

/// Errors raised while opening or interpreting screenplay files.
#[derive(Debug, Error)]
pub enum ScriptImportError {
    /// The script could not be read from its selected path.
    #[error("could not read screenplay: {0}")]
    Io(#[from] std::io::Error),
    /// The Final Draft XML document is malformed.
    #[error("invalid Final Draft XML: {0}")]
    Xml(#[from] quick_xml::Error),
    /// The Highland archive is invalid or does not contain a screenplay text file.
    #[error("invalid Highland screenplay archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// The selected file type has no registered screenplay importer.
    #[error("unsupported screenplay file type: {0}")]
    Unsupported(String),
    /// A Highland screenplay entry is not valid UTF-8.
    #[error("Highland screenplay text is not valid UTF-8")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    /// A file could not be interpreted as a screenplay.
    #[error("screenplay contains no recognized scenes")]
    NoScenes,
}

/// Imports scenes from a supported screenplay file path.
///
/// The extension selects Final Draft XML, Fountain text, or a Highland ZIP
/// archive. Scene estimates use the existing eighths-per-page time convention.
pub fn import_script(path: impl AsRef<Path>) -> Result<Vec<Scene>, ScriptImportError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "fdx" => parse_final_draft(&bytes),
        "fountain" | "fnt" | "txt" => {
            let text = String::from_utf8(bytes)?;
            parse_fountain(&text)
        }
        "highland" => parse_highland(&bytes),
        _ => Err(ScriptImportError::Unsupported(extension)),
    }
}

/// Converts Final Draft XML into scene records with cast and page estimates.
pub fn parse_final_draft(xml: &[u8]) -> Result<Vec<Scene>, ScriptImportError> {
    let mut reader = Reader::from_reader(Cursor::new(xml));
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut paragraphs = Vec::new();
    let mut paragraph_depth = 0usize;
    let mut paragraph_type = String::new();
    let mut paragraph_text = String::new();
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(event) if event.name().as_ref() == b"Paragraph" => {
                if paragraph_depth == 0 {
                    paragraph_type = event
                        .attributes()
                        .flatten()
                        .find(|attribute| attribute.key.as_ref() == b"Type")
                        .map(|attribute| String::from_utf8_lossy(&attribute.value).into_owned())
                        .unwrap_or_else(|| "Action".to_owned());
                    paragraph_text.clear();
                }
                paragraph_depth += 1;
            }
            Event::Empty(event) if event.name().as_ref() == b"Paragraph" => {
                if paragraph_depth == 0 {
                    let kind = event
                        .attributes()
                        .flatten()
                        .find(|attribute| attribute.key.as_ref() == b"Type")
                        .map(|attribute| String::from_utf8_lossy(&attribute.value).into_owned())
                        .unwrap_or_else(|| "Action".to_owned());
                    paragraphs.push((kind, String::new()));
                }
            }
            Event::Start(event) if event.name().as_ref() == b"Text" && paragraph_depth == 1 => {
                in_text = true;
            }
            Event::End(event) if event.name().as_ref() == b"Text" => in_text = false,
            Event::Text(text) if in_text => {
                paragraph_text.push_str(&text.unescape()?);
            }
            Event::CData(text) if in_text => {
                paragraph_text.push_str(&String::from_utf8_lossy(&text));
            }
            Event::End(event) if event.name().as_ref() == b"Paragraph" => {
                paragraph_depth = paragraph_depth.saturating_sub(1);
                if paragraph_depth == 0 && !paragraph_type.is_empty() {
                    paragraphs.push((paragraph_type.clone(), paragraph_text.trim().to_owned()));
                    paragraph_type.clear();
                    paragraph_text.clear();
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    let scenes = build_scenes(paragraphs);
    if scenes.is_empty() {
        Err(ScriptImportError::NoScenes)
    } else {
        Ok(scenes)
    }
}

/// Converts Fountain screenplay text into scene records.
pub fn parse_fountain(text: &str) -> Result<Vec<Scene>, ScriptImportError> {
    let mut paragraphs = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('=') {
            index += 1;
            continue;
        }
        let heading = line.strip_prefix('.').unwrap_or(line);
        if is_scene_heading(heading) {
            paragraphs.push(("Scene Heading".to_owned(), heading.to_owned()));
            index += 1;
            continue;
        }
        let next_line = lines
            .get(index + 1)
            .map(|line| line.trim())
            .unwrap_or_default();
        if is_character_cue(line) && !next_line.is_empty() && !is_scene_heading(next_line) {
            paragraphs.push(("Character".to_owned(), line.to_owned()));
            index += 1;
            while index < lines.len() {
                let dialogue = lines[index].trim();
                if dialogue.is_empty() || is_scene_heading(dialogue) {
                    break;
                }
                paragraphs.push(("Dialogue".to_owned(), dialogue.to_owned()));
                index += 1;
            }
        } else {
            paragraphs.push(("Action".to_owned(), line.to_owned()));
            index += 1;
        }
    }
    let scenes = build_scenes(paragraphs);
    if scenes.is_empty() {
        Err(ScriptImportError::NoScenes)
    } else {
        Ok(scenes)
    }
}

/// Extracts and imports the screenplay text embedded in a Highland archive.
pub fn parse_highland(archive: &[u8]) -> Result<Vec<Scene>, ScriptImportError> {
    let mut archive = ZipArchive::new(Cursor::new(archive))?;
    let entry_name = ["text.fountain", "text.markdown", "text.txt"]
        .into_iter()
        .find(|candidate| archive.file_names().any(|name| name.ends_with(candidate)))
        .ok_or(ScriptImportError::NoScenes)?;
    let name = archive
        .file_names()
        .find(|name| name.ends_with(entry_name))
        .map(str::to_owned)
        .ok_or(ScriptImportError::NoScenes)?;
    let mut entry = archive.by_name(&name)?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    parse_fountain(&String::from_utf8(bytes)?)
}

/// Groups screenplay paragraphs into numbered scenes with rough page estimates.
fn build_scenes(paragraphs: Vec<(String, String)>) -> Vec<Scene> {
    let mut scenes: Vec<Scene> = Vec::new();
    let mut cast = BTreeSet::new();
    let mut line_count = 0usize;

    for (kind, text) in paragraphs {
        if kind == "Scene Heading" {
            finish_scene(&mut scenes, &mut cast, line_count);
            let (number, heading) = split_scene_number(&text);
            let mut scene = Scene::new(heading.to_ascii_uppercase());
            scene.scene_number = number.unwrap_or_else(|| (scenes.len() + 1).to_string());
            scene.day_night_type = classify_time_of_day(&scene.title);
            scenes.push(scene);
            line_count = 2;
            continue;
        }
        let Some(scene) = scenes.last_mut() else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        if kind == "Character" {
            let character = clean_character_name(&text);
            if !character.is_empty() {
                cast.insert(character);
            }
            line_count += 2;
        } else if kind == "Dialogue" {
            line_count += text.chars().count().div_ceil(36).max(1);
        } else if kind == "Parenthetical" {
            line_count += text.chars().count().div_ceil(32).max(1);
        } else {
            line_count += text.chars().count().div_ceil(58).max(1) + 1;
            if scene.summary.is_empty() {
                scene.summary = text.clone();
            }
        }
    }
    finish_scene(&mut scenes, &mut cast, line_count);
    scenes
}

/// Finalizes the line-count estimate and cast list for the current scene.
fn finish_scene(scenes: &mut [Scene], cast: &mut BTreeSet<String>, line_count: usize) {
    let Some(scene) = scenes.last_mut() else {
        cast.clear();
        return;
    };
    scene.duration = ((line_count as f64 / 54.0) * 8.0).round().max(1.0) as i32;
    scene.estimated_time = scene.duration * 15;
    scene.cast = std::mem::take(cast).into_iter().collect();
}

/// Identifies standard interior/exterior screenplay scene headings.
fn is_scene_heading(line: &str) -> bool {
    let uppercase = line.to_ascii_uppercase();
    ["INT.", "EXT.", "INT/EXT", "INT./EXT.", "I/E."]
        .iter()
        .any(|prefix| uppercase.starts_with(prefix))
}

/// Detects an uppercase screenplay character cue followed by dialogue.
fn is_character_cue(line: &str) -> bool {
    let name = line.trim_end_matches('^').trim();
    !name.is_empty()
        && name
            .chars()
            .any(|character| character.is_ascii_alphabetic())
        && name == name.to_ascii_uppercase()
        && name.len() <= 38
        && !name.ends_with(':')
}

/// Separates optional numeric scene identifiers from the heading.
fn split_scene_number(heading: &str) -> (Option<String>, String) {
    let heading = heading.trim();
    if let Some(rest) = heading.strip_prefix('#') {
        if let Some((number, rest)) = rest.split_once('#') {
            if valid_scene_number(number) {
                return (Some(number.to_owned()), rest.trim().to_owned());
            }
        }
    }
    if let Some((prefix, suffix)) = heading.rsplit_once('#') {
        if let Some((heading_text, marker)) = prefix.rsplit_once('#') {
            let number = marker.trim_start_matches('#');
            if valid_scene_number(number) && suffix.trim().is_empty() {
                return (Some(number.to_owned()), heading_text.trim().to_owned());
            }
        }
    }
    let heading = heading.strip_prefix('.').unwrap_or(heading).trim();
    let Some((first, rest)) = heading.split_once(char::is_whitespace) else {
        return (None, heading.to_owned());
    };
    let number = first.trim_end_matches('.');
    if valid_scene_number(number) {
        (Some(number.to_owned()), rest.trim().to_owned())
    } else {
        (None, heading.to_owned())
    }
}

/// Checks the ASCII numeric-plus-letter scene-number convention.
fn valid_scene_number(number: &str) -> bool {
    !number.is_empty()
        && number
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
        && number
            .chars()
            .all(|character| character.is_ascii_digit() || character.is_ascii_alphabetic())
}

/// Identifies the time-of-day category encoded by a scene heading.
fn classify_time_of_day(heading: &str) -> String {
    let heading = heading.to_ascii_uppercase();
    if heading.contains("DAWN") || heading.contains("SUNRISE") {
        "DAWN"
    } else if heading.contains("DUSK") || heading.contains("SUNSET") || heading.contains("TWILIGHT")
    {
        "DUSK"
    } else if heading.contains("AFTERNOON") {
        "AFTERNOON"
    } else if heading.contains("NIGHT") || heading.contains("EVENING") {
        "NIGHT"
    } else {
        "DAY"
    }
    .to_owned()
}

/// Cleans screenplay extensions from a character cue.
fn clean_character_name(name: &str) -> String {
    let uppercase = name.to_ascii_uppercase();
    let cleaned = uppercase
        .split_once('(')
        .map_or(uppercase.as_str(), |(name, _)| name);
    cleaned.trim().to_owned()
}

/// Parses the page-length input used by the scene editor.
pub fn parse_page_estimate(input: &str) -> Option<i32> {
    parse_pages_to_eighths(input)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    use super::{parse_final_draft, parse_fountain, parse_highland};

    /// Imports Fountain scene headings, scene numbers, and character cues.
    #[test]
    fn fountain_import_extracts_scenes_and_cast() {
        let scenes = parse_fountain(
            "Title: Sample\n\n# Act One\n\nINT. HOUSE - DAY #3A#\n\nMAYA\nHello.\n\nEXT. PARK - NIGHT\n\nRain falls.\n",
        )
        .expect("Fountain scenes parse");

        assert_eq!(scenes.len(), 2);
        assert_eq!(scenes[0].scene_number, "3A");
        assert_eq!(scenes[0].title, "INT. HOUSE - DAY");
        assert_eq!(scenes[0].cast, ["MAYA"]);
        assert_eq!(scenes[1].day_night_type, "NIGHT");
    }

    /// Imports Final Draft XML and supports nested dual-dialogue paragraphs.
    #[test]
    fn final_draft_import_handles_nested_paragraphs() {
        let xml = br#"<FinalDraft><Content>
            <Paragraph Type="Scene Heading"><Text>1. INT. HOUSE - DAY</Text></Paragraph>
            <Paragraph Type="Character"><Text>MAYA</Text></Paragraph>
            <Paragraph Type="Dialogue"><Text>Hello &amp; goodbye.</Text></Paragraph>
            <Paragraph Type="Action"><Paragraph Type="Character"><Text>OMAR</Text></Paragraph></Paragraph>
            <Paragraph Type="Scene Heading"><Text>EXT. PARK - NIGHT</Text></Paragraph>
        </Content></FinalDraft>"#;

        let scenes = parse_final_draft(xml).expect("Final Draft scenes parse");
        assert_eq!(scenes.len(), 2);
        assert_eq!(scenes[0].scene_number, "1");
        assert_eq!(scenes[0].cast, ["MAYA"]);
        assert_eq!(scenes[1].day_night_type, "NIGHT");
    }

    /// Extracts the screenplay Fountain entry from a Highland TextBundle archive.
    #[test]
    fn highland_import_reads_text_fountain_entry() {
        let cursor = std::io::Cursor::new(Vec::new());
        let mut archive = ZipWriter::new(cursor);
        archive
            .start_file("text.fountain", SimpleFileOptions::default())
            .expect("archive entry starts");
        archive
            .write_all(b"INT. STAGE - DAY\n\nMAYA\nHello.\n")
            .expect("screenplay text is written");
        let archive = archive.finish().expect("archive finishes").into_inner();

        let scenes = parse_highland(&archive).expect("Highland archive parses");
        assert_eq!(scenes.len(), 1);
        assert_eq!(scenes[0].cast, ["MAYA"]);
    }
}
