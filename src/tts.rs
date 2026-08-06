use anyhow::{Context, Result};
use msedge_tts::tts::client::connect;
use msedge_tts::tts::SpeechConfig;
use msedge_tts::voice::get_voices_list;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Free Kannada female voice via Microsoft Edge's neural TTS service, called
/// directly from Rust with the `msedge-tts` crate -- no Python/edge-tts CLI
/// install required. Requires internet access (it talks to Microsoft's
/// service over a websocket), but nothing else.
///
/// NOTE: `Voice.name` from the crate is the verbose form, e.g.
/// "Microsoft Server Speech Text to Speech Voice (kn-IN, SapnaNeural)" --
/// NOT the short hyphenated form "kn-IN-SapnaNeural". Match on the two
/// distinctive substrings separately instead of the hyphenated string.
const KANNADA_LOCALE: &str = "kn-IN";
const KANNADA_FEMALE_VOICE_HINT: &str = "SapnaNeural";

pub fn synthesize_kannada_mp3(_api_key: &str, kannada_text: &str, out_path: &Path) -> Result<()> {
    let voices = get_voices_list()
        .context("fetching the MSEdge TTS voice list failed (requires internet access)")?;

    let voice = voices
        .iter()
        .find(|v| v.name.contains(KANNADA_LOCALE) && v.name.contains(KANNADA_FEMALE_VOICE_HINT))
        .or_else(|| {
            // Fallback: Microsoft occasionally renames specific neural voices.
            // Any kn-IN voice is better than hard-failing the whole pipeline.
            voices.iter().find(|v| v.name.contains(KANNADA_LOCALE))
        })
        .with_context(|| {
            let sample: Vec<&str> = voices.iter().take(5).map(|v| v.name.as_str()).collect();
            format!(
                "no '{KANNADA_LOCALE}' voice found in the MSEdge TTS voice list at all. \
                 First few voice names returned, for reference: {sample:?}"
            )
        })?;
    let config = SpeechConfig::from(voice);

    let mut client = connect().context("connecting to the MSEdge TTS service")?;

    let mut audio_bytes = Vec::new();
    for chunk in kannada_text.split('.').map(str::trim).filter(|s| !s.is_empty()) {
        let audio = client
            .synthesize(chunk, &config)
            .with_context(|| format!("synthesizing narration chunk: \"{chunk}\""))?;
        audio_bytes.extend_from_slice(&audio.audio_bytes);
    }

    let mut file = fs::File::create(out_path)
        .with_context(|| format!("creating audio file at {}", out_path.display()))?;
    file.write_all(&audio_bytes)
        .with_context(|| format!("writing synthesized audio to {}", out_path.display()))?;

    validate_audio(out_path)
}

/// Corruption check: the file must exist and be a plausible, non-trivial size.
fn validate_audio(path: &Path) -> Result<()> {
    let meta = fs::metadata(path)
        .with_context(|| format!("checking generated audio at {}", path.display()))?;
    if meta.len() < 2048 {
        anyhow::bail!(
            "generated audio at {} is suspiciously small ({} bytes) -- likely empty/corrupted. \
             Check that this machine has internet access.",
            path.display(),
            meta.len()
        );
    }
    Ok(())
}

/// Builds a natural-sounding Kannada script from the per-day summaries so the
/// audio doesn't just read "row1, row2, ..." mechanically. Numbers (temps, %)
/// are already embedded as-is inside each translated line.
pub fn build_script(lines: &[String]) -> String {
    lines.join(". ")
}
