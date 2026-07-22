//! Calligraphic **stroke display list** — the 1970s vector CRT model, 2026.
//!
//! # Why this exists
//!
//! Aircraft HUDs and vector monitors (Asteroids, Tektronix, fighter CRTs) do
//! **not** paint a full bitmap every frame as their native model. They hold a
//! **display list** of beam commands (MOVE / DRAW). A refresh processor
//! retraces the list; phosphor holds the glow between retraces.
//!
//! VGE uses that architecture on modern hosts:
//!
//! 1. **Display list** = the live image (strokes)
//! 2. **Refresh** = optional phosphor decay + execute all strokes into a pixel
//!    buffer (software beam) + host present (Kitty / FB / half-block)
//! 3. Pixels are the scanout of the beam — not the source of truth
//!
//! See: vector / calligraphic CRT, refresh display list, HUD stroke generators.

use crate::{Color, Surface, GREEN};

/// One beam command. Compact, rebuild-friendly (HUD symbols are small lists).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stroke {
    /// Set beam color (0x00RRGGBB).
    Color(Color),
    /// Absolute move (beam off).
    MoveTo { x: i32, y: i32 },
    /// Draw to absolute point (beam on).
    LineTo { x: i32, y: i32 },
    /// Draw segment without changing “current” model beyond endpoint.
    Line { x0: i32, y0: i32, x1: i32, y1: i32 },
    /// Circle outline center/radius.
    Circle { cx: i32, cy: i32, r: i32 },
    /// Thick line (integer thickness).
    LineThick {
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        thickness: i32,
    },
}

/// Live stroke display list — same role as 1970s refresh memory.
#[derive(Debug, Clone, Default)]
pub struct DisplayList {
    cmds: Vec<Stroke>,
    /// Beam position after last command (for MoveTo/LineTo sequences).
    beam: (i32, i32),
    color: Color,
}

impl DisplayList {
    pub fn new() -> Self {
        Self {
            cmds: Vec::with_capacity(256),
            beam: (0, 0),
            color: GREEN,
        }
    }

    pub fn with_capacity(n: usize) -> Self {
        Self {
            cmds: Vec::with_capacity(n),
            beam: (0, 0),
            color: GREEN,
        }
    }

    /// Wipe the list (new picture). Does not touch the pixel surface.
    pub fn clear(&mut self) {
        self.cmds.clear();
        self.beam = (0, 0);
        self.color = GREEN;
    }

    pub fn len(&self) -> usize {
        self.cmds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }

    pub fn commands(&self) -> &[Stroke] {
        &self.cmds
    }

    pub fn set_color(&mut self, c: Color) {
        self.color = c;
        self.cmds.push(Stroke::Color(c));
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        self.beam = (x, y);
        self.cmds.push(Stroke::MoveTo { x, y });
    }

    pub fn line_to(&mut self, x: i32, y: i32) {
        self.cmds.push(Stroke::LineTo { x, y });
        self.beam = (x, y);
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.cmds.push(Stroke::Line { x0, y0, x1, y1 });
        self.beam = (x1, y1);
    }

    pub fn line_thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, thickness: i32) {
        self.cmds.push(Stroke::LineThick {
            x0,
            y0,
            x1,
            y1,
            thickness,
        });
        self.beam = (x1, y1);
    }

    pub fn circle(&mut self, cx: i32, cy: i32, r: i32) {
        self.cmds.push(Stroke::Circle { cx, cy, r });
    }

    /// Polyline: move to first, line_to rest.
    pub fn polyline(&mut self, pts: &[(i32, i32)]) {
        if pts.is_empty() {
            return;
        }
        self.move_to(pts[0].0, pts[0].1);
        for p in &pts[1..] {
            self.line_to(p.0, p.1);
        }
    }

    /// Execute the beam over `surface` (software deflection). No clear.
    pub fn stroke(&self, surface: &mut Surface) {
        let mut beam = (0i32, 0i32);
        let mut color = GREEN;
        for cmd in &self.cmds {
            match *cmd {
                Stroke::Color(c) => color = c,
                Stroke::MoveTo { x, y } => beam = (x, y),
                Stroke::LineTo { x, y } => {
                    surface.line(beam.0, beam.1, x, y, color);
                    beam = (x, y);
                }
                Stroke::Line { x0, y0, x1, y1 } => {
                    surface.line(x0, y0, x1, y1, color);
                    beam = (x1, y1);
                }
                Stroke::LineThick {
                    x0,
                    y0,
                    x1,
                    y1,
                    thickness,
                } => {
                    surface.line_thick(x0, y0, x1, y1, color, thickness);
                    beam = (x1, y1);
                }
                Stroke::Circle { cx, cy, r } => {
                    surface.circle(cx, cy, r, color);
                }
            }
        }
    }

    /// Classic refresh cycle:
    /// 1. phosphor decay (or hard clear if `decay_256 == 0`)
    /// 2. stroke the full list into the surface
    ///
    /// Host present (Kitty/FB) is separate — this is the beam pass only.
    pub fn refresh(&self, surface: &mut Surface, decay_256: u32) {
        if decay_256 == 0 {
            surface.clear(0);
        } else {
            surface.decay(decay_256.min(256));
        }
        self.stroke(surface);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BLACK, GREEN};

    #[test]
    fn list_strokes_a_line() {
        let mut list = DisplayList::new();
        list.set_color(GREEN);
        list.line(0, 0, 10, 0);
        let mut s = Surface::new(32, 32);
        s.clear(BLACK);
        list.stroke(&mut s);
        assert_eq!(s.get(0, 0), Some(GREEN));
        assert_eq!(s.get(10, 0), Some(GREEN));
    }

    #[test]
    fn refresh_decay_then_stroke() {
        let mut list = DisplayList::new();
        list.line(0, 5, 20, 5);
        let mut s = Surface::new(32, 32);
        s.clear(BLACK);
        list.refresh(&mut s, 0);
        assert_eq!(s.get(10, 5), Some(GREEN));
    }
}
