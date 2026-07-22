//! Calligraphic **stroke display list** — beam commands are the picture.
//!
//! ## Update models
//!
//! | Model | When to use | Cost |
//! |-------|-------------|------|
//! | **refresh** | Full scene rebuild each frame | clear + 1× stroke |
//! | **sweep** | Few vectors move; rest stay | erase old path + draw new |
//! | **lifespan** | CRT/radar trails; per-stroke TTL | tick + clear + stroke living |
//!
//! Lifespan is a **library option**: each command has `born` + `ttl` frames
//! (`ttl == 0` = immortal). Hairlines use crisp Xiaolin Wu AA (asm).

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

/// Beam command with optional lifespan (frames).
///
/// `ttl == 0` means immortal until the list is cleared.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimedStroke {
    pub cmd: Stroke,
    /// Frame clock when this command was appended.
    pub born: u64,
    /// Frames to live after birth. `0` = immortal.
    pub ttl: u32,
}

impl TimedStroke {
    /// True if this command is still active at `clock`.
    #[inline]
    pub fn is_alive(&self, clock: u64) -> bool {
        if self.ttl == 0 {
            return true;
        }
        clock.saturating_sub(self.born) < self.ttl as u64
    }

    /// Remaining life as a fraction in `0.0..=1.0` (immortal → `1.0`).
    #[inline]
    pub fn life_frac(&self, clock: u64) -> f32 {
        if self.ttl == 0 {
            return 1.0;
        }
        let age = clock.saturating_sub(self.born);
        if age >= self.ttl as u64 {
            return 0.0;
        }
        let remaining = self.ttl as u64 - age;
        (remaining as f32) / (self.ttl as f32)
    }
}

/// Live stroke display list.
#[derive(Debug, Clone)]
pub struct DisplayList {
    cmds: Vec<TimedStroke>,
    beam: (i32, i32),
    color: Color,
    /// Current stroke width in pixels (default 1).
    width: i32,
    /// Frame clock for lifespan. Advances on [`Self::tick`].
    clock: u64,
    /// Default TTL for new commands (`0` = immortal).
    lifespan: u32,
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
            clock: 0,
            lifespan: 0,
        }
    }

    pub fn with_capacity(n: usize) -> Self {
        Self {
            cmds: Vec::with_capacity(n),
            beam: (0, 0),
            color: GREEN,
            width: 1,
            clock: 0,
            lifespan: 0,
        }
    }

    pub fn clear(&mut self) {
        self.cmds.clear();
        self.beam = (0, 0);
        self.color = GREEN;
        self.width = 1;
        // Keep clock so trail layers that share time stay coherent.
        // Callers that want a full reset can use [`Self::reset_clock`].
    }

    /// Reset the lifespan frame clock to zero.
    pub fn reset_clock(&mut self) {
        self.clock = 0;
    }

    pub fn len(&self) -> usize {
        self.cmds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }

    /// All recorded commands (including expired until the next [`Self::tick`]).
    pub fn timed(&self) -> &[TimedStroke] {
        &self.cmds
    }

    /// Beam commands only (ignores birth/ttl metadata).
    pub fn commands(&self) -> impl Iterator<Item = Stroke> + '_ {
        self.cmds.iter().map(|t| t.cmd)
    }

    /// Current frame clock.
    pub fn clock(&self) -> u64 {
        self.clock
    }

    /// Default lifespan for new commands (`0` = immortal).
    pub fn lifespan(&self) -> u32 {
        self.lifespan
    }

    /// Set default lifespan for commands appended after this call.
    ///
    /// `frames == 0` → immortal. `frames == 1` → live for one tick after birth
    /// (crisp single-frame marks). Larger values leave intentional trails.
    pub fn set_lifespan(&mut self, frames: u32) {
        self.lifespan = frames;
    }

    fn push_cmd(&mut self, cmd: Stroke) {
        self.cmds.push(TimedStroke {
            cmd,
            born: self.clock,
            ttl: self.lifespan,
        });
    }

    /// Advance one frame and drop expired strokes.
    ///
    /// Returns the number of commands removed.
    pub fn tick(&mut self) -> usize {
        self.clock = self.clock.saturating_add(1);
        let clock = self.clock;
        let before = self.cmds.len();
        self.cmds.retain(|t| t.is_alive(clock));
        before - self.cmds.len()
    }

    /// Count of commands still alive at the current clock.
    pub fn living_len(&self) -> usize {
        let clock = self.clock;
        self.cmds.iter().filter(|t| t.is_alive(clock)).count()
    }

    pub fn set_color(&mut self, c: Color) {
        // Force opaque if caller passed RGB-only (alpha 0 but non-zero rgb).
        let c = if alpha(c) == 0 && c != 0 {
            c | 0xFF00_0000
        } else {
            c
        };
        self.color = c;
        self.push_cmd(Stroke::Color(c));
    }

    /// Stroke width in pixels. `1` is a hairline. No artificial maximum.
    pub fn set_width(&mut self, px: i32) {
        let w = px.max(1);
        self.width = w;
        self.push_cmd(Stroke::Width(w));
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        self.beam = (x, y);
        self.push_cmd(Stroke::MoveTo { x, y });
    }

    pub fn line_to(&mut self, x: i32, y: i32) {
        self.push_cmd(Stroke::LineTo { x, y });
        self.beam = (x, y);
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.push_cmd(Stroke::Line { x0, y0, x1, y1 });
        self.beam = (x1, y1);
    }

    pub fn line_thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, thickness: i32) {
        self.push_cmd(Stroke::LineThick {
            x0,
            y0,
            x1,
            y1,
            thickness: thickness.max(1),
        });
        self.beam = (x1, y1);
    }

    pub fn circle(&mut self, cx: i32, cy: i32, r: i32) {
        self.push_cmd(Stroke::Circle { cx, cy, r });
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

    /// Execute the beam (draw all living commands at full opacity).
    /// Does not clear the surface.
    pub fn stroke(&self, surface: &mut Surface) {
        self.run(surface, false, false);
    }

    /// Execute living commands. If `fade` is true, scale color alpha by
    /// remaining life (trail). Immortal strokes stay full strength.
    pub fn stroke_life(&self, surface: &mut Surface, fade: bool) {
        self.run(surface, false, fade);
    }

    /// Erase this list from the surface (overwrite path pixels with transparent).
    /// Used so moving strokes **sweep** instead of full-scene redraw.
    pub fn erase(&self, surface: &mut Surface) {
        self.run(surface, true, false);
    }

    /// Sweep: erase `previous` beam path, then stroke this list.
    /// No full clear — only the vectors that moved are updated.
    ///
    /// Prefer [`Self::refresh`] when the whole scene is rebuilt each frame
    /// (full-list sweep is about 2× beam work).
    pub fn sweep(&self, surface: &mut Surface, previous: Option<&DisplayList>) {
        if let Some(prev) = previous {
            if !prev.is_empty() {
                prev.erase(surface);
            }
        }
        self.stroke(surface);
    }

    /// Full rebuild: transparent clear + stroke living commands (no fade).
    pub fn refresh(&self, surface: &mut Surface) {
        surface.clear_transparent();
        self.stroke(surface);
    }

    /// Transparent clear + stroke living commands.
    ///
    /// With `fade == true`, alpha falls with remaining life (CRT trail).
    pub fn refresh_life(&self, surface: &mut Surface, fade: bool) {
        surface.clear_transparent();
        self.stroke_life(surface, fade);
    }

    fn run(&self, surface: &mut Surface, erase: bool, fade: bool) {
        let clock = self.clock;
        let mut beam = (0i32, 0i32);
        let mut color = GREEN;
        let mut width = 1i32;
        for timed in &self.cmds {
            if !timed.is_alive(clock) {
                continue;
            }
            // Fade geometry only. Keep Color state at full strength so we do
            // not multiply alpha twice (state × segment).
            let frac = if fade && !erase {
                timed.life_frac(clock)
            } else {
                1.0
            };
            match timed.cmd {
                Stroke::Color(c) => {
                    if !erase {
                        color = c;
                    }
                }
                Stroke::Width(w) => width = w.max(1),
                Stroke::MoveTo { x, y } => beam = (x, y),
                Stroke::LineTo { x, y } => {
                    let col = if erase {
                        0
                    } else if fade {
                        scale_alpha(color, frac)
                    } else {
                        color
                    };
                    draw_seg(surface, beam.0, beam.1, x, y, col, width, erase);
                    beam = (x, y);
                }
                Stroke::Line { x0, y0, x1, y1 } => {
                    let col = if erase {
                        0
                    } else if fade {
                        scale_alpha(color, frac)
                    } else {
                        color
                    };
                    draw_seg(surface, x0, y0, x1, y1, col, width, erase);
                    beam = (x1, y1);
                }
                Stroke::LineThick {
                    x0,
                    y0,
                    x1,
                    y1,
                    thickness,
                } => {
                    let col = if erase {
                        0
                    } else if fade {
                        scale_alpha(color, frac)
                    } else {
                        color
                    };
                    draw_seg(surface, x0, y0, x1, y1, col, thickness.max(1), erase);
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
                    } else {
                        let col = if fade {
                            scale_alpha(color, frac)
                        } else {
                            color
                        };
                        if width <= 1 {
                            surface.circle(cx, cy, r, col);
                        } else {
                            let half = width / 2;
                            for o in -half..=half {
                                let rr = r + o;
                                if rr > 0 {
                                    surface.circle(cx, cy, rr, col);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Scale the alpha channel of a packed `0xAARRGGBB` color by `frac` in `0..=1`.
#[inline]
fn scale_alpha(c: Color, frac: f32) -> Color {
    if frac >= 1.0 {
        return c;
    }
    if frac <= 0.0 {
        return 0;
    }
    let a = (alpha(c) as f32 * frac).round() as u32;
    (c & 0x00FF_FFFF) | (a.clamp(0, 255) << 24)
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

    #[test]
    fn lifespan_expires_after_ttl() {
        let mut list = DisplayList::new();
        list.set_lifespan(2);
        list.set_color(GREEN);
        list.line(0, 0, 20, 0);
        assert_eq!(list.len(), 2);
        assert_eq!(list.living_len(), 2);

        // born at clock 0, ttl 2 → alive at 0 and 1, dead at 2.
        assert_eq!(list.tick(), 0); // clock=1, still alive
        assert_eq!(list.living_len(), 2);
        assert_eq!(list.tick(), 2); // clock=2, expired
        assert_eq!(list.len(), 0);
        assert_eq!(list.living_len(), 0);
    }

    #[test]
    fn immortal_survives_tick() {
        let mut list = DisplayList::new();
        list.set_lifespan(0);
        list.set_color(GREEN);
        list.line(0, 0, 5, 0);
        list.tick();
        list.tick();
        assert_eq!(list.len(), 2);
        assert_eq!(list.living_len(), 2);
    }

    #[test]
    fn life_frac_fades() {
        let t = TimedStroke {
            cmd: Stroke::Line {
                x0: 0,
                y0: 0,
                x1: 1,
                y1: 0,
            },
            born: 0,
            ttl: 4,
        };
        assert!((t.life_frac(0) - 1.0).abs() < 1e-5);
        assert!((t.life_frac(2) - 0.5).abs() < 1e-5);
        assert_eq!(t.life_frac(4), 0.0);
    }

    #[test]
    fn refresh_life_fades_alpha() {
        let mut list = DisplayList::new();
        list.set_lifespan(4);
        list.set_color(GREEN);
        list.line(0, 5, 30, 5);
        // Age halfway: remaining/ttl = 2/4 = 0.5 after two ticks?
        // born=0, ttl=4; after tick clock=1 age=1 remaining=3; after 2 ticks clock=2 age=2 remaining=2.
        list.tick();
        list.tick();
        let mut s = Surface::new(40, 16);
        list.refresh_life(&mut s, true);
        let p = s.get(15, 5).unwrap();
        let a = alpha(p);
        // Full GREEN alpha is 255; ~50% → around 128. Allow AA fringe variance.
        assert!(a > 40 && a < 200, "expected faded alpha, got {a}");
    }
}
