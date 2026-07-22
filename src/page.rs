//! **Page** compositor: clear glass, then multiple widget calls.

use crate::geom::Rect;
use crate::widget::{
    bezel_frame, label, label_centered, round_gauge, softkey_row, tape_gauge, RoundGaugeOpts,
    SoftkeyLayout, TapeOpts,
};
use crate::{Color, Surface};

/// One MFD page (one surface frame). Call any mix of widgets.
pub struct Page<'a> {
    pub surface: &'a mut Surface,
    pub bounds: Rect,
    pub font_px: f32,
}

impl<'a> Page<'a> {
    pub fn new(surface: &'a mut Surface) -> Self {
        let bounds = Rect::new(0, 0, surface.width() as i32, surface.height() as i32);
        Self {
            surface,
            bounds,
            font_px: 14.0,
        }
    }

    /// Black glass clear.
    pub fn clear(&mut self) {
        self.surface.clear_black();
    }

    pub fn bezel(&mut self) {
        bezel_frame(self.surface, self.bounds.inset(2));
    }

    pub fn softkeys(&mut self, rect: Rect, labels: &[&str], selected: Option<usize>) {
        softkey_row(
            self.surface,
            rect,
            labels,
            SoftkeyLayout {
                font_px: self.font_px,
                selected,
            },
        );
    }

    pub fn tape(&mut self, rect: Rect, opts: TapeOpts) {
        tape_gauge(self.surface, rect, opts);
    }

    pub fn round_gauge(&mut self, rect: Rect, opts: RoundGaugeOpts) {
        round_gauge(self.surface, rect, opts);
    }

    pub fn label(&mut self, x: f32, y: f32, text: &str, color: Color) {
        label(self.surface, x, y, text, color, self.font_px);
    }

    pub fn label_at(&mut self, x: f32, y: f32, text: &str, color: Color, px: f32) {
        label(self.surface, x, y, text, color, px);
    }

    pub fn label_centered(&mut self, cx: f32, cy: f32, text: &str, color: Color) {
        label_centered(self.surface, cx, cy, text, color, self.font_px);
    }

    pub fn content_rect(&self, top_chrome: i32, bot_chrome: i32) -> Rect {
        let b = self.bounds.inset(4);
        Rect::new(
            b.x,
            b.y + top_chrome,
            b.w,
            (b.h - top_chrome - bot_chrome).max(8),
        )
    }
}
