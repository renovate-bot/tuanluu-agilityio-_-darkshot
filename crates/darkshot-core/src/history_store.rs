//! Kho lịch sử trên đĩa: chỉ mục JSON ghi nguyên tử, bản sao PNG, ảnh nhỏ, giữ tối đa 50 mục.
//!
//! Bố cục dưới `root`: `index.json`, `index.corrupt.json`, `images/<id>.png`, `thumbs/<id>.png`.
//! Mọi thay đổi chỉ mục chạy dưới một khóa toàn tiến trình dùng chung cho mọi `Store`.
//! Không bao giờ panic, không bao giờ xóa tệp của người dùng (`Entry::path`).

use crate::history::{is_valid_id, parse_index, serialize_index, Entry, Kind, MAX_ENTRIES};
use crate::image::{encode_png, Image};
use crate::naming::LocalTime;
use crate::thumb::{decode_png_file, downscale, DecodeError, THUMB_MAX_H, THUMB_MAX_W};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

const INDEX: &str = "index.json";
const INDEX_TMP: &str = "index.json.tmp";
const INDEX_CORRUPT: &str = "index.corrupt.json";
const IMAGES: &str = "images";
const THUMBS: &str = "thumbs";

/// Tệp mồ côi chỉ bị xóa khi cũ hơn mức này: một `record` khác có thể đã ghi tệp
/// nhưng chưa kịp lấy khóa để thêm mục vào chỉ mục.
const ORPHAN_MIN_AGE: Duration = Duration::from_secs(60);

static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Loaded {
    pub entries: Vec<Entry>,
    pub skipped: usize,
    pub recovered: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewEntry {
    pub kind: Kind,
    pub unix_millis: u64,
    pub local: LocalTime,
    /// Tệp của người dùng (Saved/UploadFallback).
    pub path: Option<PathBuf>,
    /// Liên kết (Uploaded).
    pub url: Option<String>,
    pub expires: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreError {
    /// Lỗi đọc/ghi (không kèm đường dẫn).
    Io(String),
    /// Dữ liệu đầu vào không hợp lệ.
    Invalid,
}

fn io_err(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}

/// Giữ lại bản chỉ mục hỏng: đổi tên `index.corrupt.json` (đè bản hỏng cũ); không được thì
/// chép sang `index.corrupt.<unix>.json`. Trả `true` khi bản hỏng đã được giữ ở đâu đó.
fn preserve_corrupt(root: &Path, bytes: &[u8]) -> bool {
    if std::fs::rename(root.join(INDEX), root.join(INDEX_CORRUPT)).is_ok() {
        return true;
    }
    let unix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    std::fs::write(root.join(format!("index.corrupt.{unix}.json")), bytes).is_ok()
}

/// Đọc chỉ mục từ đĩa (gọi khi đã giữ khóa). Trả kèm cờ "được phép ghi đè `index.json`":
/// sai khi chỉ mục hỏng mà chưa giữ được bản hỏng.
/// `Err` chỉ khi lỗi I/O khác "không có tệp": bên ghi không được đè chỉ mục đọc không được.
fn read_index(root: &Path) -> Result<(Loaded, bool), StoreError> {
    let path = root.join(INDEX);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((Loaded::default(), true)),
        Err(e) => return Err(io_err(e)),
    };
    let parsed = std::str::from_utf8(&bytes)
        .ok()
        .and_then(|text| parse_index(text).ok());
    match parsed {
        Some((entries, skipped)) => Ok((
            Loaded {
                entries,
                skipped,
                recovered: false,
            },
            true,
        )),
        None => {
            let kept = preserve_corrupt(root, &bytes);
            Ok((
                Loaded {
                    entries: Vec::new(),
                    skipped: 0,
                    recovered: true,
                },
                kept,
            ))
        }
    }
}

/// Đọc chỉ mục cho bên ghi: lỗi nếu ghi đè sẽ làm mất bản hỏng chưa được giữ.
fn read_for_write(root: &Path) -> Result<Vec<Entry>, StoreError> {
    match read_index(root)? {
        (l, true) => Ok(l.entries),
        (_, false) => Err(StoreError::Io(String::from(
            "corrupt history index could not be preserved",
        ))),
    }
}

/// Ghi `bytes` vào `path` rồi `sync_all` (dữ liệu xuống đĩa trước khi đổi tên).
fn write_synced(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)?;
    // Mất điện ngay sau `rename` không được để lại tệp rỗng: dữ liệu phải xuống đĩa trước.
    f.sync_all()
}

/// Ghi chỉ mục nguyên tử: `index.json.tmp` (đã `sync_all`) rồi đổi tên đè (gọi khi đã giữ khóa).
fn write_index(root: &Path, entries: &[Entry]) -> Result<(), StoreError> {
    std::fs::create_dir_all(root).map_err(io_err)?;
    let tmp = root.join(INDEX_TMP);
    let r = write_synced(&tmp, serialize_index(entries).as_bytes())
        .and_then(|_| std::fs::rename(&tmp, root.join(INDEX)));
    if let Err(e) = r {
        let _ = std::fs::remove_file(&tmp);
        return Err(io_err(e));
    }
    Ok(())
}

/// Ghi tệp nguyên tử qua tệp tạm cùng thư mục (tên tạm có hậu tố ngẫu nhiên, không khớp mẫu `<id>.png`).
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let dir = path.parent().ok_or(StoreError::Invalid)?;
    std::fs::create_dir_all(dir).map_err(io_err)?;
    let mut r = [0u8; 4];
    getrandom::fill(&mut r).map_err(|e| StoreError::Io(e.to_string()))?;
    let name = path
        .file_name()
        .ok_or(StoreError::Invalid)?
        .to_string_lossy()
        .into_owned();
    let tmp = dir.join(format!("{name}.{:08x}.tmp", u32::from_le_bytes(r)));
    let res = write_synced(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, path));
    if let Err(e) = res {
        let _ = std::fs::remove_file(&tmp);
        return Err(io_err(e));
    }
    Ok(())
}

/// Như `history::new_id` nhưng trả lỗi thay vì panic khi nguồn ngẫu nhiên hỏng.
fn try_new_id(unix_millis: u64) -> Result<String, StoreError> {
    let mut r = [0u8; 4];
    getrandom::fill(&mut r).map_err(|e| StoreError::Io(e.to_string()))?;
    let id = format!(
        "{:013x}{:08x}",
        unix_millis & 0xF_FFFF_FFFF_FFFF,
        u32::from_le_bytes(r)
    );
    if is_valid_id(&id) {
        Ok(id)
    } else {
        Err(StoreError::Invalid)
    }
}

/// `<id>` nếu `name` có dạng `<id>.png` với id hợp lệ.
/// `(id, hex)` nếu `name` có dạng `<id>.png.<8 hex thường>.tmp` (tệp tạm của `write_atomic`).
fn tmp_parts(name: &str) -> Option<(&str, &str)> {
    let (id, hex) = name.strip_suffix(".tmp")?.split_once(".png.")?;
    let ok_hex = hex.len() == 8 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    (is_valid_id(id) && ok_hex).then_some((id, hex))
}

fn id_of_file_name(name: &str) -> Option<&str> {
    let id = name.strip_suffix(".png")?;
    is_valid_id(id).then_some(id)
}

pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: PathBuf) -> Store {
        Store { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Đường dẫn tệp do kho giữ trong `images`/`thumbs`; `None` nếu id không hợp lệ.
    fn owned_file(&self, sub: &str, id: &str) -> Option<PathBuf> {
        if !is_valid_id(id) {
            return None;
        }
        let dir = self.root.join(sub);
        let p = dir.join(format!("{id}.png"));
        (p.parent() == Some(dir.as_path()) && p.starts_with(&self.root)).then_some(p)
    }

    /// Xóa `images/<id>.png` và `thumbs/<id>.png`. Không bao giờ chạm `Entry::path`.
    fn delete_owned(&self, id: &str) {
        for sub in [IMAGES, THUMBS] {
            if let Some(p) = self.owned_file(sub, id) {
                let _ = std::fs::remove_file(p);
            }
        }
    }

    /// Nạp chỉ mục. Không có tệp → rỗng. Hỏng cú pháp hoặc version lạ → đổi tên `index.corrupt.json`
    /// (đè bản hỏng cũ), trả rỗng, `Loaded.recovered = true`. Không bao giờ panic.
    pub fn load(&self) -> Loaded {
        let _g = lock();
        read_index(&self.root).map(|(l, _)| l).unwrap_or_default()
    }

    /// Ghi một mục mới (đồng bộ): mã hóa trước, ghi tệp ảnh, rồi cập nhật chỉ mục dưới khóa.
    pub fn record(&self, new: NewEntry, img: &Image) -> Result<Entry, StoreError> {
        if !img.is_valid() {
            return Err(StoreError::Invalid);
        }
        let managed = match new.kind {
            Kind::Saved | Kind::UploadFallback => {
                if new.path.is_none() {
                    return Err(StoreError::Invalid);
                }
                false
            }
            Kind::Uploaded => {
                if !new
                    .url
                    .as_deref()
                    .is_some_and(|u| u.starts_with("https://"))
                {
                    return Err(StoreError::Invalid);
                }
                true
            }
            Kind::Copied => true,
        };

        // Mọi việc nặng làm trước khi lấy khóa.
        let id = try_new_id(new.unix_millis)?;
        let thumb_png = encode_png(&downscale(img, THUMB_MAX_W, THUMB_MAX_H))
            .map_err(|_| StoreError::Invalid)?;
        let (copy_png, bytes) = if managed {
            let png = encode_png(img).map_err(|_| StoreError::Invalid)?;
            let n = png.len() as u64;
            (Some(png), Some(n))
        } else {
            let n = new
                .path
                .as_deref()
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len());
            (None, n)
        };
        let image_file = self.owned_file(IMAGES, &id).ok_or(StoreError::Invalid)?;
        let thumb_file = self.owned_file(THUMBS, &id).ok_or(StoreError::Invalid)?;
        let cleanup = || {
            let _ = std::fs::remove_file(&image_file);
            let _ = std::fs::remove_file(&thumb_file);
        };
        if let Some(png) = &copy_png {
            if let Err(e) = write_atomic(&image_file, png) {
                cleanup();
                return Err(e);
            }
        }
        // Ảnh nhỏ thiếu thì `thumbnail` sinh lại sau, nên lỗi ghi ảnh nhỏ không chặn mục.
        let _ = write_atomic(&thumb_file, &thumb_png);

        let entry = Entry {
            id: id.clone(),
            kind: new.kind,
            unix: new.unix_millis / 1000,
            local: new.local,
            width: img.width,
            height: img.height,
            bytes,
            path: new.path,
            managed,
            url: new.url,
            expires: new.expires,
        };

        let (evicted, live_ids) = {
            let _g = lock();
            let mut cur = match read_for_write(&self.root) {
                Ok(v) => v,
                Err(e) => {
                    cleanup();
                    return Err(e);
                }
            };
            cur.insert(0, entry.clone());
            let evicted: Vec<Entry> = if cur.len() > MAX_ENTRIES {
                cur.split_off(MAX_ENTRIES)
            } else {
                Vec::new()
            };
            if let Err(e) = write_index(&self.root, &cur) {
                cleanup();
                return Err(e);
            }
            let live: Vec<String> = cur.into_iter().map(|e| e.id).collect();
            (evicted, live)
        };

        for e in &evicted {
            self.delete_owned(&e.id);
        }
        self.sweep_orphans(&live_ids);
        Ok(entry)
    }

    /// Xóa, khi cũ hơn `ORPHAN_MIN_AGE`: `<id>.png` hợp lệ trong `images/`, `thumbs/` không có trong
    /// `live`, và tệp tạm `<id>.png.<8 hex>.tmp` sót lại. Tệp lạ (tên không khớp mẫu) giữ nguyên.
    fn sweep_orphans(&self, live: &[String]) {
        let now = SystemTime::now();
        for sub in [IMAGES, THUMBS] {
            let dir = self.root.join(sub);
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            for de in rd.flatten() {
                let name = de.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                // Đường dẫn dựng lại từ các phần đã kiểm, luôn nằm trong `dir`.
                let target = if let Some(id) = id_of_file_name(name) {
                    if live.iter().any(|l| l == id) {
                        continue;
                    }
                    self.owned_file(sub, id)
                } else if let Some((id, hex)) = tmp_parts(name) {
                    Some(dir.join(format!("{id}.png.{hex}.tmp")))
                } else {
                    None
                };
                let Some(target) = target else {
                    continue;
                };
                let old = de
                    .metadata()
                    .ok()
                    .filter(|m| m.is_file())
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| now.duration_since(t).ok())
                    .is_some_and(|age| age > ORPHAN_MIN_AGE);
                if old {
                    let _ = std::fs::remove_file(target);
                }
            }
        }
    }

    /// Xóa mục `id` cùng `images/<id>.png`, `thumbs/<id>.png`. `Ok(false)` nếu không có hoặc id không hợp lệ.
    pub fn remove(&self, id: &str) -> Result<bool, StoreError> {
        if !is_valid_id(id) {
            return Ok(false);
        }
        {
            let _g = lock();
            let mut cur = read_for_write(&self.root)?;
            let before = cur.len();
            cur.retain(|e| e.id != id);
            if cur.len() == before {
                return Ok(false);
            }
            write_index(&self.root, &cur)?;
        }
        self.delete_owned(id);
        Ok(true)
    }

    /// Xóa mọi mục; trả số mục đã xóa. Xóa tệp `images/`, `thumbs/` của các mục đó, rồi quét
    /// mồ côi (ngưỡng 60 giây) để không đụng tệp của một `record` đang chờ khóa.
    /// Không xóa tệp lạ, `index.corrupt.json` hay tệp của người dùng.
    pub fn clear(&self) -> Result<usize, StoreError> {
        let (removed, live) = {
            let _g = lock();
            let removed: Vec<String> = read_for_write(&self.root)?
                .into_iter()
                .map(|e| e.id)
                .collect();
            write_index(&self.root, &[])?;
            let live: Vec<String> = read_for_write(&self.root)?
                .into_iter()
                .map(|e| e.id)
                .collect();
            (removed, live)
        };
        for id in &removed {
            self.delete_owned(id);
        }
        self.sweep_orphans(&live);
        Ok(removed.len())
    }

    /// Ảnh nhỏ của mục: đọc `thumbs/<id>.png`; thiếu/hỏng thì sinh lại từ `image_path` rồi ghi.
    pub fn thumbnail(&self, e: &Entry) -> Result<Image, DecodeError> {
        let thumb_file = self
            .owned_file(THUMBS, &e.id)
            .ok_or_else(|| DecodeError::Io(String::from("invalid id")))?;
        if let Ok(img) = decode_png_file(&thumb_file) {
            return Ok(img);
        }
        let src = e
            .image_path(&self.root)
            .ok_or_else(|| DecodeError::Io(String::from("no source image")))?;
        let full = decode_png_file(&src)?;
        let small = downscale(&full, THUMB_MAX_W, THUMB_MAX_H);
        if let Ok(png) = encode_png(&small) {
            let _ = write_atomic(&thumb_file, &png);
        }
        Ok(small)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thumb::decode_png;
    use std::sync::Arc;

    fn temp_root() -> PathBuf {
        let mut b = [0u8; 8];
        getrandom::fill(&mut b).unwrap();
        let name: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let d = std::env::temp_dir().join(format!("darkshot_hist_{name}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn index_written_via_sync_path() {
        let root = temp_root();
        let f = root.join("synced.bin");
        write_synced(&f, b"hello").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"hello");
        // Ghi đè: nội dung mới thay hẳn nội dung cũ.
        write_synced(&f, b"hi").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"hi");
        // Thư mục cha không tồn tại: lỗi, không tạo tệp.
        let missing = root.join("nope").join("x.bin");
        assert!(write_synced(&missing, b"x").is_err());
        assert!(!missing.exists());
        // Chỉ mục ghi qua đường đồng bộ đọc lại được, không để lại tệp tạm.
        write_index(&root, &[]).unwrap();
        assert!(root.join(INDEX).exists());
        assert!(!root.join(INDEX_TMP).exists());
    }

    fn lt() -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 18,
            hour: 14,
            minute: 32,
            second: 5,
        }
    }

    fn img(w: u32, h: u32, seed: u8) -> Image {
        let mut bgra = Vec::new();
        for y in 0..h {
            for x in 0..w {
                bgra.extend_from_slice(&[x as u8 ^ seed, y as u8, seed, 255]);
            }
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    fn new(kind: Kind, ms: u64) -> NewEntry {
        NewEntry {
            kind,
            unix_millis: ms,
            local: lt(),
            path: None,
            url: None,
            expires: None,
        }
    }

    fn copied(ms: u64) -> NewEntry {
        new(Kind::Copied, ms)
    }

    /// Tạo tệp PNG "của người dùng" ngoài thư mục lịch sử.
    fn user_file(dir: &Path, name: &str, im: &Image) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, encode_png(im).unwrap()).unwrap();
        p
    }

    fn saved(ms: u64, path: &Path) -> NewEntry {
        NewEntry {
            path: Some(path.to_path_buf()),
            ..new(Kind::Saved, ms)
        }
    }

    fn count_png(dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .map(|rd| {
                rd.flatten()
                    .filter(|d| d.file_name().to_str().and_then(id_of_file_name).is_some())
                    .count()
            })
            .unwrap_or(0)
    }

    #[test]
    fn record_saved_references_user_file() {
        let root = temp_root();
        let user = temp_root();
        let im = img(640, 400, 1);
        let p = user_file(&user, "shot.png", &im);
        let size = std::fs::metadata(&p).unwrap().len();
        let s = Store::new(root.join("history"));
        let e = s.record(saved(1_700_000_000_123, &p), &im).unwrap();
        assert!(!s.root().join("images").exists());
        assert!(s
            .root()
            .join("thumbs")
            .join(format!("{}.png", e.id))
            .is_file());
        assert_eq!(e.bytes, Some(size));
        assert_eq!(e.path.as_deref(), Some(p.as_path()));
        assert!(!e.managed);
        assert_eq!((e.width, e.height), (640, 400));
        assert_eq!(e.unix, 1_700_000_000);
        assert_eq!(&e.id[..13], format!("{:013x}", 1_700_000_000_123u64));
        let th = decode_png_file(&e.thumb_path(s.root())).unwrap();
        assert_eq!((th.width, th.height), (320, 200));
        assert!(!s.root().join(INDEX_TMP).exists());
        // UploadFallback cũng là tham chiếu.
        let f = s
            .record(
                NewEntry {
                    kind: Kind::UploadFallback,
                    ..saved(1_700_000_000_200, &p)
                },
                &im,
            )
            .unwrap();
        assert!(!f.managed);
        assert!(!s.root().join("images").exists());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&user);
    }

    #[test]
    fn record_uploaded_and_copied_keep_copy() {
        let root = temp_root();
        let s = Store::new(root.clone());
        let im = img(30, 20, 2);
        let up = s
            .record(
                NewEntry {
                    url: Some("https://cdn.example.com/a.png".into()),
                    expires: Some("2026-12-18".into()),
                    ..new(Kind::Uploaded, 1_000)
                },
                &im,
            )
            .unwrap();
        assert!(up.managed);
        let copy = up.image_path(&root).unwrap();
        assert_eq!(up.bytes, Some(std::fs::metadata(&copy).unwrap().len()));
        assert_eq!(decode_png_file(&copy).unwrap(), im);
        assert_eq!(up.url.as_deref(), Some("https://cdn.example.com/a.png"));
        assert_eq!(up.expires.as_deref(), Some("2026-12-18"));

        let c = s.record(copied(2_000), &im).unwrap();
        assert!(c.managed);
        assert!(c.image_path(&root).unwrap().is_file());
        assert!(c.thumb_path(&root).is_file());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn record_rejects_invalid_and_leaves_nothing() {
        let root = temp_root();
        let s = Store::new(root.join("h"));
        let im = img(4, 4, 0);
        let bad_url = NewEntry {
            url: Some("http://x.example/a.png".into()),
            ..new(Kind::Uploaded, 1)
        };
        assert_eq!(s.record(bad_url, &im), Err(StoreError::Invalid));
        assert_eq!(
            s.record(new(Kind::Uploaded, 1), &im),
            Err(StoreError::Invalid)
        );
        assert_eq!(s.record(new(Kind::Saved, 1), &im), Err(StoreError::Invalid));
        assert_eq!(
            s.record(new(Kind::UploadFallback, 1), &im),
            Err(StoreError::Invalid)
        );
        let broken = Image {
            width: 2,
            height: 2,
            bgra: vec![0; 3],
        };
        assert_eq!(s.record(copied(1), &broken), Err(StoreError::Invalid));
        assert!(!s.root().exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn newest_first_and_round_trip() {
        let root = temp_root();
        let user = temp_root();
        let s = Store::new(root.clone());
        let im = img(8, 8, 3);
        let p = user_file(&user, "a.png", &im);
        let a = s.record(saved(1_000, &p), &im).unwrap();
        let b = s
            .record(
                NewEntry {
                    url: Some("https://x.example/b".into()),
                    expires: Some("2026-11-25".into()),
                    ..new(Kind::Uploaded, 2_000)
                },
                &im,
            )
            .unwrap();
        let c = s.record(copied(3_000), &im).unwrap();
        let l = s.load();
        assert!(!l.recovered);
        assert_eq!(l.skipped, 0);
        assert_eq!(l.entries, vec![c, b, a]);
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&user);
    }

    #[test]
    fn keeps_fifty_and_deletes_evicted_files_only() {
        let root = temp_root();
        let user = temp_root();
        let s = Store::new(root.clone());
        let im = img(4, 4, 4);
        let mut all = Vec::new();
        for i in 0..55u64 {
            let e = if i < 3 {
                let p = user_file(&user, &format!("u{i}.png"), &im);
                s.record(saved(1_000 + i, &p), &im).unwrap()
            } else {
                s.record(copied(1_000 + i), &im).unwrap()
            };
            all.push(e);
        }
        let l = s.load();
        assert_eq!(l.entries.len(), MAX_ENTRIES);
        let expect: Vec<Entry> = all[5..].iter().rev().cloned().collect();
        assert_eq!(l.entries, expect);
        for e in &all[..5] {
            assert!(!e.thumb_path(&root).exists());
            assert!(!root.join("images").join(format!("{}.png", e.id)).exists());
        }
        for e in &all[..3] {
            assert!(
                e.path.as_ref().unwrap().is_file(),
                "tệp người dùng phải còn"
            );
        }
        for e in &all[5..] {
            assert!(e.thumb_path(&root).is_file());
            assert!(e.image_path(&root).unwrap().is_file());
        }
        assert_eq!(count_png(&root.join("images")), 50);
        assert_eq!(count_png(&root.join("thumbs")), 50);
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&user);
    }

    #[test]
    fn remove_entry_and_its_files() {
        let root = temp_root();
        let user = temp_root();
        let s = Store::new(root.clone());
        let im = img(4, 4, 5);
        let p = user_file(&user, "keep.png", &im);
        let a = s.record(saved(1, &p), &im).unwrap();
        let b = s.record(copied(2), &im).unwrap();
        assert_eq!(s.remove(&b.id), Ok(true));
        assert!(!b.image_path(&root).unwrap().exists());
        assert!(!b.thumb_path(&root).exists());
        assert_eq!(s.remove(&b.id), Ok(false));
        assert_eq!(s.remove(&a.id), Ok(true));
        assert!(p.is_file(), "tệp người dùng phải còn");
        assert!(!a.thumb_path(&root).exists());
        assert!(s.load().entries.is_empty());
        let c = s.record(copied(3), &im).unwrap();
        assert_eq!(s.remove("../index.json"), Ok(false));
        assert_eq!(s.remove(""), Ok(false));
        assert_eq!(s.remove(&"0".repeat(21)), Ok(false));
        assert!(root.join(INDEX).is_file());
        assert_eq!(s.load().entries, vec![c]);
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&user);
    }

    #[test]
    fn clear_removes_owned_files_only() {
        let root = temp_root();
        let user = temp_root();
        let s = Store::new(root.clone());
        let im = img(4, 4, 6);
        let p = user_file(&user, "mine.png", &im);
        s.record(saved(1, &p), &im).unwrap();
        s.record(copied(2), &im).unwrap();
        s.record(copied(3), &im).unwrap();
        let stranger = root.join("images").join("notes.txt");
        std::fs::write(&stranger, b"keep me").unwrap();
        let odd = root.join("images").join("ABC.png");
        std::fs::write(&odd, b"keep me too").unwrap();
        std::fs::write(root.join(INDEX_CORRUPT), b"old junk").unwrap();
        assert_eq!(s.clear(), Ok(3));
        assert!(s.load().entries.is_empty());
        assert_eq!(count_png(&root.join("images")), 0);
        assert_eq!(count_png(&root.join("thumbs")), 0);
        assert!(p.is_file());
        assert!(stranger.is_file());
        assert!(odd.is_file());
        assert!(root.join(INDEX_CORRUPT).is_file());
        assert_eq!(s.clear(), Ok(0));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&user);
    }

    #[test]
    fn corrupt_index_is_recovered() {
        for junk in [
            &b"{{{ not json"[..],
            b"{\"version\":99,\"entries\":[]}",
            b"\xff\xfe\x00",
        ] {
            let root = temp_root();
            let s = Store::new(root.clone());
            std::fs::write(root.join(INDEX), junk).unwrap();
            let l = s.load();
            assert!(l.entries.is_empty());
            assert!(l.recovered);
            assert_eq!(std::fs::read(root.join(INDEX_CORRUPT)).unwrap(), junk);
            assert!(!root.join(INDEX).exists());
            let e = s.record(copied(5), &img(2, 2, 7)).unwrap();
            let l = s.load();
            assert!(!l.recovered);
            assert_eq!(l.entries, vec![e]);
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn corrupt_index_found_by_record_is_recovered() {
        let root = temp_root();
        let s = Store::new(root.clone());
        std::fs::write(root.join(INDEX), b"garbage").unwrap();
        let e = s.record(copied(5), &img(2, 2, 7)).unwrap();
        assert_eq!(std::fs::read(root.join(INDEX_CORRUPT)).unwrap(), b"garbage");
        assert_eq!(s.load().entries, vec![e]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn skips_bad_entries() {
        let root = temp_root();
        let s = Store::new(root.clone());
        let im = img(2, 2, 8);
        s.record(copied(1), &im).unwrap();
        s.record(copied(2), &im).unwrap();
        let text = std::fs::read_to_string(root.join(INDEX)).unwrap();
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v["entries"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"id": "../x", "kind": "saved"}));
        std::fs::write(root.join(INDEX), v.to_string()).unwrap();
        let l = s.load();
        assert_eq!(l.entries.len(), 2);
        assert_eq!(l.skipped, 1);
        assert!(!l.recovered);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn index_write_failure_rolls_back() {
        let root = temp_root();
        let s = Store::new(root.clone());
        let im = img(4, 4, 9);
        let ok = s.record(copied(1), &im).unwrap();
        assert!(!root.join(INDEX_TMP).exists());
        // Thay chỉ mục bằng một thư mục: đọc/đổi tên đều thất bại.
        let saved_index = std::fs::read(root.join(INDEX)).unwrap();
        std::fs::remove_file(root.join(INDEX)).unwrap();
        std::fs::create_dir(root.join(INDEX)).unwrap();
        let r = s.record(copied(2), &im);
        assert!(matches!(r, Err(StoreError::Io(_))), "{r:?}");
        assert_eq!(count_png(&root.join("images")), 1);
        assert_eq!(count_png(&root.join("thumbs")), 1);
        assert!(ok.image_path(&root).unwrap().is_file());
        assert!(!root.join(INDEX_TMP).exists());
        assert!(matches!(s.remove(&ok.id), Err(StoreError::Io(_))));
        assert!(matches!(s.clear(), Err(StoreError::Io(_))));
        assert!(ok.image_path(&root).unwrap().is_file());
        // Lỗi đọc không phải hỏng nội dung: không đổi tên, không panic.
        let l = s.load();
        assert!(l.entries.is_empty() && !l.recovered);
        std::fs::remove_dir(root.join(INDEX)).unwrap();
        std::fs::write(root.join(INDEX), saved_index).unwrap();
        assert_eq!(s.load().entries, vec![ok]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn orphans_older_than_a_minute_are_swept() {
        let root = temp_root();
        let s = Store::new(root.clone());
        let im = img(4, 4, 10);
        s.record(copied(1), &im).unwrap();
        let old_id = "0000000000001aaaaaaaa";
        let fresh_id = "0000000000002bbbbbbbb";
        let mut files = Vec::new();
        for sub in ["images", "thumbs"] {
            for id in [old_id, fresh_id] {
                let p = root.join(sub).join(format!("{id}.png"));
                std::fs::write(&p, b"x").unwrap();
                files.push(p);
            }
        }
        let past = SystemTime::now() - Duration::from_secs(120);
        for p in files
            .iter()
            .filter(|p| p.to_string_lossy().contains(old_id))
        {
            let f = std::fs::OpenOptions::new().write(true).open(p).unwrap();
            f.set_modified(past).unwrap();
        }
        let stranger = root.join("images").join("old.txt");
        std::fs::write(&stranger, b"x").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&stranger)
            .unwrap()
            .set_modified(past)
            .unwrap();
        s.record(copied(2), &im).unwrap();
        for sub in ["images", "thumbs"] {
            assert!(!root.join(sub).join(format!("{old_id}.png")).exists());
            assert!(root.join(sub).join(format!("{fresh_id}.png")).exists());
        }
        assert!(stranger.is_file());
        assert_eq!(s.load().entries.len(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn write_aged(p: &Path, age_secs: u64) {
        std::fs::write(p, b"x").unwrap();
        if age_secs > 0 {
            std::fs::OpenOptions::new()
                .write(true)
                .open(p)
                .unwrap()
                .set_modified(SystemTime::now() - Duration::from_secs(age_secs))
                .unwrap();
        }
    }

    #[test]
    fn clear_spares_fresh_unindexed_files() {
        let root = temp_root();
        let s = Store::new(root.clone());
        let im = img(4, 4, 16);
        let e = s.record(copied(1), &im).unwrap();
        // Tệp của một `record` đang chờ khóa: mới tinh, chưa có trong chỉ mục.
        let fresh = "0000000000003cccccccc";
        let old = "0000000000004dddddddd";
        for sub in ["images", "thumbs"] {
            write_aged(&root.join(sub).join(format!("{fresh}.png")), 0);
            write_aged(&root.join(sub).join(format!("{old}.png")), 120);
        }
        assert_eq!(s.clear(), Ok(1));
        assert!(!e.image_path(&root).unwrap().exists());
        assert!(!e.thumb_path(&root).exists());
        for sub in ["images", "thumbs"] {
            assert!(root.join(sub).join(format!("{fresh}.png")).is_file());
            assert!(!root.join(sub).join(format!("{old}.png")).exists());
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    fn timestamped_corrupt(root: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(root)
            .unwrap()
            .flatten()
            .map(|d| d.path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy().into_owned();
                p.is_file()
                    && n.strip_prefix("index.corrupt.")
                        .and_then(|r| r.strip_suffix(".json"))
                        .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
            })
            .collect()
    }

    /// Chặn `index.corrupt.json` bằng một thư mục không rỗng để `rename` thất bại.
    fn block_corrupt_name(root: &Path) {
        let d = root.join(INDEX_CORRUPT);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("x"), b"x").unwrap();
    }

    #[test]
    fn corrupt_index_copied_when_rename_fails() {
        let root = temp_root();
        let s = Store::new(root.clone());
        block_corrupt_name(&root);
        std::fs::write(root.join(INDEX), b"junk!").unwrap();
        let l = s.load();
        assert!(l.recovered && l.entries.is_empty());
        let kept = timestamped_corrupt(&root);
        assert_eq!(kept.len(), 1);
        assert_eq!(std::fs::read(&kept[0]).unwrap(), b"junk!");
        let e = s.record(copied(1), &img(2, 2, 1)).unwrap();
        assert_eq!(s.load().entries, vec![e]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn corrupt_index_never_overwritten_when_unpreservable() {
        let root = temp_root();
        let s = Store::new(root.clone());
        block_corrupt_name(&root);
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for n in now.saturating_sub(5)..now + 120 {
            std::fs::create_dir(root.join(format!("index.corrupt.{n}.json"))).unwrap();
        }
        std::fs::write(root.join(INDEX), b"junk!").unwrap();
        let l = s.load();
        assert!(l.recovered && l.entries.is_empty());
        let im = img(2, 2, 2);
        assert!(matches!(s.record(copied(1), &im), Err(StoreError::Io(_))));
        assert!(matches!(
            s.remove("0000000000001aaaaaaaa"),
            Err(StoreError::Io(_))
        ));
        assert!(matches!(s.clear(), Err(StoreError::Io(_))));
        assert_eq!(std::fs::read(root.join(INDEX)).unwrap(), b"junk!");
        assert_eq!(count_png(&root.join("images")), 0);
        assert_eq!(count_png(&root.join("thumbs")), 0);
        // Gỡ vật chặn: bản hỏng được giữ, ghi chạy lại bình thường.
        std::fs::remove_dir_all(root.join(INDEX_CORRUPT)).unwrap();
        let e = s.record(copied(2), &im).unwrap();
        assert_eq!(std::fs::read(root.join(INDEX_CORRUPT)).unwrap(), b"junk!");
        assert_eq!(s.load().entries, vec![e]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn old_temp_files_are_swept_strictly() {
        let root = temp_root();
        let s = Store::new(root.clone());
        let im = img(4, 4, 17);
        s.record(copied(1), &im).unwrap();
        let id = "0000000000001aaaaaaaa";
        let mut keep = Vec::new();
        let mut gone = Vec::new();
        for sub in ["images", "thumbs"] {
            let d = root.join(sub);
            let p = d.join(format!("{id}.png.0123abcd.tmp"));
            write_aged(&p, 120);
            gone.push(p);
            for (name, age) in [
                (format!("{id}.png.89abcdef.tmp"), 0),
                (format!("{id}.png.4567CDEF.tmp"), 120),
                (format!("{id}.png.0123abc.tmp"), 120),
                (format!("{id}.png.tmp"), 120),
                (format!("{id}.png.0123abcd.tmp.bak"), 120),
                ("x.png.0123abcd.tmp".to_string(), 120),
                (format!("{}.png.0123abcd.tmp", "A".repeat(21)), 120),
            ] {
                let p = d.join(name);
                write_aged(&p, age);
                keep.push(p);
            }
        }
        s.record(copied(2), &im).unwrap();
        for p in &gone {
            assert!(!p.exists(), "{}", p.display());
        }
        for p in &keep {
            assert!(p.is_file(), "{}", p.display());
        }
        assert_eq!(s.load().entries.len(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn assert_index_valid(root: &Path) -> Vec<Entry> {
        let text = std::fs::read_to_string(root.join(INDEX)).unwrap();
        let (v, skipped) = parse_index(&text).unwrap();
        assert_eq!(skipped, 0);
        v
    }

    #[test]
    fn concurrent_records_lose_nothing() {
        let root = temp_root();
        let shared = Arc::new(Store::new(root.clone()));
        let im = Arc::new(img(16, 16, 11));
        let mut hs = Vec::new();
        for t in 0..8u64 {
            // Nửa số luồng dùng chung một Store, nửa còn lại mỗi luồng một Store cùng root.
            let s = if t % 2 == 0 {
                Arc::clone(&shared)
            } else {
                Arc::new(Store::new(root.clone()))
            };
            let im = Arc::clone(&im);
            hs.push(std::thread::spawn(move || {
                for i in 0..10u64 {
                    s.record(copied(t * 100 + i), &im).unwrap();
                }
            }));
        }
        for h in hs {
            h.join().unwrap();
        }
        let l = shared.load();
        assert_eq!(l.entries.len(), MAX_ENTRIES.min(80));
        let mut ids: Vec<&String> = l.entries.iter().map(|e| &e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), l.entries.len());
        assert_index_valid(&root);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn concurrent_eighty_fit_when_under_cap() {
        // 8 × 10 = 80 > 50, nên đếm đủ 80 trên một kho có trần riêng là không thể;
        // thay vào đó đếm 80 bản ghi thành công và 80 id khác nhau trả về.
        let root = temp_root();
        let s = Arc::new(Store::new(root.clone()));
        let im = Arc::new(img(8, 8, 12));
        let mut hs = Vec::new();
        for t in 0..8u64 {
            let s = Arc::clone(&s);
            let im = Arc::clone(&im);
            hs.push(std::thread::spawn(move || {
                (0..10u64)
                    .map(|i| s.record(copied(t * 100 + i), &im).unwrap().id)
                    .collect::<Vec<_>>()
            }));
        }
        let mut ids: Vec<String> = hs.into_iter().flat_map(|h| h.join().unwrap()).collect();
        assert_eq!(ids.len(), 80);
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 80);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn concurrent_record_and_remove_keep_index_valid() {
        let root = temp_root();
        let im = Arc::new(img(8, 8, 13));
        let mut hs = Vec::new();
        for t in 0..4u64 {
            let s = Store::new(root.clone());
            let im = Arc::clone(&im);
            hs.push(std::thread::spawn(move || {
                for i in 0..15u64 {
                    let e = s.record(copied(t * 1000 + i), &im).unwrap();
                    if i % 2 == 0 {
                        assert_eq!(s.remove(&e.id), Ok(true));
                    }
                }
            }));
        }
        for h in hs {
            h.join().unwrap();
        }
        let v = assert_index_valid(&root);
        assert_eq!(v.len(), 4 * 7);
        assert_eq!(count_png(&root.join("images")), 28);
        assert_eq!(count_png(&root.join("thumbs")), 28);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cap_holds_under_load() {
        let root = temp_root();
        let im = Arc::new(img(8, 8, 14));
        let mut hs = Vec::new();
        for t in 0..8u64 {
            let s = Store::new(root.clone());
            let im = Arc::clone(&im);
            hs.push(std::thread::spawn(move || {
                for i in 0..15u64 {
                    s.record(copied(t * 1000 + i), &im).unwrap();
                }
            }));
        }
        for h in hs {
            h.join().unwrap();
        }
        let v = assert_index_valid(&root);
        assert_eq!(v.len(), 50);
        // Mục bị đẩy ra được xóa tệp ngay; không còn tệp mồ côi.
        assert_eq!(count_png(&root.join("images")), 50);
        assert_eq!(count_png(&root.join("thumbs")), 50);
        for e in &v {
            assert!(e.image_path(&root).unwrap().is_file());
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn thumbnail_reads_or_regenerates() {
        let root = temp_root();
        let user = temp_root();
        let s = Store::new(root.clone());
        let im = img(640, 400, 15);
        let c = s.record(copied(1), &im).unwrap();
        let t1 = s.thumbnail(&c).unwrap();
        assert_eq!((t1.width, t1.height), (320, 200));
        // Ảnh nhỏ bị xóa → sinh lại từ bản sao và ghi lại.
        std::fs::remove_file(c.thumb_path(&root)).unwrap();
        let t2 = s.thumbnail(&c).unwrap();
        assert_eq!(t2, t1);
        assert!(c.thumb_path(&root).is_file());
        // Thư mục thumbs mất hẳn.
        std::fs::remove_dir_all(root.join("thumbs")).unwrap();
        assert_eq!(s.thumbnail(&c).unwrap(), t1);
        // Ảnh nhỏ hỏng → sinh lại.
        std::fs::write(c.thumb_path(&root), b"junk").unwrap();
        assert_eq!(s.thumbnail(&c).unwrap(), t1);
        assert_eq!(
            decode_png(&std::fs::read(c.thumb_path(&root)).unwrap()).unwrap(),
            t1
        );

        // Saved: nguồn là tệp người dùng.
        let p = user_file(&user, "s.png", &im);
        let sv = s.record(saved(2, &p), &im).unwrap();
        std::fs::remove_file(sv.thumb_path(&root)).unwrap();
        assert_eq!(s.thumbnail(&sv).unwrap(), t1);
        // Nguồn mất và không có ảnh nhỏ → Err(Io); nhưng nếu ảnh nhỏ còn thì vẫn đọc được.
        std::fs::remove_file(&p).unwrap();
        assert_eq!(s.thumbnail(&sv).unwrap(), t1);
        std::fs::remove_file(sv.thumb_path(&root)).unwrap();
        assert!(matches!(s.thumbnail(&sv), Err(DecodeError::Io(_))));
        // Mất tệp Saved không làm load lỗi.
        let l = s.load();
        assert_eq!(l.entries.len(), 2);
        assert!(!l.recovered);
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&user);
    }
}
