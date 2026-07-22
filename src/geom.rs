//! Shared layout types for widgets and pages.

/// Integer rectangle (x, y, w, h) in surface pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(self) -> i32 {
        self.y + self.h
    }

    pub fn center(self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    pub fn inset(self, m: i32) -> Self {
        Self {
            x: self.x + m,
            y: self.y + m,
            w: (self.w - 2 * m).max(0),
            h: (self.h - 2 * m).max(0),
        }
    }
}
