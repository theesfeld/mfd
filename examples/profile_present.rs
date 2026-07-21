use std::io::Write;
use std::time::Instant;
use vge::term::{present_at, surface_size_for_viewport, TermBackend, Viewport};
use vge::{Surface, BLACK, GREEN};
fn main() {
    let mut err = std::io::stderr();
    for (name, b) in [
        ("ascii", TermBackend::Ascii),
        ("half", TermBackend::HalfBlock),
        ("kitty", TermBackend::Kitty),
    ] {
        let vp = Viewport {
            col: 0,
            row: 0,
            cols: 80,
            rows: 24,
        };
        let (w, h) = surface_size_for_viewport(b, vp);
        let mut s = Surface::new(w, h);
        s.clear(BLACK);
        s.line(0, 0, w as i32 - 1, h as i32 - 1, GREEN);
        let n = 60u32;
        let t0 = Instant::now();
        for _ in 0..n {
            let _ = present_at(&s, b, vp);
        }
        let d = t0.elapsed();
        let _ = writeln!(
            err,
            "{name:5} {w}x{h} present => {:.0} fps ({d:?})",
            n as f64 / d.as_secs_f64()
        );
    }
}
