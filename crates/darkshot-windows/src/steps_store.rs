//! Phiên chụp từng bước trên đĩa: thư mục `paths::steps_dir()` gồm ảnh `NNN.png` và `session.json`.
//!
//! Thứ tự ghi bảo đảm dữ liệu không mất: ảnh mã hóa và ghi nguyên tử trên luồng nền
//! `darkshot-steps` (hàng đợi), kết quả báo về luồng giao diện bằng `PostMessage`; chỉ khi đó bước
//! mới vào danh sách và `session.json` được ghi nguyên tử (tệp tạm, `sync_all`, đổi tên). Khởi động
//! thì khôi phục; `session.json` hỏng thì cả thư mục được đổi tên và giữ lại.
//!
//! Trạng thái phiên chỉ sống trên luồng giao diện (`STORE`); luồng nền chỉ ghi ảnh.

use crate::paths;
use darkshot_core::i18n::Lang;
use darkshot_core::image::{encode_png, Image};
use darkshot_core::naming::LocalTime;
use darkshot_core::steps::{Step, StepSession, MAX_STEPS};
use std::cell::RefCell;
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

pub use darkshot_core::steps::Full;

/// Gửi tới cửa sổ chính khi luồng nền ghi xong (hoặc lỗi) một ảnh bước; cửa sổ gọi `drain_saved`.
pub const WM_STEPS_SAVED: u32 = WM_APP + 13;

/// Tên tệp phiên trong thư mục phiên.
pub const SESSION_FILE: &str = "session.json";

/// Thời gian chờ ghi ảnh đang dở khi kết thúc phiên.
const END_FLUSH: Duration = Duration::from_secs(3);

/// Kết quả ghi một ảnh bước, từ luồng nền về luồng giao diện.
struct Saved {
    gen: u64,
    file: String,
    result: Result<(), String>,
}

/// Kết quả áp các lần ghi ảnh đã xong.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Drained {
    /// Số bước đã thêm vào phiên.
    pub added: usize,
    /// Lý do của các lần ghi ảnh lỗi (bước không được thêm).
    pub errors: Vec<String>,
}

/// Bộ đếm việc chưa xong của hàng ghi: `flush` chờ nó về 0.
#[derive(Default)]
struct Pending {
    n: Mutex<usize>,
    cv: Condvar,
}

impl Pending {
    fn lock(&self) -> MutexGuard<'_, usize> {
        // Chỉ là một số đếm: poison không làm nó sai, dùng tiếp.
        self.n.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn inc(&self) {
        let mut g = self.lock();
        *g = g.saturating_add(1);
    }

    fn dec(&self) {
        let mut g = self.lock();
        *g = g.saturating_sub(1);
        if *g == 0 {
            self.cv.notify_all();
        }
    }

    fn wait_empty(&self, timeout: Duration) -> bool {
        let g = self.lock();
        match self.cv.wait_timeout_while(g, timeout, |n| *n > 0) {
            Ok((g, _)) => *g == 0,
            Err(e) => *e.into_inner().0 == 0,
        }
    }
}

/// Giảm bộ đếm khi rời phạm vi, kể cả khi việc ghi panic.
struct DoneGuard<'a>(&'a Pending);

impl Drop for DoneGuard<'_> {
    fn drop(&mut self) {
        self.0.dec();
    }
}

type Job = Box<dyn FnOnce() + Send>;

/// Hàng ghi ảnh: một luồng `darkshot-steps` sinh lười ở lần đẩy đầu, ghi tuần tự theo thứ tự đẩy.
struct Writer {
    tx: Option<Sender<Job>>,
    pending: Arc<Pending>,
}

impl Writer {
    fn new() -> Writer {
        Writer {
            tx: None,
            pending: Arc::new(Pending::default()),
        }
    }

    fn spawn(&self) -> Option<Sender<Job>> {
        let (tx, rx) = mpsc::channel::<Job>();
        let pending = self.pending.clone();
        std::thread::Builder::new()
            .name("darkshot-steps".into())
            .spawn(move || {
                for job in rx {
                    let _done = DoneGuard(&pending);
                    // Việc tự bắt panic của nó; đây chỉ là lưới an toàn để luồng sống tiếp.
                    let _ = catch_unwind(AssertUnwindSafe(job));
                }
            })
            .ok()
            .map(|_| tx)
    }

    /// Đẩy một việc; `false` khi không có luồng ghi (việc bị bỏ, không chạy trên luồng gọi).
    fn push(&mut self, job: Job) -> bool {
        self.pending.inc();
        let mut job = job;
        // Lần thứ hai chỉ khi luồng ghi đã chết (đầu nhận bị hủy): sinh lại một lần.
        for _ in 0..2 {
            if self.tx.is_none() {
                self.tx = self.spawn();
            }
            let Some(sender) = self.tx.as_ref() else {
                break;
            };
            match sender.send(job) {
                Ok(()) => return true,
                Err(mpsc::SendError(back)) => {
                    job = back;
                    self.tx = None;
                }
            }
        }
        drop(job);
        self.pending.dec();
        false
    }

    fn flush(&self, timeout: Duration) -> bool {
        self.pending.wait_empty(timeout)
    }
}

/// Ghi `bytes` vào `path` nguyên tử: tệp tạm cùng thư mục (`<tên>.tmp`), `sync_all`, đổi tên đè.
/// Không tạo thư mục. Lỗi thì xóa tệp tạm.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    let write = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        // Mất điện ngay sau khi đổi tên không được để lại tệp rỗng.
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    };
    let r = write();
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

/// Mã hóa PNG rồi ghi nguyên tử (chạy trên luồng nền).
fn write_png(path: &Path, img: &Image, lang: Lang) -> Result<(), String> {
    let png = encode_png(img).map_err(|e| e.message(lang))?;
    write_atomic(path, &png).map_err(|e| e.to_string())
}

/// Phiên chụp từng bước trong một thư mục. Chỉ dùng trên một luồng (luồng giao diện).
pub struct Store {
    root: PathBuf,
    /// `None`: không có phiên.
    session: Option<StepSession>,
    /// Có thay đổi chưa xuất (thêm, xóa, đổi thứ tự, ghi chú).
    dirty: bool,
    /// Thế hệ phiên: kết quả ghi của phiên đã kết thúc bị bỏ.
    gen: u64,
    /// Tên tệp đã giữ chỗ cho các ảnh đang ghi.
    in_flight: Vec<String>,
    writer: Writer,
    results: Arc<Mutex<Vec<Saved>>>,
    notify: Arc<dyn Fn() + Send + Sync>,
}

impl Store {
    /// Kho cho thư mục phiên `root`; `notify` được gọi trên luồng nền sau mỗi lần ghi ảnh.
    /// Không đụng đĩa.
    pub fn new(root: PathBuf, notify: Box<dyn Fn() + Send + Sync>) -> Store {
        Store {
            root,
            session: None,
            dirty: false,
            gen: 0,
            in_flight: Vec::new(),
            writer: Writer::new(),
            results: Arc::new(Mutex::new(Vec::new())),
            notify: Arc::from(notify),
        }
    }

    fn results(&self) -> MutexGuard<'_, Vec<Saved>> {
        self.results.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn active(&self) -> bool {
        self.session.is_some()
    }

    pub fn count(&self) -> usize {
        self.session.as_ref().map_or(0, |s| s.steps.len())
    }

    pub fn steps(&self) -> &[Step] {
        self.session.as_ref().map_or(&[], |s| &s.steps)
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_exported(&mut self) {
        self.dirty = false;
    }

    /// Đường dẫn ảnh của bước thứ `i` (theo thứ tự hiển thị).
    pub fn image_path(&self, i: usize) -> Option<PathBuf> {
        self.steps().get(i).map(|s| self.root.join(&s.file))
    }

    /// Ghi `session.json` nguyên tử theo phiên hiện tại.
    fn persist(&self) -> std::io::Result<()> {
        let Some(s) = &self.session else {
            return Ok(());
        };
        std::fs::create_dir_all(&self.root)?;
        write_atomic(&self.root.join(SESSION_FILE), s.to_json().as_bytes())
    }

    /// Bắt đầu phiên trống (đang có phiên thì không làm gì): tạo thư mục và `session.json`.
    pub fn start(&mut self) -> std::io::Result<()> {
        if self.active() {
            return Ok(());
        }
        self.gen += 1;
        self.in_flight.clear();
        self.session = Some(StepSession::default());
        self.dirty = false;
        let r = self.persist();
        if r.is_err() {
            // Không ghi được phiên lên đĩa: không có phiên.
            self.session = None;
        }
        r
    }

    /// Tên tệp mới không trùng bước nào, ảnh đang ghi nào, hay tệp nào đang có trên đĩa.
    fn reserve_name(&self) -> String {
        let mut probe = StepSession {
            steps: self.steps().to_vec(),
        };
        probe.steps.extend(self.in_flight.iter().map(|f| Step {
            file: f.clone(),
            caption: String::new(),
        }));
        let mut name = probe.next_file_name();
        for _ in 0..999 {
            if !self.root.join(&name).exists() {
                break;
            }
            probe.steps.push(Step {
                file: name,
                caption: String::new(),
            });
            name = probe.next_file_name();
        }
        name
    }

    /// Thêm một bước: giữ tên, đẩy việc mã hóa và ghi ảnh vào hàng nền. Bước chỉ vào danh sách
    /// (và `session.json`) khi `drain` áp kết quả ghi thành công. Không có phiên thì bỏ qua.
    /// Đủ `MAX_STEPS` (kể cả ảnh đang ghi) thì `Err(Full)`.
    pub fn add(&mut self, img: Image) -> Result<(), Full> {
        if !self.active() {
            return Ok(());
        }
        if self.count() + self.in_flight.len() >= MAX_STEPS {
            return Err(Full);
        }
        let file = self.reserve_name();
        self.in_flight.push(file.clone());
        let gen = self.gen;
        if let Err(e) = std::fs::create_dir_all(&self.root) {
            self.results().push(Saved {
                gen,
                file,
                result: Err(e.to_string()),
            });
            (self.notify)();
            return Ok(());
        }
        let path = self.root.join(&file);
        let results = self.results.clone();
        let notify = self.notify.clone();
        let job_file = file.clone();
        let lang = crate::lang::current();
        let job: Job = Box::new(move || {
            let result = catch_unwind(AssertUnwindSafe(|| write_png(&path, &img, lang)))
                .unwrap_or_else(|_| Err("panic".to_string()));
            results
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(Saved {
                    gen,
                    file: job_file,
                    result,
                });
            notify();
        });
        if !self.writer.push(job) {
            self.results().push(Saved {
                gen,
                file,
                result: Err("thread".to_string()),
            });
            (self.notify)();
        }
        Ok(())
    }

    /// Áp các lần ghi ảnh đã xong: thành công thì thêm bước và ghi `session.json`; lỗi thì không
    /// thêm và trả lý do. Kết quả của phiên đã kết thúc bị bỏ (ảnh của nó bị xóa).
    pub fn drain(&mut self) -> Drained {
        let done: Vec<Saved> = std::mem::take(&mut *self.results());
        let mut out = Drained::default();
        for d in done {
            if d.gen != self.gen || !self.active() {
                let in_use = self.steps().iter().any(|s| s.file == d.file)
                    || self.in_flight.contains(&d.file);
                if d.result.is_ok() && !in_use {
                    let _ = std::fs::remove_file(self.root.join(&d.file));
                }
                continue;
            }
            self.in_flight.retain(|f| *f != d.file);
            match d.result {
                Ok(()) => {
                    let added = self
                        .session
                        .as_mut()
                        .is_some_and(|s| s.add(d.file.clone()).is_ok());
                    if added {
                        out.added += 1;
                        self.dirty = true;
                    } else {
                        let _ = std::fs::remove_file(self.root.join(&d.file));
                    }
                }
                Err(e) => out.errors.push(e),
            }
        }
        if out.added > 0 {
            let _ = self.persist();
        }
        out
    }

    /// Chờ các ảnh đang ghi xong, tối đa `timeout`; `true` khi hàng rỗng.
    pub fn flush(&self, timeout: Duration) -> bool {
        self.writer.flush(timeout)
    }

    /// Khôi phục phiên từ đĩa lúc khởi động (`now` đặt tên thư mục hỏng). `session.json` hỏng thì
    /// đổi tên cả thư mục thành `current.corrupt-<thời gian>` và không có phiên; ảnh thiếu thì bỏ
    /// bước đó. Không có `session.json`, hay không đọc được tệp: không có phiên, không đụng gì.
    pub fn restore_at(&mut self, now: LocalTime) {
        if self.active() {
            return;
        }
        let Ok(bytes) = std::fs::read(self.root.join(SESSION_FILE)) else {
            return;
        };
        let parsed = String::from_utf8(bytes)
            .ok()
            .and_then(|s| StepSession::from_json(&s));
        let Some(mut sess) = parsed else {
            self.keep_corrupt(now);
            return;
        };
        let before = sess.steps.len();
        sess.steps.retain(|s| self.root.join(&s.file).is_file());
        let dropped = sess.steps.len() != before;
        self.gen += 1;
        self.in_flight.clear();
        // Không biết phiên đã xuất chưa: coi là chưa (hỏi lại trước khi xóa).
        self.dirty = !sess.steps.is_empty();
        self.session = Some(sess);
        if dropped {
            let _ = self.persist();
        }
    }

    /// Đổi tên thư mục phiên hỏng sang tên chưa có; không đổi được thì để nguyên.
    fn keep_corrupt(&self, now: LocalTime) {
        let base = format!(
            "{}.corrupt-{:04}{:02}{:02}-{:02}{:02}{:02}",
            self.root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "current".to_string()),
            now.year,
            now.month,
            now.day,
            now.hour,
            now.minute,
            now.second
        );
        for k in 1..=100 {
            let name = if k == 1 {
                base.clone()
            } else {
                format!("{base}-{k}")
            };
            let target = self.root.with_file_name(name);
            if target.exists() {
                continue;
            }
            if std::fs::rename(&self.root, &target).is_ok() {
                return;
            }
        }
    }

    /// Kết thúc phiên: chờ ảnh đang ghi (tối đa 3 s), bỏ phiên, xóa thư mục phiên.
    pub fn end(&mut self) -> std::io::Result<()> {
        let _ = self.flush(END_FLUSH);
        // Kết quả còn lại thuộc phiên cũ: bỏ (ảnh nằm trong thư mục sắp xóa).
        self.results().clear();
        self.gen += 1;
        self.in_flight.clear();
        self.session = None;
        self.dirty = false;
        match std::fs::remove_dir_all(&self.root) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// Sửa phiên bằng `f`; đổi thì ghi `session.json` và đánh dấu chưa xuất.
    fn edit(&mut self, f: impl FnOnce(&mut StepSession) -> bool) -> bool {
        let changed = self.session.as_mut().is_some_and(f);
        if changed {
            self.dirty = true;
            let _ = self.persist();
        }
        changed
    }

    /// Xóa bước `i`: ghi `session.json` trước, rồi xóa tệp ảnh.
    pub fn remove(&mut self, i: usize) -> bool {
        let Some(file) = self.steps().get(i).map(|s| s.file.clone()) else {
            return false;
        };
        let changed = self.edit(|s| s.remove(i));
        if changed {
            let _ = std::fs::remove_file(self.root.join(file));
        }
        changed
    }

    pub fn move_up(&mut self, i: usize) -> bool {
        self.edit(|s| s.move_up(i))
    }

    pub fn move_down(&mut self, i: usize) -> bool {
        self.edit(|s| s.move_down(i))
    }

    /// Đặt ghi chú (cắt `MAX_CAPTION` ký tự); ghi chú không đổi thì không ghi gì (vẫn `true`).
    pub fn set_caption(&mut self, i: usize, text: &str) -> bool {
        let same = self.steps().get(i).map(|s| s.caption == text);
        if same == Some(true) {
            return true;
        }
        self.edit(|s| s.set_caption(i, text))
    }

    #[cfg(test)]
    fn set_session_for_test(&mut self, s: StepSession) {
        self.session = Some(s);
    }
}

// ---- Phiên của tiến trình (luồng giao diện) ----

/// HWND của cửa sổ chính nhận `WM_STEPS_SAVED` (0: chưa có).
static WINDOW: AtomicIsize = AtomicIsize::new(0);

thread_local! {
    static STORE: RefCell<Option<Store>> = const { RefCell::new(None) };
    static LISTENERS: RefCell<Vec<fn()>> = const { RefCell::new(Vec::new()) };
}

fn post_saved() {
    let h = WINDOW.load(Ordering::SeqCst);
    if h == 0 {
        return;
    }
    // SAFETY: PostMessageW an toàn từ luồng bất kỳ; cửa sổ đã hủy thì lỗi và ta bỏ qua (kết quả
    // vẫn nằm trong hàng, `flush` lúc thoát áp nó).
    unsafe {
        let _ = PostMessageW(
            Some(HWND(h as *mut c_void)),
            WM_STEPS_SAVED,
            WPARAM(0),
            LPARAM(0),
        );
    }
}

/// Đặt cửa sổ chính nhận `WM_STEPS_SAVED`.
pub fn set_window(hwnd: HWND) {
    WINDOW.store(hwnd.0 as isize, Ordering::SeqCst);
}

fn with<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    STORE.with(|s| {
        let mut b = s.borrow_mut();
        let st = b.get_or_insert_with(|| Store::new(paths::steps_dir(), Box::new(post_saved)));
        f(st)
    })
}

/// Đăng ký hàm được gọi sau mỗi thay đổi của phiên (bắt đầu, thêm, sửa, kết thúc).
pub fn on_change(f: fn()) {
    LISTENERS.with(|l| l.borrow_mut().push(f));
}

/// Gọi các hàm đã đăng ký; chỉ gọi khi đã trả mượn `STORE`.
fn changed() {
    let all = LISTENERS.with(|l| l.borrow().clone());
    for f in all {
        f();
    }
}

/// Bắt đầu phiên trống (đang có phiên thì giữ nguyên).
pub fn start() -> std::io::Result<()> {
    let r = with(|s| s.start());
    changed();
    r
}

pub fn active() -> bool {
    with(|s| s.active())
}

pub fn count() -> usize {
    with(|s| s.count())
}

/// Thêm ảnh bước (ghi nền); bước hiện ra khi `drain_saved` áp kết quả.
pub fn add(image: Image) -> Result<(), Full> {
    with(|s| s.add(image))
}

/// Áp các lần ghi ảnh đã xong (ở `WM_STEPS_SAVED`); trả lý do của các lần lỗi.
pub fn drain_saved() -> Vec<String> {
    let d = with(|s| s.drain());
    if d.added > 0 {
        changed();
    }
    d.errors
}

/// Khôi phục phiên lúc khởi động.
pub fn restore() {
    let now = paths::local_now();
    with(|s| s.restore_at(now));
    changed();
}

/// Kết thúc phiên và xóa thư mục phiên.
pub fn end() -> std::io::Result<()> {
    let r = with(|s| s.end());
    changed();
    r
}

/// Lúc thoát: chờ ảnh đang ghi tối đa `timeout` rồi áp kết quả (thư mục phiên được giữ).
/// Trả lý do của các lần ghi lỗi (không còn ai để báo).
pub fn flush(timeout: Duration) -> Vec<String> {
    with(|s| {
        let _ = s.flush(timeout);
        s.drain().errors
    })
}

/// Các bước theo thứ tự hiển thị (cho cửa sổ Xem lại).
#[allow(dead_code)]
pub fn steps() -> Vec<Step> {
    with(|s| s.steps().to_vec())
}

/// Đường dẫn ảnh của bước `i` (cho cửa sổ Xem lại).
#[allow(dead_code)]
pub fn image_path(i: usize) -> Option<PathBuf> {
    with(|s| s.image_path(i))
}

pub fn dirty() -> bool {
    with(|s| s.dirty())
}

/// Đã xuất: kết thúc phiên sau đó không cần hỏi lại (tới khi có thay đổi mới).
#[allow(dead_code)]
pub fn mark_exported() {
    with(|s| s.mark_exported());
    changed();
}

fn edit_and_notify(f: impl FnOnce(&mut Store) -> bool) -> bool {
    let r = with(f);
    if r {
        changed();
    }
    r
}

#[allow(dead_code)]
pub fn remove(i: usize) -> bool {
    edit_and_notify(|s| s.remove(i))
}

#[allow(dead_code)]
pub fn move_up(i: usize) -> bool {
    edit_and_notify(|s| s.move_up(i))
}

#[allow(dead_code)]
pub fn move_down(i: usize) -> bool {
    edit_and_notify(|s| s.move_down(i))
}

#[allow(dead_code)]
pub fn set_caption(i: usize, text: &str) -> bool {
    edit_and_notify(|s| s.set_caption(i, text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::image::Image;
    use darkshot_core::naming::LocalTime;
    use darkshot_core::steps::{StepSession, MAX_STEPS};
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    const WAIT: Duration = Duration::from_secs(20);

    /// Thư mục gốc tạm riêng cho mỗi kiểm thử; `current` bên trong chưa tồn tại.
    fn tmp(name: &str) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("darkshot_steps_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base.join("current")
    }

    fn cleanup(root: &Path) {
        if let Some(base) = root.parent() {
            let _ = std::fs::remove_dir_all(base);
        }
    }

    fn img(v: u8) -> Image {
        Image {
            width: 3,
            height: 2,
            bgra: vec![v; 3 * 2 * 4],
        }
    }

    fn store(root: &Path) -> Store {
        Store::new(root.to_path_buf(), Box::new(|| {}))
    }

    fn now() -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 18,
            hour: 14,
            minute: 5,
            second: 59,
        }
    }

    /// Thêm một bước và chờ luồng nền ghi xong, rồi áp kết quả.
    fn add_wait(s: &mut Store, v: u8) {
        s.add(img(v)).unwrap();
        assert!(s.flush(WAIT));
        let d = s.drain();
        assert!(d.errors.is_empty(), "{:?}", d.errors);
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    fn json(root: &Path) -> StepSession {
        let s = std::fs::read_to_string(root.join(SESSION_FILE)).unwrap();
        StepSession::from_json(&s).unwrap()
    }

    fn files(s: &StepSession) -> Vec<String> {
        s.steps.iter().map(|x| x.file.clone()).collect()
    }

    #[test]
    fn add_three_steps_writes_pngs_then_json() {
        let root = tmp("add3");
        let mut s = store(&root);
        assert!(!s.active());
        s.start().unwrap();
        assert!(s.active());
        assert_eq!(s.count(), 0);
        assert!(json(&root).steps.is_empty());
        for v in [10, 20, 30] {
            add_wait(&mut s, v);
        }
        assert_eq!(s.count(), 3);
        assert!(s.dirty());
        assert_eq!(
            names(&root),
            ["001.png", "002.png", "003.png", "session.json"]
        );
        assert_eq!(files(&json(&root)), ["001.png", "002.png", "003.png"]);
        let png = std::fs::read(root.join("002.png")).unwrap();
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert_eq!(s.image_path(2), Some(root.join("003.png")));
        assert_eq!(s.image_path(3), None);
        cleanup(&root);
    }

    /// `session.json` chỉ nhận bước khi kết quả ghi PNG đã được áp (sau khi ảnh nằm trên đĩa).
    #[test]
    fn json_updates_only_after_png_result() {
        let root = tmp("order");
        let mut s = store(&root);
        s.start().unwrap();
        s.add(img(1)).unwrap();
        assert!(s.flush(WAIT));
        // Ảnh đã ghi, kết quả chưa áp: json và danh sách chưa có bước.
        assert!(root.join("001.png").is_file());
        assert!(json(&root).steps.is_empty());
        assert_eq!(s.count(), 0);
        let d = s.drain();
        assert_eq!((d.added, d.errors.len()), (1, 0));
        assert_eq!(files(&json(&root)), ["001.png"]);
        assert_eq!(s.count(), 1);
        cleanup(&root);
    }

    #[test]
    fn restore_resumes_session_with_captions() {
        let root = tmp("restore");
        {
            let mut s = store(&root);
            s.start().unwrap();
            add_wait(&mut s, 1);
            add_wait(&mut s, 2);
            assert!(s.set_caption(1, "Bấm Lưu"));
            s.mark_exported();
        }
        let mut s = store(&root);
        s.restore_at(now());
        assert!(s.active());
        assert_eq!(s.count(), 2);
        assert_eq!(s.steps()[1].caption, "Bấm Lưu");
        // Không biết đã xuất hay chưa sau khi khởi động lại: coi là chưa xuất.
        assert!(s.dirty());
        // Thêm tiếp không trùng tên cũ.
        add_wait(&mut s, 3);
        assert_eq!(files(&json(&root)), ["001.png", "002.png", "003.png"]);
        cleanup(&root);
    }

    #[test]
    fn restore_without_folder_or_json_has_no_session() {
        let root = tmp("none");
        let mut s = store(&root);
        s.restore_at(now());
        assert!(!s.active());
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("001.png"), b"x").unwrap();
        s.restore_at(now());
        assert!(!s.active());
        // Không có json: thư mục giữ nguyên.
        assert!(root.join("001.png").is_file());
        cleanup(&root);
    }

    #[test]
    fn corrupt_json_renames_folder_and_keeps_images() {
        let root = tmp("corrupt");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("001.png"), b"img").unwrap();
        std::fs::write(root.join(SESSION_FILE), b"{rac").unwrap();
        let mut s = store(&root);
        s.restore_at(now());
        assert!(!s.active());
        assert!(!root.exists());
        let kept = root.with_file_name("current.corrupt-20261118-140559");
        assert_eq!(std::fs::read(kept.join("001.png")).unwrap(), b"img");
        assert_eq!(std::fs::read(kept.join(SESSION_FILE)).unwrap(), b"{rac");
        // Lần hỏng thứ hai cùng giây: không đè thư mục đã giữ.
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(SESSION_FILE), b"[]").unwrap();
        s.restore_at(now());
        assert!(!root.exists());
        assert!(kept.join("001.png").is_file());
        let kept2 = root.with_file_name("current.corrupt-20261118-140559-2");
        assert_eq!(std::fs::read(kept2.join(SESSION_FILE)).unwrap(), b"[]");
        // Sau đó bắt đầu phiên mới được bình thường.
        s.start().unwrap();
        assert!(s.active() && root.join(SESSION_FILE).is_file());
        cleanup(&root);
    }

    #[test]
    fn missing_png_drops_step_on_restore() {
        let root = tmp("missing");
        std::fs::create_dir_all(&root).unwrap();
        let mut sess = StepSession::default();
        for f in ["001.png", "002.png", "003.png"] {
            sess.add(f.to_string()).unwrap();
        }
        sess.set_caption(2, "ba");
        std::fs::write(root.join(SESSION_FILE), sess.to_json()).unwrap();
        std::fs::write(root.join("001.png"), b"a").unwrap();
        std::fs::write(root.join("003.png"), b"c").unwrap();
        let mut s = store(&root);
        s.restore_at(now());
        assert!(s.active());
        let got: Vec<&str> = s.steps().iter().map(|x| x.file.as_str()).collect();
        assert_eq!(got, ["001.png", "003.png"]);
        assert_eq!(s.steps()[1].caption, "ba");
        // json được ghi lại theo danh sách mới.
        assert_eq!(files(&json(&root)), ["001.png", "003.png"]);
        cleanup(&root);
    }

    #[test]
    fn remove_and_move_update_json() {
        let root = tmp("edit");
        let mut s = store(&root);
        s.start().unwrap();
        for v in [1, 2, 3] {
            add_wait(&mut s, v);
        }
        assert!(s.move_up(2));
        assert_eq!(files(&json(&root)), ["001.png", "003.png", "002.png"]);
        assert!(s.move_down(0));
        assert_eq!(files(&json(&root)), ["003.png", "001.png", "002.png"]);
        assert!(!s.move_up(0));
        assert!(!s.move_down(2));
        assert!(s.set_caption(0, "đầu"));
        assert_eq!(json(&root).steps[0].caption, "đầu");
        assert!(s.remove(1));
        assert_eq!(files(&json(&root)), ["003.png", "002.png"]);
        assert!(!root.join("001.png").exists());
        assert!(!s.remove(5));
        assert_eq!(s.count(), 2);
        cleanup(&root);
    }

    #[test]
    fn dirty_tracks_changes_since_export() {
        let root = tmp("dirty");
        let mut s = store(&root);
        s.start().unwrap();
        assert!(!s.dirty());
        add_wait(&mut s, 1);
        assert!(s.dirty());
        s.mark_exported();
        assert!(!s.dirty());
        assert!(s.set_caption(0, "x"));
        assert!(s.dirty());
        s.mark_exported();
        assert!(!s.move_up(0));
        assert!(!s.dirty());
        cleanup(&root);
    }

    #[test]
    fn end_removes_folder() {
        let root = tmp("end");
        let mut s = store(&root);
        s.start().unwrap();
        add_wait(&mut s, 1);
        s.end().unwrap();
        assert!(!s.active());
        assert_eq!(s.count(), 0);
        assert!(!root.exists());
        // Kết thúc khi không có phiên: vô hại.
        s.end().unwrap();
        cleanup(&root);
    }

    /// Kết thúc ngay sau khi thêm: việc ghi đang dở xong trước khi thư mục bị xóa, không để lại tệp.
    #[test]
    fn end_with_write_in_flight_leaves_nothing() {
        let root = tmp("endflight");
        let mut s = store(&root);
        s.start().unwrap();
        s.add(img(9)).unwrap();
        s.end().unwrap();
        assert!(!root.exists());
        let d = s.drain();
        assert_eq!((d.added, d.errors.len()), (0, 0));
        assert_eq!(s.count(), 0);
        cleanup(&root);
    }

    #[test]
    fn full_counts_writes_in_flight() {
        let root = tmp("full");
        let mut s = store(&root);
        s.start().unwrap();
        let mut sess = StepSession::default();
        for k in 1..MAX_STEPS {
            sess.add(format!("{k:03}.png")).unwrap();
        }
        s.set_session_for_test(sess);
        assert_eq!(s.count(), MAX_STEPS - 1);
        // Bước thứ 99 đang ghi: bước thứ 100 bị từ chối ngay.
        s.add(img(1)).unwrap();
        assert_eq!(s.add(img(2)), Err(Full));
        assert!(s.flush(WAIT));
        assert_eq!(s.drain().added, 1);
        assert_eq!(s.count(), MAX_STEPS);
        assert_eq!(s.add(img(3)), Err(Full));
        cleanup(&root);
    }

    #[test]
    fn new_names_skip_files_already_on_disk() {
        let root = tmp("orphan");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("001.png"), b"orphan").unwrap();
        let mut s = store(&root);
        s.start().unwrap();
        add_wait(&mut s, 1);
        add_wait(&mut s, 2);
        let got: Vec<&str> = s.steps().iter().map(|x| x.file.as_str()).collect();
        assert_eq!(got, ["002.png", "003.png"]);
        assert_eq!(std::fs::read(root.join("001.png")).unwrap(), b"orphan");
        cleanup(&root);
    }

    /// Không ghi được ảnh: báo lỗi, không thêm bước, json giữ nguyên.
    #[test]
    fn png_write_failure_reports_error_and_adds_nothing() {
        let root = tmp("fail");
        let mut s = store(&root);
        s.start().unwrap();
        // Thư mục phiên bị thay bằng một tệp: không tạo được ảnh.
        std::fs::remove_dir_all(&root).unwrap();
        std::fs::write(&root, b"not a dir").unwrap();
        s.add(img(1)).unwrap();
        assert!(s.flush(WAIT));
        let d = s.drain();
        assert_eq!(d.added, 0);
        assert_eq!(d.errors.len(), 1);
        assert_eq!(s.count(), 0);
        // Hết chỗ giữ tên của lượt hỏng: lượt sau vẫn được tính đủ chỗ.
        std::fs::remove_file(&root).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        add_wait(&mut s, 2);
        assert_eq!(s.count(), 1);
        cleanup(&root);
    }

    #[test]
    fn add_without_session_does_nothing() {
        let root = tmp("inactive");
        let mut s = store(&root);
        assert_eq!(s.add(img(1)), Ok(()));
        assert!(s.flush(WAIT));
        let d = s.drain();
        assert_eq!((d.added, d.errors.len()), (0, 0));
        assert!(!root.exists());
        cleanup(&root);
    }

    /// Không bao giờ để lại tệp tạm (`*.tmp`) sau các lần ghi.
    #[test]
    fn writes_leave_no_temp_files() {
        let root = tmp("notmp");
        let mut s = store(&root);
        s.start().unwrap();
        for v in [1, 2] {
            add_wait(&mut s, v);
        }
        s.set_caption(0, "a");
        s.move_down(0);
        s.remove(0);
        assert!(names(&root).iter().all(|n| !n.ends_with(".tmp")));
        assert_eq!(names(&root), ["001.png", "session.json"]);
        cleanup(&root);
    }

    #[test]
    fn notify_called_after_each_write() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let root = tmp("notify");
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        let mut s = Store::new(
            root.clone(),
            Box::new(move || {
                h.fetch_add(1, Ordering::SeqCst);
            }),
        );
        s.start().unwrap();
        add_wait(&mut s, 1);
        add_wait(&mut s, 2);
        assert_eq!(hits.load(Ordering::SeqCst), 2);
        cleanup(&root);
    }

    #[test]
    fn message_id_is_unique() {
        use windows::Win32::UI::WindowsAndMessaging::WM_APP;
        assert_eq!(WM_STEPS_SAVED, WM_APP + 13);
    }
}
