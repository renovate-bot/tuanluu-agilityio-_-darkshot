//! Cổng xuất của Privacy Shield: mọi ảnh sắp rời ứng dụng (Lưu, Chép, Tải lên, Ghim, Thêm bước,
//! Chép/Ghim từ Lịch sử, xuất các bước) đi qua `submit`. Shield tắt: ảnh đi tiếp ngay dưới dạng
//! `PreparedOutput::unscanned`. Shield bật: quét nền (`ocr::start_scan`), không phát hiện thì đi
//! tiếp; có phát hiện hoặc lỗi thì cửa sổ Duyệt (`review_window`) quyết định. Đây là nơi DUY NHẤT
//! tạo `PreparedOutput` (luật "privacy boundary" của `scripts/check.ps1`).
//!
//! Hai lớp: `GateCore` là máy trạng thái thuần (không Win32, không ảnh) quyết định mỗi yêu cầu chạy
//! đúng một lần hoặc không chạy; phần vỏ giữ ảnh theo mã lượt và thực hiện lệnh của lõi. Mọi thứ ở
//! đây chỉ dùng trên luồng giao diện chính (`thread_local!`). Module này không ghi log.

use crate::ocr;
use crate::pin;
use crate::review_window::{self, ReviewRequest, WM_PRIVACY_REVIEW};
use darkshot_core::actions::NoticeKind;
use darkshot_core::i18n::Msg;
use darkshot_core::image::Image;
use darkshot_core::privacy::{
    Finding, PreparedOutput, Protection, RedactError, RedactionPlan, ReviewOutcome, ReviewPhase,
    ScanError,
};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::Arc;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    KillTimer, PeekMessageW, PostMessageW, SetTimer, MSG, PM_REMOVE, WM_APP, WM_TIMER,
};

/// Số lượt chờ tối đa (đang quét, chờ duyệt, đang duyệt, đang làm phẳng, chờ lớp phủ đóng).
pub const MAX_PENDING: usize = 3;
/// Quét lâu hơn mức này thì mở cửa sổ "Đang quét…".
pub const SCANNING_WINDOW_DELAY_MS: u32 = 800;
/// Kết quả quét: WPARAM = mã lượt quét, LPARAM = `Box<Result<ScanReport, ScanError>>`.
pub const WM_PRIVACY_SCAN: u32 = WM_APP + 19;
/// Tiến độ quét: WPARAM = mã lượt quét, LPARAM = `(k << 16) | n` (không Box).
pub const WM_PRIVACY_PROGRESS: u32 = WM_APP + 21;
/// Ảnh lớn đã làm phẳng xong trên luồng nền: WPARAM = mã lượt, LPARAM = `Box<FlatResult>`.
pub const WM_PRIVACY_FLAT: u32 = WM_APP + 22;
/// Ảnh nhiều điểm hơn mức này được làm phẳng trên luồng nền.
const FLATTEN_INLINE_MAX_PIXELS: u64 = 4_000_000;
/// Hẹn giờ 800 ms của lượt `id` có mã `TIMER_BASE + id` (mã 1..3 là của `main`).
const TIMER_BASE: usize = 0x0001_0000;
/// Hẹn giờ canh cửa sổ Duyệt mất mà không gửi kết quả.
const WATCHDOG_TIMER_ID: usize = TIMER_BASE - 1;
const WATCHDOG_MS: u32 = 1000;

/// Đầu ra người dùng đã yêu cầu cho một ảnh.
pub enum OutputAction {
    Save,
    Copy,
    Upload,
    Pin {
        place: pin::PinPlace,
    },
    AddStep,
    /// Chép ảnh từ Lịch sử (Task 132 nối).
    #[cfg_attr(not(test), allow(dead_code))]
    HistoryCopy,
    HistoryPin,
    /// Một bước của lượt xuất các bước; kết quả về qua `Done` (Task 132 nối).
    #[cfg_attr(not(test), allow(dead_code))]
    StepExport {
        step: u32,
        steps: u32,
    },
}

impl OutputAction {
    /// Tên hành động hiện trong cửa sổ Duyệt.
    fn label(&self) -> Msg {
        match self {
            OutputAction::Save => Msg::PrivacyActionSave,
            OutputAction::Copy | OutputAction::HistoryCopy => Msg::PrivacyActionCopy,
            OutputAction::Upload => Msg::PrivacyActionUpload,
            OutputAction::Pin { .. } | OutputAction::HistoryPin => Msg::PrivacyActionPin,
            OutputAction::AddStep => Msg::PrivacyActionAddStep,
            OutputAction::StepExport { .. } => Msg::PrivacyActionExportSteps,
        }
    }

    /// Đầu ra mở cửa sổ (ảnh ghim): chờ lớp phủ đóng rồi mới chạy.
    fn windowed(&self) -> bool {
        matches!(self, OutputAction::Pin { .. } | OutputAction::HistoryPin)
    }

    fn step(&self) -> Option<(u32, u32)> {
        match self {
            OutputAction::StepExport { step, steps } => Some((*step, *steps)),
            _ => None,
        }
    }
}

/// Nhận kết quả của `submit_with`: `Some` = ảnh đã qua cổng, `None` = Hủy hoặc lỗi (không xuất).
/// Không được gọi khi `submit_with` trả `Err` và khi ứng dụng thoát (`shutdown`).
pub type Done = Box<dyn FnOnce(Option<PreparedOutput>)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitError {
    /// Đã đủ `MAX_PENDING` lượt chờ; ảnh bị bỏ, đã thông báo.
    QueueFull,
    /// Cổng đã đóng (đang thoát); ảnh bị bỏ.
    WindowFailed,
}

// ---------------------------------------------------------------------------------------------
// Lõi thuần: máy trạng thái của cổng.
// ---------------------------------------------------------------------------------------------

/// Mã lượt (cũng là token của cửa sổ Duyệt) và mã lượt quét; tăng dần, không dùng lại.
pub type Id = usize;

/// Ảnh sẵn sàng theo cách nào (vỏ dựng `PreparedOutput` tương ứng).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ready {
    /// Shield tắt lúc nhận.
    Unscanned,
    /// Quét xong, không phát hiện.
    NoFindings,
    /// Có phát hiện, người dùng xác nhận không che.
    ReviewedNone,
    /// Ảnh đã làm phẳng (vỏ đang giữ kết quả của lệnh `Flatten`).
    Flattened,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Note {
    QueueFull,
    WindowFailed,
    RedactFailed,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Kết quả của lượt quét `scan`.
    ScanDone {
        scan: Id,
        result: Result<Vec<Finding>, ScanError>,
    },
    /// Hẹn giờ 800 ms của lượt `id`.
    Timer {
        id: Id,
    },
    /// Lựa chọn của người dùng trong cửa sổ Duyệt của lượt `id`.
    Outcome {
        id: Id,
        outcome: ReviewOutcome,
    },
    /// Không mở được cửa sổ Duyệt cho lượt `id`.
    OpenFailed {
        id: Id,
    },
    /// Cửa sổ Duyệt của lượt `id` đã mất mà không có kết quả.
    ReviewGone {
        id: Id,
    },
    /// Lệnh `Flatten` của lượt `id` xong (`ok` = có ảnh đã che).
    Flattened {
        id: Id,
        ok: bool,
    },
    OverlayClosed,
    Shutdown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    StartScan {
        id: Id,
        scan: Id,
    },
    StartTimer {
        id: Id,
    },
    OpenReview {
        id: Id,
        phase: ReviewPhase,
        findings: Vec<Finding>,
    },
    Update {
        id: Id,
        result: Result<Vec<Finding>, ScanError>,
    },
    CloseReview {
        id: Id,
    },
    /// Làm phẳng `plan` lên ảnh của lượt; xong thì gửi `Event::Flattened`.
    Flatten {
        id: Id,
        plan: RedactionPlan,
        after_error: bool,
    },
    /// Chạy đầu ra của lượt (đúng một lần); lượt đã rời bảng.
    Dispatch {
        id: Id,
        how: Ready,
    },
    /// Bỏ lượt và ảnh của nó; không xuất gì.
    Drop {
        id: Id,
    },
    Notify(Note),
    /// Đóng mọi cửa sổ Duyệt, không nhận kết quả nào nữa.
    CloseAll,
}

enum State {
    Scanning {
        scan: Id,
        /// Đã quá 800 ms: muốn có cửa sổ "Đang quét…".
        slow: bool,
        /// Cửa sổ Duyệt của lượt đang mở.
        shown: bool,
        /// Quét lại sau "Thử lại": kết quả luôn về cửa sổ (vùng tự thêm còn đó).
        retried: bool,
    },
    /// Có kết quả cần duyệt, chờ tới lượt mở cửa sổ.
    Waiting(Result<Vec<Finding>, ScanError>),
    Reviewing {
        scan_ok: bool,
        found: usize,
    },
    Flattening,
    /// Sẵn sàng, chờ lớp phủ đóng (đầu ra mở cửa sổ).
    Held(Ready),
}

struct Entry {
    id: Id,
    windowed: bool,
    state: State,
}

/// Máy trạng thái của cổng. Không giữ ảnh, không gọi hệ điều hành.
#[derive(Default)]
pub struct GateCore {
    next: Id,
    entries: Vec<Entry>,
    /// Lượt có cửa sổ Duyệt đang mở (mỗi lúc một cửa sổ).
    open: Option<Id>,
    closed: bool,
}

impl GateCore {
    pub fn new() -> GateCore {
        GateCore::default()
    }

    /// Số lượt đang chờ.
    #[cfg(test)]
    pub fn pending(&self) -> usize {
        self.entries.len()
    }

    /// Lượt có cửa sổ Duyệt đang mở.
    pub fn open(&self) -> Option<Id> {
        self.open
    }

    /// Lượt đang quét bằng mã `scan` và có cửa sổ đang mở.
    pub fn shown_for_scan(&self, scan: Id) -> Option<Id> {
        self.entries.iter().find_map(|e| match e.state {
            State::Scanning {
                scan: s,
                shown: true,
                ..
            } if s == scan => Some(e.id),
            _ => None,
        })
    }

    /// Lượt `id` đang chờ kết quả làm phẳng.
    pub fn is_flattening(&self, id: Id) -> bool {
        self.entries
            .iter()
            .any(|e| e.id == id && matches!(e.state, State::Flattening))
    }

    fn token(&mut self) -> Id {
        self.next += 1;
        self.next
    }

    fn index(&self, id: Id) -> Option<usize> {
        self.entries.iter().position(|e| e.id == id)
    }

    /// Yêu cầu mới. `shield`: công tắc đọc lúc nhận; `windowed`: đầu ra mở cửa sổ. `None` = từ chối.
    pub fn submit(&mut self, shield: bool, windowed: bool) -> (Option<Id>, Vec<Cmd>) {
        if !shield {
            // Shield tắt: như trước khi có cổng, không xếp hàng, không chờ gì.
            let id = self.token();
            return (
                Some(id),
                vec![Cmd::Dispatch {
                    id,
                    how: Ready::Unscanned,
                }],
            );
        }
        if self.closed {
            return (None, Vec::new());
        }
        if self.entries.len() >= MAX_PENDING {
            return (None, vec![Cmd::Notify(Note::QueueFull)]);
        }
        let id = self.token();
        let scan = self.token();
        self.entries.push(Entry {
            id,
            windowed,
            state: State::Scanning {
                scan,
                slow: false,
                shown: false,
                retried: false,
            },
        });
        (
            Some(id),
            vec![Cmd::StartScan { id, scan }, Cmd::StartTimer { id }],
        )
    }

    /// Một sự kiện; `overlay` = lớp phủ chọn vùng đang mở.
    pub fn step(&mut self, ev: Event, overlay: bool) -> Vec<Cmd> {
        let mut out = Vec::new();
        if self.closed {
            return out;
        }
        match ev {
            Event::ScanDone { scan, result } => self.on_scan(scan, result, overlay, &mut out),
            Event::Timer { id } => {
                if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
                    if let State::Scanning { slow, .. } = &mut e.state {
                        *slow = true;
                    }
                }
            }
            Event::Outcome { id, outcome } => self.on_outcome(id, outcome, overlay, &mut out),
            Event::OpenFailed { id } => {
                if self.open == Some(id) {
                    out.push(Cmd::Notify(Note::WindowFailed));
                    self.discard(id, &mut out);
                }
            }
            Event::ReviewGone { id } => {
                if self.open == Some(id) {
                    self.discard(id, &mut out);
                }
            }
            Event::Flattened { id, ok } => {
                if let Some(i) = self.index(id) {
                    if matches!(self.entries[i].state, State::Flattening) {
                        if ok {
                            self.ready(i, Ready::Flattened, overlay, &mut out);
                        } else {
                            out.push(Cmd::Notify(Note::RedactFailed));
                            self.discard(id, &mut out);
                        }
                    }
                }
            }
            Event::OverlayClosed => {
                let mut i = 0;
                while i < self.entries.len() {
                    if let State::Held(how) = self.entries[i].state {
                        let e = self.entries.remove(i);
                        out.push(Cmd::Dispatch { id: e.id, how });
                    } else {
                        i += 1;
                    }
                }
            }
            Event::Shutdown => {
                self.closed = true;
                self.open = None;
                out.push(Cmd::CloseAll);
                out.extend(self.entries.drain(..).map(|e| Cmd::Drop { id: e.id }));
                return out;
            }
        }
        self.try_open(overlay, &mut out);
        out
    }

    /// Bỏ lượt `id` (không xuất); nhả cửa sổ nếu là lượt đang mở.
    fn discard(&mut self, id: Id, out: &mut Vec<Cmd>) {
        if let Some(i) = self.index(id) {
            self.entries.remove(i);
            out.push(Cmd::Drop { id });
        }
        if self.open == Some(id) {
            self.open = None;
        }
    }

    /// Ảnh của lượt thứ `i` đã sẵn sàng: chạy ngay (lượt rời bảng TRƯỚC khi chạy), trừ đầu ra mở
    /// cửa sổ khi lớp phủ đang mở.
    fn ready(&mut self, i: usize, how: Ready, overlay: bool, out: &mut Vec<Cmd>) {
        if self.entries[i].windowed && overlay {
            self.entries[i].state = State::Held(how);
        } else {
            let e = self.entries.remove(i);
            out.push(Cmd::Dispatch { id: e.id, how });
        }
    }

    fn on_scan(
        &mut self,
        scan: Id,
        result: Result<Vec<Finding>, ScanError>,
        overlay: bool,
        out: &mut Vec<Cmd>,
    ) {
        // Chỉ lượt quét hiện hành của một lượt đang quét; mã cũ, trùng hay lạ thì bỏ.
        let found = self
            .entries
            .iter()
            .enumerate()
            .find_map(|(i, e)| match e.state {
                State::Scanning {
                    scan: s,
                    shown,
                    retried,
                    ..
                } if s == scan => Some((i, e.id, shown, retried)),
                _ => None,
            });
        let Some((i, id, shown, retried)) = found else {
            return;
        };
        let clean = matches!(&result, Ok(f) if f.is_empty());
        if clean && !retried {
            if shown {
                out.push(Cmd::CloseReview { id });
                self.open = None;
            }
            self.ready(i, Ready::NoFindings, overlay, out);
        } else if shown {
            self.entries[i].state = State::Reviewing {
                scan_ok: result.is_ok(),
                found: result.as_ref().map_or(0, Vec::len),
            };
            out.push(Cmd::Update { id, result });
        } else {
            self.entries[i].state = State::Waiting(result);
        }
    }

    fn on_outcome(&mut self, id: Id, outcome: ReviewOutcome, overlay: bool, out: &mut Vec<Cmd>) {
        // Chỉ cửa sổ đang mở của đúng lượt này mới có tiếng nói.
        if self.open != Some(id) {
            return;
        }
        let Some(i) = self.index(id) else {
            self.open = None;
            return;
        };
        let reviewing = match self.entries[i].state {
            State::Reviewing { scan_ok, found } => Some((scan_ok, found)),
            _ => None,
        };
        match (outcome, reviewing) {
            (ReviewOutcome::Cancel, _) => self.discard(id, out),
            (ReviewOutcome::Retry, Some((false, _))) => {
                let scan = self.token();
                self.entries[i].state = State::Scanning {
                    scan,
                    slow: true,
                    shown: true,
                    retried: true,
                };
                out.push(Cmd::StartScan { id, scan });
            }
            // Thử lại ngoài trạng thái lỗi: cửa sổ vẫn mở, không làm gì.
            (ReviewOutcome::Retry, _) => {}
            (ReviewOutcome::Apply(plan), Some((true, _))) if !plan.rects.is_empty() => {
                self.entries[i].state = State::Flattening;
                self.open = None;
                out.push(Cmd::Flatten {
                    id,
                    plan,
                    after_error: false,
                });
            }
            (ReviewOutcome::ContinueNone, Some((true, found))) => {
                self.open = None;
                let how = if found == 0 {
                    Ready::NoFindings
                } else {
                    Ready::ReviewedNone
                };
                self.ready(i, how, overlay, out);
            }
            (ReviewOutcome::ContinueUnscanned(plan), Some((false, _))) => {
                self.entries[i].state = State::Flattening;
                self.open = None;
                out.push(Cmd::Flatten {
                    id,
                    plan,
                    after_error: true,
                });
            }
            // Kết quả cuối không khớp trạng thái quét (cửa sổ đã đóng): không bao giờ xuất.
            _ => {
                out.push(Cmd::Notify(Note::RedactFailed));
                self.discard(id, out);
            }
        }
    }

    /// Mở cửa sổ Duyệt cho lượt chờ đầu tiên (theo thứ tự nhận) nếu chưa có cửa sổ nào và lớp phủ
    /// không mở.
    fn try_open(&mut self, overlay: bool, out: &mut Vec<Cmd>) {
        if overlay || self.open.is_some() {
            return;
        }
        let wants = |e: &Entry| {
            matches!(
                e.state,
                State::Waiting(_)
                    | State::Scanning {
                        slow: true,
                        shown: false,
                        ..
                    }
            )
        };
        let Some(e) = self.entries.iter_mut().find(|e| wants(e)) else {
            return;
        };
        let (phase, findings, next) = match std::mem::replace(&mut e.state, State::Flattening) {
            State::Waiting(Ok(f)) => {
                let found = f.len();
                (
                    ReviewPhase::Findings,
                    f,
                    State::Reviewing {
                        scan_ok: true,
                        found,
                    },
                )
            }
            State::Waiting(Err(err)) => (
                ReviewPhase::Error(err),
                Vec::new(),
                State::Reviewing {
                    scan_ok: false,
                    found: 0,
                },
            ),
            State::Scanning { scan, retried, .. } => (
                ReviewPhase::Scanning { tile: None },
                Vec::new(),
                State::Scanning {
                    scan,
                    slow: true,
                    shown: true,
                    retried,
                },
            ),
            other => {
                e.state = other;
                return;
            }
        };
        e.state = next;
        self.open = Some(e.id);
        out.push(Cmd::OpenReview {
            id: e.id,
            phase,
            findings,
        });
    }
}

// ---------------------------------------------------------------------------------------------
// Vỏ: giữ ảnh theo mã lượt, thực hiện lệnh của lõi. Chỉ trên luồng giao diện chính.
// ---------------------------------------------------------------------------------------------

/// Nơi nhận ảnh đã qua cổng (main: `run_on_image`, `pin::open`, `steps_store::add`).
pub type Sink = fn(OutputAction, PreparedOutput);
type Notify = fn(NoticeKind, &str, &str);
type FlatResult = Result<PreparedOutput, RedactError>;

struct Slot {
    id: Id,
    action: OutputAction,
    /// Ảnh gốc của lượt (bản duy nhất; luồng quét và cửa sổ Duyệt giữ `Arc`).
    image: Option<Arc<Image>>,
    /// Ảnh đã làm phẳng, chờ `Dispatch`.
    prepared: Option<PreparedOutput>,
    done: Option<Done>,
}

thread_local! {
    static CORE: RefCell<GateCore> = RefCell::new(GateCore::new());
    static SLOTS: RefCell<Vec<Slot>> = const { RefCell::new(Vec::new()) };
    static SINK: Cell<Option<Sink>> = const { Cell::new(None) };
    static NOTIFY: Cell<Option<Notify>> = const { Cell::new(None) };
    static ENABLED: Cell<Option<fn() -> bool>> = const { Cell::new(None) };
    /// Cửa sổ nhận thông điệp của cổng (cửa sổ ẩn của ứng dụng).
    static WINDOW: Cell<Option<HWND>> = const { Cell::new(None) };
    /// Hẹn giờ canh đang chạy; số lần liên tiếp thấy cửa sổ Duyệt đã mất.
    static WATCHDOG: Cell<bool> = const { Cell::new(false) };
    static STRIKES: Cell<u8> = const { Cell::new(0) };
}

/// Cửa sổ nhận `WM_PRIVACY_*` và hẹn giờ của cổng; thủ tục cửa sổ đó gọi `handle_message`.
pub fn set_window(hwnd: HWND) {
    WINDOW.with(|w| w.set(Some(hwnd)));
}

pub fn set_sink(f: Sink) {
    SINK.with(|s| s.set(Some(f)));
}

/// Hàm báo qua khay (main).
pub fn set_notifier(f: Notify) {
    NOTIFY.with(|n| n.set(Some(f)));
}

/// Nguồn của công tắc `privacy_shield` (main đọc cấu hình đang dùng).
pub fn set_enabled_source(f: fn() -> bool) {
    ENABLED.with(|e| e.set(Some(f)));
}

/// Privacy Shield đang bật theo cấu hình hiện hành.
pub fn enabled() -> bool {
    ENABLED.with(|e| e.get()).is_some_and(|f| f())
}

/// Nhận ảnh đã phẳng. Shield tắt → chạy ngay với `PreparedOutput::unscanned`. Shield bật → luôn
/// quét; đầu ra chỉ chạy sau khi quét sạch hoặc người dùng quyết định trong cửa sổ Duyệt.
pub fn submit(action: OutputAction, image: Image) -> Result<(), SubmitError> {
    submit_inner(action, image, None)
}

/// Như `submit`, kết quả về `done` thay vì đầu ra thường (lượt xuất các bước, Task 132).
#[cfg_attr(not(test), allow(dead_code))]
pub fn submit_with(action: OutputAction, image: Image, done: Done) -> Result<(), SubmitError> {
    submit_inner(action, image, Some(done))
}

/// Lưu/Chép từ ảnh ghim: ảnh ghim mang `protection` của lần qua cổng trước. Shield tắt → như
/// trước. Shield bật → chỉ ảnh đã quét mà không bị sửa (`NoFindings`, `ReviewedNoneApplied`) đi
/// thẳng; mọi trạng thái khác (ghim lúc Shield tắt, bỏ qua lỗi, đã che) được quét lại.
pub fn submit_pinned(
    action: OutputAction,
    image: Image,
    protection: Protection,
) -> Result<(), SubmitError> {
    if enabled() {
        let prepared = match protection {
            Protection::NoFindings => PreparedOutput::no_findings(image),
            Protection::ReviewedNoneApplied => PreparedOutput::reviewed_none(image),
            Protection::Disabled
            | Protection::Redacted { .. }
            | Protection::SkippedAfterError { .. } => return submit(action, image),
        };
        deliver(action, None, Some(prepared));
        return Ok(());
    }
    submit(action, image)
}

fn submit_inner(action: OutputAction, image: Image, done: Option<Done>) -> Result<(), SubmitError> {
    let shield = enabled();
    let (id, cmds) = CORE.with(|c| c.borrow_mut().submit(shield, action.windowed()));
    let res = match id {
        Some(id) => {
            SLOTS.with(|s| {
                s.borrow_mut().push(Slot {
                    id,
                    action,
                    image: Some(Arc::new(image)),
                    prepared: None,
                    done,
                })
            });
            Ok(())
        }
        // Ảnh bị bỏ ở đây (không xuất).
        None if cmds.is_empty() => Err(SubmitError::WindowFailed),
        None => Err(SubmitError::QueueFull),
    };
    run(cmds);
    res
}

/// Lớp phủ chọn vùng vừa đóng: mở cửa sổ Duyệt đang chờ, chạy ảnh ghim đang giữ.
pub fn on_overlay_closed() {
    feed(Event::OverlayClosed);
}

/// Thoát: bỏ mọi lượt chờ (không lưu ảnh chưa duyệt), đóng cửa sổ Duyệt, không ghi gì. Gọi lại vô hại.
pub fn shutdown() {
    feed(Event::Shutdown);
    let Some(hwnd) = WINDOW.with(|w| w.get()) else {
        return;
    };
    // SAFETY: hủy hẹn giờ của cửa sổ thuộc luồng này; không có thì lỗi và ta bỏ qua.
    unsafe {
        let _ = KillTimer(Some(hwnd), WATCHDOG_TIMER_ID);
    }
    WATCHDOG.with(|w| w.set(false));
    // Kết quả còn trong hàng đợi không ai nhận nữa: giải phóng Box (quyết định 120).
    review_window::drain_outcomes(hwnd, WM_PRIVACY_REVIEW);
    let mut m = MSG::default();
    // SAFETY: chỉ lấy thông điệp của cổng gửi tới cửa sổ này khỏi hàng đợi của luồng; mỗi Box lấy một lần.
    unsafe {
        while PeekMessageW(
            &mut m,
            Some(hwnd),
            WM_PRIVACY_SCAN,
            WM_PRIVACY_SCAN,
            PM_REMOVE,
        )
        .as_bool()
        {
            drop(ocr::take_scan_result(m.lParam));
        }
        while PeekMessageW(
            &mut m,
            Some(hwnd),
            WM_PRIVACY_FLAT,
            WM_PRIVACY_FLAT,
            PM_REMOVE,
        )
        .as_bool()
        {
            drop(take_flat(m.lParam));
        }
    }
}

/// Thông điệp của cổng tới cửa sổ chính. `None`: không phải của cổng.
pub fn handle_message(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    match msg {
        WM_PRIVACY_SCAN => {
            // SAFETY: thông điệp do `ocr::start_scan` gửi tới cửa sổ này, lấy đúng một lần.
            let report = unsafe { ocr::take_scan_result(lp) };
            let scan = wp.0;
            // Cửa sổ "Đang quét…" đã bị đóng (Hủy còn nằm trong hàng đợi): Hủy thắng kết quả quét.
            let shown = CORE.with(|c| c.borrow().shown_for_scan(scan));
            if let Some(id) = shown {
                if !review_window::is_open(id) {
                    feed(Event::ReviewGone { id });
                }
            }
            feed(Event::ScanDone {
                scan,
                result: report.map(|r| r.findings),
            });
            Some(LRESULT(0))
        }
        WM_PRIVACY_PROGRESS => {
            if let Some(id) = CORE.with(|c| c.borrow().shown_for_scan(wp.0)) {
                let v = lp.0 as usize;
                review_window::progress(id, ((v >> 16) & 0xFFFF) as u32, (v & 0xFFFF) as u32);
            }
            Some(LRESULT(0))
        }
        WM_PRIVACY_REVIEW => {
            // SAFETY: thông điệp do cửa sổ Duyệt gửi tới cửa sổ này, lấy đúng một lần.
            let outcome = unsafe { review_window::take_outcome(lp) };
            feed(Event::Outcome {
                id: wp.0,
                outcome: *outcome,
            });
            Some(LRESULT(0))
        }
        WM_PRIVACY_FLAT => {
            // SAFETY: thông điệp do luồng làm phẳng của cổng gửi tới cửa sổ này, lấy đúng một lần.
            let res = unsafe { take_flat(lp) };
            let id = wp.0;
            // Lượt đã bị bỏ (thoát, trùng): ảnh trong Box bị thả ở đây.
            if CORE.with(|c| c.borrow().is_flattening(id)) {
                let ok = store_prepared(id, *res);
                feed(Event::Flattened { id, ok });
            }
            Some(LRESULT(0))
        }
        WM_TIMER if wp.0 == WATCHDOG_TIMER_ID => {
            on_watchdog();
            sync_watchdog();
            Some(LRESULT(0))
        }
        WM_TIMER if wp.0 > TIMER_BASE => {
            // SAFETY: hủy bộ hẹn giờ một lần của cửa sổ này.
            unsafe {
                let _ = KillTimer(Some(hwnd), wp.0);
            }
            feed(Event::Timer {
                id: wp.0 - TIMER_BASE,
            });
            Some(LRESULT(0))
        }
        _ => None,
    }
}

/// Lấy lại `Box` của một thông điệp `WM_PRIVACY_FLAT`.
///
/// # Safety
/// `lp` là LPARAM của đúng một thông điệp do luồng làm phẳng gửi (đã lấy khỏi hàng đợi), gọi một lần.
unsafe fn take_flat(lp: LPARAM) -> Box<FlatResult> {
    // SAFETY: theo hợp đồng của hàm, lp là con trỏ Box<FlatResult> do `post_flat` tạo.
    unsafe { Box::from_raw(lp.0 as *mut FlatResult) }
}

/// Gửi kết quả làm phẳng về luồng chính; gửi lỗi thì lấy lại Box (ảnh bị thả, không xuất).
fn post_flat(hwnd: isize, id: Id, res: FlatResult) {
    let p = Box::into_raw(Box::new(res));
    // SAFETY: PostMessageW an toàn từ luồng bất kỳ; thất bại thì không ai nhận con trỏ nên lấy lại.
    unsafe {
        if PostMessageW(
            Some(HWND(hwnd as *mut c_void)),
            WM_PRIVACY_FLAT,
            WPARAM(id),
            LPARAM(p as isize),
        )
        .is_err()
        {
            drop(Box::from_raw(p));
        }
    }
}

/// Cửa sổ Duyệt mất mà không có kết quả nào tới (gửi thông điệp lỗi): sau hai lần canh liên tiếp
/// thì coi như Hủy, để hàng đợi không kẹt.
fn on_watchdog() {
    let gone = CORE
        .with(|c| c.borrow().open())
        .filter(|id| !review_window::is_open(*id));
    match gone {
        Some(id) if STRIKES.with(|s| s.get()) >= 1 => {
            STRIKES.with(|s| s.set(0));
            feed(Event::ReviewGone { id });
        }
        Some(_) => STRIKES.with(|s| s.set(1)),
        None => STRIKES.with(|s| s.set(0)),
    }
}

/// Hẹn giờ canh chạy đúng khi có cửa sổ Duyệt đang mở.
fn sync_watchdog() {
    let Some(hwnd) = WINDOW.with(|w| w.get()) else {
        return;
    };
    let want = CORE.with(|c| c.borrow().open().is_some());
    if want == WATCHDOG.with(|w| w.get()) {
        return;
    }
    // SAFETY: hẹn giờ gắn với cửa sổ của luồng này; WM_TIMER về `handle_message`.
    unsafe {
        if want {
            SetTimer(Some(hwnd), WATCHDOG_TIMER_ID, WATCHDOG_MS, None);
        } else {
            let _ = KillTimer(Some(hwnd), WATCHDOG_TIMER_ID);
        }
    }
    STRIKES.with(|s| s.set(0));
    WATCHDOG.with(|w| w.set(want));
}

/// Đưa một sự kiện vào lõi rồi thực hiện các lệnh (và sự kiện phát sinh) tới khi hết.
fn feed(ev: Event) {
    let cmds = CORE.with(|c| c.borrow_mut().step(ev, crate::overlay_open()));
    run(cmds);
}

/// Thực hiện lệnh của lõi. Không giữ khóa nào khi gọi ra ngoài (đầu ra, cửa sổ, thông báo), nên
/// một lời gọi `submit` lồng vào vẫn an toàn.
fn run(cmds: Vec<Cmd>) {
    let mut queue: VecDeque<Cmd> = cmds.into();
    while let Some(cmd) = queue.pop_front() {
        if let Some(ev) = exec(cmd) {
            queue.extend(CORE.with(|c| c.borrow_mut().step(ev, crate::overlay_open())));
        }
    }
    sync_watchdog();
}

fn take_slot(id: Id) -> Option<Slot> {
    SLOTS.with(|s| {
        let mut s = s.borrow_mut();
        let i = s.iter().position(|x| x.id == id)?;
        Some(s.remove(i))
    })
}

fn with_slot<R>(id: Id, f: impl FnOnce(&mut Slot) -> R) -> Option<R> {
    SLOTS.with(|s| s.borrow_mut().iter_mut().find(|x| x.id == id).map(f))
}

fn kill_timer(id: Id) {
    if let Some(hwnd) = WINDOW.with(|w| w.get()) {
        // SAFETY: hủy hẹn giờ của cửa sổ thuộc luồng này; không có thì lỗi và ta bỏ qua.
        unsafe {
            let _ = KillTimer(Some(hwnd), TIMER_BASE + id);
        }
    }
}

fn notify(note: Note) {
    let body = match note {
        Note::QueueFull => Msg::PrivacyQueueFull,
        Note::WindowFailed => Msg::PrivacyWindowFailed,
        Note::RedactFailed => Msg::PrivacyRedactFailed,
    };
    let kind = match note {
        Note::QueueFull => NoticeKind::Info,
        Note::WindowFailed | Note::RedactFailed => NoticeKind::Error,
    };
    if let Some(f) = NOTIFY.with(|n| n.get()) {
        f(
            kind,
            crate::lang::tr(Msg::PrivacyReviewTitle),
            crate::lang::tr(body),
        );
    }
}

/// Ảnh của lượt thành `Image` sở hữu: thường `Arc` chỉ còn một chủ (luồng quét đã thả, cửa sổ Duyệt
/// đã đóng); còn chủ khác (lượt quét quá hạn chưa dừng) thì sao chép.
fn owned(image: Arc<Image>) -> Image {
    Arc::try_unwrap(image).unwrap_or_else(|a| (*a).clone())
}

/// Làm phẳng `plan` lên `image` (bản mới; ảnh gốc bị thả). Không bao giờ trả ảnh chưa che khi
/// `after_error` sai: plan không phủ điểm nào là lỗi.
fn flatten(image: Arc<Image>, plan: &RedactionPlan, after_error: bool) -> FlatResult {
    let image = owned(image);
    if after_error {
        PreparedOutput::skipped_after_error(image, plan)
    } else {
        PreparedOutput::redacted(image, plan)
    }
}

/// Giữ ảnh đã làm phẳng của lượt `id` tới lệnh `Dispatch`; trả về có ảnh hay không.
fn store_prepared(id: Id, res: FlatResult) -> bool {
    match res {
        Ok(p) => with_slot(id, |s| s.prepared = Some(p)).is_some(),
        Err(_) => false,
    }
}

/// Chạy đầu ra với ảnh đã qua cổng (hoặc báo Hủy cho `done`).
fn deliver(action: OutputAction, done: Option<Done>, prepared: Option<PreparedOutput>) {
    match (done, prepared) {
        (Some(done), prepared) => done(prepared),
        (None, Some(prepared)) => {
            if let Some(sink) = SINK.with(|s| s.get()) {
                sink(action, prepared);
            }
        }
        (None, None) => {}
    }
}

/// Thực hiện một lệnh; trả sự kiện phát sinh ngay (nếu có).
fn exec(cmd: Cmd) -> Option<Event> {
    match cmd {
        Cmd::StartScan { id, scan } => {
            let image = with_slot(id, |s| s.image.clone()).flatten();
            let started = match (image, WINDOW.with(|w| w.get())) {
                (Some(image), Some(hwnd)) => ocr::start_scan_with_progress(
                    image,
                    hwnd,
                    WM_PRIVACY_SCAN,
                    Some(WM_PRIVACY_PROGRESS),
                    scan,
                )
                .is_ok(),
                _ => false,
            };
            // Không tạo được luồng quét: trạng thái lỗi trong cửa sổ Duyệt, không bao giờ "sạch".
            (!started).then_some(Event::ScanDone {
                scan,
                result: Err(ScanError::Failed),
            })
        }
        Cmd::StartTimer { id } => {
            if let Some(hwnd) = WINDOW.with(|w| w.get()) {
                // SAFETY: hẹn giờ gắn với cửa sổ của luồng này; WM_TIMER về `handle_message`.
                // Không đặt được thì chỉ mất cửa sổ "Đang quét…".
                unsafe {
                    SetTimer(Some(hwnd), TIMER_BASE + id, SCANNING_WINDOW_DELAY_MS, None);
                }
            }
            None
        }
        Cmd::OpenReview {
            id,
            phase,
            findings,
        } => {
            let req = with_slot(id, |s| {
                s.image.clone().map(|image| ReviewRequest {
                    token: id,
                    image,
                    action: s.action.label(),
                    step: s.action.step(),
                    phase,
                    findings,
                })
            })
            .flatten();
            let opened = match (req, WINDOW.with(|w| w.get())) {
                (Some(req), Some(hwnd)) => {
                    review_window::open(req, hwnd, WM_PRIVACY_REVIEW).is_ok()
                }
                _ => false,
            };
            (!opened).then_some(Event::OpenFailed { id })
        }
        Cmd::Update { id, result } => {
            // Cửa sổ đã mất thì kết quả Hủy đang tới hoặc hẹn giờ canh dọn lượt.
            let _ = review_window::update(id, result);
            None
        }
        Cmd::CloseReview { id } => {
            review_window::close(id);
            None
        }
        Cmd::Flatten {
            id,
            plan,
            after_error,
        } => {
            let Some(image) = with_slot(id, |s| s.image.take()).flatten() else {
                return Some(Event::Flattened { id, ok: false });
            };
            let pixels = u64::from(image.width) * u64::from(image.height);
            let target = WINDOW.with(|w| w.get()).map(|h| h.0 as isize);
            match target {
                // Ảnh lớn có vùng che: làm phẳng trên luồng nền, kết quả về bằng thông điệp.
                Some(target) if pixels > FLATTEN_INLINE_MAX_PIXELS && !plan.rects.is_empty() => {
                    let spawned = std::thread::Builder::new()
                        .name("darkshot-privacy-redact".into())
                        .spawn(move || post_flat(target, id, flatten(image, &plan, after_error)))
                        .is_ok();
                    // Không tạo được luồng: không xuất (ảnh đã bị thả cùng closure).
                    (!spawned).then_some(Event::Flattened { id, ok: false })
                }
                _ => {
                    let ok = store_prepared(id, flatten(image, &plan, after_error));
                    Some(Event::Flattened { id, ok })
                }
            }
        }
        Cmd::Dispatch { id, how } => {
            kill_timer(id);
            let slot = take_slot(id)?;
            let prepared = match (how, slot.image, slot.prepared) {
                (Ready::Flattened, _, Some(p)) => Some(p),
                (Ready::Unscanned, Some(image), _) => Some(PreparedOutput::unscanned(owned(image))),
                (Ready::NoFindings, Some(image), _) => {
                    Some(PreparedOutput::no_findings(owned(image)))
                }
                (Ready::ReviewedNone, Some(image), _) => {
                    Some(PreparedOutput::reviewed_none(owned(image)))
                }
                // Không có ảnh khớp với cách sẵn sàng: không xuất.
                _ => None,
            };
            if prepared.is_none() {
                notify(Note::RedactFailed);
            }
            deliver(slot.action, slot.done, prepared);
            None
        }
        Cmd::Drop { id } => {
            kill_timer(id);
            if let Some(slot) = take_slot(id) {
                deliver(slot.action, slot.done, None);
            }
            None
        }
        Cmd::Notify(note) => {
            notify(note);
            None
        }
        Cmd::CloseAll => {
            review_window::close_all();
            // Thoát: không gọi `done` (bên chờ cũng đang bị hủy); ảnh và closure bị thả.
            let slots: Vec<Slot> = SLOTS.with(|s| std::mem::take(&mut *s.borrow_mut()));
            for s in &slots {
                kill_timer(s.id);
            }
            drop(slots);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::geometry::Rect;
    use darkshot_core::privacy::{Category, Confidence, Mapping, RuleId};

    fn finding(id: u32, x: i32) -> Finding {
        Finding {
            id,
            category: Category::Email,
            confidence: Confidence::High,
            rects: vec![Rect {
                x,
                y: 4,
                w: 30,
                h: 10,
            }],
            rule: RuleId::EmailPlain,
            mapping: Mapping::Word,
        }
    }

    fn plan(xs: &[i32]) -> RedactionPlan {
        RedactionPlan {
            rects: xs
                .iter()
                .map(|&x| Rect {
                    x,
                    y: 4,
                    w: 30,
                    h: 10,
                })
                .collect(),
        }
    }

    fn dispatches(cmds: &[Cmd]) -> Vec<(Id, Ready)> {
        cmds.iter()
            .filter_map(|c| match c {
                Cmd::Dispatch { id, how } => Some((*id, *how)),
                _ => None,
            })
            .collect()
    }

    /// Nhận một lượt Shield bật; trả (mã lượt, mã lượt quét).
    fn accept(g: &mut GateCore, windowed: bool) -> (Id, Id) {
        let (id, cmds) = g.submit(true, windowed);
        let id = id.expect("accepted");
        let scan = match cmds.as_slice() {
            [Cmd::StartScan { id: i, scan }, Cmd::StartTimer { id: t }] => {
                assert_eq!((*i, *t), (id, id));
                *scan
            }
            other => panic!("unexpected {other:?}"),
        };
        (id, scan)
    }

    fn done(scan: Id, f: &[Finding]) -> Event {
        Event::ScanDone {
            scan,
            result: Ok(f.to_vec()),
        }
    }

    fn failed(scan: Id, e: ScanError) -> Event {
        Event::ScanDone {
            scan,
            result: Err(e),
        }
    }

    fn outcome(id: Id, outcome: ReviewOutcome) -> Event {
        Event::Outcome { id, outcome }
    }

    #[test]
    fn message_ids() {
        // WM_APP+1..+18 đã dùng ở nơi khác (xem `main::tests::steps_message_ids`).
        assert_eq!(WM_PRIVACY_SCAN, WM_APP + 19);
        assert_eq!(WM_PRIVACY_REVIEW, WM_APP + 20);
        assert_eq!(WM_PRIVACY_PROGRESS, WM_APP + 21);
        assert_eq!(WM_PRIVACY_FLAT, WM_APP + 22);
        assert_eq!(MAX_PENDING, 3);
        assert_eq!(SCANNING_WINDOW_DELAY_MS, 800);
    }

    #[test]
    fn shield_off_dispatches_once_unscanned() {
        let mut g = GateCore::new();
        let (id, cmds) = g.submit(false, false);
        let id = id.unwrap();
        assert_eq!(
            cmds,
            vec![Cmd::Dispatch {
                id,
                how: Ready::Unscanned
            }]
        );
        assert_eq!(g.pending(), 0);
        // Không còn gì chạy lần hai.
        assert!(g.step(Event::Timer { id }, false).is_empty());
        assert!(g.step(done(id, &[]), false).is_empty());
        assert!(g.step(Event::OverlayClosed, false).is_empty());
    }

    #[test]
    fn shield_off_ignores_full_queue_and_overlay() {
        let mut g = GateCore::new();
        for _ in 0..MAX_PENDING {
            accept(&mut g, false);
        }
        // Shield tắt (đổi công tắc khi còn lượt chờ): hành vi như trước, không xếp hàng.
        let (id, cmds) = g.submit(false, true);
        assert_eq!(dispatches(&cmds), vec![(id.unwrap(), Ready::Unscanned)]);
        assert_eq!(g.pending(), MAX_PENDING);
    }

    #[test]
    fn no_findings_dispatches_without_window() {
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        assert_eq!(g.pending(), 1);
        let cmds = g.step(done(scan, &[]), false);
        assert_eq!(
            cmds,
            vec![Cmd::Dispatch {
                id,
                how: Ready::NoFindings
            }]
        );
        assert_eq!(g.pending(), 0);
        // Hẹn giờ muộn và kết quả trùng: bỏ.
        assert!(g.step(Event::Timer { id }, false).is_empty());
        assert!(g.step(done(scan, &[]), false).is_empty());
        assert!(g.step(done(scan, &[finding(0, 1)]), false).is_empty());
    }

    #[test]
    fn slow_scan_opens_scanning_window_then_closes_it() {
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        let cmds = g.step(Event::Timer { id }, false);
        assert_eq!(
            cmds,
            vec![Cmd::OpenReview {
                id,
                phase: ReviewPhase::Scanning { tile: None },
                findings: Vec::new()
            }]
        );
        assert_eq!(g.open(), Some(id));
        assert_eq!(g.shown_for_scan(scan), Some(id));
        // Hẹn giờ trùng: không mở lần hai.
        assert!(g.step(Event::Timer { id }, false).is_empty());
        let cmds = g.step(done(scan, &[]), false);
        assert_eq!(
            cmds,
            vec![
                Cmd::CloseReview { id },
                Cmd::Dispatch {
                    id,
                    how: Ready::NoFindings
                }
            ]
        );
        assert_eq!((g.pending(), g.open()), (0, None));
    }

    #[test]
    fn slow_scan_with_findings_updates_the_open_window() {
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        g.step(Event::Timer { id }, false);
        let f = vec![finding(0, 1), finding(1, 50)];
        let cmds = g.step(done(scan, &f), false);
        assert_eq!(
            cmds,
            vec![Cmd::Update {
                id,
                result: Ok(f.clone())
            }]
        );
        assert_eq!(g.shown_for_scan(scan), None);
        // Che một phần: plan đi nguyên vẹn tới lệnh làm phẳng.
        let cmds = g.step(outcome(id, ReviewOutcome::Apply(plan(&[50]))), false);
        assert_eq!(
            cmds,
            vec![Cmd::Flatten {
                id,
                plan: plan(&[50]),
                after_error: false
            }]
        );
        assert!(g.is_flattening(id));
        assert_eq!((g.pending(), g.open()), (1, None));
        let cmds = g.step(Event::Flattened { id, ok: true }, false);
        assert_eq!(
            cmds,
            vec![Cmd::Dispatch {
                id,
                how: Ready::Flattened
            }]
        );
        assert_eq!(g.pending(), 0);
    }

    #[test]
    fn findings_open_review_and_apply_all() {
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        let f = vec![finding(0, 1), finding(1, 50)];
        let cmds = g.step(done(scan, &f), false);
        assert_eq!(
            cmds,
            vec![Cmd::OpenReview {
                id,
                phase: ReviewPhase::Findings,
                findings: f.clone()
            }]
        );
        // Chưa có quyết định: không Dispatch.
        assert!(g.step(Event::Timer { id }, false).is_empty());
        let cmds = g.step(outcome(id, ReviewOutcome::Apply(plan(&[1, 50]))), false);
        assert_eq!(
            cmds,
            vec![Cmd::Flatten {
                id,
                plan: plan(&[1, 50]),
                after_error: false
            }]
        );
        assert_eq!(
            dispatches(&g.step(Event::Flattened { id, ok: true }, false)),
            vec![(id, Ready::Flattened)]
        );
    }

    #[test]
    fn continue_none_dispatches_reviewed_none() {
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        g.step(done(scan, &[finding(0, 1)]), false);
        let cmds = g.step(outcome(id, ReviewOutcome::ContinueNone), false);
        assert_eq!(
            cmds,
            vec![Cmd::Dispatch {
                id,
                how: Ready::ReviewedNone
            }]
        );
        assert_eq!((g.pending(), g.open()), (0, None));
    }

    #[test]
    fn cancel_drops_and_nothing_runs_later() {
        // Hủy khi đang duyệt.
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        g.step(done(scan, &[finding(0, 1)]), false);
        assert_eq!(
            g.step(outcome(id, ReviewOutcome::Cancel), false),
            vec![Cmd::Drop { id }]
        );
        assert_eq!((g.pending(), g.open()), (0, None));
        for ev in [
            outcome(id, ReviewOutcome::Apply(plan(&[1]))),
            outcome(id, ReviewOutcome::ContinueNone),
            done(scan, &[]),
            Event::Flattened { id, ok: true },
            Event::OverlayClosed,
        ] {
            assert!(g.step(ev, false).is_empty());
        }
        // Hủy ở cửa sổ "Đang quét…": kết quả quét tới sau bị bỏ.
        let (id, scan) = accept(&mut g, false);
        g.step(Event::Timer { id }, false);
        assert_eq!(
            g.step(outcome(id, ReviewOutcome::Cancel), false),
            vec![Cmd::Drop { id }]
        );
        assert!(g.step(done(scan, &[]), false).is_empty());
        assert_eq!(g.pending(), 0);
    }

    #[test]
    fn scan_error_never_dispatches_without_confirmation() {
        for e in [
            ScanError::NoLanguage,
            ScanError::Failed,
            ScanError::Timeout,
            ScanError::TooLarge,
        ] {
            let mut g = GateCore::new();
            let (id, scan) = accept(&mut g, false);
            let cmds = g.step(failed(scan, e), false);
            assert_eq!(
                cmds,
                vec![Cmd::OpenReview {
                    id,
                    phase: ReviewPhase::Error(e),
                    findings: Vec::new()
                }]
            );
            assert_eq!(g.pending(), 1);
            // Hai lần bấm xác nhận nằm trong cửa sổ; cổng chỉ đi tiếp khi nhận ContinueUnscanned.
            let cmds = g.step(
                outcome(id, ReviewOutcome::ContinueUnscanned(plan(&[]))),
                false,
            );
            assert_eq!(
                cmds,
                vec![Cmd::Flatten {
                    id,
                    plan: plan(&[]),
                    after_error: true
                }]
            );
            assert_eq!(
                dispatches(&g.step(Event::Flattened { id, ok: true }, false)),
                vec![(id, Ready::Flattened)]
            );
        }
    }

    #[test]
    fn outcome_not_matching_the_scan_state_fails_closed() {
        // Lỗi quét mà nhận Apply / ContinueNone: không bao giờ thành "đã quét".
        for bad in [
            ReviewOutcome::Apply(plan(&[1])),
            ReviewOutcome::ContinueNone,
        ] {
            let mut g = GateCore::new();
            let (id, scan) = accept(&mut g, false);
            g.step(failed(scan, ScanError::Failed), false);
            let cmds = g.step(outcome(id, bad), false);
            assert_eq!(
                cmds,
                vec![Cmd::Notify(Note::RedactFailed), Cmd::Drop { id }]
            );
            assert_eq!((g.pending(), g.open()), (0, None));
        }
        // Quét được mà nhận ContinueUnscanned, hoặc Apply với plan rỗng: không xuất.
        for bad in [
            ReviewOutcome::ContinueUnscanned(plan(&[1])),
            ReviewOutcome::Apply(plan(&[])),
        ] {
            let mut g = GateCore::new();
            let (id, scan) = accept(&mut g, false);
            g.step(done(scan, &[finding(0, 1)]), false);
            let cmds = g.step(outcome(id, bad), false);
            assert_eq!(
                cmds,
                vec![Cmd::Notify(Note::RedactFailed), Cmd::Drop { id }]
            );
        }
        // Cửa sổ "Đang quét…" mà nhận kết quả cuối khác Hủy: không xuất.
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        g.step(Event::Timer { id }, false);
        let cmds = g.step(outcome(id, ReviewOutcome::ContinueNone), false);
        assert_eq!(
            cmds,
            vec![Cmd::Notify(Note::RedactFailed), Cmd::Drop { id }]
        );
        assert!(g.step(done(scan, &[]), false).is_empty());
    }

    #[test]
    fn retry_rescans_with_a_new_token_and_old_results_are_dropped() {
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        g.step(failed(scan, ScanError::Timeout), false);
        let cmds = g.step(outcome(id, ReviewOutcome::Retry), false);
        let scan2 = match cmds.as_slice() {
            [Cmd::StartScan { id: i, scan: s }] => {
                assert_eq!(*i, id);
                *s
            }
            other => panic!("unexpected {other:?}"),
        };
        assert_ne!(scan2, scan);
        assert_eq!((g.pending(), g.open()), (1, Some(id)));
        assert_eq!(g.shown_for_scan(scan2), Some(id));
        // Thử lại lần hai khi đang quét: bỏ.
        assert!(g.step(outcome(id, ReviewOutcome::Retry), false).is_empty());
        // Kết quả muộn của lượt quét cũ (quá hạn): bỏ, kể cả khi nó "sạch".
        assert!(g.step(done(scan, &[]), false).is_empty());
        assert_eq!(g.pending(), 1);
        // Lỗi lần nữa: về cửa sổ.
        let cmds = g.step(failed(scan2, ScanError::Failed), false);
        assert_eq!(
            cmds,
            vec![Cmd::Update {
                id,
                result: Err(ScanError::Failed)
            }]
        );
        let cmds = g.step(outcome(id, ReviewOutcome::Retry), false);
        let scan3 = match cmds.as_slice() {
            [Cmd::StartScan { scan: s, .. }] => *s,
            other => panic!("unexpected {other:?}"),
        };
        assert!(scan3 != scan && scan3 != scan2);
        // Lần này sạch: sau Thử lại kết quả luôn về cửa sổ (vùng tự thêm còn đó), không tự chạy.
        let cmds = g.step(done(scan3, &[]), false);
        assert_eq!(
            cmds,
            vec![Cmd::Update {
                id,
                result: Ok(Vec::new())
            }]
        );
        assert_eq!(g.pending(), 1);
        // Không phát hiện + người dùng đi tiếp không che = "không phát hiện".
        let cmds = g.step(outcome(id, ReviewOutcome::ContinueNone), false);
        assert_eq!(dispatches(&cmds), vec![(id, Ready::NoFindings)]);
        assert_eq!(g.pending(), 0);
    }

    #[test]
    fn retry_then_manual_area_is_flattened() {
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        g.step(failed(scan, ScanError::Failed), false);
        let cmds = g.step(outcome(id, ReviewOutcome::Retry), false);
        let [Cmd::StartScan { scan: scan2, .. }] = cmds.as_slice() else {
            panic!("unexpected {cmds:?}");
        };
        g.step(done(*scan2, &[]), false);
        let cmds = g.step(outcome(id, ReviewOutcome::Apply(plan(&[7]))), false);
        assert_eq!(
            cmds,
            vec![Cmd::Flatten {
                id,
                plan: plan(&[7]),
                after_error: false
            }]
        );
    }

    #[test]
    fn retry_is_ignored_unless_the_scan_failed() {
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        g.step(done(scan, &[finding(0, 1)]), false);
        assert!(g.step(outcome(id, ReviewOutcome::Retry), false).is_empty());
        assert_eq!((g.pending(), g.open()), (1, Some(id)));
    }

    #[test]
    fn duplicate_late_and_foreign_messages_run_nothing_twice() {
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        // Mã lạ.
        assert!(g.step(done(scan + 100, &[]), false).is_empty());
        assert!(g
            .step(outcome(id + 100, ReviewOutcome::ContinueNone), false)
            .is_empty());
        assert!(g.step(Event::Timer { id: id + 100 }, false).is_empty());
        assert!(g
            .step(
                Event::Flattened {
                    id: id + 100,
                    ok: true
                },
                false
            )
            .is_empty());
        // Kết quả duyệt khi chưa có cửa sổ nào mở cho lượt này.
        assert!(g
            .step(outcome(id, ReviewOutcome::ContinueNone), false)
            .is_empty());
        assert!(g.step(outcome(id, ReviewOutcome::Cancel), false).is_empty());
        // Đã làm phẳng xong khi chưa ai yêu cầu.
        assert!(g.step(Event::Flattened { id, ok: true }, false).is_empty());
        assert_eq!(g.pending(), 1);
        g.step(done(scan, &[finding(0, 1)]), false);
        // Kết quả quét trùng: không mở lần hai, không đổi trạng thái.
        assert!(g.step(done(scan, &[]), false).is_empty());
        assert!(g.step(done(scan, &[finding(0, 1)]), false).is_empty());
        let first = g.step(outcome(id, ReviewOutcome::Apply(plan(&[1]))), false);
        assert_eq!(first.len(), 1);
        // Apply trùng: một lệnh làm phẳng.
        assert!(g
            .step(outcome(id, ReviewOutcome::Apply(plan(&[1]))), false)
            .is_empty());
        assert!(g.step(outcome(id, ReviewOutcome::Cancel), false).is_empty());
        assert_eq!(
            dispatches(&g.step(Event::Flattened { id, ok: true }, false)),
            vec![(id, Ready::Flattened)]
        );
        // Flattened trùng: một Dispatch.
        assert!(g.step(Event::Flattened { id, ok: true }, false).is_empty());
        assert!(g.step(Event::Flattened { id, ok: false }, false).is_empty());
        assert_eq!(g.pending(), 0);
    }

    #[test]
    fn queue_is_limited_to_three_including_the_open_review() {
        let mut g = GateCore::new();
        let (a, sa) = accept(&mut g, false);
        let (b, sb) = accept(&mut g, false);
        let (_c, _sc) = accept(&mut g, false);
        g.step(done(sa, &[finding(0, 1)]), false);
        assert_eq!(g.open(), Some(a));
        let (id, cmds) = g.submit(true, false);
        assert_eq!(id, None);
        assert_eq!(cmds, vec![Cmd::Notify(Note::QueueFull)]);
        assert_eq!(g.pending(), MAX_PENDING);
        // Đang làm phẳng vẫn tính (ảnh còn trong bộ nhớ).
        g.step(outcome(a, ReviewOutcome::Apply(plan(&[1]))), false);
        assert_eq!(g.submit(true, false).0, None);
        g.step(Event::Flattened { id: a, ok: true }, false);
        assert_eq!(g.pending(), 2);
        // Có chỗ: nhận tiếp.
        let (d, _sd) = accept(&mut g, false);
        assert_eq!(g.pending(), 3);
        assert_eq!(g.submit(true, true).0, None);
        // Lượt bị từ chối không để lại gì.
        g.step(done(sb, &[]), false);
        assert_eq!(g.pending(), 2);
        assert!(d > b);
    }

    #[test]
    fn one_review_window_at_a_time_in_order() {
        let mut g = GateCore::new();
        let (a, sa) = accept(&mut g, false);
        let (b, sb) = accept(&mut g, false);
        let (c, sc) = accept(&mut g, false);
        // b xong trước: mở cho b.
        let cmds = g.step(done(sb, &[finding(0, 1)]), false);
        assert!(matches!(cmds.as_slice(), [Cmd::OpenReview { id, .. }] if *id == b));
        // c lỗi, a có phát hiện: chờ.
        assert!(g.step(failed(sc, ScanError::Failed), false).is_empty());
        assert!(g.step(done(sa, &[finding(0, 9)]), false).is_empty());
        // a quét chậm trong lúc chờ: không mở cửa sổ thứ hai.
        assert!(g.step(Event::Timer { id: a }, false).is_empty());
        assert_eq!(g.open(), Some(b));
        // b xong: tới a (nhận trước c), rồi c.
        let cmds = g.step(outcome(b, ReviewOutcome::ContinueNone), false);
        assert_eq!(dispatches(&cmds), vec![(b, Ready::ReviewedNone)]);
        assert!(cmds.iter().any(|x| matches!(x,
            Cmd::OpenReview { id, phase: ReviewPhase::Findings, findings } if *id == a && findings.len() == 1)));
        let cmds = g.step(outcome(a, ReviewOutcome::Cancel), false);
        assert_eq!(cmds[0], Cmd::Drop { id: a });
        assert!(matches!(&cmds[1],
            Cmd::OpenReview { id, phase: ReviewPhase::Error(ScanError::Failed), .. } if *id == c));
        assert_eq!(cmds.len(), 2);
        assert_eq!(g.open(), Some(c));
    }

    #[test]
    fn overlay_defers_review_windows_until_it_closes() {
        let mut g = GateCore::new();
        let (a, sa) = accept(&mut g, false);
        let (b, _sb) = accept(&mut g, false);
        // Lớp phủ đang mở: không mở cửa sổ nào.
        assert!(g.step(done(sa, &[finding(0, 1)]), true).is_empty());
        assert!(g.step(Event::Timer { id: b }, true).is_empty());
        assert_eq!(g.open(), None);
        let cmds = g.step(Event::OverlayClosed, false);
        assert!(matches!(cmds.as_slice(),
            [Cmd::OpenReview { id, phase: ReviewPhase::Findings, .. }] if *id == a));
        // a xong: b (đang quét chậm) được cửa sổ "Đang quét…".
        let cmds = g.step(outcome(a, ReviewOutcome::Cancel), false);
        assert_eq!(cmds[0], Cmd::Drop { id: a });
        assert!(matches!(&cmds[1],
            Cmd::OpenReview { id, phase: ReviewPhase::Scanning { tile: None }, .. } if *id == b));
    }

    #[test]
    fn overlay_holds_windowed_outputs_only() {
        let mut g = GateCore::new();
        let (pin, sp) = accept(&mut g, true);
        let (save, ss) = accept(&mut g, false);
        // Lưu: chạy ngay cả khi lớp phủ mở (không mở cửa sổ nào).
        assert_eq!(
            dispatches(&g.step(done(ss, &[]), true)),
            vec![(save, Ready::NoFindings)]
        );
        // Ghim: giữ tới khi lớp phủ đóng; vẫn tính trong hàng.
        assert!(g.step(done(sp, &[]), true).is_empty());
        assert_eq!(g.pending(), 1);
        assert!(g.step(done(sp, &[]), true).is_empty());
        let cmds = g.step(Event::OverlayClosed, false);
        assert_eq!(
            cmds,
            vec![Cmd::Dispatch {
                id: pin,
                how: Ready::NoFindings
            }]
        );
        assert!(g.step(Event::OverlayClosed, false).is_empty());
        assert_eq!(g.pending(), 0);
        // Ghim đã che, làm phẳng xong lúc lớp phủ mở: cũng giữ.
        let (pin, sp) = accept(&mut g, true);
        g.step(done(sp, &[finding(0, 1)]), false);
        g.step(outcome(pin, ReviewOutcome::Apply(plan(&[1]))), true);
        assert!(g
            .step(Event::Flattened { id: pin, ok: true }, true)
            .is_empty());
        assert_eq!(
            dispatches(&g.step(Event::OverlayClosed, false)),
            vec![(pin, Ready::Flattened)]
        );
    }

    #[test]
    fn shutdown_drops_everything_and_nothing_runs_after() {
        let mut g = GateCore::new();
        let (a, sa) = accept(&mut g, false); // sẽ đang duyệt
        let (b, sb) = accept(&mut g, true); // sẽ đang quét
        g.step(done(sa, &[finding(0, 1)]), false);
        let cmds = g.step(Event::Shutdown, false);
        assert_eq!(
            cmds,
            vec![Cmd::CloseAll, Cmd::Drop { id: a }, Cmd::Drop { id: b }]
        );
        assert_eq!((g.pending(), g.open()), (0, None));
        for ev in [
            done(sb, &[]),
            outcome(a, ReviewOutcome::Apply(plan(&[1]))),
            outcome(a, ReviewOutcome::ContinueNone),
            Event::Flattened { id: a, ok: true },
            Event::Timer { id: b },
            Event::OverlayClosed,
            Event::Shutdown,
        ] {
            assert!(g.step(ev, false).is_empty());
        }
        // Sau khi đóng: Shield bật thì từ chối im lặng; tắt thì như trước.
        assert_eq!(g.submit(true, false), (None, Vec::new()));
        let (id, cmds) = g.submit(false, false);
        assert_eq!(dispatches(&cmds), vec![(id.unwrap(), Ready::Unscanned)]);
    }

    #[test]
    fn shutdown_drops_flattening_and_held() {
        let mut g = GateCore::new();
        let (a, sa) = accept(&mut g, false);
        let (b, sb) = accept(&mut g, true);
        g.step(done(sa, &[finding(0, 1)]), false);
        g.step(outcome(a, ReviewOutcome::Apply(plan(&[1]))), false);
        g.step(done(sb, &[]), true);
        assert_eq!(g.pending(), 2);
        let cmds = g.step(Event::Shutdown, false);
        assert_eq!(
            cmds,
            vec![Cmd::CloseAll, Cmd::Drop { id: a }, Cmd::Drop { id: b }]
        );
        assert!(g
            .step(Event::Flattened { id: a, ok: true }, false)
            .is_empty());
        assert!(g.step(Event::OverlayClosed, false).is_empty());
    }

    #[test]
    fn review_window_failure_does_not_export() {
        let mut g = GateCore::new();
        let (a, sa) = accept(&mut g, false);
        let (b, sb) = accept(&mut g, false);
        g.step(done(sa, &[finding(0, 1)]), false);
        g.step(failed(sb, ScanError::Failed), false);
        let cmds = g.step(Event::OpenFailed { id: a }, false);
        assert_eq!(cmds[0], Cmd::Notify(Note::WindowFailed));
        assert_eq!(cmds[1], Cmd::Drop { id: a });
        // Lượt kế được thử mở.
        assert!(matches!(&cmds[2], Cmd::OpenReview { id, .. } if *id == b));
        assert_eq!(cmds.len(), 3);
        let cmds = g.step(Event::OpenFailed { id: b }, false);
        assert_eq!(
            cmds,
            vec![Cmd::Notify(Note::WindowFailed), Cmd::Drop { id: b }]
        );
        assert_eq!((g.pending(), g.open()), (0, None));
        // OpenFailed cho lượt không có cửa sổ đang mở: bỏ.
        let (c, _) = accept(&mut g, false);
        assert!(g.step(Event::OpenFailed { id: c }, false).is_empty());
        assert_eq!(g.pending(), 1);
    }

    #[test]
    fn flatten_failure_does_not_export() {
        let mut g = GateCore::new();
        let (id, scan) = accept(&mut g, false);
        g.step(done(scan, &[finding(0, 1)]), false);
        g.step(outcome(id, ReviewOutcome::Apply(plan(&[1]))), false);
        let cmds = g.step(Event::Flattened { id, ok: false }, false);
        assert_eq!(
            cmds,
            vec![Cmd::Notify(Note::RedactFailed), Cmd::Drop { id }]
        );
        assert!(g.step(Event::Flattened { id, ok: true }, false).is_empty());
        assert_eq!(g.pending(), 0);
    }

    #[test]
    fn review_window_gone_without_outcome_frees_the_queue() {
        let mut g = GateCore::new();
        let (a, sa) = accept(&mut g, false);
        let (b, sb) = accept(&mut g, false);
        g.step(done(sa, &[finding(0, 1)]), false);
        g.step(done(sb, &[finding(0, 1)]), false);
        // Không phải lượt đang mở: bỏ.
        assert!(g.step(Event::ReviewGone { id: b }, false).is_empty());
        let cmds = g.step(Event::ReviewGone { id: a }, false);
        assert_eq!(cmds[0], Cmd::Drop { id: a });
        assert!(matches!(&cmds[1], Cmd::OpenReview { id, .. } if *id == b));
        // Kết quả muộn của cửa sổ đã mất: không chạy gì.
        assert!(g
            .step(outcome(a, ReviewOutcome::ContinueNone), false)
            .is_empty());
    }

    /// Mọi dãy sự kiện: mỗi lượt kết thúc nhiều nhất một lần (Dispatch hoặc Drop), Dispatch của lượt
    /// Shield bật chỉ có sau một kết quả quét sạch của đúng lượt quét hiện hành hoặc một quyết định
    /// của người dùng, hàng không vượt `MAX_PENDING`, và sau Shutdown mọi lượt đã kết thúc.
    #[test]
    fn random_event_sequences_keep_exactly_once() {
        struct Rng(u64);
        impl Rng {
            fn next(&mut self, n: usize) -> usize {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((self.0 >> 33) as usize) % n
            }
        }
        #[derive(Default, Clone)]
        struct Seen {
            shield: bool,
            scans: Vec<Id>,
            dispatched: u32,
            dropped: u32,
            authorized: bool,
            flatten_ok: bool,
        }
        // Đếm để chắc dãy ngẫu nhiên thật sự đi qua mọi kiểu kết thúc.
        let mut kinds = [0u32; 4];
        let mut drops = 0u32;
        let mut reviews = 0u32;
        for seed in 0..400u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) + 1);
            let mut g = GateCore::new();
            let mut seen: Vec<(Id, Seen)> = Vec::new();
            let mut overlay = false;
            let mut shut = false;
            let steps = 60 + rng.next(60);
            for step in 0..=steps {
                let ids: Vec<Id> = seen.iter().map(|(i, _)| *i).collect();
                let pick = |rng: &mut Rng| -> Id {
                    if ids.is_empty() {
                        0
                    } else {
                        ids[rng.next(ids.len())]
                    }
                };
                let cmds = if step == steps {
                    shut = true;
                    g.step(Event::Shutdown, overlay)
                } else {
                    match rng.next(12) {
                        0 | 1 => {
                            let shield = rng.next(5) != 0;
                            let (id, cmds) = g.submit(shield, rng.next(2) == 0);
                            if let Some(id) = id {
                                seen.push((
                                    id,
                                    Seen {
                                        shield,
                                        ..Seen::default()
                                    },
                                ));
                            }
                            cmds
                        }
                        2..=4 => {
                            let id = pick(&mut rng);
                            let s = seen.iter().find(|(i, _)| *i == id).map(|(_, s)| s.clone());
                            let scans = s.map(|s| s.scans).unwrap_or_default();
                            // Mã quét bất kỳ đã cấp cho lượt (cũ hoặc mới), hoặc mã lạ.
                            let scan = if scans.is_empty() {
                                rng.next(50)
                            } else {
                                scans[rng.next(scans.len())]
                            };
                            let clean = rng.next(3) == 0;
                            let result = match rng.next(3) {
                                0 => Err(ScanError::Failed),
                                _ if clean => Ok(Vec::new()),
                                _ => Ok(vec![finding(0, 1)]),
                            };
                            // Kết quả sạch của lượt quét HIỆN HÀNH (của lượt nào giữ mã đó) mới cho
                            // phép đi tiếp.
                            if matches!(&result, Ok(f) if f.is_empty()) {
                                if let Some((_, s)) =
                                    seen.iter_mut().find(|(_, s)| s.scans.last() == Some(&scan))
                                {
                                    s.authorized = true;
                                }
                            }
                            g.step(Event::ScanDone { scan, result }, overlay)
                        }
                        5 => g.step(Event::Timer { id: pick(&mut rng) }, overlay),
                        6..=8 => {
                            let id = pick(&mut rng);
                            let outcome = match rng.next(5) {
                                0 => ReviewOutcome::Cancel,
                                1 => ReviewOutcome::Retry,
                                2 => ReviewOutcome::ContinueNone,
                                3 => ReviewOutcome::Apply(plan(&[1])),
                                _ => ReviewOutcome::ContinueUnscanned(plan(&[])),
                            };
                            if !matches!(outcome, ReviewOutcome::Cancel | ReviewOutcome::Retry) {
                                if let Some((_, s)) = seen.iter_mut().find(|(i, _)| *i == id) {
                                    s.authorized = true;
                                }
                            }
                            g.step(Event::Outcome { id, outcome }, overlay)
                        }
                        9 => {
                            let id = pick(&mut rng);
                            let ok = rng.next(4) != 0;
                            if ok && g.is_flattening(id) {
                                if let Some((_, s)) = seen.iter_mut().find(|(i, _)| *i == id) {
                                    s.flatten_ok = true;
                                }
                            }
                            g.step(Event::Flattened { id, ok }, overlay)
                        }
                        10 => {
                            overlay = !overlay;
                            if overlay {
                                Vec::new()
                            } else {
                                g.step(Event::OverlayClosed, false)
                            }
                        }
                        _ => match rng.next(2) {
                            0 => g.step(Event::ReviewGone { id: pick(&mut rng) }, overlay),
                            _ => g.step(Event::OpenFailed { id: pick(&mut rng) }, overlay),
                        },
                    }
                };
                let mut opened = 0;
                for c in &cmds {
                    match c {
                        Cmd::StartScan { id, scan } => {
                            let s = &mut seen.iter_mut().find(|(i, _)| i == id).unwrap().1;
                            assert!(s.shield, "seed {seed}: scan for a shield-off request");
                            assert!(!s.scans.contains(scan), "seed {seed}: scan token reused");
                            s.scans.push(*scan);
                        }
                        Cmd::Dispatch { id, how } => {
                            let s = &mut seen.iter_mut().find(|(i, _)| i == id).unwrap().1;
                            s.dispatched += 1;
                            kinds[match how {
                                Ready::Unscanned => 0,
                                Ready::NoFindings => 1,
                                Ready::ReviewedNone => 2,
                                Ready::Flattened => 3,
                            }] += 1;
                            assert_eq!(*how == Ready::Unscanned, !s.shield, "seed {seed}: {how:?}");
                            if s.shield {
                                assert!(
                                    s.authorized,
                                    "seed {seed}: dispatch without scan/decision"
                                );
                            }
                            if *how == Ready::Flattened {
                                assert!(s.flatten_ok, "seed {seed}: no flattened image");
                            }
                            assert!(
                                !overlay || !matches!(how, Ready::Unscanned) || !s.shield,
                                "seed {seed}"
                            );
                        }
                        Cmd::Drop { id } => {
                            seen.iter_mut().find(|(i, _)| i == id).unwrap().1.dropped += 1;
                            drops += 1;
                        }
                        Cmd::OpenReview { id, .. } => {
                            opened += 1;
                            reviews += 1;
                            assert!(!overlay, "seed {seed}: review window under the overlay");
                            assert_eq!(g.open(), Some(*id), "seed {seed}");
                        }
                        _ => {}
                    }
                }
                assert!(opened <= 1, "seed {seed}: two review windows in one step");
                assert!(g.pending() <= MAX_PENDING, "seed {seed}");
                for (id, s) in &seen {
                    assert!(
                        s.dispatched + s.dropped <= 1,
                        "seed {seed}: request {id} ended {} + {} times",
                        s.dispatched,
                        s.dropped
                    );
                }
            }
            assert!(shut);
            assert_eq!((g.pending(), g.open()), (0, None), "seed {seed}");
            for (id, s) in &seen {
                assert_eq!(
                    s.dispatched + s.dropped,
                    1,
                    "seed {seed}: request {id} never ended"
                );
            }
        }
        assert!(kinds.iter().all(|n| *n > 20), "{kinds:?}");
        assert!(drops > 100 && reviews > 100, "{drops} {reviews}");
    }

    // -----------------------------------------------------------------------------------------
    // Vỏ của cổng với cửa sổ ẩn thật: đầu ra giả (sink tiêm được), không tệp, không mạng.
    // -----------------------------------------------------------------------------------------

    use std::rc::Rc;
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, DispatchMessageW, FindWindowW, TranslateMessage,
        HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_KEYDOWN,
    };

    thread_local! {
        static GOT: RefCell<Vec<(Msg, Protection, Image)>> = const { RefCell::new(Vec::new()) };
        static NOTES: RefCell<Vec<(NoticeKind, String)>> = const { RefCell::new(Vec::new()) };
        static SHIELD: Cell<bool> = const { Cell::new(false) };
    }

    fn fake_sink(action: OutputAction, prepared: PreparedOutput) {
        let label = action.label();
        let protection = prepared.protection();
        GOT.with(|g| {
            g.borrow_mut()
                .push((label, protection, prepared.into_image()))
        });
    }

    fn fake_notify(kind: NoticeKind, _title: &str, body: &str) {
        NOTES.with(|n| n.borrow_mut().push((kind, body.to_string())));
    }

    fn shield() -> bool {
        SHIELD.with(|s| s.get())
    }

    fn got() -> usize {
        GOT.with(|g| g.borrow().len())
    }

    fn pending() -> usize {
        CORE.with(|c| c.borrow().pending())
    }

    /// Cửa sổ ẩn đóng vai cửa sổ chính; hủy thì cổng đóng theo.
    struct Host(HWND);

    impl Host {
        /// Đầu ra giả, có cửa sổ nhận thông điệp.
        fn new(shield_on: bool) -> Host {
            // SAFETY: lớp "STATIC" có sẵn; con của HWND_MESSAGE không hiện; hủy trong Drop.
            let h = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!("darkshot-gate-test"),
                    WINDOW_STYLE(0),
                    0,
                    0,
                    0,
                    0,
                    Some(HWND_MESSAGE),
                    None,
                    None,
                    None,
                )
            }
            .expect("message-only window");
            set_window(h);
            wire(shield_on);
            Host(h)
        }
    }

    impl Drop for Host {
        fn drop(&mut self) {
            shutdown();
            // SAFETY: cửa sổ do luồng này tạo, hủy một lần.
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }

    fn wire(shield_on: bool) {
        set_sink(fake_sink);
        set_notifier(fake_notify);
        set_enabled_source(shield);
        SHIELD.with(|s| s.set(shield_on));
    }

    /// Bơm thông điệp của luồng tới khi `done` hoặc hết `ms`; thông điệp của cổng đi `handle_message`.
    fn pump(host: &Host, ms: u64, done: impl Fn() -> bool) -> bool {
        let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        loop {
            let mut m = MSG::default();
            // SAFETY: bơm thông điệp của luồng kiểm thử.
            unsafe {
                while PeekMessageW(&mut m, None, 0, 0, PM_REMOVE).as_bool() {
                    if m.hwnd == host.0
                        && handle_message(m.hwnd, m.message, m.wParam, m.lParam).is_some()
                    {
                        continue;
                    }
                    if review_window::pre_translate(&m) {
                        continue;
                    }
                    let _ = TranslateMessage(&m);
                    DispatchMessageW(&m);
                }
            }
            if done() {
                return true;
            }
            if std::time::Instant::now() >= end {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    fn sample(w: u32, h: u32) -> Image {
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for i in 0..w * h {
            bgra.extend_from_slice(&[(i % 251) as u8, (i % 13) as u8 * 19, 0x80, 0xFF]);
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    /// Ảnh không hợp lệ: luồng quét trả lỗi ngay, không gọi WinRT.
    fn invalid() -> Image {
        Image {
            width: 0,
            height: 0,
            bgra: Vec::new(),
        }
    }

    #[test]
    fn shield_off_submit_reaches_the_sink_exactly_once() {
        let host = Host::new(false);
        let img = sample(40, 30);
        assert!(!enabled());
        assert_eq!(submit(OutputAction::Save, img.clone()), Ok(()));
        // Chạy ngay trong `submit`, đúng ảnh gốc, `Protection::Disabled`.
        GOT.with(|g| {
            let g = g.borrow();
            assert_eq!(g.len(), 1);
            assert!(g[0].0 == Msg::PrivacyActionSave);
            assert_eq!(g[0].1, Protection::Disabled);
            assert_eq!(g[0].2, img);
        });
        assert_eq!(pending(), 0);
        // Không có gì chạy lần hai (không hẹn giờ, không thông điệp).
        assert!(!pump(&host, 60, || got() > 1));
        assert_eq!(got(), 1);
        assert!(NOTES.with(|n| n.borrow().is_empty()));
        // Mọi loại đầu ra đều đi cùng đường.
        for action in [
            OutputAction::Copy,
            OutputAction::Upload,
            OutputAction::AddStep,
            OutputAction::HistoryCopy,
            OutputAction::HistoryPin,
            OutputAction::Pin {
                place: pin::PinPlace::CenterOnCursorMonitor,
            },
        ] {
            assert_eq!(submit(action, sample(8, 8)), Ok(()));
        }
        assert_eq!(got(), 7);
        assert!(GOT.with(|g| g.borrow().iter().all(|x| x.1 == Protection::Disabled)));
    }

    #[test]
    fn shield_off_submit_with_calls_done_once_and_not_the_sink() {
        let _host = Host::new(false);
        let calls = Rc::new(Cell::new(0u32));
        let c = calls.clone();
        let img = sample(16, 16);
        let want = img.clone();
        let done: Done = Box::new(move |p| {
            c.set(c.get() + 1);
            let p = p.expect("prepared");
            assert_eq!(p.protection(), Protection::Disabled);
            assert_eq!(p.image(), &want);
        });
        let action = OutputAction::StepExport { step: 2, steps: 5 };
        assert_eq!(action.step(), Some((2, 5)));
        assert!(action.label() == Msg::PrivacyActionExportSteps);
        assert_eq!(submit_with(action, img, done), Ok(()));
        assert_eq!(calls.get(), 1);
        assert_eq!(got(), 0);
        assert_eq!(Rc::strong_count(&calls), 1);
    }

    #[test]
    fn pinned_image_goes_straight_only_when_already_scanned() {
        let _host = Host::new(false);
        // Shield tắt: như trước, mọi trạng thái đi thẳng.
        for p in [
            Protection::Disabled,
            Protection::NoFindings,
            Protection::Redacted { regions: 2 },
            Protection::SkippedAfterError { regions: 0 },
        ] {
            assert_eq!(submit_pinned(OutputAction::Copy, sample(8, 8), p), Ok(()));
        }
        assert_eq!(got(), 4);
        assert!(GOT.with(|g| g.borrow().iter().all(|x| x.1 == Protection::Disabled)));
        GOT.with(|g| g.borrow_mut().clear());
        // Shield bật: ảnh đã quét mà không bị sửa đi thẳng, giữ trạng thái.
        SHIELD.with(|s| s.set(true));
        let img = sample(12, 9);
        for p in [Protection::NoFindings, Protection::ReviewedNoneApplied] {
            assert_eq!(submit_pinned(OutputAction::Save, img.clone(), p), Ok(()));
        }
        GOT.with(|g| {
            let g = g.borrow();
            assert_eq!(g.len(), 2);
            assert_eq!(g[0].1, Protection::NoFindings);
            assert_eq!(g[1].1, Protection::ReviewedNoneApplied);
            assert!(g.iter().all(|x| x.2 == img));
        });
        assert_eq!(pending(), 0);
        // Chưa quét (ghim lúc Shield tắt, bỏ qua lỗi) hoặc đã che: xếp hàng quét lại, chưa xuất gì.
        for p in [
            Protection::Disabled,
            Protection::SkippedAfterError { regions: 1 },
            Protection::Redacted { regions: 1 },
        ] {
            assert_eq!(submit_pinned(OutputAction::Save, invalid(), p), Ok(()));
        }
        assert_eq!((got(), pending()), (2, 3));
    }

    #[test]
    fn shield_on_without_a_gate_window_never_exports() {
        // Không có cửa sổ nhận thông điệp: không quét được, không mở được cửa sổ Duyệt.
        wire(true);
        assert_eq!(submit(OutputAction::Save, sample(20, 20)), Ok(()));
        assert_eq!((got(), pending()), (0, 0));
        NOTES.with(|n| {
            let n = n.borrow();
            assert_eq!(n.len(), 1);
            assert_eq!(n[0].0, NoticeKind::Error);
            assert_eq!(n[0].1, crate::lang::tr(Msg::PrivacyWindowFailed));
        });
        // `done` được báo Hủy đúng một lần.
        let calls = Rc::new(Cell::new(0u32));
        let c = calls.clone();
        let done: Done = Box::new(move |p| {
            assert!(p.is_none());
            c.set(c.get() + 1);
        });
        let action = OutputAction::StepExport { step: 1, steps: 1 };
        assert_eq!(submit_with(action, sample(20, 20), done), Ok(()));
        assert_eq!((calls.get(), got(), pending()), (1, 0, 0));
    }

    #[test]
    fn shield_on_queue_full_drops_the_new_image_and_shutdown_drops_the_rest() {
        let host = Host::new(true);
        let calls = Rc::new(Cell::new(0u32));
        let c = calls.clone();
        let done: Done = Box::new(move |_| c.set(c.get() + 1));
        // Không bơm thông điệp: ba lượt nằm ở "đang quét".
        assert_eq!(submit(OutputAction::Save, invalid()), Ok(()));
        assert_eq!(submit(OutputAction::Upload, invalid()), Ok(()));
        let action = OutputAction::StepExport { step: 1, steps: 2 };
        assert_eq!(submit_with(action, invalid(), done), Ok(()));
        assert_eq!((got(), pending()), (0, 3));
        assert_eq!(
            submit(OutputAction::Copy, sample(8, 8)),
            Err(SubmitError::QueueFull)
        );
        assert_eq!((got(), pending()), (0, 3));
        NOTES.with(|n| {
            let n = n.borrow();
            assert_eq!(n.len(), 1);
            assert_eq!(n[0].0, NoticeKind::Info);
            assert_eq!(n[0].1, crate::lang::tr(Msg::PrivacyQueueFull));
        });
        // Đổi công tắc không ảnh hưởng lượt đang chờ; lượt mới (Shield tắt) chạy như trước.
        SHIELD.with(|s| s.set(false));
        assert_eq!(submit(OutputAction::Copy, sample(8, 8)), Ok(()));
        assert_eq!((got(), pending()), (1, 3));
        SHIELD.with(|s| s.set(true));
        // Thoát khi đang chờ: bỏ hết, không ghi gì, không gọi `done` (closure bị thả).
        shutdown();
        assert_eq!((got(), pending()), (1, 0));
        assert_eq!((calls.get(), Rc::strong_count(&calls)), (0, 1));
        assert!(SLOTS.with(|s| s.borrow().is_empty()));
        // Kết quả quét tới muộn không chạy gì.
        assert!(!pump(&host, 150, || got() > 1));
        shutdown();
        assert_eq!(got(), 1);
        // Sau khi đóng: Shield bật thì từ chối, tắt thì như trước.
        assert_eq!(
            submit(OutputAction::Save, sample(8, 8)),
            Err(SubmitError::WindowFailed)
        );
        SHIELD.with(|s| s.set(false));
        assert_eq!(submit(OutputAction::Save, sample(8, 8)), Ok(()));
        assert_eq!(got(), 2);
    }

    #[test]
    fn stray_gate_messages_are_freed_and_run_nothing() {
        use darkshot_core::privacy::ScanReport;
        let host = Host::new(true);
        let boxed = |v: Result<ScanReport, ScanError>| LPARAM(Box::into_raw(Box::new(v)) as isize);
        let clean = Ok(ScanReport {
            findings: Vec::new(),
            tiles: 1,
            elapsed_ms: 1,
        });
        // Kết quả quét "sạch" với mã lạ: không xuất gì.
        assert!(handle_message(host.0, WM_PRIVACY_SCAN, WPARAM(999), boxed(clean)).is_some());
        let apply = Box::into_raw(Box::new(ReviewOutcome::Apply(plan(&[1]))));
        assert!(handle_message(
            host.0,
            WM_PRIVACY_REVIEW,
            WPARAM(999),
            LPARAM(apply as isize)
        )
        .is_some());
        let flat: FlatResult = Ok(PreparedOutput::no_findings(sample(4, 4)));
        let flat = Box::into_raw(Box::new(flat));
        assert!(
            handle_message(host.0, WM_PRIVACY_FLAT, WPARAM(999), LPARAM(flat as isize)).is_some()
        );
        assert!(
            handle_message(host.0, WM_PRIVACY_PROGRESS, WPARAM(999), LPARAM(0x10002)).is_some()
        );
        assert!(handle_message(host.0, WM_TIMER, WPARAM(TIMER_BASE + 999), LPARAM(0)).is_some());
        assert!(handle_message(host.0, WM_TIMER, WPARAM(WATCHDOG_TIMER_ID), LPARAM(0)).is_some());
        assert_eq!((got(), pending()), (0, 0));
        // Hẹn giờ của `main` (mã 1..3) và thông điệp khác: không phải của cổng.
        for id in 1..=3 {
            assert!(handle_message(host.0, WM_TIMER, WPARAM(id), LPARAM(0)).is_none());
        }
        assert!(handle_message(host.0, WM_APP + 2, WPARAM(0), LPARAM(0)).is_none());
        assert!(handle_message(host.0, WM_APP + 18, WPARAM(0), LPARAM(0)).is_none());
    }

    // Các kiểm thử dưới mở cửa sổ Duyệt thật và chạy Windows OCR thật trên ảnh chữ tổng hợp (GDI):
    // cargo test -p darkshot-windows privacy_gate::tests::manual -- --ignored --test-threads=1
    // Chỉ gửi thông điệp thẳng tới cửa sổ của darkshot (không giả lập chuột/phím thật).

    fn review_hwnd() -> Option<HWND> {
        // SAFETY: chỉ tìm cửa sổ theo tên lớp của cửa sổ Duyệt.
        unsafe { FindWindowW(w!("darkshot-privacy-review"), None) }.ok()
    }

    fn post(h: HWND, msg: u32, wp: usize) {
        // SAFETY: gửi vào hàng đợi cửa sổ Duyệt do luồng này tạo.
        unsafe {
            PostMessageW(Some(h), msg, WPARAM(wp), LPARAM(1)).unwrap();
        }
    }

    const VK_RETURN: usize = 0x0D;

    /// Ảnh trắng có một dòng chữ chứa địa chỉ email giả.
    fn email_image(w: i32, h: i32) -> Image {
        crate::ocr::tests::text_image(w, h, Some("Contact user@example.com today"))
    }

    fn black_pixels(img: &Image) -> usize {
        img.bgra
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| **p == [0, 0, 0, 255])
            .count()
    }

    /// Chờ cửa sổ Duyệt ở trạng thái có phát hiện; `None` khi máy không quét được (không có gói OCR).
    fn wait_findings(host: &Host) -> Option<HWND> {
        assert!(pump(host, 30_000, || {
            CORE.with(|c| {
                let c = c.borrow();
                c.entries
                    .iter()
                    .any(|e| matches!(e.state, State::Reviewing { .. }))
            })
        }));
        let scan_ok = CORE.with(|c| {
            c.borrow()
                .entries
                .iter()
                .any(|e| matches!(e.state, State::Reviewing { scan_ok: true, .. }))
        });
        let h = review_hwnd().expect("review window");
        if !scan_ok {
            // Trạng thái lỗi: đóng như bấm ✕, không có gì được xuất.
            post(h, WM_CLOSE, 0);
            assert!(pump(host, 2000, || pending() == 0));
            assert_eq!(got(), 0);
            return None;
        }
        Some(h)
    }

    #[test]
    #[ignore = "Windows OCR thật; chạy tay"]
    fn manual_clean_image_runs_once_without_a_window() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let host = Host::new(true);
        let img = crate::ocr::tests::text_image(600, 160, Some("Plain words only here"));
        assert_eq!(submit(OutputAction::Save, img.clone()), Ok(()));
        assert_eq!((got(), pending()), (0, 1));
        assert!(pump(&host, 30_000, || got() == 1 || review_hwnd().is_some()));
        if got() == 0 {
            // Không quét được trên máy này: cửa sổ lỗi, Hủy thì không xuất.
            assert!(wait_findings(&host).is_none());
            return;
        }
        GOT.with(|g| {
            let g = g.borrow();
            assert_eq!(g[0].1, Protection::NoFindings);
            assert_eq!(g[0].2, img);
        });
        // Cửa sổ "Đang quét…" (nếu quét quá 800 ms) đã tự đóng; không có lần chạy thứ hai.
        assert!(!pump(&host, 1500, || got() > 1));
        assert!(review_hwnd().is_none());
        assert_eq!((got(), pending()), (1, 0));
    }

    #[test]
    #[ignore = "Windows OCR thật, mở cửa sổ thật; chạy tay"]
    fn manual_email_is_covered_before_the_output_runs() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let host = Host::new(true);
        let img = email_image(900, 160);
        assert_eq!(black_pixels(&img), black_pixels(&img.clone()));
        assert_eq!(submit(OutputAction::Upload, img.clone()), Ok(()));
        let Some(h) = wait_findings(&host) else {
            return;
        };
        // Chưa có quyết định: chưa xuất gì.
        assert!(!pump(&host, 300, || got() > 0));
        // Enter = "Che và tiếp tục" với mọi mục chọn sẵn.
        post(h, WM_KEYDOWN, VK_RETURN);
        assert!(pump(&host, 5000, || got() == 1));
        GOT.with(|g| {
            let g = g.borrow();
            assert!(g[0].0 == Msg::PrivacyActionUpload);
            assert!(
                matches!(g[0].1, Protection::Redacted { regions } if regions >= 1),
                "{:?}",
                g[0].1
            );
            let out = &g[0].2;
            assert_eq!((out.width, out.height), (img.width, img.height));
            assert_ne!(out, &img);
            // Vùng che là điểm đen đặc; ảnh gốc (chữ khử răng cưa trên nền trắng) có ít hơn nhiều.
            assert!(
                black_pixels(out) > black_pixels(&img) + 2000,
                "{} vs {}",
                black_pixels(out),
                black_pixels(&img)
            );
        });
        assert!(!pump(&host, 400, || got() > 1));
        assert_eq!((got(), pending()), (1, 0));
        assert!(review_hwnd().is_none());
    }

    #[test]
    #[ignore = "Windows OCR thật, mở cửa sổ thật; chạy tay"]
    fn manual_cancel_exports_nothing() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let host = Host::new(true);
        let calls = Rc::new(Cell::new((0u32, 0u32)));
        let c = calls.clone();
        let done: Done = Box::new(move |p| {
            let (some, none) = c.get();
            c.set(if p.is_some() {
                (some + 1, none)
            } else {
                (some, none + 1)
            });
        });
        let action = OutputAction::StepExport { step: 3, steps: 12 };
        assert_eq!(submit_with(action, email_image(900, 160), done), Ok(()));
        let Some(h) = wait_findings(&host) else {
            return;
        };
        post(h, WM_CLOSE, 0);
        assert!(pump(&host, 2000, || pending() == 0));
        assert!(!pump(&host, 300, || got() > 0));
        assert_eq!(calls.get(), (0, 1));
        assert_eq!(got(), 0);
        assert!(review_hwnd().is_none());
    }

    #[test]
    #[ignore = "Windows OCR thật, mở cửa sổ thật; chạy tay"]
    fn manual_large_image_is_flattened_off_the_ui_thread() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let host = Host::new(true);
        // 2600 × 1600 = 4,16 MP > 4 MP: làm phẳng trên luồng `darkshot-privacy-redact`.
        let img = email_image(2600, 1600);
        assert!(u64::from(img.width) * u64::from(img.height) > FLATTEN_INLINE_MAX_PIXELS);
        assert_eq!(submit(OutputAction::Copy, img.clone()), Ok(()));
        let Some(h) = wait_findings(&host) else {
            return;
        };
        post(h, WM_KEYDOWN, VK_RETURN);
        // Sau Apply lượt ở trạng thái "đang làm phẳng" tới khi thông điệp của luồng nền về.
        assert!(pump(&host, 2000, || review_hwnd().is_none()));
        assert!(pump(&host, 10_000, || got() == 1));
        GOT.with(|g| {
            let g = g.borrow();
            assert!(matches!(g[0].1, Protection::Redacted { .. }));
            assert!(black_pixels(&g[0].2) > black_pixels(&img) + 2000);
        });
        assert_eq!((got(), pending()), (1, 0));
    }

    #[test]
    #[ignore = "Windows OCR thật, mở cửa sổ thật; chạy tay"]
    fn manual_two_requests_are_reviewed_one_at_a_time() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let host = Host::new(true);
        assert_eq!(submit(OutputAction::Save, email_image(900, 160)), Ok(()));
        assert_eq!(submit(OutputAction::Copy, email_image(800, 160)), Ok(()));
        let Some(h1) = wait_findings(&host) else {
            return;
        };
        // Cả hai quét xong: vẫn chỉ một cửa sổ.
        assert!(pump(&host, 30_000, || {
            CORE.with(|c| {
                c.borrow()
                    .entries
                    .iter()
                    .all(|e| !matches!(e.state, State::Scanning { .. }))
            })
        }));
        assert_eq!((got(), pending()), (0, 2));
        post(h1, WM_KEYDOWN, VK_RETURN);
        assert!(pump(&host, 5000, || got() == 1));
        // Lượt thứ hai được mở sau khi lượt đầu xong; Hủy nó.
        assert!(pump(&host, 2000, || review_hwnd().is_some_and(|h| h != h1)));
        let h2 = review_hwnd().unwrap();
        post(h2, WM_CLOSE, 0);
        assert!(pump(&host, 2000, || pending() == 0));
        assert_eq!(got(), 1);
        assert!(GOT.with(|g| g.borrow()[0].0 == Msg::PrivacyActionSave));
    }
}
