use crate::html_render::{to_file_url, PosterLayout};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Renders `html_path` to a PNG at exactly `width`x`height` using a locally
/// installed Chromium/Chrome in headless mode. This replaces the old manual
/// fontdue+rustybuzz glyph rasterizer, which could not reliably render
/// Kannada conjuncts/vowel-signs (that's why text showed as boxes). A real
/// browser engine handles complex-script shaping correctly, and CSS
/// `background-image` handles compositing over ig-bg.png / yt-bg.png.
///
/// Requires Chrome, Chromium, or Edge to be installed and on PATH (or common
/// install locations). Install on Ubuntu: `sudo apt install chromium-browser`
/// (or `chromium`). On Windows/Mac, a normal Chrome install is enough.
pub fn screenshot_html(html_path: &Path, layout: &PosterLayout, out_path: &Path) -> Result<()> {
    let browser = find_browser_binary()
        .context("no Chrome/Chromium/Edge binary found on PATH or in common install locations")?;

    let html_abs = std::fs::canonicalize(html_path)
        .with_context(|| format!("resolving path to {}", html_path.display()))?;
    let html_url = to_file_url(&html_abs)?;

    // Chromium's headless --screenshot writer can fail to resolve a relative
    // output path on Windows ("cannot find the path specified"), so make sure
    // the target directory exists and pass Chrome an absolute path.
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory {}", parent.display()))?;
    }
    let out_abs = {
        let parent = out_path
            .parent()
            .map(std::fs::canonicalize)
            .transpose()
            .with_context(|| format!("resolving output directory for {}", out_path.display()))?
            .unwrap_or_else(|| PathBuf::from("."));
        let file_name = out_path
            .file_name()
            .with_context(|| format!("output path {} has no file name", out_path.display()))?;
        parent.join(file_name)
    };

    let window_size = format!("{},{}", layout.width, layout.height + 100); // small chrome-UI slack
    let screenshot_arg = format!("--screenshot={}", out_abs.display());

    // A dedicated, throwaway user-data-dir is essential: without it, if you
    // already have a normal (non-headless) Chrome window open, this command
    // gets silently forwarded to that existing process over IPC -- all the
    // --headless/--screenshot flags are ignored, and it just opens a new tab
    // showing your default search/new-tab page instead of rendering our HTML.
    let profile_dir = std::env::temp_dir().join(format!(
        "weather-report-chrome-profile-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&profile_dir)
        .with_context(|| format!("creating temp Chrome profile dir at {}", profile_dir.display()))?;

    let status = Command::new(&browser)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--hide-scrollbars",
            "--default-background-color=00000000",
            "--force-device-scale-factor=1",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-extensions",
            "--disable-sync",
            "--run-all-compositor-stages-before-draw",
            "--virtual-time-budget=10000",
            &format!("--user-data-dir={}", profile_dir.display()),
            &format!("--window-size={window_size}"),
            &screenshot_arg,
            &html_url,
        ])
        .status()
        .with_context(|| format!("spawning {} (is it installed?)", browser))?;

    let _ = std::fs::remove_dir_all(&profile_dir);

    if !status.success() {
        anyhow::bail!(
            "{} exited with an error while rendering {}",
            browser,
            html_path.display()
        );
    }

    validate_png(&out_abs, layout.width, layout.height)
}

/// Basic corruption check: file must exist, be non-trivially sized, and decode
/// as a PNG with roughly the requested dimensions (Chromium's --screenshot can
/// crop to the viewport, so we allow it to be >= requested minus the small
/// window-chrome slack rather than requiring an exact match).
fn validate_png(path: &Path, expected_w: u32, expected_h: u32) -> Result<()> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("checking generated PNG at {}", path.display()))?;
    if meta.len() < 1024 {
        anyhow::bail!(
            "generated PNG at {} is suspiciously small ({} bytes) -- likely corrupted or blank",
            path.display(),
            meta.len()
        );
    }

    let img = image::open(path)
        .with_context(|| format!("decoding generated PNG at {} to validate it", path.display()))?;
    let (w, h) = (img.width(), img.height());
    if w < expected_w.saturating_sub(4) || h < expected_h.saturating_sub(120) {
        anyhow::bail!(
            "generated PNG at {} is {}x{}, expected roughly {}x{} -- render likely failed",
            path.display(),
            w,
            h,
            expected_w,
            expected_h
        );
    }

    // Catches the "background image didn't load in time" failure mode: sample
    // pixels across the image and bail if it's suspiciously close to a single
    // flat color (a near-blank/white page instead of the background artwork).
    let rgb = img.to_rgb8();
    let (sw, sh) = (rgb.width(), rgb.height());
    if sw > 0 && sh > 0 {
        let mut min = [255u8; 3];
        let mut max = [0u8; 3];
        let step_x = (sw / 40).max(1);
        let step_y = (sh / 40).max(1);
        let mut x = 0;
        while x < sw {
            let mut y = 0;
            while y < sh {
                let p = rgb.get_pixel(x, y).0;
                for c in 0..3 {
                    min[c] = min[c].min(p[c]);
                    max[c] = max[c].max(p[c]);
                }
                y += step_y;
            }
            x += step_x;
        }
        let spread: u32 = (0..3).map(|c| (max[c] as u32) - (min[c] as u32)).sum();
        if spread < 12 {
            anyhow::bail!(
                "generated PNG at {} looks nearly flat-colored (background image likely \
                 failed to load before the screenshot was taken)",
                path.display()
            );
        }
    }

    Ok(())
}

fn find_browser_binary() -> Option<String> {
    let candidates = [
        "chromium",
        "chromium-browser",
        "google-chrome",
        "google-chrome-stable",
        "microsoft-edge",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ];

    for candidate in candidates {
        if Path::new(candidate).is_absolute() {
            if Path::new(candidate).exists() {
                return Some(candidate.to_string());
            }
            continue;
        }
        if Command::new(candidate).arg("--version").output().is_ok() {
            return Some(candidate.to_string());
        }
    }
    None
}
