use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Free Kannada female voice via Microsoft Edge's neural TTS (no API key
/// needed). Requires the `edge-tts` CLI: `pip install edge-tts`.
/// https://pypi.org/project/edge-tts/
const KANNADA_FEMALE_VOICE: &str = "kn-IN-SapnaNeural";

/// Synthesizes `kannada_text` to an MP3 using edge-tts. Writes the script to a
/// temp .txt file first (edge-tts handles long text more reliably via --file
/// than via a single --text argument on some shells/platforms).
pub fn synthesize_kannada_mp3(_api_key: &str, kannada_text: &str, out_path: &Path) -> Result<()> {
    if which("edge-tts").is_none() {
        anyhow::bail!(
            "edge-tts is not installed or not on PATH. Install it with: pip install edge-tts"
        );
    }

    let script_path = out_path.with_extension("script.txt");
    fs::write(&script_path, kannada_text)
        .with_context(|| format!("writing narration script to {}", script_path.display()))?;

    let status = Command::new("edge-tts")
        .arg("--voice")
        .arg(KANNADA_FEMALE_VOICE)
        .arg("--file")
        .arg(&script_path)
        .arg("--write-media")
        .arg(out_path)
        .status()
        .context("spawning edge-tts (is it installed? `pip install edge-tts`)")?;

    if !status.success() {
        anyhow::bail!("edge-tts failed to synthesize audio; check its output above");
    }

    validate_audio(out_path)
}

/// Corruption check: the file must exist and be a plausible, non-trivial size.
/// edge-tts occasionally writes a 0-byte file on network/voice errors without
/// a non-zero exit code, so size is checked explicitly here.
fn validate_audio(path: &Path) -> Result<()> {
    let meta = fs::metadata(path)
        .with_context(|| format!("checking generated audio at {}", path.display()))?;
    if meta.len() < 2048 {
        anyhow::bail!(
            "generated audio at {} is suspiciously small ({} bytes) -- likely empty/corrupted. \
             Check that edge-tts has network access and that the voice name '{}' is valid.",
            path.display(),
            meta.len(),
            KANNADA_FEMALE_VOICE
        );
    }
    Ok(())
}

fn which(bin: &str) -> Option<()> {
    Command::new(bin)
        .arg("--help")
        .output()
        .ok()
        .filter(|o| o.status.success() || !o.stdout.is_empty())
        .map(|_| ())
}

/// Builds a natural-sounding Kannada script from the per-day summaries so the
/// audio doesn't just read "row1, row2, ..." mechanically. Numbers (temps, %)
/// are already embedded as-is inside each translated line.
pub fn build_script(lines: &[String]) -> String {
    lines.join(". ")
}
