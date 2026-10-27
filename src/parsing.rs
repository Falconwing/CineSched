//! Input parsing helpers used by scene editors and schedule summaries.

/// Converts integer, decimal, and fractional page lengths to eighths.
///
/// Integer input is treated as an eighth count, matching CineSched's existing
/// scene-entry behavior. Decimal values are interpreted as page counts.
pub fn parse_pages_to_eighths(input: &str) -> Option<i32> {
    let value = input.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(eighths) = value.parse::<i32>() {
        return Some(eighths);
    }
    if let Ok(pages) = value.parse::<f64>() {
        return Some((pages * 8.0).round() as i32);
    }
    if let Some((whole, fraction)) = value.split_once(' ') {
        let whole = whole.parse::<i32>().ok()?;
        let fraction = parse_fraction(fraction)?;
        return Some(whole * 8 + fraction);
    }
    parse_fraction(value)
}

/// Parses the numerator and denominator of a page fraction.
fn parse_fraction(value: &str) -> Option<i32> {
    let (numerator, denominator) = value.split_once('/')?;
    let numerator = numerator.parse::<i32>().ok()?;
    let denominator = denominator.parse::<i32>().ok()?;
    if denominator <= 0 {
        return None;
    }
    Some(numerator * 8 / denominator)
}

/// Converts common hour, minute, and `H:MM` inputs to total minutes.
pub fn parse_time_to_minutes(input: &str) -> Option<i32> {
    let value = input.trim();
    if value.is_empty() {
        return None;
    }
    if let Some((hours, minutes)) = value.split_once(':') {
        let hours = hours.parse::<i32>().ok()?;
        let minutes = minutes.parse::<i32>().ok()?;
        if hours < 0 || !(0..60).contains(&minutes) {
            return None;
        }
        return Some(hours * 60 + minutes);
    }
    if let Ok(decimal) = value.parse::<f64>() {
        return Some(if decimal <= 14.0 {
            (decimal * 60.0) as i32
        } else {
            decimal as i32
        });
    }
    let integer = value.parse::<i32>().ok()?;
    Some(if integer <= 10 { integer * 60 } else { integer })
}

#[cfg(test)]
mod tests {
    use super::{parse_pages_to_eighths, parse_time_to_minutes};

    /// Exercises common screenplay page-length formats and malformed values.
    #[test]
    fn parses_page_lengths() {
        assert_eq!(parse_pages_to_eighths("15"), Some(15));
        assert_eq!(parse_pages_to_eighths("1 7/8"), Some(15));
        assert_eq!(parse_pages_to_eighths("2.5"), Some(20));
        assert_eq!(parse_pages_to_eighths("bad"), None);
        assert_eq!(parse_pages_to_eighths("1/0"), None);
    }

    /// Exercises the established hour/minute input convention.
    #[test]
    fn parses_time_inputs() {
        assert_eq!(parse_time_to_minutes("4"), Some(240));
        assert_eq!(parse_time_to_minutes("15"), Some(15));
        assert_eq!(parse_time_to_minutes("2:30"), Some(150));
        assert_eq!(parse_time_to_minutes("2:60"), None);
        assert_eq!(parse_time_to_minutes("bad"), None);
    }
}
