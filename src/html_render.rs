use crate::model::DayForecast;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Requirement #1: write the extracted forecast as a clean, human-editable HTML
/// table (not raw JSON, not a rendered poster). This is the file you hand-edit
/// if you want to tweak wording/values before generating images, and it's also
/// what the narration script is read from.
pub fn write_data_html(days: &[DayForecast], title: &str, out_path: &Path) -> Result<()> {
    let mut rows = String::new();
    for day in days {
        rows.push_str(&format!(
            "    <tr>\n      <td>{}</td>\n      <td>{}</td>\n      <td>{}</td>\n      <td>{}</td>\n      <td>{}</td>\n      <td>{}</td>\n    </tr>\n",
            escape(&day.day_name),
            escape(&day.date),
            escape(&day.condition),
            escape(&day.high_temp),
            escape(&day.low_temp),
            escape(&day.summary),
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="kn">
<head>
<meta charset="UTF-8">
<title>{title}</title>
</head>
<body>
  <h1>{title}</h1>
  <table border="1" cellpadding="6" cellspacing="0">
    <thead>
      <tr>
        <th>Day</th><th>Date</th><th>Condition</th><th>High</th><th>Low</th><th>Summary</th>
      </tr>
    </thead>
    <tbody>
{rows}    </tbody>
  </table>
</body>
</html>
"#,
        title = escape(title),
        rows = rows
    );

    fs::write(out_path, html)
        .with_context(|| format!("writing extracted-data HTML to {}", out_path.display()))
}

/// Static, aspect-ratio-specific options. Row sizing (font size, row height,
/// column widths) is NOT here -- it's computed per-render in `render_poster_html`
/// from the actual number of days, so N rows always fit inside the available
/// height without shrinking below their text's line-height (that mismatch was
/// the cause of the earlier overlapping-text bug).
pub struct PosterLayout {
    pub width: u32,
    pub height: u32,
    pub padding: u32,
    pub title_size: u32,
    pub subtitle_size: u32,
    pub header_height: u32,
    pub max_row_font: u32,
    pub min_row_font: u32,
}

impl PosterLayout {
    pub fn instagram_9x16() -> Self {
        Self {
            width: 1080,
            height: 1920,
            padding: 56,
            title_size: 52,
            subtitle_size: 28,
            header_height: 150,
            max_row_font: 30,
            min_row_font: 16,
        }
    }

    pub fn youtube_16x9() -> Self {
        Self {
            width: 1920,
            height: 1080,
            padding: 48,
            title_size: 46,
            subtitle_size: 24,
            header_height: 110,
            max_row_font: 26,
            min_row_font: 14,
        }
    }
}

/// Requirement #2/#3: inject the same extracted data into the editable poster
/// template (assets/templates/poster.html), over the chosen background image,
/// at the chosen aspect ratio. No icons are drawn -- text only, per requirement 3.
/// Row height/font are computed from `days.len()` so rows never compress below
/// their text (no overlap) no matter how many days are passed in.
pub fn render_poster_html(
    days: &[DayForecast],
    title: &str,
    subtitle: &str,
    template_path: &Path,
    font_path: &Path,
    bg_image_path: &Path,
    layout: &PosterLayout,
    out_path: &Path,
) -> Result<()> {
    let template = fs::read_to_string(template_path)
        .with_context(|| format!("reading poster template at {}", template_path.display()))?;

    let n = days.len().max(1) as u32;
    let row_gap = 8u32;
    let available_height = layout
        .height
        .saturating_sub(2 * layout.padding)
        .saturating_sub(layout.header_height);
    let table_height = available_height;
    let row_height = (table_height.saturating_sub(row_gap * (n.saturating_sub(1)))) / n;
    let row_height = row_height.max(40);

    // Scale font to the row height, clamped to a sane readable range so text
    // never grows taller than the row (which is what caused overlap) and
    // never shrinks below legibility.
    let row_font_size = ((row_height as f32 * 0.30) as u32).clamp(layout.min_row_font, layout.max_row_font);
    let summary_font_size = ((row_font_size as f32 * 0.78) as u32).max(layout.min_row_font.saturating_sub(2).max(11));
    let row_pad_v = (row_height.saturating_sub(row_font_size * 2)) / 2;
    let row_pad_h = (layout.padding / 3).max(10);
    // How many lines of wrapped summary text fit in one row at summary_font_size.
    let summary_max_lines = ((row_height as f32 - 2.0 * row_pad_v as f32) / (summary_font_size as f32 * 1.2))
        .floor()
        .max(1.0) as u32;

    let mut rows = String::new();
    for day in days {
        rows.push_str(&format!(
            "    <tr>\n      <td class=\"day\">{} {}</td>\n      <td class=\"condition\">{}</td>\n      <td class=\"temps\">{} / {}</td>\n      <td class=\"summary\">{}</td>\n    </tr>\n",
            escape(&day.day_name),
            escape(&day.date),
            escape(&day.condition),
            escape(&day.high_temp),
            escape(&day.low_temp),
            escape(&day.summary),
        ));
    }

    let html = template
        .replace("{{TITLE}}", &escape(title))
        .replace("{{SUBTITLE}}", &escape(subtitle))
        .replace("{{FONT_PATH}}", &to_file_url(font_path)?)
        .replace("{{BG_IMAGE}}", &to_file_url(bg_image_path)?)
        .replace("{{WIDTH}}", &layout.width.to_string())
        .replace("{{HEIGHT}}", &layout.height.to_string())
        .replace("{{PADDING}}", &layout.padding.to_string())
        .replace("{{TITLE_SIZE}}", &layout.title_size.to_string())
        .replace("{{SUBTITLE_SIZE}}", &layout.subtitle_size.to_string())
        .replace("{{HEADER_HEIGHT}}", &layout.header_height.to_string())
        .replace("{{TABLE_HEIGHT}}", &table_height.to_string())
        .replace("{{ROW_GAP}}", &row_gap.to_string())
        .replace("{{ROW_HEIGHT}}", &row_height.to_string())
        .replace("{{ROW_PAD_V}}", &row_pad_v.to_string())
        .replace("{{ROW_PAD_H}}", &row_pad_h.to_string())
        .replace("{{ROW_FONT_SIZE}}", &row_font_size.to_string())
        .replace("{{SUMMARY_FONT_SIZE}}", &summary_font_size.to_string())
        .replace("{{SUMMARY_MAX_LINES}}", &summary_max_lines.to_string())
        .replace("{{COL_DAY_PCT}}", "20")
        .replace("{{COL_CONDITION_PCT}}", "22")
        .replace("{{COL_TEMPS_PCT}}", "18")
        .replace("{{COL_SUMMARY_PCT}}", "40")
        .replace("{{ROWS}}", &rows);

    fs::write(out_path, html)
        .with_context(|| format!("writing poster HTML to {}", out_path.display()))
}

fn abs_path(path: &Path) -> Result<String> {
    let abs = fs::canonicalize(path)
        .with_context(|| format!("resolving absolute path for {}", path.display()))?;
    // On Windows, canonicalize() returns a \\?\ prefixed path; strip it so file:// URLs work.
    let s = abs.to_string_lossy().to_string();
    Ok(s.strip_prefix(r"\\?\").unwrap_or(&s).replace('\\', "/"))
}

/// Builds a correct `file://` URL from an absolute filesystem path, for both
/// platform conventions: Windows drive-letter paths ("D:/x") need a THIRD
/// slash (`file:///D:/x`), or the browser parses "D:" as a hostname and the
/// resource silently fails to load (which is what caused missing background
/// images/fonts on Windows). POSIX paths already start with "/", so a plain
/// double-slash prefix ("file://" + "/home/x") is already correct for them.
pub fn to_file_url(path: &Path) -> Result<String> {
    let p = abs_path(path)?;
    if p.starts_with('/') {
        Ok(format!("file://{p}"))
    } else {
        Ok(format!("file:///{p}"))
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
