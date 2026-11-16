//! Luồng hành động sau khi chọn vùng: tải lên, lưu, chép, hủy.

use std::path::{Path, PathBuf};

use crate::config::Expiry;
use crate::image::Image;
use crate::naming::LocalTime;
use crate::upload::Uploader;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Upload,
    Save,
    Copy,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Error,
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
    let _ = (ctx, action, img);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upload::UploadError;
    use std::sync::Mutex;

    fn temp_dir() -> PathBuf {
        let mut b = [0u8; 8];
        getrandom::fill(&mut b).unwrap();
        let name: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let d = std::env::temp_dir().join(format!("shotme_act_{name}"));
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
    }
    impl FakeUploader {
        fn new(result: Result<String, UploadError>) -> Self {
            FakeUploader {
                result,
                keys: Mutex::new(vec![]),
            }
        }
    }
    impl Uploader for FakeUploader {
        fn upload(&self, key: &str, _png: &[u8]) -> Result<String, UploadError> {
            self.keys.lock().unwrap().push(key.to_string());
            match &self.result {
                Ok(s) => Ok(s.clone()),
                Err(UploadError::Network(m)) => Err(UploadError::Network(m.clone())),
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
        }
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
    fn save_creates_dir_and_file() {
        let d = temp_dir().join("sub");
        let (cb, n) = (FakeClipboard::default(), FakeNotifier::default());
        let out = run_action(&ctx(&d, None, &cb, &n), Action::Save, &img());
        let expected = d.join("shotme_2026-11-16_14-03-05.png");
        assert_eq!(out, Outcome::Saved(expected.clone()));
        let bytes = std::fs::read(&expected).unwrap();
        assert_eq!(bytes[..8], SIG);
        let notes = n.notes.lock().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].0, NoticeKind::Info);
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
        assert_eq!(n.notes.lock().unwrap()[0].0, NoticeKind::Info);
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
        assert_eq!(notes[0].0, NoticeKind::Info);
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
        let path = d.join("shotme_2026-11-16_14-03-05.png");
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
        let path = d.join("shotme_2026-11-16_14-03-05.png");
        assert_eq!(
            out,
            Outcome::UploadFailedSaved {
                path: path.clone(),
                needs_settings: true
            }
        );
        assert!(path.exists());
        assert_eq!(n.notes.lock().unwrap()[0].0, NoticeKind::Error);
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
}
