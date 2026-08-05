mod fetch;
mod image_gen;
mod model;
mod translate;
mod tts;
mod video;

use anyhow::Result;
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    // --- config -------------------------------------------------------
    let out_dir = PathBuf::from("output");
    std::fs::create_dir_all(&out_dir)?;

    let html_path = out_dir.join("mysore_forecast.html");
    let png_path = out_dir.join("mysore_forecast_kn.png");
    let audio_path = out_dir.join("mysore_forecast_kn.wav");
    let mp4_path = out_dir.join("mysore_forecast_kn.mp4");

    // Path to a Kannada-capable font. Use the bundled Noto Sans Kannada font in assets.
    let font_path = PathBuf::from("assets/NotoSansKannada-VariableFont_wdth,wght.ttf");

    // API keys are optional. If they're not configured, the app falls back to
    // free local behavior instead of failing at startup.
    let translate_key = env::var("GOOGLE_TRANSLATE_API_KEY").unwrap_or_default();
    let tts_key = env::var("GOOGLE_TTS_API_KEY").unwrap_or_default();

    if translate_key.trim().is_empty() && tts_key.trim().is_empty() {
        println!("No Google API keys configured; using free fallback mode.");
    }

    // --- 1. fetch + parse ----------------------------------------------
    println!("Fetching AccuWeather 10-day forecast for Mysore...");
    let mut days = fetch::fetch_and_parse(&html_path)?;
    days.truncate(10);
    println!("Parsed {} day(s). Raw HTML saved to {}", days.len(), html_path.display());

    // --- 2. translate ----------------------------------------------------
    println!("Translating to Kannada...");
    for day in days.iter_mut() {
        day.day_name = translate::to_kannada(&translate_key, &day.day_name)?;
        day.condition = translate::to_kannada(&translate_key, &day.condition)?;
        if !day.summary.is_empty() {
            day.summary = translate::to_kannada(&translate_key, &day.summary)?;
        }
        // Numbers (high/low temps, precip %) are intentionally left as-is.
    }
    let title_kn = translate::to_kannada(&translate_key, "Mysore - 10 Days weather Forecast")?;

    // --- 3. render PNG -----------------------------------------------------
    println!("Rendering PNG...");
    image_gen::render_png(&days, &font_path, &png_path, &title_kn)?;
    println!("Saved image to {}", png_path.display());

    // --- 4. TTS ------------------------------------------------------------
    println!("Synthesizing Kannada narration...");
    let lines: Vec<String> = days
        .iter()
        .map(|d| {
            format!(
                "{} {}, ಗರಿಷ್ಠ ತಾಪಮಾನ {} ಮತ್ತು ಕನಿಷ್ಠ ತಾಪಮಾನ {}. ಹವಾಮಾನ {}. {}",
                d.day_name,
                d.date,
                d.high_temp,
                d.low_temp,
                d.condition,
                d.summary
            )
        })
        .collect();
    let mut script_lines = Vec::with_capacity(days.len() + 1);
    script_lines.push("ನಮಸ್ಕಾರ ವೀಕ್ಷಕರೆ, ಮೈಸೂರಿನ ಮುಂದಿನ 10 ದಿನಗಳ ಹವಾಮಾನ ವರದಿ ಈಗಿದೇ".to_string());
    script_lines.extend(lines);
    let script = tts::build_script(&script_lines);
    tts::synthesize_kannada_mp3(&tts_key, &script, &audio_path)?;
    println!("Saved audio to {}", audio_path.display());

    // --- 5. merge into video -------------------------------------------------
    println!("Merging into MP4...");
    match video::merge_png_and_audio(&png_path, &audio_path, &mp4_path) {
        Ok(_) => println!("Saved video to {}", mp4_path.display()),
        Err(err) => println!("Skipping MP4 creation: {}", err),
    }

    Ok(())
}
