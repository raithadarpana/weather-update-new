use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Merges a still PNG with the narration MP3 into an MP4. Compared to the old
/// version, this now (a) fails loudly instead of silently if ffmpeg errors,
/// and (b) validates the resulting file with ffprobe so a corrupt/zero-length
/// output is caught immediately instead of being handed to you unusable.
pub fn merge_png_and_audio(png_path: &Path, audio_path: &Path, mp4_out: &Path) -> Result<()> {
    let ffmpeg_ok = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ffmpeg_ok {
        anyhow::bail!("ffmpeg is unavailable; please install ffmpeg and retry");
    }

    let status = Command::new("ffmpeg")
        .args(["-y", "-loop", "1", "-i"])
        .arg(png_path)
        .arg("-i")
        .arg(audio_path)
        .args([
            "-c:v", "libx264",
            "-tune", "stillimage",
            "-c:a", "aac",
            "-b:a", "192k",
            "-pix_fmt", "yuv420p",
            "-shortest",
            "-movflags", "+faststart",
        ])
        .arg(mp4_out)
        .status()
        .context("spawning ffmpeg (is it installed and on PATH?)")?;

    if !status.success() {
        anyhow::bail!("ffmpeg failed to create MP4 at {}; check ffmpeg output above", mp4_out.display());
    }

    validate_mp4(mp4_out)
}

/// Corruption check: parse the file with ffprobe and require both a video
/// stream and an audio stream with non-zero duration. This is what the old
/// code was missing -- ffmpeg can exit 0 while still writing an unplayable
/// file if inputs were bad (e.g. the previous silent-placeholder WAV bug).
fn validate_mp4(path: &Path) -> Result<()> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "stream=codec_type,duration",
            "-of", "csv=p=0",
        ])
        .arg(path)
        .output()
        .context("spawning ffprobe to validate the generated MP4 (is ffmpeg/ffprobe installed?)")?;

    if !output.status.success() {
        anyhow::bail!(
            "ffprobe could not read {} -- the MP4 is likely corrupted:\n{}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let has_video = text.lines().any(|l| l.starts_with("video"));
    let has_audio = text.lines().any(|l| l.starts_with("audio"));
    if !has_video || !has_audio {
        anyhow::bail!(
            "generated MP4 at {} is missing a {} stream -- treating it as corrupted",
            path.display(),
            if !has_video { "video" } else { "audio" }
        );
    }

    Ok(())
}
