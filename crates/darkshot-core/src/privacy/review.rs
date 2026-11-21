//! Mô hình thuần của cửa sổ Duyệt (Privacy Review): danh sách vùng che, chọn/bỏ chọn, vùng tự thêm,
//! xác nhận và kết quả trả cho cổng. Không phụ thuộc hệ điều hành; không mang chữ OCR.
//!
//! Mỗi khung của một phát hiện là một `Redaction` trong `items()`; mọi khung của cùng một phát
//! hiện (ví dụ khóa PEM nhiều dòng) gom thành MỘT mục danh sách (`rows()`), bật/tắt cùng nhau.

use super::model::{Confidence, Finding, Redaction, RedactionPlan, RedactionSource, ScanError};
use crate::geometry::Rect;

/// Cạnh nhỏ nhất (px ảnh) của một vùng tự thêm.
pub const MIN_MANUAL: i32 = 4;

/// Trạng thái của cửa sổ Duyệt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewPhase {
    /// Đang quét; `tile` = (dải hiện tại tính từ 1, tổng số dải) khi đã biết.
    Scanning {
        tile: Option<(u32, u32)>,
    },
    Findings,
    Error(ScanError),
}

/// Thanh xác nhận đang hiện trong cửa sổ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirm {
    None,
    /// "Che và tiếp tục" khi không mục nào được chọn.
    EmptySelection,
    /// "Tiếp tục không quét" sau lỗi quét.
    Unscanned,
}

/// Kết quả người dùng chọn, trả cho cổng.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewOutcome {
    /// Che các vùng đã chọn rồi chạy hành động (plan không rỗng).
    Apply(RedactionPlan),
    /// Có phát hiện, người dùng xác nhận không che gì.
    ContinueNone,
    /// Quét lỗi, người dùng xác nhận hai lần; plan = vùng đang chọn (có thể rỗng).
    ContinueUnscanned(RedactionPlan),
    /// Quét lại cùng ảnh; cửa sổ ở lại, về `Scanning`.
    Retry,
    Cancel,
}

/// Kéo vẽ vùng tự thêm (px ảnh): điểm bắt đầu và điểm hiện tại.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drag {
    pub start: (i32, i32),
    pub cur: (i32, i32),
}

impl Drag {
    /// Khung chuẩn hóa (w, h ≥ 0) giữa hai điểm.
    pub fn rect(&self) -> Rect {
        let (x0, x1) = (self.start.0.min(self.cur.0), self.start.0.max(self.cur.0));
        let (y0, y1) = (self.start.1.min(self.cur.1), self.start.1.max(self.cur.1));
        Rect {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewModel {
    items: Vec<Redaction>,
    /// Độ tin cậy theo mã phát hiện (`Redaction` không mang trường này).
    conf: Vec<(u32, Confidence)>,
    phase: ReviewPhase,
    confirm: Confirm,
    image_w: u32,
    image_h: u32,
    /// Mục danh sách (chỉ số `rows()`) đang trỏ bằng chuột (trên ảnh hay trên danh sách).
    pub hover: Option<usize>,
    /// Mục danh sách đang chọn bằng bàn phím.
    pub cursor: Option<usize>,
    /// Chế độ "Thêm vùng che" đang bật.
    adding: bool,
    drag: Option<Drag>,
}

fn detected(findings: &[Finding]) -> Vec<Redaction> {
    let mut out = Vec::new();
    for f in findings {
        for r in &f.rects {
            out.push(Redaction {
                rect: *r,
                source: RedactionSource::Detected {
                    finding: f.id,
                    category: f.category,
                },
                selected: true,
            });
        }
    }
    out
}

fn confidences(findings: &[Finding]) -> Vec<(u32, Confidence)> {
    findings.iter().map(|f| (f.id, f.confidence)).collect()
}

fn area(r: Rect) -> i64 {
    i64::from(r.w.max(0)) * i64::from(r.h.max(0))
}

impl ReviewModel {
    pub fn new(phase: ReviewPhase, findings: Vec<Finding>, image_w: u32, image_h: u32) -> Self {
        let (items, conf) = match phase {
            ReviewPhase::Findings => (detected(&findings), confidences(&findings)),
            _ => (Vec::new(), Vec::new()),
        };
        ReviewModel {
            items,
            conf,
            phase,
            confirm: Confirm::None,
            image_w,
            image_h,
            hover: None,
            cursor: None,
            adding: false,
            drag: None,
        }
    }

    pub fn phase(&self) -> ReviewPhase {
        self.phase
    }

    pub fn confirm(&self) -> Confirm {
        self.confirm
    }

    pub fn image_size(&self) -> (u32, u32) {
        (self.image_w, self.image_h)
    }

    /// Kết quả quét về: `Scanning` → `Findings`/`Error`. Vùng tự thêm được giữ (cuối danh sách).
    /// Ngoài `Scanning` thì bỏ qua (kết quả muộn).
    pub fn set_result(&mut self, r: Result<Vec<Finding>, ScanError>) {
        if !matches!(self.phase, ReviewPhase::Scanning { .. }) {
            return;
        }
        let manual: Vec<Redaction> = self
            .items
            .iter()
            .copied()
            .filter(|i| i.source == RedactionSource::Manual)
            .collect();
        match r {
            Ok(findings) => {
                self.items = detected(&findings);
                self.conf = confidences(&findings);
                self.phase = ReviewPhase::Findings;
            }
            Err(e) => {
                self.items.clear();
                self.conf.clear();
                self.phase = ReviewPhase::Error(e);
            }
        }
        self.items.extend(manual);
        self.confirm = Confirm::None;
        self.hover = None;
        self.cursor = self.cursor.filter(|&c| c < self.row_count());
    }

    /// Tiến độ quét (dải `tile` tính từ 1 trên `tiles`). Chỉ khi đang `Scanning`.
    pub fn set_progress(&mut self, tile: u32, tiles: u32) {
        if let ReviewPhase::Scanning { .. } = self.phase {
            let t = (tiles > 0).then(|| (tile.clamp(1, tiles), tiles));
            self.phase = ReviewPhase::Scanning { tile: t };
        }
    }

    /// Độ tin cậy của phát hiện `finding` (mã trong `RedactionSource::Detected`).
    pub fn confidence(&self, finding: u32) -> Option<Confidence> {
        self.conf.iter().find(|c| c.0 == finding).map(|c| c.1)
    }

    pub fn items(&self) -> &[Redaction] {
        &self.items
    }

    /// Các mục danh sách: mỗi mục là một khoảng liền trong `items()` (cùng một phát hiện, hay một
    /// vùng tự thêm).
    pub fn rows(&self) -> Vec<std::ops::Range<usize>> {
        let mut out: Vec<std::ops::Range<usize>> = Vec::new();
        for (i, it) in self.items.iter().enumerate() {
            let same = match (out.last(), it.source) {
                (Some(last), RedactionSource::Detected { finding, .. }) => matches!(
                    self.items[last.start].source,
                    RedactionSource::Detected { finding: f, .. } if f == finding
                ),
                _ => false,
            };
            if same {
                if let Some(last) = out.last_mut() {
                    last.end = i + 1;
                }
            } else {
                out.push(i..i + 1);
            }
        }
        out
    }

    pub fn row_count(&self) -> usize {
        self.rows().len()
    }

    /// Mục danh sách chứa khung `item`.
    pub fn row_of(&self, item: usize) -> Option<usize> {
        self.rows().iter().position(|r| r.contains(&item))
    }

    fn changed(&mut self) {
        // Mọi thay đổi lựa chọn bỏ thanh xác nhận: phải bấm lại nút chính.
        self.confirm = Confirm::None;
    }

    fn editable(&self) -> bool {
        !matches!(self.phase, ReviewPhase::Scanning { .. })
    }

    /// Bật/tắt khung `i` cùng mọi khung của cùng phát hiện.
    pub fn toggle(&mut self, i: usize) {
        if !self.editable() {
            return;
        }
        let Some(row) = self.row_of(i) else {
            return;
        };
        let range = self.rows()[row].clone();
        let on = !self.items[i].selected;
        for it in &mut self.items[range] {
            it.selected = on;
        }
        self.changed();
    }

    pub fn select_all(&mut self) {
        if !self.editable() {
            return;
        }
        self.items.iter_mut().for_each(|i| i.selected = true);
        self.changed();
    }

    pub fn deselect_all(&mut self) {
        if !self.editable() {
            return;
        }
        self.items.iter_mut().for_each(|i| i.selected = false);
        self.changed();
    }

    /// Thêm vùng tự thêm (đã chọn) cuối danh sách. Khung được chuẩn hóa và kẹp vào ảnh; cạnh nhỏ
    /// hơn `MIN_MANUAL` px (sau khi kẹp) thì bỏ, trả `false`.
    pub fn add_manual(&mut self, r: Rect) -> bool {
        if !self.editable() {
            return false;
        }
        let (x0, x1) = (r.x.min(r.right()), r.x.max(r.right()));
        let (y0, y1) = (r.y.min(r.bottom()), r.y.max(r.bottom()));
        let (iw, ih) = (
            i32::try_from(self.image_w).unwrap_or(i32::MAX),
            i32::try_from(self.image_h).unwrap_or(i32::MAX),
        );
        let (x0, x1) = (x0.clamp(0, iw), x1.clamp(0, iw));
        let (y0, y1) = (y0.clamp(0, ih), y1.clamp(0, ih));
        if x1 - x0 < MIN_MANUAL || y1 - y0 < MIN_MANUAL {
            return false;
        }
        self.items.push(Redaction {
            rect: Rect {
                x: x0,
                y: y0,
                w: x1 - x0,
                h: y1 - y0,
            },
            source: RedactionSource::Manual,
            selected: true,
        });
        self.changed();
        true
    }

    /// Xóa khung `i`, chỉ khi là vùng tự thêm.
    pub fn remove(&mut self, i: usize) -> bool {
        if !self.editable() {
            return false;
        }
        match self.items.get(i) {
            Some(it) if it.source == RedactionSource::Manual => {
                self.items.remove(i);
                let n = self.row_count();
                self.hover = None;
                self.cursor = self.cursor.and_then(|c| (n > 0).then(|| c.min(n - 1)));
                self.changed();
                true
            }
            _ => false,
        }
    }

    /// Khung chứa điểm `(x, y)` px ảnh: nhỏ nhất; bằng nhau thì khung sau (vẽ trên cùng).
    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        let mut best: Option<(usize, i64)> = None;
        for (i, it) in self.items.iter().enumerate() {
            let r = it.rect;
            if r.w <= 0 || r.h <= 0 || x < r.x || y < r.y || x >= r.right() || y >= r.bottom() {
                continue;
            }
            let a = area(r);
            if best.is_none_or(|(_, b)| a <= b) {
                best = Some((i, a));
            }
        }
        best.map(|(i, _)| i)
    }

    pub fn any_selected(&self) -> bool {
        self.items.iter().any(|i| i.selected)
    }

    pub fn selected_rects(&self) -> Vec<Rect> {
        self.items
            .iter()
            .filter(|i| i.selected)
            .map(|i| i.rect)
            .collect()
    }

    fn plan(&self) -> RedactionPlan {
        RedactionPlan {
            rects: self.selected_rects(),
        }
    }

    /// Nút chính: Findings → Apply(plan) | Confirm::EmptySelection; Error → Confirm::Unscanned.
    pub fn primary(&mut self) -> Option<ReviewOutcome> {
        if self.confirm != Confirm::None {
            return None;
        }
        match self.phase {
            ReviewPhase::Findings if self.any_selected() => Some(ReviewOutcome::Apply(self.plan())),
            ReviewPhase::Findings => {
                self.confirm = Confirm::EmptySelection;
                None
            }
            ReviewPhase::Error(_) => {
                self.confirm = Confirm::Unscanned;
                None
            }
            ReviewPhase::Scanning { .. } => None,
        }
    }

    /// Nút đồng ý trên thanh xác nhận.
    pub fn confirm_yes(&mut self) -> Option<ReviewOutcome> {
        let c = std::mem::replace(&mut self.confirm, Confirm::None);
        match (c, self.phase) {
            (Confirm::EmptySelection, ReviewPhase::Findings) if !self.any_selected() => {
                Some(ReviewOutcome::ContinueNone)
            }
            (Confirm::Unscanned, ReviewPhase::Error(_)) => {
                Some(ReviewOutcome::ContinueUnscanned(self.plan()))
            }
            _ => None,
        }
    }

    /// Nút "Quay lại" trên thanh xác nhận.
    pub fn confirm_back(&mut self) {
        self.confirm = Confirm::None;
    }

    /// "Thử lại" (chỉ khi lỗi): về `Scanning`, giữ vùng tự thêm.
    pub fn retry(&mut self) -> Option<ReviewOutcome> {
        if !matches!(self.phase, ReviewPhase::Error(_)) {
            return None;
        }
        self.phase = ReviewPhase::Scanning { tile: None };
        self.confirm = Confirm::None;
        self.adding = false;
        self.drag = None;
        self.hover = None;
        Some(ReviewOutcome::Retry)
    }

    pub fn adding(&self) -> bool {
        self.adding
    }

    /// Bật/tắt chế độ "Thêm vùng che" (chỉ khi có phát hiện hay lỗi). Tắt thì bỏ khung đang kéo.
    pub fn set_adding(&mut self, on: bool) {
        self.adding = on && self.editable();
        if !self.adding {
            self.drag = None;
        }
    }

    pub fn drag(&self) -> Option<Drag> {
        self.drag
    }

    /// Bắt đầu kéo tại `(x, y)` px ảnh (chỉ trong chế độ thêm).
    pub fn drag_begin(&mut self, x: i32, y: i32) -> bool {
        if !self.adding {
            return false;
        }
        self.drag = Some(Drag {
            start: (x, y),
            cur: (x, y),
        });
        true
    }

    pub fn drag_move(&mut self, x: i32, y: i32) {
        if let Some(d) = &mut self.drag {
            d.cur = (x, y);
        }
    }

    /// Bỏ khung đang kéo, không thêm (mất chuột giữa chừng).
    pub fn cancel_drag(&mut self) {
        self.drag = None;
    }

    /// Thả: thêm vùng (nếu đủ lớn). Chế độ thêm vẫn bật để kéo tiếp.
    pub fn drag_end(&mut self) -> bool {
        match self.drag.take() {
            Some(d) => self.add_manual(d.rect()),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::privacy::model::{Category, Confidence, Mapping, RuleId};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn finding(id: u32, category: Category, rects: Vec<Rect>) -> Finding {
        Finding {
            id,
            category,
            confidence: Confidence::High,
            rects,
            rule: RuleId::EmailPlain,
            mapping: Mapping::Word,
        }
    }

    /// Ba phát hiện: email (1 khung), PEM (2 khung), điện thoại (1 khung).
    fn sample() -> Vec<Finding> {
        vec![
            finding(0, Category::Email, vec![rect(10, 10, 100, 20)]),
            finding(
                1,
                Category::PrivateKey,
                vec![rect(10, 50, 300, 20), rect(10, 70, 300, 20)],
            ),
            Finding {
                confidence: Confidence::Medium,
                ..finding(2, Category::Phone, vec![rect(200, 10, 80, 20)])
            },
        ]
    }

    fn model() -> ReviewModel {
        ReviewModel::new(ReviewPhase::Findings, sample(), 400, 300)
    }

    fn scanning() -> ReviewModel {
        ReviewModel::new(ReviewPhase::Scanning { tile: None }, Vec::new(), 400, 300)
    }

    fn selected(m: &ReviewModel) -> Vec<bool> {
        m.items().iter().map(|i| i.selected).collect()
    }

    #[test]
    fn every_item_selected_by_default_and_multi_rect_finding_is_one_row() {
        let m = model();
        assert_eq!(m.items().len(), 4);
        assert!(m.items().iter().all(|i| i.selected));
        assert_eq!(m.rows(), vec![0..1, 1..3, 3..4]);
        assert_eq!(m.row_count(), 3);
        assert_eq!(m.row_of(2), Some(1));
        assert_eq!(m.row_of(9), None);
        assert_eq!(
            m.items()[1].source,
            RedactionSource::Detected {
                finding: 1,
                category: Category::PrivateKey
            }
        );
        assert_eq!(m.confirm(), Confirm::None);
        assert_eq!(m.phase(), ReviewPhase::Findings);
        assert_eq!(m.confidence(2), Some(Confidence::Medium));
        assert_eq!(m.confidence(0), Some(Confidence::High));
        assert_eq!(m.confidence(7), None);
    }

    #[test]
    fn toggle_flips_whole_finding() {
        let mut m = model();
        m.toggle(2);
        assert_eq!(selected(&m), vec![true, false, false, true]);
        m.toggle(1);
        assert_eq!(selected(&m), vec![true, true, true, true]);
        m.toggle(0);
        assert_eq!(selected(&m), vec![false, true, true, true]);
        m.toggle(99);
        assert_eq!(selected(&m), vec![false, true, true, true]);
    }

    #[test]
    fn select_and_deselect_all() {
        let mut m = model();
        m.deselect_all();
        assert!(!m.any_selected());
        assert!(m.selected_rects().is_empty());
        m.select_all();
        assert_eq!(selected(&m), vec![true; 4]);
        assert_eq!(m.selected_rects().len(), 4);
    }

    #[test]
    fn add_manual_clamps_and_rejects_small_or_outside() {
        let mut m = model();
        assert!(m.add_manual(rect(390, 290, 50, 50)));
        let last = *m.items().last().unwrap();
        assert_eq!(last.rect, rect(390, 290, 10, 10));
        assert_eq!(last.source, RedactionSource::Manual);
        assert!(last.selected);
        // Khung âm (kéo ngược) được chuẩn hóa.
        assert!(m.add_manual(rect(50, 60, -20, -30)));
        assert_eq!(m.items().last().unwrap().rect, rect(30, 30, 20, 30));
        // Nhỏ hơn 4×4.
        assert!(!m.add_manual(rect(0, 0, 3, 10)));
        assert!(!m.add_manual(rect(0, 0, 10, 3)));
        assert!(m.add_manual(rect(0, 0, 4, 4)));
        // Ngoài ảnh hoặc còn quá ít sau khi kẹp.
        assert!(!m.add_manual(rect(500, 500, 40, 40)));
        assert!(!m.add_manual(rect(-50, -50, 40, 40)));
        assert!(!m.add_manual(rect(398, 0, 40, 40)));
        assert_eq!(m.items().len(), 7);
        assert_eq!(m.row_count(), 6);
    }

    #[test]
    fn remove_only_manual() {
        let mut m = model();
        assert!(!m.remove(0));
        assert!(!m.remove(1));
        assert!(!m.remove(42));
        assert!(m.add_manual(rect(0, 100, 30, 30)));
        assert!(m.add_manual(rect(0, 200, 30, 30)));
        assert_eq!(m.items().len(), 6);
        m.cursor = Some(4);
        assert!(m.remove(5));
        assert_eq!(m.items().len(), 5);
        assert_eq!(m.cursor, Some(3));
        assert!(m.remove(4));
        assert_eq!(m.items().len(), 4);
        assert!(m
            .items()
            .iter()
            .all(|i| i.source != RedactionSource::Manual));
    }

    #[test]
    fn hit_prefers_smallest_then_topmost() {
        let mut m = model();
        assert_eq!(m.hit(20, 15), Some(0));
        assert_eq!(m.hit(20, 75), Some(2));
        assert_eq!(m.hit(5, 5), None);
        // Cạnh phải/dưới không tính.
        assert_eq!(m.hit(110, 15), None);
        assert_eq!(m.hit(109, 29), Some(0));
        // Vùng tự thêm nhỏ hơn nằm trong email: thắng.
        assert!(m.add_manual(rect(15, 12, 10, 10)));
        assert_eq!(m.hit(20, 15), Some(4));
        // Bằng diện tích: khung sau thắng.
        assert!(m.add_manual(rect(15, 12, 10, 10)));
        assert_eq!(m.hit(20, 15), Some(5));
    }

    #[test]
    fn primary_applies_selected_plan() {
        let mut m = model();
        m.toggle(0);
        let out = m.primary();
        assert_eq!(
            out,
            Some(ReviewOutcome::Apply(RedactionPlan {
                rects: vec![
                    rect(10, 50, 300, 20),
                    rect(10, 70, 300, 20),
                    rect(200, 10, 80, 20)
                ]
            }))
        );
    }

    #[test]
    fn empty_selection_needs_confirmation() {
        let mut m = model();
        m.deselect_all();
        assert_eq!(m.primary(), None);
        assert_eq!(m.confirm(), Confirm::EmptySelection);
        // Nút chính lần nữa khi đang xác nhận: không làm gì.
        assert_eq!(m.primary(), None);
        m.confirm_back();
        assert_eq!(m.confirm(), Confirm::None);
        assert_eq!(m.confirm_yes(), None, "no bar shown");
        assert_eq!(m.primary(), None);
        assert_eq!(m.confirm_yes(), Some(ReviewOutcome::ContinueNone));
        assert_eq!(m.confirm(), Confirm::None);
    }

    #[test]
    fn changing_selection_drops_confirmation() {
        let mut m = model();
        m.deselect_all();
        assert_eq!(m.primary(), None);
        m.toggle(0);
        assert_eq!(m.confirm(), Confirm::None);
        assert_eq!(m.confirm_yes(), None);
        assert_eq!(
            m.primary(),
            Some(ReviewOutcome::Apply(RedactionPlan {
                rects: vec![rect(10, 10, 100, 20)]
            }))
        );
        m.deselect_all();
        assert_eq!(m.primary(), None);
        assert!(m.add_manual(rect(0, 100, 30, 30)));
        assert_eq!(m.confirm(), Confirm::None);
    }

    #[test]
    fn error_phase_continue_needs_two_steps() {
        let mut m = ReviewModel::new(
            ReviewPhase::Error(ScanError::NoLanguage),
            sample(),
            400,
            300,
        );
        assert!(m.items().is_empty(), "error phase has no detected items");
        assert_eq!(m.primary(), None);
        assert_eq!(m.confirm(), Confirm::Unscanned);
        m.confirm_back();
        assert_eq!(m.confirm(), Confirm::None);
        assert_eq!(m.primary(), None);
        assert_eq!(
            m.confirm_yes(),
            Some(ReviewOutcome::ContinueUnscanned(RedactionPlan::default()))
        );
    }

    #[test]
    fn error_phase_manual_areas_go_into_unscanned_plan() {
        let mut m = ReviewModel::new(ReviewPhase::Error(ScanError::Timeout), Vec::new(), 400, 300);
        assert!(m.add_manual(rect(5, 5, 20, 20)));
        assert!(m.add_manual(rect(50, 50, 20, 20)));
        m.toggle(1);
        assert_eq!(m.primary(), None);
        assert_eq!(
            m.confirm_yes(),
            Some(ReviewOutcome::ContinueUnscanned(RedactionPlan {
                rects: vec![rect(5, 5, 20, 20)]
            }))
        );
    }

    #[test]
    fn scanning_accepts_nothing_but_result() {
        let mut m = scanning();
        assert_eq!(m.primary(), None);
        assert_eq!(m.confirm(), Confirm::None);
        assert!(!m.add_manual(rect(0, 0, 50, 50)));
        m.set_adding(true);
        assert!(!m.adding());
        assert_eq!(m.retry(), None);
        m.set_progress(2, 5);
        assert_eq!(m.phase(), ReviewPhase::Scanning { tile: Some((2, 5)) });
        m.set_progress(9, 5);
        assert_eq!(m.phase(), ReviewPhase::Scanning { tile: Some((5, 5)) });
        m.set_result(Ok(sample()));
        assert_eq!(m.phase(), ReviewPhase::Findings);
        assert_eq!(m.items().len(), 4);
        // Kết quả muộn (không còn Scanning) bị bỏ.
        m.set_result(Err(ScanError::Failed));
        assert_eq!(m.phase(), ReviewPhase::Findings);
        assert_eq!(m.items().len(), 4);
        m.set_progress(1, 2);
        assert_eq!(m.phase(), ReviewPhase::Findings);
    }

    #[test]
    fn retry_keeps_manual_areas_through_new_result() {
        let mut m = scanning();
        m.set_result(Err(ScanError::Failed));
        assert_eq!(m.phase(), ReviewPhase::Error(ScanError::Failed));
        assert!(m.add_manual(rect(5, 5, 20, 20)));
        m.set_adding(true);
        assert!(m.drag_begin(1, 1));
        assert_eq!(m.primary(), None);
        assert_eq!(m.retry(), Some(ReviewOutcome::Retry));
        assert_eq!(m.phase(), ReviewPhase::Scanning { tile: None });
        assert_eq!(m.confirm(), Confirm::None);
        assert!(!m.adding());
        assert_eq!(m.drag(), None);
        assert_eq!(m.retry(), None, "only from error");
        m.set_result(Ok(sample()));
        assert_eq!(m.phase(), ReviewPhase::Findings);
        assert_eq!(m.items().len(), 5);
        assert_eq!(m.items()[4].source, RedactionSource::Manual);
        assert_eq!(m.items()[4].rect, rect(5, 5, 20, 20));
        assert_eq!(m.rows().len(), 4);
        // Lỗi lần nữa: bỏ phát hiện, giữ vùng tay.
        assert_eq!(m.retry(), None);
        let mut m2 = scanning();
        m2.set_result(Err(ScanError::Timeout));
        assert!(m2.add_manual(rect(5, 5, 20, 20)));
        m2.retry();
        m2.set_result(Err(ScanError::NoLanguage));
        assert_eq!(m2.items().len(), 1);
        assert_eq!(m2.phase(), ReviewPhase::Error(ScanError::NoLanguage));
    }

    #[test]
    fn drag_adds_manual_area() {
        let mut m = model();
        assert!(!m.drag_begin(0, 0), "needs add mode");
        m.set_adding(true);
        assert!(m.adding());
        assert!(m.drag_begin(100, 100));
        m.drag_move(40, 160);
        assert_eq!(m.drag().unwrap().rect(), rect(40, 100, 60, 60));
        assert!(m.drag_end());
        assert_eq!(m.items().last().unwrap().rect, rect(40, 100, 60, 60));
        assert!(m.adding(), "stays in add mode");
        assert!(m.drag_begin(10, 10));
        m.drag_move(12, 12);
        assert!(!m.drag_end(), "too small");
        assert!(!m.drag_end(), "no drag");
        assert!(m.drag_begin(10, 10));
        m.drag_move(90, 90);
        m.cancel_drag();
        assert_eq!(m.drag(), None);
        assert!(!m.drag_end());
        assert!(m.drag_begin(10, 10));
        m.set_adding(false);
        assert_eq!(m.drag(), None);
        assert_eq!(m.items().len(), 5);
    }

    #[test]
    fn set_result_with_no_findings_keeps_manual_only() {
        let mut m = scanning();
        m.set_result(Ok(Vec::new()));
        assert_eq!(m.phase(), ReviewPhase::Findings);
        assert!(m.items().is_empty());
        assert_eq!(m.primary(), None);
        assert_eq!(m.confirm(), Confirm::EmptySelection);
        assert_eq!(m.confirm_yes(), Some(ReviewOutcome::ContinueNone));
    }
}
