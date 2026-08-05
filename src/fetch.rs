use crate::model::DayForecast;
use anyhow::{Context, Result};
use scraper::{Html, Selector};
use std::fs;
use std::path::Path;

const WEATHER_API_URL: &str = "https://api.open-meteo.com/v1/forecast";
const DEFAULT_CITY: &str = "Mysore";

/// Fetches a weather forecast from Open-Meteo, saves the raw JSON to `html_out_path`,
/// and converts it into up to 10 DayForecast entries.
/// If the request fails or no network access is available, a small built-in sample
/// forecast is used so the app still produces output without any paid service.
pub fn fetch_and_parse(html_out_path: &Path) -> Result<Vec<DayForecast>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
        )
        .build()?;

    let url = format!(
        "{WEATHER_API_URL}?latitude=12.30&longitude=76.65&daily=weathercode,temperature_2m_max,temperature_2m_min,precipitation_probability_max&timezone=Asia%2FKolkata&forecast_days=10"
    );

    match client.get(&url).send() {
        Ok(response) => {
            if response.status().is_success() {
                let body = response.text().context("reading response body")?;
                fs::write(html_out_path, &body).context("saving raw weather response")?;
                return parse_open_meteo(&body);
            }

            let body = response.text().unwrap_or_default();
            if body.is_empty() {
                println!("Weather API request failed; using built-in fallback forecast.");
            } else {
                println!("Weather API request failed, using fallback: {body}");
            }
        }
        Err(err) => {
            println!("Weather API request error: {err}; using built-in fallback forecast.");
        }
    }

    build_fallback_forecast(html_out_path)
}

fn parse_open_meteo(body: &str) -> Result<Vec<DayForecast>> {
    let parsed: serde_json::Value = serde_json::from_str(body).context("parsing weather API response")?;
    let daily = parsed["daily"].as_object().context("weather API response missing daily data")?;

    let times = daily["time"].as_array().context("weather API response missing time values")?;
    let codes = daily["weathercode"].as_array().context("weather API response missing weather codes")?;
    let highs = daily["temperature_2m_max"].as_array().context("weather API response missing max temperature values")?;
    let lows = daily["temperature_2m_min"].as_array().context("weather API response missing min temperature values")?;
    let precip = daily["precipitation_probability_max"].as_array().context("weather API response missing precipitation values")?;

    let mut results = Vec::new();
    for i in 0..times.len().min(10) {
        let date = times[i].as_str().unwrap_or_default().to_string();
        let day_name = format_day_name(&date);
        let condition = weather_code_to_condition(codes[i].as_u64().unwrap_or_default());
        let high_temp = format_temperature(highs[i].as_f64().unwrap_or_default());
        let low_temp = format_temperature(lows[i].as_f64().unwrap_or_default());
        let precip_chance = format_precip(precip[i].as_u64().unwrap_or_default());
        let summary = weather_summary(codes[i].as_u64().unwrap_or_default(), precip[i].as_u64().unwrap_or_default());

        results.push(DayForecast {
            day_name,
            date: format_day_label(&date),
            condition,
            high_temp,
            low_temp,
            precip_chance,
            summary,
        });
    }

    if results.is_empty() {
        anyhow::bail!("Weather API did not return any forecast rows");
    }

    Ok(results)
}

fn build_fallback_forecast(html_out_path: &Path) -> Result<Vec<DayForecast>> {
    let sample = serde_json::json!({
        "city": DEFAULT_CITY,
        "daily": {
            "time": ["2026-08-04", "2026-08-05", "2026-08-06", "2026-08-07", "2026-08-08", "2026-08-09", "2026-08-10", "2026-08-11", "2026-08-12", "2026-08-13"],
            "weathercode": [0, 1, 2, 3, 61, 80, 0, 1, 2, 3],
            "temperature_2m_max": [31.0, 32.0, 30.0, 29.0, 28.0, 27.0, 31.0, 32.0, 31.0, 30.0],
            "temperature_2m_min": [21.0, 22.0, 21.0, 20.0, 19.0, 18.0, 21.0, 22.0, 21.0, 20.0],
            "precipitation_probability_max": [5, 10, 20, 35, 60, 70, 10, 5, 15, 25]
        }
    });

    fs::write(html_out_path, serde_json::to_vec_pretty(&sample).context("serializing fallback weather data")?)
        .context("saving fallback weather data")?;

    parse_open_meteo(&serde_json::to_string(&sample).context("serializing fallback weather JSON")?)
}

fn format_temperature(value: f64) -> String {
    format!("{value:.0}°")
}

fn format_precip(value: u64) -> String {
    format!("{value}%")
}

fn format_day_label(date: &str) -> String {
    date.split('-').collect::<Vec<_>>().get(1..3).map(|parts| format!("{} {}", parts[0], parts[1])).unwrap_or_else(|| date.to_string())
}

fn format_day_name(date: &str) -> String {
    let year_month_day: Vec<&str> = date.split('-').collect();
    if year_month_day.len() != 3 {
        return "Day".to_string();
    }

    let day = year_month_day[2].parse::<u32>().unwrap_or_default();
    let month = year_month_day[1].parse::<u32>().unwrap_or_default();
    let year = year_month_day[0].parse::<u32>().unwrap_or_default();

    if let Some(datetime) = chrono::NaiveDate::from_ymd_opt(year as i32, month, day) {
        return datetime.format("%a").to_string();
    }

    "Day".to_string()
}

fn weather_code_to_condition(code: u64) -> String {
    match code {
        0 => "Sunny".to_string(),
        1 => "Mostly Sunny".to_string(),
        2 => "Partly Cloudy".to_string(),
        3 => "Cloudy".to_string(),
        45 | 48 => "Foggy".to_string(),
        51 | 53 | 55 | 61 | 63 | 65 | 80 | 81 | 82 => "Rain".to_string(),
        71 | 73 | 75 | 77 => "Snow".to_string(),
        95 | 96 | 99 => "Thunderstorm".to_string(),
        _ => "Clear".to_string(),
    }
}

fn weather_summary(code: u64, precip: u64) -> String {
    let state = match code {
        0 => "ಬಿಸಿಲಿನ ವಾತಾವರಣ",
        1 => "ಬಹುಶಃ ಬಿಸಿಲು",
        2 => "ಭಾಗಶಃ ಮೋಡಗಾಲ",
        3 => "ಮೋಡಭರಿತ ವಾತಾವರಣ",
        45 | 48 => "ಮಂಜಿನ ವಾತಾವರಣ",
        51 | 53 | 55 | 61 | 63 | 65 | 80 | 81 | 82 => "ಮಳೆ ಅಥವಾ ಮಿಂಚುಳ್ಳ ವಾತಾವರಣ",
        71 | 73 | 75 | 77 => "ಹಿಮ ವಾತಾವರಣ",
        95 | 96 | 99 => "ಗುಡುಗು ಸಹಿತ ಮಳೆಯ ಸಾಧ್ಯತೆ",
        _ => "ಸ್ಪಷ್ಟ ವಾತಾವರಣ",
    };

    let precip_phrase = match precip {
        0..=15 => "ಕಡಿಮೆ ಮಳೆ ಸಾಧ್ಯತೆ",
        16..=50 => "ಮಧ್ಯಮ ಮಳೆ ಸಾಧ್ಯತೆ",
        _ => "ಮಳೆ ಸಾಧ್ಯತೆ ಹೆಚ್ಚು",
    };

    format!("{}; {}.", state, precip_phrase)
}

fn parse_daily_list(html: &str) -> Result<Vec<DayForecast>> {
    let doc = Html::parse_document(html);

    // AccuWeather's 10-day page renders each day as an <a class="daily-list-item"> row
    // inside a container with id/class containing "daily-list". Selectors kept loose
    // on purpose; tighten/adjust after inspecting the saved HTML.
    let item_sel = Selector::parse("a.daily-list-item").unwrap();
    let day_sel = Selector::parse(".date p, .day").unwrap();
    let date_sel = Selector::parse(".date .module-header, .date span").unwrap();
    let cond_sel = Selector::parse(".phrase, .cond").unwrap();
    let high_sel = Selector::parse(".temp-hi, .high").unwrap();
    let low_sel = Selector::parse(".temp-lo, .low").unwrap();
    let precip_sel = Selector::parse(".precip").unwrap();

    let mut results = Vec::new();

    for item in doc.select(&item_sel).take(10) {
        let day_name = first_text(&item, &day_sel).unwrap_or_default();
        let date = first_text(&item, &date_sel).unwrap_or_default();
        let condition = first_text(&item, &cond_sel).unwrap_or_default();
        let high_temp = first_text(&item, &high_sel).unwrap_or_default();
        let low_temp = first_text(&item, &low_sel).unwrap_or_default();
        let precip_chance = first_text(&item, &precip_sel).unwrap_or_default();

        results.push(DayForecast {
            day_name,
            date,
            condition,
            high_temp,
            low_temp,
            precip_chance,
            summary: String::new(), // fill in from a per-day detail fetch if desired
        });
    }

    if results.is_empty() {
        anyhow::bail!(
            "No daily-list items matched. AccuWeather markup likely changed or the \
             page served a bot-check. Inspect the saved HTML and update selectors in \
             src/fetch.rs."
        );
    }

    Ok(results)
}

fn first_text(item: &scraper::ElementRef, sel: &Selector) -> Option<String> {
    item.select(sel)
        .next()
        .map(|e| e.text().collect::<Vec<_>>().join(" ").trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::weather_code_to_condition;

    #[test]
    fn maps_clear_weather_code_to_sunny_text() {
        assert_eq!(weather_code_to_condition(0), "Sunny");
    }
}
