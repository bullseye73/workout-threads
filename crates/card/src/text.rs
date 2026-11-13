//! 내장 Pretendard로 글자를 RGBA 버퍼에 직접 합성한다.
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};

static MEDIUM: &[u8] = include_bytes!("../assets/fonts/Pretendard-Medium.otf");
static SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Pretendard-SemiBold.otf");
static BOLD: &[u8] = include_bytes!("../assets/fonts/Pretendard-Bold.otf");

#[derive(Clone, Copy)]
pub enum Weight {
    Medium,
    SemiBold,
    Bold,
}

pub fn font(w: Weight) -> FontRef<'static> {
    let bytes = match w {
        Weight::Medium => MEDIUM,
        Weight::SemiBold => SEMIBOLD,
        Weight::Bold => BOLD,
    };
    FontRef::try_from_slice(bytes).expect("내장 폰트")
}

/// 글자 한 줄의 스타일.
#[derive(Clone, Copy)]
pub struct Style {
    pub weight: Weight,
    pub size: f32,
    pub kern: f32,
}

impl Style {
    pub fn ascent(&self) -> f32 {
        font(self.weight)
            .as_scaled(PxScale::from(self.size))
            .ascent()
    }
    /// 줄 높이 (ascent − descent).
    pub fn height(&self) -> f32 {
        font(self.weight)
            .as_scaled(PxScale::from(self.size))
            .height()
    }
}

/// 버퍼(RGBA8, 불투명) 위의 캔버스.
pub struct Canvas<'a> {
    pub buf: &'a mut [u8],
    pub w: u32,
    pub h: u32,
}

impl Canvas<'_> {
    /// 한 픽셀에 색을 알파 합성.
    pub fn blend(&mut self, x: i32, y: i32, rgb: [u8; 3], a: f32) {
        if x < 0 || y < 0 || x as u32 >= self.w || y as u32 >= self.h || a <= 0.0 {
            return;
        }
        let i = ((y as u32 * self.w + x as u32) * 4) as usize;
        let a = a.min(1.0);
        for c in 0..3 {
            let d = self.buf[i + c] as f32;
            self.buf[i + c] = (rgb[c] as f32 * a + d * (1.0 - a)).round() as u8;
        }
    }

    /// 기준선 (x, baseline)에서 글자를 그리고 너비를 돌려준다.
    pub fn text(
        &mut self,
        st: &Style,
        s: &str,
        x: f32,
        baseline: f32,
        rgb: [u8; 3],
        alpha: f32,
    ) -> f32 {
        let (glyphs, width) = layout(st, s, x, baseline);
        let f = font(st.weight);
        for g in glyphs {
            if let Some(o) = f.outline_glyph(g) {
                let b = o.px_bounds();
                o.draw(|gx, gy, cov| {
                    self.blend(
                        b.min.x as i32 + gx as i32,
                        b.min.y as i32 + gy as i32,
                        rgb,
                        cov * alpha,
                    )
                });
            }
        }
        width
    }
}

fn layout(st: &Style, s: &str, x: f32, baseline: f32) -> (Vec<ab_glyph::Glyph>, f32) {
    let f = font(st.weight);
    let sf = f.as_scaled(PxScale::from(st.size));
    let mut caret = x;
    let mut prev = None;
    let mut out = Vec::new();
    for ch in s.chars() {
        let id = sf.glyph_id(ch);
        if let Some(p) = prev {
            caret += sf.kern(p, id);
        }
        out.push(id.with_scale_and_position(st.size, ab_glyph::point(caret, baseline)));
        caret += sf.h_advance(id) + st.kern;
        prev = Some(id);
    }
    (out, caret - x)
}
