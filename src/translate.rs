use anyhow::Result;

/// Translates `text` from English to Kannada using a built-in dictionary only.
/// Google Cloud Translate is intentionally NOT used (no API key, no network
/// call) -- day names, months, and common weather condition words are covered
/// by the dictionary below; anything else (e.g. full sentences) is expected to
/// already be authored in Kannada upstream (see fetch::weather_summary, which
/// builds Kannada summary text directly) or is left as-is.
pub fn to_kannada(text: &str) -> Result<String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    // If the whole string is just a number/unit combo (e.g. "34°", "10%"), skip translation.
    if text.chars().all(|c| c.is_ascii_digit() || "°%.- ".contains(c)) {
        return Ok(text.to_string());
    }

    if let Some(translation) = dictionary_translation(text) {
        return Ok(translation);
    }

    Ok(text.to_string())
}

fn dictionary_translation(text: &str) -> Option<String> {
    let normalized = text.trim();
    let mapping = [
        ("Mysore - 10 Day Forecast", "ಮೈಸೂರು - 10 ದಿನಗಳ ಮುನ್ಸೂಚನೆ"),
        ("Mysore - 10 Days Weather Forecast", "ಮೈಸೂರು - 10 ದಿನಗಳ ಹವಾಮಾನ ಮುನ್ಸೂಚನೆ"),
        ("10 day forecast", "10 ದಿನಗಳ ಮುನ್ಸೂಚನೆ"),
        ("Sunny", "ಬಿಸಿಲು"),
        ("Mostly Sunny", "ಜಾಸ್ತಿಯಷ್ಟು ಸೂರ್ಯ"),
        ("Partly Cloudy", "ಭಾಗಶಃ ಮೋಡ"),
        ("Cloudy", "ಮೋಡ"),
        ("Foggy", "ಮಂಜು"),
        ("Rain", "ಮಳೆ"),
        ("Snow", "ಹಿಮ"),
        ("Thunderstorm", "ಗುಡುಗು ಸಹಿತ ಮಳೆ"),
        ("Clear", "ಸ್ಪಷ್ಟ"),
        ("Mysore", "ಮೈಸೂರು"),
        ("Day", "ದಿನ"),
        ("Night", "ರಾತ್ರಿ"),
        ("Now", "ಈಗ"),
        ("Chance of rain", "ಮಳೆ ಸಾಧ್ಯತೆ"),
    ];

    if let Some(&(_, translated)) = mapping.iter().find(|&&(eng, _)| eng == normalized) {
        return Some(translated.to_string());
    }

    if let Some(day) = translate_day_name(normalized) {
        return Some(day.to_string());
    }

    if let Some(month) = translate_month_name(normalized) {
        return Some(month.to_string());
    }

    None
}

fn translate_day_name(text: &str) -> Option<&'static str> {
    match text.to_lowercase().as_str() {
        "mon" | "monday" => Some("ಸೋಮವಾರ"),
        "tue" | "tuesday" => Some("ಮಂಗಳವಾರ"),
        "wed" | "wednesday" => Some("ಬುಧವಾರ"),
        "thu" | "thursday" => Some("ಗುರುವಾರ"),
        "fri" | "friday" => Some("ಶುಕ್ರವಾರ"),
        "sat" | "saturday" => Some("ಶನಿವಾರ"),
        "sun" | "sunday" => Some("ಭಾನುವಾರ"),
        _ => None,
    }
}

fn translate_month_name(text: &str) -> Option<&'static str> {
    match text.to_lowercase().as_str() {
        "jan" | "january" => Some("ಜನ"),
        "feb" | "february" => Some("ಫೆಬ್ರ"),
        "mar" | "march" => Some("ಮಾರ್ಚ್"),
        "apr" | "april" => Some("ಏಪ್ರಿ"),
        "may" => Some("ಮೇ"),
        "jun" | "june" => Some("ಜೂನ್"),
        "jul" | "july" => Some("ಜುಲೈ"),
        "aug" | "august" => Some("ಆಗ"),
        "sep" | "september" => Some("ಸೆಪ್ಟೆ"),
        "oct" | "october" => Some("ಅಕ್ಟೋ"),
        "nov" | "november" => Some("ನವೆಂ"),
        "dec" | "december" => Some("ಡಿಸೆಂ"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::to_kannada;

    #[test]
    fn falls_back_to_dictionary_translation() {
        let translated = to_kannada("Rain");
        assert!(translated.is_ok());
        assert_eq!(translated.unwrap(), "ಮಳೆ");
    }

    #[test]
    fn leaves_unmapped_text_as_is() {
        let translated = to_kannada("Cloudy with light rain");
        assert!(translated.is_ok());
        assert_eq!(translated.unwrap(), "Cloudy with light rain");
    }
}
