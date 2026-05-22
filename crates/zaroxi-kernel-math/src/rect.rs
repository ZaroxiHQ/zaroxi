#![allow(dead_code)]
use crate::vec2::Vec2;
use crate::size::Size;

/// Axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Create a new Rect from components.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Create a rect from minimum corner and size.
    pub const fn from_min_size(origin: Vec2, size: Size) -> Self {
        Self {
            x: origin.x,
            y: origin.y,
            width: size.width,
            height: size.height,
        }
    }

    /// Minimum corner (x,y).
    pub const fn min(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Maximum corner (x+width, y+height).
    pub const fn max(self) -> Vec2 {
        Vec2::new(self.x + self.width, self.y + self.height)
    }

    /// Center point of the rect.
    pub fn center(self) -> Vec2 {
        Vec2::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    /// Contains check: inclusive on min edge, exclusive on max edge.
    pub fn contains(self, p: Vec2) -> bool {
        let x0 = self.x;
        let y0 = self.y;
        let x1 = self.x + self.width;
        let y1 = self.y + self.height;
        (p.x >= x0) && (p.x < x1) && (p.y >= y0) && (p.y < y1)
    }

    /// Intersects other rect (standard AABB overlap).
    pub fn intersects(self, other: Rect) -> bool {
        let a_left = self.x;
        let a_right = self.x + self.width;
        let a_top = self.y;
        let a_bottom = self.y + self.height;

        let b_left = other.x;
        let b_right = other.x + other.width;
        let b_top = other.y;
        let b_bottom = other.y + other.height;

        !(a_right <= b_left || b_right <= a_left || a_bottom <= b_top || b_bottom <= a_top)
    }

    /// Translate by vector.
    pub fn translate(self, v: Vec2) -> Self {
        Self {
            x: self.x + v.x,
            y: self.y + v.y,
            width: self.width,
            height: self.height,
        }
    }

    /// Inflate rect by amount on all sides.
    pub fn inflate(self, amount: f32) -> Self {
        Self {
            x: self.x - amount,
            y: self.y - amount,
            width: self.width + amount * 2.0,
            height: self.height + amount * 2.0,
        }
    }

    /// Deflate rect by amount on all sides.
    pub fn deflate(self, amount: f32) -> Self {
        self.inflate(-amount)
    }

    /// Split the rect from the top by `height`. Returns (top, remainder).
    /// Clamps height between 0 and self.height.
    pub fn split_top(self, height: f32) -> (Rect, Rect) {
        let h = if height < 0.0 {
            0.0
        } else if height > self.height {
            self.height
        } else {
            height
        };

        let top = Rect::new(self.x, self.y, self.width, h);
        let bottom = Rect::new(self.x, self.y + h, self.width, self.height - h);
        (top, bottom)
    }

    /// Split the rect from the bottom by `height`. Returns (remainder, bottom).
    /// Clamps height between 0 and self.height.
    pub fn split_bottom(self, height: f32) -> (Rect, Rect) {
        let h = if height < 0.0 {
            0.0
        } else if height > self.height {
            self.height
        } else {
            height
        };

        let bottom = Rect::new(self.x, self.y + (self.height - h), self.width, h);
        let top = Rect::new(self.x, self.y, self.width, self.height - h);
        (top, bottom)
    }

    /// Split the rect from the left by `width`. Returns (left, remainder).
    /// Clamps width between 0 and self.width.
    pub fn split_left(self, width: f32) -> (Rect, Rect) {
        let w = if width < 0.0 {
            0.0
        } else if width > self.width {
            self.width
        } else {
            width
        };

        let left = Rect::new(self.x, self.y, w, self.height);
        let right = Rect::new(self.x + w, self.y, self.width - w, self.height);
        (left, right)
    }

    /// Split the rect from the right by `width`. Returns (remainder, right).
    /// Clamps width between 0 and self.width.
    pub fn split_right(self, width: f32) -> (Rect, Rect) {
        let w = if width < 0.0 {
            0.0
        } else if width > self.width {
            self.width
        } else {
            width
        };

        let right = Rect::new(self.x + (self.width - w), self.y, w, self.height);
        let left = Rect::new(self.x, self.y, self.width - w, self.height);
        (left, right)
    }
}

pub use Rect;
