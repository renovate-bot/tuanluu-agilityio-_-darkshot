//! Xuất phiên chụp từng bước ra thư mục người dùng chọn: tạo thư mục con mới
//! `darkshot-steps-YYYY-MM-DD_HH-mm` (`_2`, `_3`… nếu tên đã có), chép ảnh theo thứ tự hiển thị
//! thành `01.png`, `02.png`… và ghi `steps.md`. Chạy trên luồng nền. Bước có ảnh thay thế (đã qua
//! Privacy Shield lúc xuất, mã hóa PNG sẵn ở `steps_window`) được ghi byte PNG đó thay cho bản chép;
//! tệp của phiên không đổi.
//!
//! An toàn: mọi thứ được ghi vào một thư mục tạm mới (`.darkshot-steps-partial-…`, tạo bằng
//! `create_dir` nên không bao giờ là thư mục có sẵn; tệp tạo bằng `create_new`), rồi đổi tên một lần
//! sang tên cuối bằng `MoveFileExW` không cờ thay thế (tên đã có thì lỗi, thử `_2`…). Lỗi giữa chừng
//! thì xóa thư mục tạm; thư mục tạm còn sót (thoát giữa chừng) được dọn ở lần xuất sau.

use darkshot_core::i18n::Lang;
use darkshot_core::naming::LocalTime;
use darkshot_core::steps::{export_dir_name, export_names, markdown, Step};
use std::path::{Path, PathBuf};
use windows::core::HSTRING;
use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVE_FILE_FLAGS};

/// Tên tệp Markdown trong thư mục xuất.
pub const MARKDOWN_FILE: &str = "steps.md";

/// Tiền tố thư mục tạm của một lượt xuất đang chạy (đổi tên sang tên cuối khi xong).
pub const STAGING_PREFIX: &str = ".darkshot-steps-partial-";

/// Số hậu tố tối đa thử khi tên thư mục đã có (`_2` … `_999`).
const MAX_SUFFIX: u32 = 999;

/// Một lần xuất: thư mục cha, thời điểm (đặt tên, ghi trong Markdown), ngôn ngữ của Markdown, các
/// bước theo thứ tự hiển thị và đường dẫn ảnh nguồn tương ứng, cùng ảnh thay thế `(chỉ số bước,
/// byte PNG của ảnh đã qua cổng xuất)` cho các bước chưa quét lúc chụp (Privacy Shield bật lúc
/// xuất). Giữ byte PNG thay cho ảnh BGRA để lượt xuất nhiều bước không giữ mọi ảnh thô.
#[derive(Clone, Debug)]
pub struct ExportJob {
    pub parent: PathBuf,
    pub when: LocalTime,
    pub lang: Lang,
    pub steps: Vec<Step>,
    pub sources: Vec<PathBuf>,
    pub replacements: Vec<(usize, Vec<u8>)>,
}

/// Tạo thư mục con mới `base` (hoặc `base_2`, `base_3`…) trong `parent`; không bao giờ dùng tên đã
/// có. Trả đường dẫn thư mục vừa tạo.
pub fn create_unique_dir(parent: &Path, base: &str) -> std::io::Result<PathBuf> {
    let mut last = std::io::Error::from(std::io::ErrorKind::AlreadyExists);
    for k in 1..=MAX_SUFFIX {
        let name = if k == 1 {
            base.to_string()
        } else {
            format!("{base}_{k}")
        };
        let dir = parent.join(name);
        // `create_dir` không tạo thư mục cha và lỗi `AlreadyExists` khi tên đã có (thư mục hay tệp):
        // kiểm và tạo là một bước, không có kẽ hở giữa hai lần.
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last = e,
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

/// Chép `src` sang tệp mới `dst` (lỗi nếu `dst` đã có).
fn copy_new(src: &Path, dst: &Path) -> std::io::Result<()> {
    let mut from = std::fs::File::open(src)?;
    let mut to = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dst)?;
    // Không `sync_all` từng ảnh: thư mục chỉ mang tên cuối sau khi `steps.md` (ghi cuối) đã
    // `sync_all`; ảnh nguồn vẫn còn trong phiên nếu mất điện giữa chừng.
    std::io::copy(&mut from, &mut to).map(|_| ())
}

/// Ghi `bytes` vào tệp mới `path` (lỗi nếu đã có).
fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

/// Chép ảnh (hay ghi ảnh thay thế) và ghi `steps.md` vào thư mục `dir` (vừa tạo, rỗng).
fn fill(dir: &Path, job: &ExportJob) -> std::io::Result<()> {
    let names = export_names(job.steps.len());
    for (k, (src, name)) in job.sources.iter().zip(&names).enumerate() {
        let dst = dir.join(name);
        match job.replacements.iter().find(|(i, _)| *i == k) {
            Some((_, png)) => write_new(&dst, png)?,
            None => copy_new(src, &dst)?,
        }
    }
    let md = markdown(job.lang, job.when, &job.steps);
    write_new(&dir.join(MARKDOWN_FILE), md.as_bytes())
}

/// Tên tệp do lượt xuất tạo: `steps.md` hay `NN.png`/`NNN.png`.
fn export_file_name(name: &str) -> bool {
    name == MARKDOWN_FILE
        || name
            .strip_suffix(".png")
            .is_some_and(|d| (2..=3).contains(&d.len()) && d.bytes().all(|b| b.is_ascii_digit()))
}

/// Thư mục tạm còn sót của lượt xuất bị ngắt: tên có tiền tố, chỉ chứa tệp do lượt xuất tạo.
fn is_stale_staging(dir: &Path) -> bool {
    let named = dir
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with(STAGING_PREFIX));
    if !named || !dir.is_dir() {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.into_iter().all(|e| {
        e.is_ok_and(|e| {
            e.file_type().is_ok_and(|t| t.is_file())
                && export_file_name(&e.file_name().to_string_lossy())
        })
    })
}

/// Dọn các thư mục tạm còn sót trong `parent` (bỏ qua lỗi).
fn clean_stale(parent: &Path) {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if is_stale_staging(&p) {
            let _ = std::fs::remove_dir_all(&p);
        }
    }
}

/// Đổi tên `staging` sang `base` (hoặc `base_2`…) trong `parent`, không bao giờ thay thứ đã có.
fn publish(staging: &Path, parent: &Path, base: &str) -> std::io::Result<PathBuf> {
    let mut last = std::io::Error::from(std::io::ErrorKind::AlreadyExists);
    for k in 1..=MAX_SUFFIX {
        let name = if k == 1 {
            base.to_string()
        } else {
            format!("{base}_{k}")
        };
        let target = parent.join(name);
        // SAFETY: hai chuỗi sống suốt lời gọi. Không có MOVEFILE_REPLACE_EXISTING: tên đích đã có
        // (tệp hay thư mục) thì lỗi ERROR_ALREADY_EXISTS/ERROR_FILE_EXISTS, không đè.
        let r = unsafe {
            MoveFileExW(
                &HSTRING::from(staging.as_os_str()),
                &HSTRING::from(target.as_os_str()),
                MOVE_FILE_FLAGS(0),
            )
        };
        match r {
            Ok(()) => return Ok(target),
            Err(e) => {
                let io = std::io::Error::from_raw_os_error(e.code().0 & 0xFFFF);
                if io.kind() == std::io::ErrorKind::AlreadyExists {
                    last = io;
                } else {
                    return Err(io);
                }
            }
        }
    }
    Err(last)
}

/// Xuất `job`; trả thư mục vừa tạo, hay lý do lỗi (chữ của hệ thống). Lỗi thì không để lại gì.
pub fn export(job: &ExportJob) -> Result<PathBuf, String> {
    let stray = job.replacements.iter().any(|(i, _)| *i >= job.steps.len());
    if job.steps.len() != job.sources.len() || stray {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput).to_string());
    }
    clean_stale(&job.parent);
    let staging = create_unique_dir(
        &job.parent,
        &format!("{STAGING_PREFIX}{}", std::process::id()),
    )
    .map_err(|e| e.to_string())?;
    let done = fill(&staging, job)
        .and_then(|()| publish(&staging, &job.parent, &export_dir_name(job.when)));
    match done {
        Ok(dir) => Ok(dir),
        Err(e) => {
            // Không nửa vời: bỏ cả thư mục tạm (chỉ chứa tệp của lượt này).
            let _ = std::fs::remove_dir_all(&staging);
            Err(e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::image::{encode_png, Image};

    fn when() -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 18,
            hour: 14,
            minute: 5,
            second: 59,
        }
    }

    /// Thư mục cha tạm riêng cho mỗi kiểm thử (đã tạo, rỗng).
    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "darkshot_steps_export_{}_{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    fn step(file: &str, caption: &str) -> Step {
        Step {
            file: file.to_string(),
            caption: caption.to_string(),
            scanned: false,
        }
    }

    /// Phiên nguồn: `files` ghi với nội dung riêng (`img-<tên>`).
    fn sources(dir: &Path, files: &[&str]) -> Vec<PathBuf> {
        std::fs::create_dir_all(dir).unwrap();
        files
            .iter()
            .map(|f| {
                let p = dir.join(f);
                std::fs::write(&p, format!("img-{f}")).unwrap();
                p
            })
            .collect()
    }

    #[test]
    fn unique_dir_never_reuses_an_existing_name() {
        let parent = tmp("unique");
        let base = "darkshot-steps-2026-11-18_14-05";
        let first = create_unique_dir(&parent, base).unwrap();
        assert_eq!(first, parent.join(base));
        assert!(first.is_dir());
        std::fs::write(first.join("marker.txt"), b"keep").unwrap();
        // Tên `_2` đã bị một tệp chiếm: cũng tính là đã có.
        std::fs::write(parent.join(format!("{base}_2")), b"file").unwrap();
        let third = create_unique_dir(&parent, base).unwrap();
        assert_eq!(third, parent.join(format!("{base}_3")));
        assert!(third.is_dir());
        // Thứ đã có giữ nguyên.
        assert_eq!(names(&first), ["marker.txt"]);
        assert_eq!(
            std::fs::read(parent.join(format!("{base}_2"))).unwrap(),
            b"file"
        );
        assert!(names(&third).is_empty());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn export_copies_images_in_display_order_and_writes_markdown() {
        let parent = tmp("order");
        let src = parent.join("session");
        // Thứ tự hiển thị khác thứ tự tên tệp.
        let files = ["003.png", "001.png", "002.png"];
        let paths = sources(&src, &files);
        let steps = vec![
            step("003.png", "Mở menu"),
            step("001.png", ""),
            step("002.png", "Bấm Lưu\nrồi đóng"),
        ];
        let job = ExportJob {
            parent: parent.clone(),
            when: when(),
            lang: Lang::Vi,
            steps: steps.clone(),
            sources: paths,
            replacements: Vec::new(),
        };
        let out = export(&job).unwrap();
        assert_eq!(out, parent.join(export_dir_name(when())));
        assert_eq!(names(&out), ["01.png", "02.png", "03.png", MARKDOWN_FILE]);
        for (k, f) in files.iter().enumerate() {
            let got = std::fs::read_to_string(out.join(format!("{:02}.png", k + 1))).unwrap();
            assert_eq!(got, format!("img-{f}"), "step {}", k + 1);
        }
        let md = std::fs::read_to_string(out.join(MARKDOWN_FILE)).unwrap();
        assert_eq!(md, markdown(Lang::Vi, when(), &steps));
        // Nguồn không bị đụng.
        assert_eq!(names(&src), ["001.png", "002.png", "003.png"]);
        // Tiếng Anh: Markdown theo ngôn ngữ của lượt xuất; lượt thứ hai vào `_2`.
        let job_en = ExportJob {
            lang: Lang::En,
            ..job
        };
        let out2 = export(&job_en).unwrap();
        assert_eq!(out2, parent.join(format!("{}_2", export_dir_name(when()))));
        let md = std::fs::read_to_string(out2.join(MARKDOWN_FILE)).unwrap();
        assert_eq!(md, markdown(Lang::En, when(), &steps));
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn export_never_writes_into_an_existing_folder() {
        let parent = tmp("existing");
        let base = parent.join(export_dir_name(when()));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("01.png"), b"old").unwrap();
        let paths = sources(&parent.join("session"), &["001.png"]);
        let job = ExportJob {
            parent: parent.clone(),
            when: when(),
            lang: Lang::En,
            steps: vec![step("001.png", "a")],
            sources: paths,
            replacements: Vec::new(),
        };
        let out = export(&job).unwrap();
        assert_ne!(out, base);
        assert_eq!(names(&base), ["01.png"]);
        assert_eq!(std::fs::read(base.join("01.png")).unwrap(), b"old");
        assert_eq!(names(&out), ["01.png", MARKDOWN_FILE]);
        let _ = std::fs::remove_dir_all(&parent);
    }

    /// Xuất thành công không để lại thư mục tạm; tên cuối đã bị một tệp chiếm thì sang `_2`, tệp
    /// giữ nguyên.
    #[test]
    fn export_leaves_no_staging_folder_and_skips_taken_names() {
        let parent = tmp("staging");
        let base = export_dir_name(when());
        std::fs::write(parent.join(&base), b"file").unwrap();
        let paths = sources(&parent.join("session"), &["001.png"]);
        let job = ExportJob {
            parent: parent.clone(),
            when: when(),
            lang: Lang::En,
            steps: vec![step("001.png", "")],
            sources: paths,
            replacements: Vec::new(),
        };
        let out = export(&job).unwrap();
        assert_eq!(out, parent.join(format!("{base}_2")));
        assert_eq!(std::fs::read(parent.join(&base)).unwrap(), b"file");
        assert_eq!(
            names(&parent),
            [base.clone(), format!("{base}_2"), "session".to_string()]
        );
        let _ = std::fs::remove_dir_all(&parent);
    }

    /// Thư mục tạm còn sót của lượt xuất bị ngắt (thoát giữa chừng) được dọn ở lần xuất sau; thư mục
    /// cùng tiền tố mà có tệp lạ thì không đụng.
    #[test]
    fn stale_staging_folders_are_cleaned_up() {
        let parent = tmp("stale");
        let stale = parent.join(format!("{STAGING_PREFIX}old"));
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("01.png"), b"x").unwrap();
        std::fs::write(stale.join(MARKDOWN_FILE), b"y").unwrap();
        let foreign = parent.join(format!("{STAGING_PREFIX}mine"));
        std::fs::create_dir_all(&foreign).unwrap();
        std::fs::write(foreign.join("notes.txt"), b"keep").unwrap();
        let paths = sources(&parent.join("session"), &["001.png"]);
        let job = ExportJob {
            parent: parent.clone(),
            when: when(),
            lang: Lang::En,
            steps: vec![step("001.png", "")],
            sources: paths,
            replacements: Vec::new(),
        };
        export(&job).unwrap();
        assert!(!stale.exists());
        assert_eq!(std::fs::read(foreign.join("notes.txt")).unwrap(), b"keep");
        let _ = std::fs::remove_dir_all(&parent);
    }

    /// Ảnh nguồn thứ hai mất giữa chừng: lỗi, không để lại thư mục xuất dở.
    #[test]
    fn failure_midway_leaves_no_partial_folder() {
        let parent = tmp("midway");
        let src = parent.join("session");
        let mut paths = sources(&src, &["001.png", "003.png"]);
        paths.insert(1, src.join("002.png"));
        let job = ExportJob {
            parent: parent.clone(),
            when: when(),
            lang: Lang::Vi,
            steps: vec![
                step("001.png", ""),
                step("002.png", ""),
                step("003.png", ""),
            ],
            sources: paths,
            replacements: Vec::new(),
        };
        let err = export(&job).unwrap_err();
        assert!(!err.is_empty());
        assert_eq!(names(&parent), ["session"]);
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn missing_parent_fails_without_creating_it() {
        let parent = tmp("noparent").join("not-there");
        let job = ExportJob {
            parent: parent.clone(),
            when: when(),
            lang: Lang::En,
            steps: Vec::new(),
            sources: Vec::new(),
            replacements: Vec::new(),
        };
        assert!(export(&job).is_err());
        assert!(!parent.exists());
        if let Some(p) = parent.parent() {
            let _ = std::fs::remove_dir_all(p);
        }
    }

    /// Ảnh tổng hợp `w × h` màu `v`, có vùng đen đặc `black` (x, y, w, h) như ảnh đã che.
    fn covered(w: u32, h: u32, v: u8, black: (u32, u32, u32, u32)) -> Image {
        let mut bgra = vec![v; (w * h * 4) as usize];
        for y in black.1..black.1 + black.3 {
            for x in black.0..black.0 + black.2 {
                let i = ((y * w + x) * 4) as usize;
                bgra[i..i + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
        }
        for px in bgra.chunks_mut(4) {
            px[3] = 255;
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    /// Bước có ảnh thay thế: tệp xuất là đúng byte PNG đã mã hóa sẵn của ảnh đó (đọc lại ra đúng
    /// điểm đen); bước khác chép nguyên; tệp của phiên không bị viết lại.
    #[test]
    fn replacements_are_written_instead_of_copied() {
        let parent = tmp("replace");
        let src = parent.join("session");
        let files = ["001.png", "002.png", "003.png"];
        let paths = sources(&src, &files);
        let before: Vec<Vec<u8>> = paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
        let first = covered(20, 10, 180, (2, 3, 6, 4));
        let third = covered(12, 12, 90, (0, 0, 12, 5));
        let job = ExportJob {
            parent: parent.clone(),
            when: when(),
            lang: Lang::En,
            steps: files.iter().map(|f| step(f, "")).collect(),
            sources: paths.clone(),
            replacements: vec![
                (2, encode_png(&third).unwrap()),
                (0, encode_png(&first).unwrap()),
            ],
        };
        let out = export(&job).unwrap();
        assert_eq!(
            std::fs::read(out.join("01.png")).unwrap(),
            encode_png(&first).unwrap()
        );
        assert_eq!(names(&out), ["01.png", "02.png", "03.png", MARKDOWN_FILE]);
        for (name, want) in [("01.png", &first), ("03.png", &third)] {
            let got = darkshot_core::thumb::decode_png_file(&out.join(name)).unwrap();
            assert_eq!((got.width, got.height), (want.width, want.height), "{name}");
            for (i, (g, w)) in got.bgra.chunks(4).zip(want.bgra.chunks(4)).enumerate() {
                assert_eq!(g[..3], w[..3], "{name} pixel {i}");
            }
        }
        let black = |img: &Image, x: u32, y: u32| {
            let i = ((y * img.width + x) * 4) as usize;
            img.bgra[i..i + 3] == [0, 0, 0]
        };
        let got = darkshot_core::thumb::decode_png_file(&out.join("01.png")).unwrap();
        assert!(black(&got, 4, 5) && !black(&got, 15, 8));
        // Bước không thay thế: bản chép nguyên.
        assert_eq!(std::fs::read(out.join("02.png")).unwrap(), b"img-002.png");
        for (p, b) in paths.iter().zip(&before) {
            assert_eq!(&std::fs::read(p).unwrap(), b, "{p:?}");
        }
        let _ = std::fs::remove_dir_all(&parent);
    }

    /// Ảnh thay thế cho bước không có (lỗi lập trình): lỗi, không để lại gì. (Ảnh không mã hóa
    /// được bị chặn trước đó, ở `steps_window`, khi cổng trả ảnh.)
    #[test]
    fn bad_replacements_fail_without_leaving_anything() {
        let parent = tmp("badrepl");
        let paths = sources(&parent.join("session"), &["001.png"]);
        let png = encode_png(&covered(4, 4, 7, (0, 0, 1, 1))).unwrap();
        let job = ExportJob {
            parent: parent.clone(),
            when: when(),
            lang: Lang::En,
            steps: vec![step("001.png", "")],
            sources: paths,
            replacements: vec![(1, png)],
        };
        assert!(!export(&job).unwrap_err().is_empty());
        assert_eq!(names(&parent), ["session"]);
        let _ = std::fs::remove_dir_all(&parent);
    }

    /// Tệp đích đã có (không bao giờ ghi đè): ghi byte PNG cũng tạo mới, lỗi nếu tên đã có.
    #[test]
    fn replacement_bytes_never_overwrite() {
        let dir = tmp("nooverwrite");
        let dst = dir.join("01.png");
        std::fs::write(&dst, b"old").unwrap();
        assert!(write_new(&dst, b"new").is_err());
        assert_eq!(std::fs::read(&dst).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Số bước và số ảnh nguồn lệch nhau (lỗi lập trình): báo lỗi, không tạo gì.
    #[test]
    fn mismatched_job_is_rejected() {
        let parent = tmp("mismatch");
        let job = ExportJob {
            parent: parent.clone(),
            when: when(),
            lang: Lang::En,
            steps: vec![step("001.png", "")],
            sources: Vec::new(),
            replacements: Vec::new(),
        };
        assert!(export(&job).is_err());
        assert!(names(&parent).is_empty());
        let _ = std::fs::remove_dir_all(&parent);
    }
}
