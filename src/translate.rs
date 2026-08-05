use anyhow::{Context, Result};
use serde_json::json;

/// Translates `text` from English to Kannada (kn) using Google Cloud Translation API
/// when a key is configured. If no key is available, a built-in Kannada dictionary
/// is used for common weather terms and day labels.
pub fn to_kannada(api_key: &str, text: &str) -> Result<String> {
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

    if api_key.trim().is_empty() {
        return Ok(text.to_string());
    }

    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post("https://translation.googleapis.com/language/translate/v2")
        .query(&[("key", api_key)])
        .json(&json!({
            "q": text,
            "source": "en",
            "target": "kn",
            "format": "text"
        }))
        .send()
        .context("calling Google Translate API")?
        .json()
        .context("parsing Translate API response")?;

    resp["data"]["translations"][0]["translatedText"]
        .as_str()
        .map(|s| s.to_string())
        .with_context(|| format!("unexpected Translate API response: {resp}"))
}

fn dictionary_translation(text: &str) -> Option<String> {
    let normalized = text.trim();
    let mapping = [
        ("Mysore - 10 Day Forecast", "ಮೈಸೂರು - 10 ದಿನಗಳ ಮುನ್ಸೂಚನೆ"),
        ("10 day forecast", "10 ದಿನಗಳ ಮುನ್ಸೂಚನೆ"),
        ("10 ದಿನಗಳ ಹವಾಮಾನ ಮುನ್ಸೂಚನೆ", "10 ದಿನಗಳ ಹವಾಮಾನ ಮುನ್ಸೂಚನೆ"),
        ("Sunny", "ಸೂರ್ಯಪ್ರಕಾಶ"),
        ("Mostly Sunny", "ಮುಖ್ಯತ: ಸೂರ್ಯಪ್ರಕಾಶ"),
        ("Partly Cloudy", "ಭಾಗಶಃ ಮೋಡಗಾಲ"),
        ("Cloudy", "ಮೋಡಗಾಲ"),
        ("Foggy", "ಕೂದಲು"),
        ("Rain", "ಮಳೆಯ"),
        ("Snow", "ಹಿಮ"),
        ("Thunderstorm", "ಮಿಂಚುಗಾಳಿ"),
        ("Clear", "ಸ್ಪಷ್ಟ"),
        ("Mysore", "ಮೈಸೂರು"),
        ("Day", "ದಿನ"),
        ("Night", "ರಾತ್ರಿ"),
        ("Now", "ಈಗ"),
        ("Chance of rain", "ಮಳೆ ಸಾಧ್ಯತೆ"),
        ("Rain", "ಮಳೆ"),
        ("Sunny", "ಬಿಸಿಲು"),
        ("Mostly Sunny", "ಜಾಸ್ತಿಯಷ್ಟು ಸೂರ್ಯ"),
        ("Partly Cloudy", "ಭಾಗಶಃ ಮೋಡ"),
        ("Cloudy", "ಮೋಡ"),
        ("Thunderstorm", "ಕಿರಣಗಾಳಿ"),
        ("Foggy", "ಕೂದಲು"),
        ("Rain", "ಮಳೆಯ"),
        ("Snow", "ಹಿಮ"),
        ("Clear", "ಸ್ಪಷ್ಟ"),
        ("Mon", "ಗುರುವಾರ"),
    ];

    // Days and months are handled separately below.
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
        "jan" | "january" => Some("ಏ.ಮಾ"),
        "feb" | "february" => Some("ಫೆ.ಮಾ"),
        "mar" | "march" => Some("ಮಾ"),
        "apr" | "april" => Some("ಎಪ್ರಿ"),
        "may" => Some("ಮೇ"),
        "jun" | "june" => Some("ಜೂನ್"),
        "jul" | "july" => Some("ಜುಲೈ"),
        "aug" | "august" => Some("ಆಗ"),
        "sep" | "september" => Some("ಸೆಪ್ಟೆ"),
        "oct" | "october" => Some("ಅಕ್ಟೋ"),
        "nov" | "november" => Some("ನವೆ"),
        "dec" | "december" => Some("ಡಿಸೆ"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::to_kannada;

    #[test]
    fn falls_back_to_dictionary_translation_when_no_api_key_is_configured() {
        let translated = to_kannada("", "Rain");

        assert!(translated.is_ok());
        assert_eq!(translated.unwrap(), "ಮಳೆಯ");
    }
}

#[cfg(test)]
mod tests {
    use super::to_kannada;

    #[test]
    fn falls_back_to_the_original_text_when_no_api_key_is_configured() {
        let translated = to_kannada("", "Cloudy with light rain");

        assert!(translated.is_ok());
        assert_eq!(translated.unwrap(), "Cloudy with light rain");
    }
}
