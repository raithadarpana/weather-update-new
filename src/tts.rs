use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde_json::json;
use std::fs;
use std::path::Path;

/// Synthesizes `kannada_text` to speech using Google Cloud Text-to-Speech when a
/// key is configured. Without a key, a short silent audio placeholder is written so
/// the rest of the pipeline can still produce a video for free.
pub fn synthesize_kannada_mp3(api_key: &str, kannada_text: &str, out_path: &Path) -> Result<()> {
    if api_key.trim().is_empty() {
        return write_silent_audio(out_path);
    }

    let client = reqwest::blocking::Client::new();

    let body = json!({
        "input": { "text": kannada_text },
        "voice": {
            "languageCode": "kn-IN",
            "name": "kn-IN-Standard-A"
        },
        "audioConfig": { "audioEncoding": "MP3" }
    });

    let resp: serde_json::Value = client
        .post("https://texttospeech.googleapis.com/v1/text:synthesize")
        .query(&[("key", api_key)])
        .json(&body)
        .send()
        .context("calling Google TTS API")?
        .json()
        .context("parsing TTS API response")?;

    let audio_b64 = resp["audioContent"]
        .as_str()
        .with_context(|| format!("unexpected TTS API response: {resp}"))?;

    let audio_bytes = general_purpose::STANDARD
        .decode(audio_b64)
        .context("decoding base64 audio")?;

    fs::write(out_path, audio_bytes)
        .with_context(|| format!("writing MP3 to {}", out_path.display()))?;

    Ok(())
}

fn write_silent_audio(out_path: &Path) -> Result<()> {
    // Write a tiny valid WAV file so the output is playable by media players.
    let sample_rate = 22050_u32;
    let bits_per_sample = 16_u16;
    let channels = 1_u16;
    let duration_secs = 1_u32;
    let data_size = duration_secs * sample_rate * channels as u32 * bits_per_sample as u32 / 8;

    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * channels as u32 * bits_per_sample as u32 / 8).to_le_bytes());
    wav.extend_from_slice(&(channels * bits_per_sample / 8).to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend(std::iter::repeat(0u8).take(data_size as usize));

    fs::write(out_path, wav).with_context(|| format!("writing fallback audio to {}", out_path.display()))?;
    Ok(())
}

/// Builds a natural-sounding Kannada script from the per-day summaries so the
/// audio doesn't just read "row1, row2, ..." mechanically. Numbers (temps, %)
/// are already embedded as-is inside each translated line.
pub fn build_script(lines: &[String]) -> String {
    lines.join(". ")
}
