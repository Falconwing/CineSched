//! English and Spanish labels for the Rust desktop application.

use chrono::{Datelike, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

/// The user-selectable language for application controls and report headings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// English labels, used for projects created before language selection existed.
    #[default]
    English,
    /// Spanish labels for the bilingual CineSched interface.
    Spanish,
}

impl Language {
    /// Returns the language's localized name for the language selector.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
        }
    }
}

/// Formats the month heading using the selected language's month name.
pub fn format_month(date: NaiveDate, language: Language) -> String {
    if language == Language::English {
        return date.format("%B %Y").to_string();
    }

    format!("{} {}", spanish_month(date.month()), date.year())
}

/// Formats a full date with translated weekday and month names.
pub fn format_long_date(date: NaiveDate, language: Language) -> String {
    if language == Language::English {
        return date.format("%A, %B %d, %Y").to_string();
    }

    let weekday = match date.weekday() {
        Weekday::Mon => "lunes",
        Weekday::Tue => "martes",
        Weekday::Wed => "miércoles",
        Weekday::Thu => "jueves",
        Weekday::Fri => "viernes",
        Weekday::Sat => "sábado",
        Weekday::Sun => "domingo",
    };
    format!(
        "{weekday}, {} de {} de {}",
        date.day(),
        spanish_month(date.month()),
        date.year()
    )
}

fn spanish_month(month: u32) -> &'static str {
    match month {
        1 => "enero",
        2 => "febrero",
        3 => "marzo",
        4 => "abril",
        5 => "mayo",
        6 => "junio",
        7 => "julio",
        8 => "agosto",
        9 => "septiembre",
        10 => "octubre",
        11 => "noviembre",
        12 => "diciembre",
        _ => "",
    }
}

/// Translates a stable English source label, preserving unknown text unchanged.
pub fn translate(language: Language, source: &str) -> String {
    if language == Language::English {
        return source.to_owned();
    }

    match source {
        "New" => "Nuevo",
        "Open..." => "Abrir...",
        "Open Recent" => "Abrir recientes",
        "No recent projects" => "No hay proyectos recientes",
        "Clear Recent Projects" => "Borrar proyectos recientes",
        "Save" => "Guardar",
        "Save As..." => "Guardar como...",
        "Import Script..." => "Importar guion...",
        "Calendar" => "Calendario",
        "Stripboard" => "Tira de rodaje",
        "Production Setup" => "Configuración de producción",
        "Dark Mode" => "Modo oscuro",
        "Show Cast" => "Mostrar reparto",
        "Show Estimated Time" => "Mostrar tiempo estimado",
        "Include Holds in DOOD" => "Incluir pausas en el informe DOOD",
        "Scan Conflicts" => "Buscar conflictos",
        "Check Duplicate Numbers" => "Comprobar números duplicados",
        "Lock Schedule" => "Bloquear planificación",
        "Unlock" => "Desbloquear",
        "Lock Report" => "Informe de cambios",
        "Undo" => "Deshacer",
        "Redo" => "Rehacer",
        "Export PDF..." => "Exportar PDF...",
        "Breakdowns PDF..." => "PDF de desglose...",
        "DOOD PDF..." => "PDF de DOOD...",
        "Month PDF..." => "PDF mensual...",
        "Stripboard PDF..." => "PDF de tiras...",
        "One-Line PDF..." => "PDF de plan de rodaje...",
        "An autosaved project is available to recover." => {
            "Hay un proyecto guardado automáticamente disponible para recuperar."
        }
        "Recover" => "Recuperar",
        "Discard" => "Descartar",
        "Add Scene" => "Añadir escena",
        "Add 30-Day Calendar" => "Añadir calendario de 30 días",
        "Call Sheet" => "Orden de rodaje",
        "General and Schedule" => "Información general y planificación",
        "General Call" => "Llamado general",
        "Estimated Workday Schedule" => "Horario estimado de trabajo",
        "Quote of the Day" => "Frase del día",
        "Ready to Shoot" => "Listos para rodar",
        "Snack" => "Merienda",
        "Dinner" => "Cena",
        "Estimated Wrap" => "Fin de jornada estimado",
        "Production Manager Contact" => "Contacto de producción",
        "Assistant Director Contact" => "Contacto de dirección",
        "Nearest Hospital / Emergency Facility" => "Hospital / centro de emergencias más cercano",
        "Weather and Sun Times" => "Clima y horarios solares",
        "Temperature" => "Temperatura",
        "Sky Condition" => "Estado del cielo",
        "Precipitation and Wind" => "Precipitación y viento",
        "Sunrise / Sunset" => "Amanecer / atardecer",
        "Locations and Basecamp" => "Locaciones y base",
        "Basecamp" => "Base",
        "Add Location" => "Añadir locación",
        "Add from Location Roster" => "Añadir desde la lista de locaciones",
        "Populate from Scene Locations" => "Completar desde las locaciones de escena",
        "Remove" => "Quitar",
        "Cast Calls" => "Llamados del reparto",
        "Populate / Update Cast from Scenes" => "Completar / actualizar reparto desde escenas",
        "Add Cast Row" => "Añadir fila de reparto",
        "Character" => "Personaje",
        "Actor" => "Actor",
        "Scenes" => "Escenas",
        "Pickup" => "Recogida",
        "HMU / Wardrobe" => "Maquillaje / vestuario",
        "On Set" => "En set",
        "Wrap" => "Fin de jornada",
        "Location #" => "Locación n.º",
        "Cast member" => "Integrante del reparto",
        "Crew Calls" => "Llamados del equipo",
        "Load Crew from Production Setup" => "Cargar equipo desde configuración",
        "Add Crew Row" => "Añadir fila del equipo",
        "Role" => "Función",
        "Name" => "Nombre",
        "Call Time" => "Hora de llamado",
        "Phone" => "Teléfono",
        "Production Notes" => "Notas de producción",
        "Call sheet notes" => "Notas de la orden de rodaje",
        "Production notes (one per line)" => "Notas de producción (una por línea)",
        "Export Call Sheet PDF..." => "Exportar orden de rodaje PDF...",
        "Company" => "Compañía",
        "Cast and Character Roster" => "Lista de reparto y personajes",
        "New unavailability range (YYYY-MM-DD)" => "Nuevo período de indisponibilidad (AAAA-MM-DD)",
        "Add Cast Member" => "Añadir integrante del reparto",
        "Crew Roster" => "Lista del equipo",
        "Add Crew Member" => "Añadir integrante del equipo",
        "Add range" => "Añadir período",
        "Enter dates using YYYY-MM-DD" => "Introduce fechas con el formato AAAA-MM-DD",
        "Cast Availability Conflicts" => "Conflictos de disponibilidad del reparto",
        "No cast availability conflicts found." => {
            "No se encontraron conflictos de disponibilidad del reparto."
        }
        "Schedule Lock Report" => "Informe de bloqueo de planificación",
        "No schedule changes since the lock." => {
            "No hay cambios en la planificación desde el bloqueo."
        }
        "Added:" => "Añadidos:",
        "Removed:" => "Quitados:",
        "Edit Scene" => "Editar escena",
        "Scene Number" => "Número de escena",
        "Page Eighths" => "Octavos de página",
        "Estimated Minutes" => "Minutos estimados",
        "Time of Day" => "Momento del día",
        "Cast (comma separated)" => "Reparto (separado por comas)",
        "Summary" => "Sinopsis",
        "Real Location" => "Locación real",
        "Completed" => "Completada",
        "Breakdown Notes" => "Notas del desglose",
        "Daily" => "Diario",
        "Previous Day" => "Día anterior",
        "Next Day" => "Día siguiente",
        "Company Move" => "Traslado de compañía",
        "Meal Break" => "Pausa para comer",
        "Notice" => "Aviso",
        "Custom Banner" => "Aviso personalizado",
        "Search scenes..." => "Buscar escenas...",
        "Sun" => "Dom",
        "Mon" => "Lun",
        "Tue" => "Mar",
        "Wed" => "Mié",
        "Thu" => "Jue",
        "Fri" => "Vie",
        "Sat" => "Sáb",
        "Unavailable" => "No disponible",
        "through" => "hasta",
        "Heading" => "Encabezado",
        "Address" => "Dirección",
        "Exported" => "Exportado",
        "scenes selected" => "escenas seleccionadas",
        "Schedule selected" => "Programar selección",
        "Choose day" => "Elegir día",
        "Add Day" => "Añadir día",
        "Boneyard" => "Escenas sin programar",
        "Drop scheduled scenes here to return them to the Boneyard" => {
            "Suelta aquí las escenas programadas para devolverlas a las escenas sin programar"
        }
        "No unscheduled scenes." => "No hay escenas sin programar.",
        "Select visible" => "Seleccionar visibles",
        "Clear selection" => "Borrar selección",
        "Schedule" => "Planificación",
        "Today" => "Hoy",
        "Add Calendar Event" => "Añadir evento al calendario",
        "Add Banner" => "Añadir aviso",
        "Add Meal" => "Añadir comida",
        "Scene" => "Escena",
        "Day" => "Día",
        "Night" => "Noche",
        "Dawn" => "Amanecer",
        "Dusk" => "Atardecer",
        "Afternoon" => "Tarde",
        "Custom" => "Personalizado",
        "DAY" => "DÍA",
        "NIGHT" => "NOCHE",
        "DAWN" => "AMANECER",
        "DUSK" => "ATARDECER",
        "AFTERNOON" => "TARDE",
        "CUSTOM" => "PERSONALIZADO",
        "Show Order" => "Orden del guion",
        "Default" => "Predeterminado",
        "Location" => "Locación",
        "INT/EXT" => "INT/EXT",
        "Cast" => "Reparto",
        "Day/Night" => "Día/noche",
        "Scene title" => "Título de escena",
        "Created a new project" => "Se creó un proyecto nuevo",
        "Schedule locked" => "Planificación bloqueada",
        "Schedule unlocked" => "Planificación desbloqueada",
        "No duplicate scene numbers found" => "No hay números de escena duplicados",
        "Undid the last change" => "Se deshizo el último cambio",
        "Redid the last change" => "Se rehizo el último cambio",
        "Project" => "Proyecto",
        "Language" => "Idioma",
        "SHOOTING SCHEDULE" => "PLAN DE RODAJE",
        "PRODUCTION CALENDAR" => "CALENDARIO DE PRODUCCIÓN",
        "STRIPBOARD" => "TIRA DE RODAJE",
        "ONE-LINE SHOOTING SCHEDULE" => "PLAN DE RODAJE",
        "CALL SHEET" => "ORDEN DE RODAJE",
        "CONTINUED" => "CONTINUACIÓN",
        "GENERAL CALL" => "LLAMADO GENERAL",
        "QUOTE OF THE DAY" => "FRASE DEL DÍA",
        "SCHEDULE" => "HORARIO",
        "CONTACTS" => "CONTACTOS",
        "MILESTONES" => "HORARIOS CLAVE",
        "WEATHER" => "CLIMA",
        "PRODUCER" => "PRODUCCIÓN",
        "1ST AD" => "AYUDANTE DE DIRECCIÓN",
        "READY TO SHOOT" => "LISTOS PARA RODAR",
        "LUNCH" => "ALMUERZO",
        "SNACK" => "MERIENDA",
        "WRAP" => "FIN DE JORNADA",
        "TEMP" => "TEMP.",
        "SKY" => "CIELO",
        "WIND" => "VIENTO",
        "SUN" => "SOL",
        "BASECAMP" => "BASE",
        "NEAREST HOSPITAL" => "HOSPITAL MÁS CERCANO",
        "LOCATIONS" => "LOCACIONES",
        "SCENES" => "ESCENAS",
        "SCENE" => "ESCENA",
        "SET / DESCRIPTION" => "DECORADO / DESCRIPCIÓN",
        "CAST" => "REPARTO",
        "PAGES" => "PÁGINAS",
        "LOCATION" => "LOCACIÓN",
        "CAST CALLS" => "LLAMADOS DEL REPARTO",
        "ACTOR / CHARACTER" => "ACTOR / PERSONAJE",
        "PICKUP" => "RECOGIDA",
        "HMU" => "MAQUILLAJE",
        "SET" => "SET",
        "CREW CALLS" => "LLAMADOS DEL EQUIPO",
        "ROLE" => "FUNCIÓN",
        "NAME" => "NOMBRE",
        "CALL" => "LLAMADO",
        "PHONE" => "TELÉFONO",
        "PRODUCTION NOTES" => "NOTAS DE PRODUCCIÓN",
        "END OF DAY" => "FIN DE JORNADA",
        "min" => "min",
        "DAY / NIGHT" => "DÍA / NOCHE",
        "SCRIPT PAGES" => "PÁGINAS DE GUIÓN",
        "BREAKDOWN SHEET" => "HOJA DE DESGLOSE",
        "BREAKDOWN SHEET #" => "HOJA DE DESGLOSE N.º",
        "PROJECT" => "PROYECTO",
        "SCENE #" => "ESCENA N.º",
        "INT / EXT" => "INT / EXT",
        "SETTING" => "DECORADO",
        "DESCRIPTION" => "DESCRIPCIÓN",
        "EXTRAS / BACKGROUND" => "EXTRAS / FIGURACIÓN",
        "WARDROBE" => "VESTUARIO",
        "HAIR & MAKEUP" => "PELUQUERÍA Y MAQUILLAJE",
        "PROPS" => "UTILERÍA",
        "SET DRESSING" => "DECORACIÓN",
        "VEHICLES" => "VEHÍCULOS",
        "SPECIAL EQUIPMENT" => "EQUIPO ESPECIAL",
        "STUNTS" => "DOBLES Y ACROBACIAS",
        "SFX" => "EFECTOS ESPECIALES",
        "VFX" => "EFECTOS VISUALES",
        "NOTES" => "NOTAS",
        "DAYS OUT OF DAYS" => "DÍAS DE TRABAJO DEL REPARTO",
        "CHARACTER" => "PERSONAJE",
        "TOT" => "TOT",
        "WRK" => "TRAB",
        "HLD" => "ESP",
        "PAGE" => "PÁGINA",
        "Page" => "Página",
        "Start Work" => "Inicio de trabajo",
        "Work" => "Trabajo",
        "Hold" => "Espera",
        "Work Finish" => "Fin de trabajo",
        "Start / Work / Finish" => "Inicio / trabajo / fin",
        "Untitled Movie" => "Película sin título",
        "scene" => "escena",
        "scenes" => "escenas",
        "pages" => "páginas",
        "pg" => "pág.",
        "more" => "más",
        "SCENE BREAKDOWNS" => "DESGLOSE DE ESCENAS",
        "DATE" => "FECHA",
        "Saved" => "Guardado",
        "Opened" => "Abierto",
        "Autosave is unavailable on this platform" => {
            "El guardado automático no está disponible en esta plataforma"
        }
        "Autosave recovery is unavailable" => {
            "La recuperación del guardado automático no está disponible"
        }
        "Recovered the latest autosaved project" => {
            "Se recuperó la última versión guardada automáticamente"
        }
        "No autosaved project is available" => {
            "No hay un proyecto guardado automáticamente disponible"
        }
        "Discarded the recovered project snapshot" => "Se descartó la copia de recuperación",
        "Project changed but autosave path is unavailable" => {
            "El proyecto cambió, pero no está disponible la ruta de guardado automático"
        }
        "Could not interpret the selected calendar date" => {
            "No se pudo interpretar la fecha seleccionada"
        }
        "Added 30 calendar days" => "Se añadieron 30 días al calendario",
        "Swapped schedule and call sheet between days" => {
            "Se intercambiaron la planificación y la orden de rodaje entre los días"
        }
        "Could not swap the selected shooting days" => {
            "No se pudieron intercambiar los días de rodaje seleccionados"
        }
        "Could not move scene to the selected day" => {
            "No se pudo mover la escena al día seleccionado"
        }
        "duplicate scene numbers found" => "números de escena duplicados",
        "Call sheet PDF exported" => "Se exportó el PDF de la orden de rodaje",
        "Monthly calendar PDF exported" => "Se exportó el PDF del calendario mensual",
        "Stripboard PDF exported" => "Se exportó el PDF de la tira de rodaje",
        "One-line schedule PDF exported" => "Se exportó el PDF del plan de rodaje",
        "Exported " => "Exportado ",
        "Opened " => "Abierto ",
        "Saved " => "Guardado ",
        "Autosave recovery check failed: " => "Falló la comprobación de recuperación: ",
        "Autosave recovery failed: " => "Falló la recuperación del guardado automático: ",
        "Could not discard autosave: " => "No se pudo descartar el guardado automático: ",
        "Could not compare project state: " => "No se pudo comparar el estado del proyecto: ",
        "Autosave failed: " => "Falló el guardado automático: ",
        "Open failed: " => "Falló la apertura: ",
        "Save failed: " => "Falló el guardado: ",
        "Script import failed: " => "Falló la importación del guion: ",
        "Schedule export failed: " => "Falló la exportación de la planificación: ",
        "Call sheet export failed: " => "Falló la exportación de la orden de rodaje: ",
        "Breakdown export failed: " => "Falló la exportación del desglose: ",
        "DOOD export failed: " => "Falló la exportación del informe de días: ",
        "Calendar export failed: " => "Falló la exportación del calendario: ",
        "Stripboard export failed: " => "Falló la exportación de la tira de rodaje: ",
        "One-line export failed: " => "Falló la exportación del plan de rodaje: ",
        "Could not clear old autosave: " => "No se pudo borrar el guardado automático anterior: ",
        "Could not clear autosave: " => "No se pudo borrar el guardado automático: ",
        "Could not discard autosave" => "No se pudo descartar el guardado automático",
        "Imported " => "Importadas ",
        "Returned " => "Devueltas ",
        " scenes from " => " escenas desde ",
        " scenes to the selected day" => " escenas al día seleccionado",
        " scenes to the Boneyard" => " escenas a las escenas sin programar",
        "the selected shooting day does not exist" => "el día de rodaje seleccionado no existe",
        "could not write report: " => "no se pudo escribir el informe: ",
        _ => return source.to_owned(),
    }
    .to_owned()
}

/// Translates a complete status message while preserving variable values and error details.
pub fn translate_status(language: Language, message: &str) -> String {
    if language == Language::English {
        return message.to_owned();
    }

    if let Some((path, error)) = message
        .strip_prefix("Opened ")
        .and_then(|remaining| remaining.split_once(", but could not clear old autosave: "))
    {
        return format!(
            "{}{path}, {}{error}",
            translate(language, "Opened "),
            translate(language, "Could not clear old autosave: ")
        );
    }
    if let Some((path, error)) = message
        .strip_prefix("Saved ")
        .and_then(|remaining| remaining.split_once(", but could not clear autosave: "))
    {
        return format!(
            "{}{path}, {}{error}",
            translate(language, "Saved "),
            translate(language, "Could not clear autosave: ")
        );
    }

    for english_prefix in [
        "Autosave recovery check failed: ",
        "Autosave recovery failed: ",
        "Could not discard autosave: ",
        "Could not compare project state: ",
        "Autosave failed: ",
        "Open failed: ",
        "Save failed: ",
        "Script import failed: ",
        "Schedule export failed: ",
        "Call sheet export failed: ",
        "Breakdown export failed: ",
        "DOOD export failed: ",
        "Calendar export failed: ",
        "Stripboard export failed: ",
        "One-line export failed: ",
        "Opened ",
        "Saved ",
        "Exported ",
    ] {
        if let Some(detail) = message.strip_prefix(english_prefix) {
            return format!(
                "{}{}",
                translate(language, english_prefix),
                translate_error_detail(language, detail)
            );
        }
    }

    if let Some((count, path)) = message
        .strip_prefix("Imported ")
        .and_then(|remaining| remaining.split_once(" scenes from "))
    {
        return format!("Se importaron {count} escenas desde {path}");
    }
    if message.starts_with("Returned ") {
        if let Some(count) = message
            .strip_prefix("Returned ")
            .and_then(|remaining| remaining.split_once(" scenes to the Boneyard"))
            .map(|(count, _)| count)
        {
            return format!("Se devolvieron {count} escenas a la lista de escenas sin programar");
        }
    }
    if let Some(count) = message
        .strip_suffix(" duplicate scene numbers found")
        .and_then(|_| message.split_once(' ').map(|(count, _)| count))
    {
        return format!(
            "{count} {}",
            translate(language, "duplicate scene numbers found")
        );
    }
    if let Some(count) = message
        .strip_prefix("Scheduled ")
        .and_then(|remaining| remaining.strip_suffix(" scenes"))
    {
        return format!("Se programaron {count} escenas");
    }
    if let Some(count) = message
        .strip_prefix("Moved ")
        .and_then(|remaining| remaining.strip_suffix(" scenes to the selected day"))
    {
        return format!("Se movieron {count} escenas al día seleccionado");
    }
    translate(language, message)
}

fn translate_error_detail(language: Language, detail: &str) -> String {
    for english_prefix in [
        "could not write report: ",
        "the selected shooting day does not exist",
    ] {
        if let Some(remainder) = detail.strip_prefix(english_prefix) {
            return format!("{}{}", translate(language, english_prefix), remainder);
        }
    }
    detail.to_owned()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::{format_long_date, format_month, translate, translate_status, Language};

    /// Confirms English remains the default and Spanish labels have an English fallback.
    #[test]
    fn translates_known_labels_and_preserves_unknown_text() {
        assert_eq!(Language::default(), Language::English);
        assert_eq!(translate(Language::Spanish, "Save"), "Guardar");
        assert_eq!(
            translate(Language::Spanish, "Untranslated project name"),
            "Untranslated project name"
        );
        assert_eq!(Language::Spanish.display_name(), "Español");
        let date = NaiveDate::from_ymd_opt(2025, 1, 2).expect("fixed test date is valid");
        assert_eq!(format_month(date, Language::Spanish), "enero 2025");
        assert_eq!(
            format_long_date(date, Language::Spanish),
            "jueves, 2 de enero de 2025"
        );
        assert_eq!(
            translate_status(Language::Spanish, "Saved C:\\film.json"),
            "Guardado C:\\film.json"
        );
        assert_eq!(
            translate_status(Language::Spanish, "Scheduled 3 scenes"),
            "Se programaron 3 escenas"
        );
        assert_eq!(
            translate_status(Language::Spanish, "Moved 2 scenes to the selected day"),
            "Se movieron 2 escenas al día seleccionado"
        );
        assert_eq!(
            translate_status(Language::Spanish, "Returned 4 scenes to the Boneyard"),
            "Se devolvieron 4 escenas a la lista de escenas sin programar"
        );
        assert_eq!(
            translate_status(Language::Spanish, "7 duplicate scene numbers found"),
            "7 números de escena duplicados"
        );
        assert_eq!(
            translate_status(
                Language::Spanish,
                "Schedule export failed: could not write report: access denied"
            ),
            "Falló la exportación de la planificación: no se pudo escribir el informe: access denied"
        );
    }
}
