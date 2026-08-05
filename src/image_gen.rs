use crate::model::DayForecast;
use anyhow::{anyhow, Context, Result};
use fontdue::{Font, FontSettings};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_circle_mut, draw_filled_rect_mut, draw_line_segment_mut};
use imageproc::rect::Rect;
use rustybuzz::{shape, script, Direction, Language, UnicodeBuffer};
use std::fs;
use std::path::Path;
use std::process::Command;

const WIDTH: u32 = 1200;
const ROW_HEIGHT: u32 = 130;
const HEADER_HEIGHT: u32 = 180;
const CARD_GAP: u32 = 12;

struct KannadaFont {
    font: Font,
    raw: Vec<u8>,
}

/// Renders the translated 10-day forecast into a PNG at `out_path`.
/// `font_path` MUST point to a TTF/OTF font with Kannada glyph coverage,
/// e.g. Noto Sans Kannada (https://fonts.google.com/noto/specimen/Noto+Sans+Kannada).
pub fn render_png(
    days: &[DayForecast],
    font_path: &Path,
    out_path: &Path,
    title_kn: &str,
) -> Result<()> {
    let font = load_font(font_path).unwrap_or_else(|err| {
        eprintln!("Falling back to built-in font because {}", err);
        load_default_font().unwrap_or_else(|fallback_err| {
            panic!("failed to load any font: {fallback_err}");
        })
    });

    let height = HEADER_HEIGHT + (ROW_HEIGHT + CARD_GAP) * days.len() as u32 + 20;
    let mut img = RgbaImage::from_pixel(WIDTH, height, Rgba([247, 250, 254, 255]));

    let navy = Rgba([7, 44, 112, 255]);
    let sky = Rgba([33, 111, 255, 255]);
    let card_bg = Rgba([255, 255, 255, 255]);
    let accent = Rgba([227, 239, 255, 255]);
    let dark = Rgba([25, 32, 46, 255]);
    let muted = Rgba([99, 110, 125, 255]);
    let rain_blue = Rgba([29, 114, 206, 255]);
    let cloud_grey = Rgba([148, 156, 168, 255]);

    draw_filled_rect_mut(&mut img, Rect::at(0, 0).of_size(WIDTH, HEADER_HEIGHT), navy);
    draw_text_mut(&mut img, Rgba([255, 255, 255, 255]), 34, 42, 42.0, &font, title_kn)?;
    draw_text_mut(&mut img, Rgba([227, 239, 255, 255]), 34, 96, 24.0, &font, "10 ದಿನಗಳ ಹವಾಮಾನ ಮುನ್ಸೂಚನೆ")?;
    draw_text_mut(&mut img, Rgba([227, 239, 255, 255]), 34, 130, 20.0, &font, "ಮೈಸೂರಿನ ಮುಂದಿನ ಹವಾಮಾನದ ಸ್ಥಿತಿ")?;

    for (i, day) in days.iter().enumerate() {
        let y = HEADER_HEIGHT as i32 + (i as u32 * (ROW_HEIGHT + CARD_GAP)) as i32;
        let card_y = y + 10;
        let card_h = ROW_HEIGHT - 10;

        draw_filled_rect_mut(&mut img, Rect::at(20, card_y).of_size(WIDTH - 40, card_h), card_bg);
        draw_filled_rect_mut(&mut img, Rect::at(20, card_y).of_size(12, card_h), sky);

        let line1 = format!("{}  {}", day.day_name, day.date);
        let line2 = day.condition.clone();
        let line3 = if day.summary.is_empty() {
            String::from("ಹವಾಮान ಮಾಹಿತಿ ಲಭ್ಯವಿಲ್ಲ")
        } else {
            day.summary.clone()
        };
        let line4 = format!("ಗರಿಷ್ಠ {}   ಕನಿಷ್ಠ {}   {}", day.high_temp, day.low_temp, day.precip_chance);

        draw_text_mut(&mut img, dark, 50, card_y + 34, 32.0, &font, &line1)?;
        draw_text_mut(&mut img, muted, 50, card_y + 72, 24.0, &font, &line2)?;
        draw_text_mut(&mut img, dark, 50, card_y + 104, 22.0, &font, &line3)?;
        draw_text_mut(&mut img, sky, 520, card_y + 60, 34.0, &font, &line4)?;

        draw_weather_icon(&mut img, 930, card_y + 22, &day.condition, rain_blue, cloud_grey, accent);
        draw_drop_icon(&mut img, 720, card_y + 48, rain_blue);
    }

    img.save(out_path)
        .with_context(|| format!("saving PNG to {}", out_path.display()))?;
    Ok(())
}

fn draw_text_mut(
    img: &mut RgbaImage,
    color: Rgba<u8>,
    x: i32,
    y: i32,
    size: f32,
    font: &KannadaFont,
    text: &str,
) -> Result<()> {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_script(script::KANNADA);
    buffer.set_language("kn".parse::<Language>().unwrap());
    buffer.set_direction(Direction::LeftToRight);

    let face = rustybuzz::Face::from_slice(&font.raw, 0)
        .ok_or_else(|| anyhow!("parsing font face while shaping text"))?;
    let shape_plan = rustybuzz::ShapePlan::new(
        &face,
        Direction::LeftToRight,
        Some(script::KANNADA),
        Some(&"kn".parse::<Language>().unwrap()),
        &[],
    );
    let glyph_buffer = rustybuzz::shape_with_plan(&face, &shape_plan, buffer);
    let mut pen_x = x as f32;

    let baseline = if let Some(metrics) = font.font.horizontal_line_metrics(size) {
        y as f32 + metrics.ascent
    } else {
        y as f32 + size * 0.8
    };

    let debug = text.contains("10") || text.contains("ಮೈ") || text.contains("ಮೈಸೂರ") || text.contains("ಹವಾಮಾನ");
    if debug {
        eprintln!("draw_text_mut {:?}: baseline={} text_len={} glyphs={}", text, baseline, text.len(), glyph_buffer.glyph_infos().len());
    }

    for (i, (glyph_info, glyph_pos)) in glyph_buffer.glyph_infos().iter().zip(glyph_buffer.glyph_positions()).enumerate() {
        let glyph_index = glyph_info.glyph_id as u16;
        let (metrics, bitmap) = font.font.rasterize_indexed(glyph_index, size);
        let x_offset = glyph_pos.x_offset as f32 / 64.0;
        let y_offset = glyph_pos.y_offset as f32 / 64.0;
        let glyph_x = pen_x + x_offset + metrics.xmin as f32;
        let glyph_y = baseline - (metrics.height as f32 + metrics.ymin as f32) - y_offset;

        if debug && i < 12 {
            eprintln!("  glyph {} gid={} adv={} xoff={} yoff={} xmin={} ymin={} w={} h={}", i, glyph_index, glyph_pos.x_advance, glyph_pos.x_offset, glyph_pos.y_offset, metrics.xmin, metrics.ymin, metrics.width, metrics.height);
        }

        overlay_bitmap(img, &bitmap, metrics.width, metrics.height, glyph_x as i32, glyph_y as i32, color);
        pen_x += glyph_pos.x_advance as f32 / 64.0;
    }

    Ok(())
}

fn overlay_bitmap(
    img: &mut RgbaImage,
    bitmap: &[u8],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    color: Rgba<u8>,
) {
    for row in 0..height {
        for col in 0..width {
            let alpha = bitmap[row * width + col] as f32 / 255.0;
            if alpha <= 0.01 {
                continue;
            }
            let px = x + col as i32;
            let py = y + row as i32;
            if px < 0 || py < 0 || px >= img.width() as i32 || py >= img.height() as i32 {
                continue;
            }

            let mut dest = *img.get_pixel(px as u32, py as u32);
            let src_a = alpha;
            let inv_a = 1.0 - src_a;

            dest[0] = ((color[0] as f32 * src_a) + (dest[0] as f32 * inv_a)).round() as u8;
            dest[1] = ((color[1] as f32 * src_a) + (dest[1] as f32 * inv_a)).round() as u8;
            dest[2] = ((color[2] as f32 * src_a) + (dest[2] as f32 * inv_a)).round() as u8;
            dest[3] = 255;
            img.put_pixel(px as u32, py as u32, dest);
        }
    }
}

fn draw_weather_icon(
    img: &mut RgbaImage,
    x: i32,
    y: i32,
    condition: &str,
    rain_blue: Rgba<u8>,
    cloud_grey: Rgba<u8>,
    accent: Rgba<u8>,
) {
    let kind = weather_icon_kind(condition);
    let center = (x + 40, y + 40);
    match kind {
        IconKind::Sunny => draw_sun_icon(img, center, 28, Rgba([255, 196, 0, 255]), accent),
        IconKind::MostlySunny => {
            draw_sun_icon(img, center, 20, Rgba([255, 196, 0, 255]), accent);
            draw_cloud_icon(img, (x + 52, y + 48), cloud_grey);
        }
        IconKind::Cloudy => draw_cloud_icon(img, center, cloud_grey),
        IconKind::Rain => {
            draw_cloud_icon(img, center, cloud_grey);
            draw_rain_lines(img, center, rain_blue);
        }
        IconKind::Thunderstorm => {
            draw_cloud_icon(img, center, cloud_grey);
            draw_rain_lines(img, center, rain_blue);
            draw_lightning(img, (x + 42, y + 48), Rgba([255, 219, 77, 255]));
        }
        IconKind::Snow => {
            draw_cloud_icon(img, center, cloud_grey);
            draw_snowflake(img, (x + 40, y + 62), Rgba([255, 255, 255, 255]));
        }
        IconKind::Fog | IconKind::Unknown => draw_cloud_icon(img, center, cloud_grey),
    }
}

fn draw_drop_icon(img: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>) {
    let width = 16;
    for row in 0..25 {
        let progress = row as f32 / 24.0;
        let half = (1.0 - (progress - 0.5).abs() * 2.0).max(0.0);
        let radius = (width as f32 * half).max(2.0) as i32;
        draw_filled_circle_mut(img, (x + 10, y + 5 + row), radius, color);
    }
    draw_filled_circle_mut(img, (x + 10, y + 4), 8, color);
}

fn draw_sun_icon(img: &mut RgbaImage, center: (i32, i32), radius: i32, fill: Rgba<u8>, ray: Rgba<u8>) {
    draw_filled_circle_mut(img, center, radius, fill);
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::PI / 4.0;
        let x1 = center.0 as f32 + angle.cos() * (radius as f32 + 6.0);
        let y1 = center.1 as f32 + angle.sin() * (radius as f32 + 6.0);
        let x2 = center.0 as f32 + angle.cos() * (radius as f32 + 16.0);
        let y2 = center.1 as f32 + angle.sin() * (radius as f32 + 16.0);
        draw_line_segment_mut(img, (x1, y1), (x2, y2), ray);
    }
}

fn draw_cloud_icon(img: &mut RgbaImage, center: (i32, i32), fill: Rgba<u8>) {
    draw_filled_circle_mut(img, (center.0 - 22, center.1), 18, fill);
    draw_filled_circle_mut(img, (center.0 - 4, center.1 - 12), 18, fill);
    draw_filled_circle_mut(img, (center.0 + 18, center.1), 18, fill);
    draw_filled_rect_mut(img, Rect::at(center.0 - 30, center.1 - 6).of_size(88, 32), fill);
}

fn draw_rain_lines(img: &mut RgbaImage, center: (i32, i32), color: Rgba<u8>) {
    let drops = [(center.0 - 12, center.1 + 18), (center.0, center.1 + 24), (center.0 + 12, center.1 + 18)];
    for (x, y) in drops {
        draw_line_segment_mut(img, (x as f32, y as f32), (x as f32, (y + 12) as f32), color);
        draw_line_segment_mut(img, ((x + 2) as f32, (y + 2) as f32), ((x + 2) as f32, (y + 14) as f32), color);
    }
}

fn draw_lightning(img: &mut RgbaImage, start: (i32, i32), color: Rgba<u8>) {
    let points = [
        (start.0 as f32, start.1 as f32),
        ((start.0 + 8) as f32, (start.1 + 2) as f32),
        ((start.0 + 2) as f32, (start.1 + 10) as f32),
        ((start.0 + 10) as f32, (start.1 + 10) as f32),
    ];
    for window in points.windows(2) {
        draw_line_segment_mut(img, window[0], window[1], color);
    }
}

fn draw_snowflake(img: &mut RgbaImage, center: (i32, i32), color: Rgba<u8>) {
    let offsets = [(-8, 0), (8, 0), (0, -8), (0, 8), (-6, -6), (6, -6), (-6, 6), (6, 6)];
    for (dx, dy) in offsets {
        draw_line_segment_mut(img, (center.0 as f32, center.1 as f32), ((center.0 + dx) as f32, (center.1 + dy) as f32), color);
    }
}

fn weather_icon_kind(condition: &str) -> IconKind {
    let condition = condition.to_lowercase();
    if condition.contains("thunder") || condition.contains("ತೀವ್ರ") {
        IconKind::Thunderstorm
    } else if condition.contains("rain") || condition.contains("ಮಳೆ") || condition.contains("rainy") {
        IconKind::Rain
    } else if condition.contains("snow") || condition.contains("ಹಿಮ") {
        IconKind::Snow
    } else if condition.contains("cloud") || condition.contains("ಮೋಡ") {
        IconKind::Cloudy
    } else if condition.contains("sun") || condition.contains("ಸೂರ್ಯ") {
        IconKind::Sunny
    } else if condition.contains("fog") || condition.contains("ಕೂದಲು") {
        IconKind::Fog
    } else {
        IconKind::Unknown
    }
}

enum IconKind {
    Sunny,
    MostlySunny,
    Cloudy,
    Rain,
    Thunderstorm,
    Snow,
    Fog,
    Unknown,
}

fn load_font(font_path: &Path) -> Result<KannadaFont> {
    let font_bytes = fs::read(font_path)
        .with_context(|| format!("reading font at {}", font_path.display()))?;

    let font = Font::from_bytes(font_bytes.clone(), FontSettings::default())
        .map_err(|err| anyhow!("parsing font data: {err}"))?;
    Ok(KannadaFont {
        font,
        raw: font_bytes,
    })
}

fn load_default_font() -> Result<KannadaFont> {
    let candidates = [
        r"C:\Windows\Fonts\Nirmala.ttf",
        r"C:\Windows\Fonts\NirmalaUI.ttf",
        r"C:\Windows\Fonts\Kedage.ttf",
        r"C:\Windows\Fonts\arial.ttf",
        r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\malgun.ttf",
    ];

    for candidate in candidates.iter() {
        if let Ok(bytes) = fs::read(candidate) {
            if let Ok(font) = Font::from_bytes(bytes.clone(), FontSettings::default()) {
                if rustybuzz::Face::from_slice(&bytes, 0).is_some() {
                    return Ok(KannadaFont {
                        font,
                        raw: bytes,
                    });
                }
            }
        }
    }

    if let Ok(output) = Command::new("fc-match").arg("NotoSansKannada-Regular.ttf").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if let Ok(bytes) = fs::read(&path) {
                if let Ok(font) = Font::from_bytes(bytes.clone(), FontSettings::default()) {
                    if rustybuzz::Face::from_slice(&bytes, 0).is_some() {
                        return Ok(KannadaFont {
                            font,
                            raw: bytes,
                        });
                    }
                }
            }
        }
    }

    anyhow::bail!("no suitable system font found for Kannada text. Install Noto Sans Kannada or a Kannada-capable font and place it in assets/")
}
