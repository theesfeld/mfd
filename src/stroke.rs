//! Calligraphic **stroke display list** — beam commands are the picture.
//!
//! Moving vectors **sweep**: erase the previous path, then stroke the new path.
//! No full-scene clear/redraw. Hairlines use crisp Xiaolin Wu AA (asm).

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

    /// Execute the beam (draw). Does not clear the surface.
    pub fn stroke(&self, surface: &mut Surface) {
        self.run(surface, false);
    }

    /// Erase this list from the surface (overwrite path pixels with transparent).
    /// Used so moving strokes **sweep** instead of full-scene redraw.
    pub fn erase(&self, surface: &mut Surface) {
        self.run(surface, true);
    }

    /// Sweep: erase `previous` beam path, then stroke this list.
    /// No full clear — only the vectors that moved are updated.
    pub fn sweep(&self, surface: &mut Surface, previous: Option<&DisplayList>) {
        if let Some(prev) = previous {
            if !prev.is_empty() {
                prev.erase(surface);
            }
        }
        self.stroke(surface);
    }

    /// Full rebuild: transparent clear + stroke (static scenes / first frame).
    pub fn refresh(&self, surface: &mut Surface) {
        surface.clear_transparent();
        self.stroke(surface);
    }

    fn run(&self, surface: &mut Surface, erase: bool) {
        let mut beam = (0i32, 0i32);
        let mut color = GREEN;
        let mut width = 1i32;
        for cmd in &self.cmds {
            match *cmd {
                Stroke::Color(c) => {
                    if !erase {
                        color = c;
                    }
                }
                Stroke::Width(w) => width = w.max(1),
                Stroke::MoveTo { x, y } => beam = (x, y),
                Stroke::LineTo { x, y } => {
                    draw_seg(surface, beam.0, beam.1, x, y, color, width, erase);
                    beam = (x, y);
                }
                Stroke::Line { x0, y0, x1, y1 } => {
                    draw_seg(surface, x0, y0, x1, y1, color, width, erase);
                    beam = (x1, y1);
                }
                Stroke::LineThick {
                    x0,
                    y0,
                    x1,
                    y1,
                    thickness,
                } => {
                    draw_seg(surface, x0, y0, x1, y1, color, thickness.max(1), erase);
                    beam = (x1, y1);
                }
                Stroke::Circle { cx, cy, r } => {
                    if erase {
                        // Clear AA fringe around the circle.
                        let clear_w = width.max(1) + 2;
                        for o in -clear_w..=clear_w {
                            let rr = r + o;
                            if rr > 0 {
                                surface.circle(cx, cy, rr, 0);
                            }
                        }
                    } else if width <= 1 {
                        surface.circle(cx, cy, r, color);
                    } else {
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
}

#[allow(clippy::too_many_arguments)]
fn draw_seg(
    surface: &mut Surface,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: Color,
    width: i32,
    erase: bool,
) {
    let w = width.max(1);
    if erase {
        // Solid clear of core + thick fringe so AA leftovers disappear.
        let fringe = (w + 2).max(3);
        surface.line_thick(x0, y0, x1, y1, 0, fringe);
        surface.line_fast(x0, y0, x1, y1, 0);
        return;
    }
    if w == 1 {
        // Crisp hairline: Xiaolin Wu (asm).
        surface.line_aa(x0, y0, x1, y1, color);
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
        let p0 = s.get(0, 0).unwrap();
        let p1 = s.get(10, 0).unwrap();
        assert_eq!(p0 & 0x00FF_FFFF, GREEN & 0x00FF_FFFF);
        assert_eq!(p1 & 0x00FF_FFFF, GREEN & 0x00FF_FFFF);
        assert!(alpha(p0) >= 200);
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

    #[test]
    fn sweep_erases_old_stroke() {
        let mut s = Surface::new(64, 64);
        let mut a = DisplayList::new();
        a.set_color(GREEN);
        a.line(0, 10, 40, 10);
        a.refresh(&mut s);
        assert!(alpha(s.get(20, 10).unwrap()) > 0);

        let mut b = DisplayList::new();
        b.set_color(GREEN);
        b.line(0, 30, 40, 30);
        b.sweep(&mut s, Some(&a));
        // Old y=10 mostly gone; new y=30 lit.
        assert_eq!(s.get(20, 10).map(alpha).unwrap_or(0), 0);
        assert!(alpha(s.get(20, 30).unwrap()) > 0);
    }
}
