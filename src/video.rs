use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Merges a still PNG with an MP3 narration into an MP4 when ffmpeg is available.
/// If ffmpeg is not installed, a placeholder MP4 file is written so the workflow
/// can still complete; the PNG and WAV outputs remain valid and viewable.
pub fn merge_png_and_audio(png_path: &Path, audio_path: &Path, mp4_out: &Path) -> Result<()> {
    let ffmpeg_version = Command::new("ffmpeg").arg("-version").output();

    let ffmpeg_version = ffmpeg_version
        .context("ffmpeg not found; install ffmpeg and ensure it is on PATH")?;

    if !ffmpeg_version.status.success() {
        anyhow::bail!("ffmpeg is unavailable; please install ffmpeg and retry")
    }

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-loop",
            "1",
            "-i",
        ])
        .arg(png_path)
        .arg("-i")
        .arg(audio_path)
        .args([
            "-c:v",
            "libx264",
            "-tune",
            "stillimage",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-pix_fmt",
            "yuv420p",
            "-shortest",
        ])
        .arg(mp4_out)
        .status()
        .context("spawning ffmpeg (is it installed and on PATH?)")?;

    if !status.success() {
        anyhow::bail!("ffmpeg failed to create MP4; check ffmpeg output and input files")
    }

    Ok(())
}
