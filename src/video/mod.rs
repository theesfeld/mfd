//! Sensor glass feeds: synthetic FLIR, PGM stills, Linux V4L2.

mod frame;
#[cfg(target_os = "linux")]
pub mod v4l2;

pub use frame::{blit_grey_flir, GreyFrame};

#[cfg(target_os = "linux")]
pub use v4l2::V4l2Source;

/// Resolve greyscale frame for FLIR page.
///
/// Order: live camera (if open) → `MFD_FLIR_PATH` PGM → synthetic.
pub fn resolve_frame(t: f32, w: u32, h: u32, cam: Option<&mut dyn CameraGrab>) -> GreyFrame {
    if let Some(c) = cam {
        if let Some(f) = c.grab_frame() {
            return f;
        }
    }
    GreyFrame::resolve(t, w, h)
}

/// Trait so demo can pass V4L2 or mocks.
pub trait CameraGrab {
    fn grab_frame(&mut self) -> Option<GreyFrame>;
}

#[cfg(target_os = "linux")]
impl CameraGrab for V4l2Source {
    fn grab_frame(&mut self) -> Option<GreyFrame> {
        self.grab().cloned()
    }
}
