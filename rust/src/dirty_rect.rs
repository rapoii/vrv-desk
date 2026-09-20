use serde::{Deserialize, Serialize};

/// Represents a single dirty rectangle in pixel coordinates (inclusive-exclusive or Win32 RECT style).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl DirtyRect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right: right.max(left),
            bottom: bottom.max(top),
        }
    }

    pub fn empty() -> Self {
        Self {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }

    pub fn width(&self) -> u32 {
        (self.right - self.left).max(0) as u32
    }

    pub fn height(&self) -> u32 {
        (self.bottom - self.top).max(0) as u32
    }

    pub fn area(&self) -> usize {
        self.width() as usize * self.height() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.width() == 0 || self.height() == 0
    }

    pub fn union(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    pub fn bounding_box(rects: &[Self]) -> Option<Self> {
        let non_empty: Vec<&Self> = rects.iter().filter(|r| !r.is_empty()).collect();
        if non_empty.is_empty() {
            return None;
        }

        let mut left = non_empty[0].left;
        let mut top = non_empty[0].top;
        let mut right = non_empty[0].right;
        let mut bottom = non_empty[0].bottom;

        for r in &non_empty[1..] {
            left = left.min(r.left);
            top = top.min(r.top);
            right = right.max(r.right);
            bottom = bottom.max(r.bottom);
        }

        Some(Self {
            left,
            top,
            right,
            bottom,
        })
    }

    pub fn total_area(rects: &[Self]) -> usize {
        rects.iter().map(|r| r.area()).sum()
    }
}

/// Metadata and delta metrics about changes in a captured frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirtyFrameInfo {
    pub screen_width: u32,
    pub screen_height: u32,
    pub dirty_rects: Vec<DirtyRect>,
}

impl DirtyFrameInfo {
    pub fn new(screen_width: u32, screen_height: u32, dirty_rects: Vec<DirtyRect>) -> Self {
        Self {
            screen_width,
            screen_height,
            dirty_rects,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.dirty_rects.is_empty() || self.dirty_rects.iter().all(|r| r.is_empty())
    }

    pub fn rect_count(&self) -> usize {
        self.dirty_rects.iter().filter(|r| !r.is_empty()).count()
    }

    pub fn total_dirty_area(&self) -> usize {
        DirtyRect::total_area(&self.dirty_rects)
    }

    pub fn bounding_box(&self) -> Option<DirtyRect> {
        DirtyRect::bounding_box(&self.dirty_rects)
    }

    pub fn dirty_ratio(&self) -> f32 {
        let total_pixels = (self.screen_width as usize) * (self.screen_height as usize);
        if total_pixels == 0 {
            return 0.0;
        }
        let dirty_pixels = self.total_dirty_area();
        (dirty_pixels as f32) / (total_pixels as f32)
    }
}
