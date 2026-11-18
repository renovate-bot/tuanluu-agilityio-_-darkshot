//! Luồng hành động sau khi chọn vùng: tải lên, lưu, chép, hủy.

use std::path::{Path, PathBuf};

use crate::config::{Expiry, UploadFormat};
use crate::i18n::{self, t, Lang, Msg};
use crate::image::{encode_for_upload, encode_png, Image};
use crate::naming::{local_file_name, random_object_name, LocalTime};
use crate::upload::{object_key, UploadError, Uploader};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Upload,
    Save,
    Copy,
    Pin,
    Ocr,
    /// Thêm bước vào phiên chụp từng bước (xử lý ở lớp Windows; lõi không làm gì).
    AddStep,
    /// Chụp cuộn vùng đã chọn (xử lý ở lớp Windows; lõi không làm gì).
    Scroll,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Success,
    Info,
    Error,
}

/// Thông báo loại `Success` có thể bị ẩn theo tùy chọn; các loại khác luôn hiện.
pub fn notice_visible(kind: NoticeKind, show_success: bool) -> bool {
    match kind {
        NoticeKind::Success => show_success,
        NoticeKind::Info | NoticeKind::Error => true,
    }
}

pub trait Notifier: Send + Sync {
    fn notify(&self, kind: NoticeKind, title: &str, body: &str);
}

pub trait Clipboard: Send + Sync {
    fn set_text(&self, s: &str) -> Result<(), String>;
    fn set_image(&self, img: &Image) -> Result<(), String>;
}

pub struct Ctx<'a> {
    pub save_dir: &'a Path,
    pub expiry: Expiry,
    pub now: LocalTime,
    /// `None` khi R2 chưa cấu hình.
    pub uploader: Option<&'a dyn Uploader>,
    pub clipboard: &'a dyn Clipboard,
    pub notifier: &'a dyn Notifier,
    pub lang: Lang,
    pub upload_format: UploadFormat,
    pub jpeg_quality: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Cancelled,
    Saved(PathBuf),
    Copied,
    Uploaded(String),
    UploadFailedSaved { path: PathBuf, needs_settings: bool },
    Failed,
}

pub fn run_action(ctx: &Ctx, action: Action, img: &Image) -> Outcome {
    if matches!(
        action,
        Action::Cancel | Action::Pin | Action::Ocr | Action::AddStep | Action::Scroll
    ) {
        return Outcome::Cancelled;
    }
    if !img.is_valid() {
        ctx.notifier.notify(
            NoticeKind::Error,
            t(ctx.lang, Msg::NoImageTitle),
            t(ctx.lang, Msg::NoImageBody),
        );
        return Outcome::Failed;
    }
    match action {
        Action::Cancel | Action::Pin | Action::Ocr | Action::AddStep | Action::Scroll => {
            Outcome::Cancelled
        }
        Action::Save => match save_png(ctx, img) {
            Ok(path) => {
                ctx.notifier.notify(
                    NoticeKind::Success,
                    t(ctx.lang, Msg::SavedTitle),
                    &path.display().to_string(),
                );
                Outcome::Saved(path)
            }
            Err(e) => save_failed(ctx, img, &e),
        },
        Action::Copy => match ctx.clipboard.set_image(img) {
            Ok(()) => {
                ctx.notifier.notify(
                    NoticeKind::Success,
                    t(ctx.lang, Msg::CopiedTitle),
                    t(ctx.lang, Msg::CopiedBody),
                );
                Outcome::Copied
            }
            Err(e) => {
                ctx.notifier.notify(
                    NoticeKind::Error,
                    t(ctx.lang, Msg::CopyFailedTitle),
                    &i18n::copy_failed_body(ctx.lang, &e),
                );
                Outcome::Failed
            }
        },
        Action::Upload => upload(ctx, img),
    }
}

fn upload(ctx: &Ctx, img: &Image) -> Outcome {
    let Some(uploader) = ctx.uploader else {
        return fallback(ctx, img, t(ctx.lang, Msg::UploadNotConfigured), true);
    };
    let body = match encode_for_upload(img, ctx.upload_format, ctx.jpeg_quality) {
        Ok(p) => p,
        Err(e) => {
            let reason = i18n::encode_failed(ctx.lang, &e.message(ctx.lang));
            return fallback(ctx, img, &reason, false);
        }
    };
    let key = object_key(ctx.expiry, &random_object_name(), ctx.upload_format);
    match uploader.upload(&key, &body, ctx.upload_format.content_type()) {
        Ok(link) => {
            if let Err(e) = ctx.clipboard.set_text(&link) {
                ctx.notifier.notify(
                    NoticeKind::Error,
                    t(ctx.lang, Msg::UploadedNoCopyTitle),
                    &format!("{link}\n{e}"),
                );
            } else {
                ctx.notifier
                    .notify(NoticeKind::Success, t(ctx.lang, Msg::UploadedTitle), &link);
            }
            Outcome::Uploaded(link)
        }
        Err(e) => {
            let needs_settings = e == UploadError::NotConfigured;
            fallback(ctx, img, &e.message(ctx.lang), needs_settings)
        }
    }
}

/// Lưu dự phòng sau khi tải lên không thành.
fn fallback(ctx: &Ctx, img: &Image, reason: &str, needs_settings: bool) -> Outcome {
    match save_png(ctx, img) {
        Ok(path) => {
            ctx.notifier.notify(
                NoticeKind::Error,
                t(ctx.lang, Msg::UploadFailedTitle),
                &i18n::upload_failed_body(ctx.lang, reason, &path.display().to_string()),
            );
            Outcome::UploadFailedSaved {
                path,
                needs_settings,
            }
        }
        Err(e) => save_failed(ctx, img, &e),
    }
}

fn save_failed(ctx: &Ctx, img: &Image, err: &str) -> Outcome {
    let in_clipboard = ctx.clipboard.set_image(img).is_ok();
    let body = if in_clipboard {
        i18n::save_failed_in_clipboard(ctx.lang, err)
    } else {
        i18n::save_failed_no_clipboard(ctx.lang, err)
    };
    ctx.notifier
        .notify(NoticeKind::Error, t(ctx.lang, Msg::SaveFailedTitle), &body);
    Outcome::Failed
}

/// Số hậu tố tối đa thử khi tên đích đã có người dùng.
const MAX_NAME_ATTEMPTS: u32 = 10_000;

/// Chiếm một tên chưa có trong `dir` bằng cách tạo file rỗng với `create_new` (nguyên tử),
/// thử `name`, rồi `stem_2.ext`, `stem_3.ext`... Hai lượt lưu đồng thời không bao giờ nhận cùng tên.
fn claim_path(dir: &Path, name: &str, lang: Lang) -> std::io::Result<PathBuf> {
    let (stem, ext) = match name.rfind('.') {
        Some(i) => name.split_at(i),
        None => (name, ""),
    };
    for n in 1..=MAX_NAME_ATTEMPTS {
        let candidate = if n == 1 {
            dir.join(name)
        } else {
            dir.join(format!("{stem}_{n}{ext}"))
        };
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            // Đóng file giữ chỗ ngay để bước đổi tên thay được nó.
            Ok(_) => return Ok(candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        t(lang, Msg::ClaimTooMany),
    ))
}

/// Ghi PNG an toàn: chiếm tên đích, ghi vào file tạm cùng thư mục rồi đổi tên đè lên file giữ chỗ;
/// lỗi thì xóa file tạm và file giữ chỗ, nên không bao giờ để lại PNG dở dang dưới tên cuối cùng.
fn save_png(ctx: &Ctx, img: &Image) -> Result<PathBuf, String> {
    let png = encode_png(img).map_err(|e| e.message(ctx.lang))?;
    std::fs::create_dir_all(ctx.save_dir).map_err(|e| e.to_string())?;
    let path = claim_path(ctx.save_dir, &local_file_name(&ctx.now), ctx.lang)
        .map_err(|e| e.to_string())?;
    let tmp = path.with_extension("png.tmp");
    let result = std::fs::write(&tmp, &png).and_then(|_| std::fs::rename(&tmp, &path));
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&path);
        return Err(e.to_string());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::encode_jpeg;
    use crate::upload::UploadError;
    use std::sync::Mutex;

    fn temp_dir() -> PathBuf {
        let mut b = [0u8; 8];
        getrandom::fill(&mut b).unwrap();
        let name: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let d = std::env::temp_dir().join(format!("darkshot_act_{name}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[derive(Default)]
    struct FakeClipboard {
        texts: Mutex<Vec<String>>,
        images: Mutex<Vec<Image>>,
    }
    impl Clipboard for FakeClipboard {
        fn set_text(&self, s: &str) -> Result<(), String> {
            self.texts.lock().unwrap().push(s.to_string());
            Ok(())
        }
        fn set_image(&self, img: &Image) -> Result<(), String> {
            self.images.lock().unwrap().push(img.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeNotifier {
        notes: Mutex<Vec<(NoticeKind, String, String)>>,
    }
    impl Notifier for FakeNotifier {
        fn notify(&self, kind: NoticeKind, title: &str, body: &str) {
            self.notes
                .lock()
                .unwrap()
                .push((kind, title.to_string(), body.to_string()));
        }
    }

    struct FakeUploader {
        result: Result<String, UploadError>,
        keys: Mutex<Vec<String>>,
        calls: Mutex<Vec<(String, Vec<u8>, String)>>,
    }
    impl FakeUploader {
        fn new(result: Result<String, UploadError>) -> Self {
            FakeUploader {
                result,
                keys: Mutex::new(vec![]),
                calls: Mutex::new(vec![]),
            }
        }
    }
    impl Uploader for FakeUploader {
        fn upload(&self, key: &str, body: &[u8], ct: &str) -> Result<String, UploadError> {
            self.keys.lock().unwrap().push(key.to_string());
            self.calls
                .lock()
                .unwrap()
                .push((key.to_string(), body.to_vec(), ct.to_string()));
            match &self.result {
                Ok(s) => Ok(s.clone()),
                Err(UploadError::Network(m)) => Err(UploadError::Network(m.clone())),
                Err(UploadError::NotConfigured) => Err(UploadError::NotConfigured),
                Err(_) => Err(UploadError::Auth),
            }
        }
    }

    fn now() -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 16,
            hour: 14,
            minute: 3,
            second: 5,
        }
    }

    fn img() -> Image {
        Image {
            width: 2,
            height: 1,
            bgra: vec![1, 2, 3, 255, 4, 5, 6, 255],
        }
    }

    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    fn ctx<'a>(
        dir: &'a Path,
        up: Option<&'a dyn Uploader>,
        cb: &'a FakeClipboard,
        n: &'a FakeNotifier,
    ) -> Ctx<'a> {
        Ctx {
            save_dir: dir,
            expiry: Expiry::D7,
            now: now(),
            uploader: up,
            clipboard: cb,
            notifier: n,
            lang: Lang::Vi,
            upload_format: UploadFormat::Png,
            jpeg_quality: 90,
        }
    }

    #[test]
    fn invalid_image_fails_with_one_error_notice() {
        let bad = Image {
            width: 0,
            height: 0,
            bgra: vec![],
        };
        for action in [Action::Save, Action::Upload, Action::Copy] {
            let d = temp_dir();
            let up = FakeUploader::new(Ok("https://x/y.png".to_string()));
            let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
            let out = run_action(&ctx(&d, Some(&up), &cb, &n), action, &bad);
            assert_eq!(out, Outcome::Failed, "{action:?}");
            let notes = n.notes.lock().unwrap();
            assert_eq!(notes.len(), 1, "{action:?}");
            assert_eq!(notes[0].0, NoticeKind::Error);
            assert_eq!(notes[0].1, "Không có ảnh để xử lý");
            assert_eq!(notes[0].2, "Vùng chọn trống hoặc ảnh không hợp lệ.");
            assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
            assert!(up.keys.lock().unwrap().is_empty());
            assert!(cb.images.lock().unwrap().is_empty());
            assert!(cb.texts.lock().unwrap().is_empty());
        }
        let d = temp_dir();
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, None, &cb, &n), Action::Cancel, &bad);
        assert_eq!(out, Outcome::Cancelled);
    }

    #[test]
    fn cancel_does_nothing() {
        let d = temp_dir();
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, None, &cb, &n), Action::Cancel, &img());
        assert_eq!(out, Outcome::Cancelled);
        assert!(n.notes.lock().unwrap().is_empty());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
    }

    #[test]
    fn pin_does_nothing_in_core() {
        let d = temp_dir();
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, None, &cb, &n), Action::Pin, &img());
        assert_eq!(out, Outcome::Cancelled);
        assert!(n.notes.lock().unwrap().is_empty());
        assert!(cb.images.lock().unwrap().is_empty());
        assert!(cb.texts.lock().unwrap().is_empty());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
    }

    #[test]
    fn ocr_does_nothing_in_core() {
        let d = temp_dir();
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, None, &cb, &n), Action::Ocr, &img());
        assert_eq!(out, Outcome::Cancelled);
        assert!(n.notes.lock().unwrap().is_empty());
        assert!(cb.images.lock().unwrap().is_empty());
        assert!(cb.texts.lock().unwrap().is_empty());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
    }

    #[test]
    fn add_step_does_nothing_in_core() {
        let d = temp_dir();
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, None, &cb, &n), Action::AddStep, &img());
        assert_eq!(out, Outcome::Cancelled);
        assert!(n.notes.lock().unwrap().is_empty());
        assert!(cb.images.lock().unwrap().is_empty());
        assert!(cb.texts.lock().unwrap().is_empty());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
    }

    #[test]
    fn scroll_does_nothing_in_core() {
        let d = temp_dir();
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, None, &cb, &n), Action::Scroll, &img());
        assert_eq!(out, Outcome::Cancelled);
        assert!(n.notes.lock().unwrap().is_empty());
        assert!(cb.images.lock().unwrap().is_empty());
        assert!(cb.texts.lock().unwrap().is_empty());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
    }

    #[test]
    fn save_creates_dir_and_file() {
        let d = temp_dir().join("sub");
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, None, &cb, &n), Action::Save, &img());
        let expected = d.join("darkshot_2026-11-16_14-03-05.png");
        assert_eq!(out, Outcome::Saved(expected.clone()));
        let bytes = std::fs::read(&expected).unwrap();
        assert_eq!(bytes[..8], SIG);
        let notes = n.notes.lock().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].0, NoticeKind::Success);
        // không để lại file tạm
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1);
    }

    #[test]
    fn copy_puts_image_on_clipboard() {
        let d = temp_dir();
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, None, &cb, &n), Action::Copy, &img());
        assert_eq!(out, Outcome::Copied);
        assert_eq!(*cb.images.lock().unwrap(), vec![img()]);
        assert_eq!(n.notes.lock().unwrap()[0].0, NoticeKind::Success);
    }

    #[test]
    fn upload_success_copies_link() {
        let d = temp_dir();
        let link = "https://img.example.com/d7/x.png";
        let up = FakeUploader::new(Ok(link.to_string()));
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, Some(&up), &cb, &n), Action::Upload, &img());
        assert_eq!(out, Outcome::Uploaded(link.to_string()));
        let keys = up.keys.lock().unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys[0].starts_with("d7/") && keys[0].ends_with(".png"));
        assert_eq!(keys[0].len(), 3 + 16 + 4);
        assert_eq!(*cb.texts.lock().unwrap(), vec![link.to_string()]);
        let notes = n.notes.lock().unwrap();
        assert_eq!(notes[0].0, NoticeKind::Success);
        assert_eq!(notes[0].1, "Đã chép link");
        assert_eq!(notes[0].2, link);
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
    }

    #[test]
    fn upload_failure_saves_locally() {
        let d = temp_dir();
        let up = FakeUploader::new(Err(UploadError::Network("x".into())));
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, Some(&up), &cb, &n), Action::Upload, &img());
        let path = d.join("darkshot_2026-11-16_14-03-05.png");
        assert_eq!(
            out,
            Outcome::UploadFailedSaved {
                path: path.clone(),
                needs_settings: false
            }
        );
        assert!(path.exists());
        let notes = n.notes.lock().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].0, NoticeKind::Error);
        assert!(notes[0].2.contains(&path.display().to_string()));
        assert!(cb.images.lock().unwrap().is_empty());
        assert!(cb.texts.lock().unwrap().is_empty());
    }

    #[test]
    fn upload_without_config_saves_and_asks_for_settings() {
        let d = temp_dir();
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, None, &cb, &n), Action::Upload, &img());
        let path = d.join("darkshot_2026-11-16_14-03-05.png");
        assert_eq!(
            out,
            Outcome::UploadFailedSaved {
                path: path.clone(),
                needs_settings: true
            }
        );
        assert!(path.exists());
        let notes = n.notes.lock().unwrap();
        assert_eq!(notes[0].0, NoticeKind::Error);
        assert!(notes[0].2.contains("Tùy chọn"));
    }

    #[test]
    fn uploader_not_configured_error_asks_for_settings() {
        let d = temp_dir();
        let up = FakeUploader::new(Err(UploadError::NotConfigured));
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, Some(&up), &cb, &n), Action::Upload, &img());
        let path = d.join("darkshot_2026-11-16_14-03-05.png");
        assert_eq!(
            out,
            Outcome::UploadFailedSaved {
                path: path.clone(),
                needs_settings: true
            }
        );
        assert!(path.exists());
        // Lỗi khác vẫn không đòi mở Cài đặt.
        let up = FakeUploader::new(Err(UploadError::Auth));
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, Some(&up), &cb, &n), Action::Upload, &img());
        assert!(matches!(
            out,
            Outcome::UploadFailedSaved {
                needs_settings: false,
                ..
            }
        ));
    }

    #[test]
    fn png_upload_is_unchanged() {
        let d = temp_dir();
        let up = FakeUploader::new(Ok("https://x/y.png".to_string()));
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let c = ctx(&d, Some(&up), &cb, &n);
        run_action(&c, Action::Upload, &img());
        let calls = up.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].0.ends_with(".png"));
        assert_eq!(calls[0].2, "image/png");
        assert_eq!(calls[0].1, encode_png(&img()).unwrap());
    }

    #[test]
    fn jpeg_upload_sends_jpeg() {
        let d = temp_dir();
        let link = "https://x/y.jpg";
        let up = FakeUploader::new(Ok(link.to_string()));
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let mut c = ctx(&d, Some(&up), &cb, &n);
        c.upload_format = UploadFormat::Jpeg;
        c.jpeg_quality = 80;
        let out = run_action(&c, Action::Upload, &img());
        assert_eq!(out, Outcome::Uploaded(link.to_string()));
        let calls = up.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].0.ends_with(".jpg"));
        assert_eq!(calls[0].2, "image/jpeg");
        assert_eq!(calls[0].1, encode_jpeg(&img(), 80).unwrap());
        assert_eq!(*cb.texts.lock().unwrap(), vec![link.to_string()]);
        assert_eq!(n.notes.lock().unwrap()[0].0, NoticeKind::Success);
    }

    #[test]
    fn jpeg_upload_failure_saves_png() {
        let d = temp_dir();
        let up = FakeUploader::new(Err(UploadError::Network("x".into())));
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let mut c = ctx(&d, Some(&up), &cb, &n);
        c.upload_format = UploadFormat::Jpeg;
        let out = run_action(&c, Action::Upload, &img());
        let path = d.join("darkshot_2026-11-16_14-03-05.png");
        assert_eq!(
            out,
            Outcome::UploadFailedSaved {
                path: path.clone(),
                needs_settings: false
            }
        );
        assert_eq!(std::fs::read(&path).unwrap(), encode_png(&img()).unwrap());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1);
    }

    #[test]
    fn jpeg_encode_failure_falls_back() {
        let d = temp_dir();
        let up = FakeUploader::new(Ok("https://x/y.jpg".to_string()));
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let mut c = ctx(&d, Some(&up), &cb, &n);
        c.upload_format = UploadFormat::Jpeg;
        let wide = Image {
            width: 65_536,
            height: 1,
            bgra: vec![0; 65_536 * 4],
        };
        let out = run_action(&c, Action::Upload, &wide);
        assert!(matches!(out, Outcome::UploadFailedSaved { .. }), "{out:?}");
        assert!(up.calls.lock().unwrap().is_empty());
        let path = d.join("darkshot_2026-11-16_14-03-05.png");
        assert!(path.exists());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1);
        let notes = n.notes.lock().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].0, NoticeKind::Error);
    }

    #[test]
    fn save_ignores_upload_format() {
        let d = temp_dir();
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let mut c = ctx(&d, None, &cb, &n);
        c.upload_format = UploadFormat::Jpeg;
        let out = run_action(&c, Action::Save, &img());
        let path = d.join("darkshot_2026-11-16_14-03-05.png");
        assert_eq!(out, Outcome::Saved(path.clone()));
        assert_eq!(std::fs::read(&path).unwrap(), encode_png(&img()).unwrap());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1);
    }

    #[test]
    fn notices_follow_ctx_lang() {
        let link = "https://img.example.com/d7/x.png";
        let title_for = |lang: Lang, action: Action| {
            let d = temp_dir();
            let up = FakeUploader::new(Ok(link.to_string()));
            let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
            let mut c = ctx(&d, Some(&up), &cb, &n);
            c.lang = lang;
            run_action(&c, action, &img());
            let t = n.notes.lock().unwrap()[0].1.clone();
            t
        };
        assert_eq!(title_for(Lang::En, Action::Upload), "Link copied");
        assert_eq!(title_for(Lang::Vi, Action::Upload), "Đã chép link");
        let (vi, en) = (
            title_for(Lang::Vi, Action::Save),
            title_for(Lang::En, Action::Save),
        );
        assert!(!vi.is_empty() && !en.is_empty() && vi != en);
    }

    #[test]
    fn same_second_saves_never_overwrite_each_other() {
        // Nhiều lượt lưu cùng `LocalTime` chạy đồng thời: mỗi lượt một file riêng, nội dung còn nguyên.
        const N: usize = 8;
        for _round in 0..5 {
            let d = temp_dir();
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(N));
            let handles: Vec<_> = (0..N)
                .map(|i| {
                    let d = d.clone();
                    let barrier = barrier.clone();
                    std::thread::spawn(move || {
                        let image = Image {
                            width: 1,
                            height: 1,
                            bgra: vec![i as u8, 0, 0, 255],
                        };
                        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
                        barrier.wait();
                        let out = run_action(&ctx(&d, None, &cb, &n), Action::Save, &image);
                        (image, out)
                    })
                })
                .collect();
            let results: Vec<(Image, Outcome)> =
                handles.into_iter().map(|h| h.join().unwrap()).collect();
            let mut paths = std::collections::HashSet::new();
            for (image, out) in &results {
                let Outcome::Saved(p) = out else {
                    panic!("lưu thất bại: {out:?}");
                };
                assert!(paths.insert(p.clone()), "trùng đường dẫn {}", p.display());
                assert_eq!(std::fs::read(p).unwrap(), encode_png(image).unwrap());
            }
            // Không còn file tạm hay file giữ chỗ thừa.
            assert_eq!(std::fs::read_dir(&d).unwrap().count(), N);
        }
    }

    #[test]
    fn save_skips_existing_file_without_touching_it() {
        let d = temp_dir();
        let first = d.join("darkshot_2026-11-16_14-03-05.png");
        std::fs::write(&first, b"old").unwrap();
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, None, &cb, &n), Action::Save, &img());
        assert_eq!(
            out,
            Outcome::Saved(d.join("darkshot_2026-11-16_14-03-05_2.png"))
        );
        assert_eq!(std::fs::read(&first).unwrap(), b"old");
    }

    #[test]
    fn unwritable_save_dir_keeps_image_on_clipboard() {
        let base = temp_dir();
        let file = base.join("not_a_dir");
        std::fs::write(&file, b"x").unwrap();

        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&file, None, &cb, &n), Action::Save, &img());
        assert_eq!(out, Outcome::Failed);
        assert_eq!(cb.images.lock().unwrap().len(), 1);
        let notes = n.notes.lock().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].0, NoticeKind::Error);
        drop(notes);

        let up = FakeUploader::new(Err(UploadError::Network("x".into())));
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&file, Some(&up), &cb, &n), Action::Upload, &img());
        assert_eq!(out, Outcome::Failed);
        assert_eq!(cb.images.lock().unwrap().len(), 1);
    }

    #[test]
    fn only_success_notices_can_be_hidden() {
        assert!(notice_visible(NoticeKind::Success, true));
        assert!(!notice_visible(NoticeKind::Success, false));
        for k in [NoticeKind::Info, NoticeKind::Error] {
            assert!(notice_visible(k, false) && notice_visible(k, true));
        }
    }

    /// Bộ chép clipboard có thể lỗi từng phần.
    struct PickyClipboard {
        text_ok: bool,
        image_ok: bool,
    }
    impl Clipboard for PickyClipboard {
        fn set_text(&self, _: &str) -> Result<(), String> {
            if self.text_ok {
                Ok(())
            } else {
                Err("no text".into())
            }
        }
        fn set_image(&self, _: &Image) -> Result<(), String> {
            if self.image_ok {
                Ok(())
            } else {
                Err("no image".into())
            }
        }
    }

    fn kinds_of(
        dir: &Path,
        up: Option<&dyn Uploader>,
        cb: &dyn Clipboard,
        action: Action,
    ) -> Vec<NoticeKind> {
        let n = FakeNotifier::default();
        let c = Ctx {
            save_dir: dir,
            expiry: Expiry::D7,
            now: now(),
            uploader: up,
            clipboard: cb,
            notifier: &n,
            lang: Lang::Vi,
            upload_format: UploadFormat::Png,
            jpeg_quality: 90,
        };
        run_action(&c, action, &img());
        let kinds = n.notes.lock().unwrap().iter().map(|x| x.0).collect();
        kinds
    }

    #[test]
    fn notice_kinds_of_outcomes() {
        let good = PickyClipboard {
            text_ok: true,
            image_ok: true,
        };
        let no_text = PickyClipboard {
            text_ok: false,
            image_ok: true,
        };
        let no_image = PickyClipboard {
            text_ok: true,
            image_ok: false,
        };
        let ok_up = FakeUploader::new(Ok("https://x/y.png".to_string()));
        let bad_up = FakeUploader::new(Err(UploadError::Network("x".into())));

        let ok = [NoticeKind::Success];
        let err = [NoticeKind::Error];
        assert_eq!(kinds_of(&temp_dir(), None, &good, Action::Save), ok);
        assert_eq!(kinds_of(&temp_dir(), None, &good, Action::Copy), ok);
        assert_eq!(
            kinds_of(&temp_dir(), Some(&ok_up), &good, Action::Upload),
            ok
        );
        assert_eq!(
            kinds_of(&temp_dir(), Some(&ok_up), &no_text, Action::Upload),
            err
        );
        assert_eq!(
            kinds_of(&temp_dir(), Some(&bad_up), &good, Action::Upload),
            err
        );
        assert_eq!(kinds_of(&temp_dir(), None, &no_image, Action::Copy), err);
        let file = temp_dir().join("not_a_dir");
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(kinds_of(&file, None, &good, Action::Save), err);
    }
}
