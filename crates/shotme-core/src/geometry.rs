//! Hình học vùng chọn: khung chọn, tay nắm, vị trí thanh công cụ.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn from_points(a: Point, b: Point) -> Rect {
        Rect {
            x: a.x.min(b.x),
            y: a.y.min(b.y),
            w: (a.x - b.x).abs(),
            h: (a.y - b.y).abs(),
        }
    }

    /// Cạnh phải và cạnh dưới không tính.
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Outside,
    Inside,
    Handle(Handle),
}

#[derive(Clone, Copy, Debug)]
enum Drag {
    New { anchor: Point },
    Move { start: Point, orig: Rect },
    Resize { handle: Handle, orig: Rect },
}

pub struct Selection {
    bounds: Rect,
    rect: Option<Rect>,
    drag: Option<Drag>,
}

const MIN_SIZE: i32 = 3;

fn clamp_point(p: Point, b: Rect) -> Point {
    Point {
        x: p.x.clamp(b.x, b.right()),
        y: p.y.clamp(b.y, b.bottom()),
    }
}

impl Selection {
    pub fn new(bounds: Rect) -> Self {
        Selection {
            bounds,
            rect: None,
            drag: None,
        }
    }

    pub fn rect(&self) -> Option<Rect> {
        self.rect
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub fn hit_test(&self, p: Point, handle_radius: i32) -> Hit {
        let Some(r) = self.rect else {
            return Hit::Outside;
        };
        let rad = handle_radius;
        let near = |v: i32, edge: i32| (v - edge).abs() <= rad;
        let in_x = p.x >= r.x - rad && p.x <= r.right() + rad;
        let in_y = p.y >= r.y - rad && p.y <= r.bottom() + rad;
        if in_x && in_y {
            let w = near(p.x, r.x);
            let e = near(p.x, r.right());
            let n = near(p.y, r.y);
            let s = near(p.y, r.bottom());
            let h = match (n, s, w, e) {
                (true, _, true, _) => Some(Handle::NW),
                (true, _, _, true) => Some(Handle::NE),
                (_, true, true, _) => Some(Handle::SW),
                (_, true, _, true) => Some(Handle::SE),
                (true, ..) => Some(Handle::N),
                (_, true, ..) => Some(Handle::S),
                (_, _, true, _) => Some(Handle::W),
                (_, _, _, true) => Some(Handle::E),
                _ => None,
            };
            if let Some(h) = h {
                return Hit::Handle(h);
            }
        }
        if r.contains(p) {
            Hit::Inside
        } else {
            Hit::Outside
        }
    }

    pub fn press(&mut self, p: Point, handle_radius: i32) {
        let p = clamp_point(p, self.bounds);
        self.drag = Some(match (self.hit_test(p, handle_radius), self.rect) {
            (Hit::Handle(handle), Some(orig)) => Drag::Resize { handle, orig },
            (Hit::Inside, Some(orig)) => Drag::Move { start: p, orig },
            _ => {
                self.rect = Some(Rect::from_points(p, p));
                Drag::New { anchor: p }
            }
        });
    }

    pub fn drag_to(&mut self, p: Point) {
        let Some(drag) = self.drag else { return };
        let b = self.bounds;
        let p = clamp_point(p, b);
        self.rect = Some(match drag {
            Drag::New { anchor } => Rect::from_points(anchor, p),
            Drag::Move { start, orig } => {
                let max_x = (b.right() - orig.w).max(b.x);
                let max_y = (b.bottom() - orig.h).max(b.y);
                Rect {
                    x: (orig.x + p.x - start.x).clamp(b.x, max_x),
                    y: (orig.y + p.y - start.y).clamp(b.y, max_y),
                    ..orig
                }
            }
            Drag::Resize { handle, orig } => {
                let (mut l, mut t, mut r, mut bt) = (orig.x, orig.y, orig.right(), orig.bottom());
                use Handle::*;
                if matches!(handle, W | NW | SW) {
                    l = p.x;
                }
                if matches!(handle, E | NE | SE) {
                    r = p.x;
                }
                if matches!(handle, N | NW | NE) {
                    t = p.y;
                }
                if matches!(handle, S | SW | SE) {
                    bt = p.y;
                }
                Rect::from_points(Point { x: l, y: t }, Point { x: r, y: bt })
            }
        });
    }

    pub fn release(&mut self) {
        self.drag = None;
        if let Some(r) = self.rect {
            if r.w < MIN_SIZE || r.h < MIN_SIZE {
                self.rect = None;
            }
        }
    }
}

/// Màn hình chứa `p`; nếu không có thì màn hình gần nhất.
pub fn monitor_containing(p: Point, monitors: &[Rect]) -> Rect {
    let dist = |m: &Rect| -> i64 {
        let dx = (m.x - p.x).max(p.x - (m.right() - 1)).max(0) as i64;
        let dy = (m.y - p.y).max(p.y - (m.bottom() - 1)).max(0) as i64;
        dx * dx + dy * dy
    };
    monitors
        .iter()
        .min_by_key(|m| dist(m))
        .copied()
        .unwrap_or(Rect { x: 0, y: 0, w: 0, h: 0 })
}

pub fn place_toolbar(sel: Rect, size: (i32, i32), gap: i32, monitor: Rect) -> Rect {
    let (w, h) = size;
    let mut x = sel.right() - w;
    let below = sel.bottom() + gap;
    let above = sel.y - gap - h;
    let mut y = if below + h <= monitor.bottom() {
        below
    } else if above >= monitor.y {
        above
    } else {
        x = sel.right() - gap - w;
        sel.bottom() - gap - h
    };
    x = x.min(monitor.right() - w).max(monitor.x);
    y = y.min(monitor.bottom() - h).max(monitor.y);
    Rect { x, y, w, h }
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: Rect = Rect { x: 0, y: 0, w: 1000, h: 800 };
    fn p(x: i32, y: i32) -> Point { Point { x, y } }
    fn sel(x: i32, y: i32, w: i32, h: i32) -> Selection {
        let mut s = Selection::new(B);
        s.press(p(x, y), 6);
        s.drag_to(p(x + w, y + h));
        s.release();
        s
    }

    #[test] fn new_selection_from_drag() {
        let mut s = Selection::new(B);
        s.press(p(300, 200), 6); s.drag_to(p(100, 50)); s.release();
        assert_eq!(s.rect(), Some(Rect { x: 100, y: 50, w: 200, h: 150 }));
    }
    #[test] fn tiny_selection_is_discarded() {
        let mut s = Selection::new(B);
        s.press(p(10, 10), 6); s.drag_to(p(12, 12)); s.release();
        assert_eq!(s.rect(), None);
    }
    #[test] fn drag_is_clamped_to_bounds() {
        let mut s = Selection::new(B);
        s.press(p(900, 700), 6); s.drag_to(p(5000, -40)); s.release();
        assert_eq!(s.rect(), Some(Rect { x: 900, y: 0, w: 100, h: 700 }));
    }
    #[test] fn move_keeps_size_and_stays_inside() {
        let mut s = sel(100, 100, 200, 100);
        s.press(p(150, 150), 6); s.drag_to(p(2000, 2000)); s.release();
        assert_eq!(s.rect(), Some(Rect { x: 800, y: 700, w: 200, h: 100 }));
    }
    #[test] fn resize_east_handle_and_flip() {
        let mut s = sel(100, 100, 200, 100);
        assert_eq!(s.hit_test(p(300, 150), 6), Hit::Handle(Handle::E));
        s.press(p(300, 150), 6); s.drag_to(p(40, 150)); s.release();
        assert_eq!(s.rect(), Some(Rect { x: 40, y: 100, w: 60, h: 100 }));
    }
    #[test] fn press_outside_starts_new_selection() {
        let mut s = sel(100, 100, 200, 100);
        s.press(p(500, 500), 6); s.drag_to(p(600, 650)); s.release();
        assert_eq!(s.rect(), Some(Rect { x: 500, y: 500, w: 100, h: 150 }));
    }
    #[test] fn hit_test_corners_inside_outside() {
        let s = sel(100, 100, 200, 100);
        assert_eq!(s.hit_test(p(100, 100), 6), Hit::Handle(Handle::NW));
        assert_eq!(s.hit_test(p(200, 150), 6), Hit::Inside);
        assert_eq!(s.hit_test(p(50, 50), 6), Hit::Outside);
    }
    #[test] fn toolbar_below_right_aligned() {
        assert_eq!(place_toolbar(Rect { x: 100, y: 100, w: 300, h: 200 }, (160, 40), 6, B),
                   Rect { x: 240, y: 306, w: 160, h: 40 });
    }
    #[test] fn toolbar_above_when_no_room_below() {
        assert_eq!(place_toolbar(Rect { x: 100, y: 500, w: 300, h: 290 }, (160, 40), 6, B),
                   Rect { x: 240, y: 454, w: 160, h: 40 });
    }
    #[test] fn toolbar_inside_when_selection_fills_monitor() {
        assert_eq!(place_toolbar(B, (160, 40), 6, B), Rect { x: 834, y: 754, w: 160, h: 40 });
    }
    #[test] fn toolbar_clamped_horizontally_for_narrow_selection() {
        let r = place_toolbar(Rect { x: 10, y: 100, w: 50, h: 50 }, (160, 40), 6, B);
        assert_eq!(r, Rect { x: 0, y: 156, w: 160, h: 40 });
    }
    #[test] fn toolbar_stays_in_second_monitor_with_offset_origin() {
        let m2 = Rect { x: 1000, y: 200, w: 800, h: 600 };
        let r = place_toolbar(Rect { x: 1100, y: 600, w: 300, h: 190 }, (160, 40), 6, m2);
        assert_eq!(r, Rect { x: 1240, y: 554, w: 160, h: 40 });
    }
    #[test] fn monitor_containing_picks_nearest_when_in_gap() {
        let ms = [Rect { x: 0, y: 0, w: 1000, h: 800 }, Rect { x: 1000, y: 200, w: 800, h: 600 }];
        assert_eq!(monitor_containing(p(1500, 300), &ms), ms[1]);
        assert_eq!(monitor_containing(p(1500, 50), &ms), ms[1]);
    }
}
