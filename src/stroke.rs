//! Calligraphic **stroke display list** — beam commands are the picture.
//!
//! Refresh clears the scanout to **transparent**, strokes the list, then the
//! host present paints only opaque beam pixels on top of the terminal.

use crate::{alpha, Color, Surface, GREEN};

/// One beam command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stroke {
    Color(Color),
    /// Set stroke width in pixels (1 = hairline, no upper bound).
    Width(i32),
    MoveTo {
        x: i32,
        y: i32,
    },
    LineTo {
        x: i32,
        y: i32,
    },
    Line {
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
    },
    Circle {
        cx: i32,
        cy: i32,
        r: i32,
    },
    /// Explicit width for this segment only.
    LineThick {
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        thickness: i32,
    },
}

/// Live stroke display list.
#[derive(Debug, Clone)]
pub struct DisplayList {
    cmds: Vec<Stroke>,
    beam: (i32, i32),
    color: Color,
    /// Current stroke width in pixels (default 1).
    width: i32,
}

impl Default for DisplayList {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayList {
    pub fn new() -> Self {
        Self {
            cmds: Vec::with_capacity(256),
            beam: (0, 0),
            color: GREEN,
            width: 1,
        }
    }

    pub fn with_capacity(n: usize) -> Self {
        Self {
            cmds: Vec::with_capacity(n),
            beam: (0, 0),
            color: GREEN,
            width: 1,
        }
    }

    pub fn clear(&mut self) {
        self.cmds.clear();
        self.beam = (0, 0);
        self.color = GREEN;
        self.width = 1;
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
        // Force opaque if caller passed RGB-only (alpha 0 but non-zero rgb).
        let c = if alpha(c) == 0 && c != 0 {
            c | 0xFF00_0000
        } else {
            c
        };
        self.color = c;
        self.cmds.push(Stroke::Color(c));
    }

    /// Stroke width in pixels. `1` is a hairline. No artificial maximum.
    pub fn set_width(&mut self, px: i32) {
        let w = px.max(1);
        self.width = w;
        self.cmds.push(Stroke::Width(w));
    }

    pub fn width(&self) -> i32 {
        self.width
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
            thickness: thickness.max(1),
        });
        self.beam = (x1, y1);
    }

    pub fn circle(&mut self, cx: i32, cy: i32, r: i32) {
        self.cmds.push(Stroke::Circle { cx, cy, r });
    }

    pub fn polyline(&mut self, pts: &[(i32, i32)]) {
        if pts.is_empty() {
            return;
        }
        self.move_to(pts[0].0, pts[0].1);
        for p in &pts[1..] {
            self.line_to(p.0, p.1);
        }
    }

    /// Execute the beam. Does not clear the surface.
    pub fn stroke(&self, surface: &mut Surface) {
        let mut beam = (0i32, 0i32);
        let mut color = GREEN;
        let mut width = 1i32;
        for cmd in &self.cmds {
            match *cmd {
                Stroke::Color(c) => color = c,
                Stroke::Width(w) => width = w.max(1),
                Stroke::MoveTo { x, y } => beam = (x, y),
                Stroke::LineTo { x, y } => {
                    draw_seg(surface, beam.0, beam.1, x, y, color, width);
                    beam = (x, y);
                }
                Stroke::Line { x0, y0, x1, y1 } => {
                    draw_seg(surface, x0, y0, x1, y1, color, width);
                    beam = (x1, y1);
                }
                Stroke::LineThick {
                    x0,
                    y0,
                    x1,
                    y1,
                    thickness,
                } => {
                    draw_seg(surface, x0, y0, x1, y1, color, thickness.max(1));
                    beam = (x1, y1);
                }
                Stroke::Circle { cx, cy, r } => {
                    if width <= 1 {
                        surface.circle(cx, cy, r, color);
                    } else {
                        // Concentric outlines approximate thick stroke circles.
                        let half = width / 2;
                        for o in -half..=half {
                            let rr = r + o;
                            if rr > 0 {
                                surface.circle(cx, cy, rr, color);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Clear scanout to **transparent**, then stroke the list.
    /// No phosphor. No black fill. Terminal owns the background.
    pub fn refresh(&self, surface: &mut Surface) {
        surface.clear_transparent();
        self.stroke(surface);
    }
}

fn draw_seg(surface: &mut Surface, x0: i32, y0: i32, x1: i32, y1: i32, color: Color, width: i32) {
    let w = width.max(1);
    if w == 1 {
        surface.line(x0, y0, x1, y1, color);
    } else {
        surface.line_thick(x0, y0, x1, y1, color, w);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{alpha, GREEN, TRANSPARENT};

    #[test]
    fn list_strokes_a_line() {
        let mut list = DisplayList::new();
        list.set_color(GREEN);
        list.line(0, 0, 10, 0);
        let mut s = Surface::new(32, 32);
        list.refresh(&mut s);
        assert_eq!(s.get(0, 0), Some(GREEN));
        assert_eq!(s.get(10, 0), Some(GREEN));
        // Background stays transparent.
        assert_eq!(s.get(0, 1).map(alpha).unwrap_or(0), 0);
        assert_eq!(s.get(5, 5), Some(TRANSPARENT));
    }

    #[test]
    fn width_at_least_one() {
        let mut list = DisplayList::new();
        list.set_width(0);
        assert_eq!(list.width(), 1);
        list.set_width(5);
        assert_eq!(list.width(), 5);
    }
}
