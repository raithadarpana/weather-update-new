mod fetch;
mod html_render;
mod image_gen;
mod model;
mod translate;
mod tts;
mod video;

use anyhow::Result;
use html_render::PosterLayout;
use std::path::PathBuf;

fn main() -> Result<()> {
    // --- config -------------------------------------------------------
    let out_dir = PathBuf::from("output");
    std::fs::create_dir_all(&out_dir)?;

    let data_html_path = out_dir.join("mysore_forecast.html"); // requirement #1: editable extracted-data table
    let ig_html_path = out_dir.join("poster_ig.html");
    let yt_html_path = out_dir.join("poster_yt.html");
    let ig_png_path = out_dir.join("mysore_forecast_ig.png"); // 9:16
    let yt_png_path = out_dir.join("mysore_forecast_yt.png"); // 16:9
    let audio_path = out_dir.join("mysore_forecast_kn.mp3");
    let ig_mp4_path = out_dir.join("mysore_forecast_ig.mp4");
    let yt_mp4_path = out_dir.join("mysore_forecast_yt.mp4");

    let template_path = PathBuf::from("assets/templates/poster.html");
    let font_path = PathBuf::from("assets/NotoSansKannada-VariableFont_wdth,wght.ttf");
    let ig_bg_path = PathBuf::from("assets/ig-bg.png");
    let yt_bg_path = PathBuf::from("assets/yt-bg.png");

    for (label, path) in [
        ("Kannada font", &font_path),
        ("Instagram background (assets/ig-bg.png)", &ig_bg_path),
        ("YouTube background (assets/yt-bg.png)", &yt_bg_path),
    ] {
        if !path.exists() {
            anyhow::bail!("{label} not found at {}. Add it before running.", path.display());
        }
    }

    // --- 1. fetch + parse ----------------------------------------------
    println!("Fetching 5-day forecast...");
    let mut days = fetch::fetch_and_parse(&out_dir.join("raw_weather_data.json"))?;
    days.truncate(5);
    println!("Parsed {} day(s).", days.len());

    // --- 2. translate ----------------------------------------------------
    println!("Translating to Kannada...");
    for day in days.iter_mut() {
        day.day_name = translate::to_kannada(&day.day_name)?;
        day.condition = translate::to_kannada(&day.condition)?;
        if !day.summary.is_empty() {
            day.summary = translate::to_kannada(&day.summary)?;
        }
    }
    let title_kn = translate::to_kannada("Mysore - 5 Days Weather Forecast")?;
    let subtitle_kn = translate::to_kannada("5 ದಿನಗಳ ಹವಾಮಾನ ಮುನ್ಸೂಚನೆ")?;

    // --- 3. store extracted data as HTML (requirement #1) ------------------
    println!("Writing extracted-data HTML...");
    html_render::write_data_html(&days, &title_kn, &data_html_path)?;
    println!("Saved editable data table to {}", data_html_path.display());

    // --- 4. render 2 posters (IG 9:16, YT 16:9) over the backgrounds -------
    println!("Rendering poster HTML (no icons, backgrounds composited via CSS)...");
    let ig_layout = PosterLayout::instagram_9x16();
    let yt_layout = PosterLayout::youtube_16x9();

    let source_label = "Source: open-meteo.com";
    html_render::render_poster_html(
        &days, &title_kn, &subtitle_kn, source_label, &template_path, &font_path, &ig_bg_path, &ig_layout, &ig_html_path,
    )?;
    html_render::render_poster_html(
        &days, &title_kn, &subtitle_kn, source_label, &template_path, &font_path, &yt_bg_path, &yt_layout, &yt_html_path,
    )?;

    println!("Screenshotting Instagram (9:16) image...");
    image_gen::screenshot_html(&ig_html_path, &ig_layout, &ig_png_path)?;
    println!("Saved {}", ig_png_path.display());

    println!("Screenshotting YouTube (16:9) image...");
    image_gen::screenshot_html(&yt_html_path, &yt_layout, &yt_png_path)?;
    println!("Saved {}", yt_png_path.display());

    // --- 5. TTS ------------------------------------------------------------
    // Narration is read from the same extracted data as the HTML table
    // (day, date, condition, high, low, summary) for every day, per requirement.
    // This step (and video merging below) is intentionally non-fatal: the two
    // PNGs above are the main deliverable and should always be produced even
    // if edge-tts/ffmpeg aren't set up yet on this machine.
    println!("Synthesizing Kannada narration (edge-tts, free, female voice)...");
    let lines: Vec<String> = days
        .iter()
        .map(|d| {
            format!(
                "{} {}, ಹವಾಮಾನ {}. ಗರಿಷ್ಠ ತಾಪಮಾನ {} ಮತ್ತು ಕನಿಷ್ಠ ತಾಪಮಾನ {}. {}",
                d.day_name, d.date, d.condition, d.high_temp, d.low_temp, d.summary
            )
        })
        .collect();
    let mut script_lines = Vec::with_capacity(days.len() + 1);
    script_lines.push(
        "ನಮಸ್ಕಾರ ವೀಕ್ಷಕರೇ, ರೈತ ದರ್ಪಣ ಹವಾಮಾನ ವಾರದಿ ಮಾಹಿತಿ ಚಾನೆಲ್ಗೆ ಸ್ವಾಗತ. ಮುಂದಿನ 5 ದಿನದ ಹವಾಮನ ವರದಿ ಈಗಿದೆ"
            .to_string(),
    );
    script_lines.extend(lines);
    let script = tts::build_script(&script_lines);

    // Remove any stale audio/video from a previous run before attempting to
    // regenerate them, so a failed/skipped TTS step never leaves an old file
    // sitting in output/ that looks like a fresh (but wrong) result.
    for stale in [&audio_path, &ig_mp4_path, &yt_mp4_path] {
        let _ = std::fs::remove_file(stale);
    }

    match tts::synthesize_kannada_mp3("", &script, &audio_path) {
        Ok(()) => {
            println!("Saved audio to {}", audio_path.display());

            // --- 6. merge into videos -------------------------------------------------
            println!("Merging Instagram video...");
            match video::merge_png_and_audio(&ig_png_path, &audio_path, &ig_mp4_path) {
                Ok(()) => println!("Saved {}", ig_mp4_path.display()),
                Err(err) => println!("Skipping Instagram video: {err}"),
            }

            println!("Merging YouTube video...");
            match video::merge_png_and_audio(&yt_png_path, &audio_path, &yt_mp4_path) {
                Ok(()) => println!("Saved {}", yt_mp4_path.display()),
                Err(err) => println!("Skipping YouTube video: {err}"),
            }
        }
        Err(err) => {
            println!(
                "Skipping audio/video (both PNGs above were still generated successfully): {err}"
            );
        }
    }

    Ok(())
}
