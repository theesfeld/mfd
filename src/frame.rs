//! Frame pacing for **smooth** motion.
//!
//! High FPS with uneven frame times looks choppy. Smooth motion needs
//! **even frame periods**, not maximum throughput.
//!
//! This pacer phase-locks to absolute deadlines:
//! `t_n = t0 + n * period`. If a frame overruns, it skips ahead so the
//! next deadline stays on the grid (no cascading lag).

use std::thread;
use std::time::{Duration, Instant};

/// Fixed-rate display clock with absolute deadlines.
pub struct FramePacer {
    period: Duration,
    /// Anchor for absolute phase.
    t0: Instant,
    /// Next frame index on the phase grid.
    frame_i: u64,
    last_report: Instant,
    frames_in_window: u32,
    /// Displayed FPS (completed presents / wall second).
    pub fps: f32,
    /// Mean frame time in the last report window (µs).
    pub mean_us: u32,
    /// Max frame time in the last report window (µs) — jitter peak.
    pub max_us: u32,
    /// Running max within window.
    win_max_us: u32,
    win_sum_us: u64,
}

impl FramePacer {
    /// Lock display rate to `target_hz` (e.g. 60, 120, 144). Clamped 1..=500.
    pub fn new(target_hz: u32) -> Self {
        let hz = target_hz.clamp(1, 500);
        let period = Duration::from_secs_f64(1.0 / f64::from(hz));
        let now = Instant::now();
        Self {
            period,
            t0: now,
            frame_i: 0,
            last_report: now,
            frames_in_window: 0,
            fps: hz as f32,
            mean_us: period.as_micros() as u32,
            max_us: 0,
            win_max_us: 0,
            win_sum_us: 0,
        }
    }

    pub fn period(&self) -> Duration {
        self.period
    }

    pub fn target_hz(&self) -> f32 {
        1.0 / self.period.as_secs_f32()
    }

    /// Wall-clock seconds since pacer start — use this for animation, not frame index.
    pub fn elapsed_secs(&self) -> f32 {
        self.t0.elapsed().as_secs_f32()
    }

    /// Wait until the next absolute deadline, then return.
    /// Call once per displayed frame (after present).
    pub fn wait_next(&mut self) {
        let frame_start = Instant::now();
        self.frame_i += 1;
        // Absolute phase: t0 + n·period (f64 avoids u32 overflow for long runs).
        let deadline =
            self.t0 + Duration::from_secs_f64(self.period.as_secs_f64() * self.frame_i as f64);

        let now = Instant::now();
        if now < deadline {
            let remain = deadline.saturating_duration_since(now);
            // Sleep most of the slack; spin the last ~150µs for tighter phase.
            if remain > Duration::from_micros(150) {
                thread::sleep(remain - Duration::from_micros(150));
            }
            while Instant::now() < deadline {
                std::hint::spin_loop();
            }
        } else {
            // Overran: jump phase so we do not dig a deeper hole.
            let late = now.duration_since(self.t0);
            let periods = (late.as_secs_f64() / self.period.as_secs_f64()).ceil() as u64;
            if periods > self.frame_i {
                self.frame_i = periods;
            }
        }

        let ft = frame_start.elapsed();
        let us = ft.as_micros().min(u128::from(u32::MAX)) as u32;
        self.win_sum_us += u64::from(us);
        if us > self.win_max_us {
            self.win_max_us = us;
        }
        self.frames_in_window += 1;

        if self.last_report.elapsed() >= Duration::from_millis(500) {
            let n = self.frames_in_window.max(1);
            let secs = self.last_report.elapsed().as_secs_f32().max(0.001);
            self.fps = n as f32 / secs;
            self.mean_us = (self.win_sum_us / u64::from(n)) as u32;
            self.max_us = self.win_max_us;
            self.frames_in_window = 0;
            self.win_sum_us = 0;
            self.win_max_us = 0;
            self.last_report = Instant::now();
        }
    }
}
