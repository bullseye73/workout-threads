//! 카드 배치. 기존 card.swift와 같은 디자인 (u = 짧은 변/100, 여백 5u).
use image::{DynamicImage, ImageReader};
use std::path::Path;
use tiny_skia::{
    Color, GradientStop, LineCap, LineJoin, LinearGradient, Paint, PathBuilder, Pixmap, Point,
    Rect, SpreadMode, Stroke, Transform,
};
use workout_core::photo::read_photo_meta;

use crate::CardError;
use crate::data::CardData;
use crate::text::{Canvas, Style, Weight};

const MIN_LONG_SIDE: f32 = 2160.0;
const JPEG_QUALITY: u8 = 92;
const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];

/// 사진에 카드를 얹어 JPEG 바이트로 돌려준다. EXIF는 쓰지 않는다(GPS 제거).
pub fn render_card(photo: &Path, data: &CardData) -> Result<Vec<u8>, CardError> {
    let img = load_oriented(photo)?;
    let mut pm = to_pixmap(&img)?;
    let (w, h) = (pm.width() as f32, pm.height() as f32);
    let u = w.min(h) / 100.0;
    let m = 5.0 * u;

    shade(&mut pm, 0.0, 26.0 * u, 0.45)?;
    shade(&mut pm, h, h - 52.0 * u, 0.62)?;

    let date = Style {
        weight: Weight::Medium,
        size: 3.3 * u,
        kern: 0.25 * u,
    };
    let title = Style {
        weight: Weight::SemiBold,
        size: 4.6 * u,
        kern: 0.0,
    };
    let sub = Style {
        weight: Weight::Medium,
        size: 3.3 * u,
        kern: 0.0,
    };
    let dist = Style {
        weight: Weight::Bold,
        size: 10.0 * u,
        kern: -0.2 * u,
    };
    let km = Style {
        weight: Weight::SemiBold,
        size: 4.2 * u,
        kern: 0.0,
    };

    // 아래 기준 위치 계산 (화면 좌표, 위가 0).
    let sub_top = h - m - sub.height();
    let dist_top = sub_top - 0.2 * u - dist.height();
    let route_bottom = dist_top - 1.5 * u;

    draw_route(&mut pm, &data.route, m + 0.5 * u, route_bottom, u)?;

    let (pw, ph) = (pm.width(), pm.height());
    let mut cv = Canvas {
        buf: pm.data_mut(),
        w: pw,
        h: ph,
    };
    let shadow = 0.15 * u;
    let line = |cv: &mut Canvas, st: &Style, s: &str, x: f32, top: f32, alpha: f32| {
        let base = top + st.ascent();
        cv.text(st, s, x, base + shadow, BLACK, 0.45 * alpha);
        cv.text(st, s, x, base, WHITE, alpha)
    };
    line(&mut cv, &date, &data.date, m, m, 0.85);
    line(
        &mut cv,
        &title,
        &data.title,
        m,
        m + date.height() + 0.6 * u,
        1.0,
    );
    line(&mut cv, &sub, &data.sub, m, sub_top, 0.9);
    let num_w = line(&mut cv, &dist, &data.distance, m, dist_top, 1.0);
    // " km"은 거리 숫자와 같은 기준선.
    let base = dist_top + dist.ascent();
    cv.text(&km, " km", m + num_w, base + shadow, BLACK, 0.45 * 0.9);
    cv.text(&km, " km", m + num_w, base, WHITE, 0.9);

    encode_jpeg(&pm)
}

pub fn render_card_to_file(photo: &Path, data: &CardData, out: &Path) -> Result<(), CardError> {
    std::fs::write(out, render_card(photo, data)?)?;
    Ok(())
}

/// 사진을 읽고 EXIF 방향대로 돌린다 (카드·업로드 공용).
pub(crate) fn open_oriented(photo: &Path) -> Result<DynamicImage, CardError> {
    let mut img = ImageReader::open(photo)?.with_guessed_format()?.decode()?;
    let orientation = read_photo_meta(photo, 0)
        .map(|m| m.orientation)
        .unwrap_or(1);
    if let Some(o) = image::metadata::Orientation::from_exif(orientation as u8) {
        img.apply_orientation(o);
    }
    Ok(img)
}

fn load_oriented(photo: &Path) -> Result<DynamicImage, CardError> {
    let mut img = open_oriented(photo)?;
    let long = img.width().max(img.height()) as f32;
    if long < MIN_LONG_SIDE {
        let s = MIN_LONG_SIDE / long;
        let (nw, nh) = (
            (img.width() as f32 * s) as u32,
            (img.height() as f32 * s) as u32,
        );
        img = img.resize_exact(nw, nh, image::imageops::FilterType::Lanczos3);
    }
    Ok(img)
}

fn to_pixmap(img: &DynamicImage) -> Result<Pixmap, CardError> {
    let rgba = img.to_rgb8();
    let mut pm = Pixmap::new(rgba.width(), rgba.height())
        .ok_or_else(|| CardError::Draw("빈 이미지".into()))?;
    for (dst, src) in pm
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(rgba.pixels())
    {
        dst.copy_from_slice(&[src[0], src[1], src[2], 255]);
    }
    Ok(pm)
}

/// y_from(진한 쪽, 알파 alpha) → y_to(투명) 세로 그라데이션.
fn shade(pm: &mut Pixmap, y_from: f32, y_to: f32, alpha: f32) -> Result<(), CardError> {
    let shader = LinearGradient::new(
        Point::from_xy(0.0, y_from),
        Point::from_xy(0.0, y_to),
        vec![
            GradientStop::new(0.0, Color::from_rgba(0.0, 0.0, 0.0, alpha).unwrap()),
            GradientStop::new(1.0, Color::from_rgba(0.0, 0.0, 0.0, 0.0).unwrap()),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    )
    .ok_or_else(|| CardError::Draw("그라데이션".into()))?;
    let rect = Rect::from_ltrb(0.0, y_from.min(y_to), pm.width() as f32, y_from.max(y_to))
        .ok_or_else(|| CardError::Draw("음영 영역".into()))?;
    let paint = Paint {
        shader,
        anti_alias: true,
        ..Paint::default()
    };
    pm.fill_rect(rect, &paint, Transform::identity(), None);
    Ok(())
}

/// 26u × 20u 상자에 비율을 지켜 맞춘 흰 경로 선. 왼쪽 아래 = (left, bottom).
fn draw_route(
    pm: &mut Pixmap,
    route: &[[f64; 2]],
    left: f32,
    bottom: f32,
    u: f32,
) -> Result<(), CardError> {
    if route.len() < 2 {
        return Ok(());
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in route {
        min_x = min_x.min(p[0]);
        max_x = max_x.max(p[0]);
        min_y = min_y.min(p[1]);
        max_y = max_y.max(p[1]);
    }
    let k = ((26.0 * u) as f64 / (max_x - min_x).max(1e-9))
        .min((20.0 * u) as f64 / (max_y - min_y).max(1e-9));
    let mut pb = PathBuilder::new();
    for (i, p) in route.iter().enumerate() {
        let x = left + ((p[0] - min_x) * k) as f32;
        let y = bottom - ((p[1] - min_y) * k) as f32;
        if i == 0 {
            pb.move_to(x, y)
        } else {
            pb.line_to(x, y)
        }
    }
    let path = pb.finish().ok_or_else(|| CardError::Draw("경로".into()))?;
    let stroke = |width: f32| Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    let mut paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    paint.set_color(Color::from_rgba(0.0, 0.0, 0.0, 0.3).unwrap());
    pm.stroke_path(
        &path,
        &paint,
        &stroke(0.55 * u + 1.2 * u),
        Transform::identity(),
        None,
    );
    paint.set_color(Color::WHITE);
    pm.stroke_path(
        &path,
        &paint,
        &stroke(0.55 * u),
        Transform::identity(),
        None,
    );
    Ok(())
}

fn encode_jpeg(pm: &Pixmap) -> Result<Vec<u8>, CardError> {
    let rgb: Vec<u8> = pm
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let img = image::RgbImage::from_raw(pm.width(), pm.height(), rgb)
        .ok_or_else(|| CardError::Draw("버퍼".into()))?;
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&img)?;
    Ok(out)
}
