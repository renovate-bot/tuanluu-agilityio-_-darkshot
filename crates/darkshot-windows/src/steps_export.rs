//! Xuất phiên chụp từng bước ra thư mục người dùng chọn: tạo thư mục con mới
//! `darkshot-steps-YYYY-MM-DD_HH-mm` (`_2`, `_3`… nếu tên đã có), chép ảnh theo thứ tự hiển thị
//! thành `01.png`, `02.png`… và ghi `steps.md`. Chạy trên luồng nền.
//!
//! An toàn: thư mục con được tạo bằng `create_dir` (lỗi nếu tên đã có, tệp hay thư mục), nên không
//! bao giờ ghi vào thư mục có sẵn; mọi tệp tạo bằng `create_new`. Lỗi giữa chừng thì xóa thư mục con
//! vừa tạo (không để lại kết quả nửa vời).

use darkshot_core::i18n::Lang;
use darkshot_core::naming::LocalTime;
use darkshot_core::steps::{export_dir_name, export_names, markdown, Step};
use std::path::{Path, PathBuf};

/// Tên tệp Markdown trong thư mục xuất.
pub const MARKDOWN_FILE: &str = "steps.md";

/// Tiền tố thư mục tạm của một lượt xuất đang chạy (đổi tên sang tên cuối khi xong).
pub const STAGING_PREFIX: &str = ".darkshot-steps-partial-";

/// Số hậu tố tối đa thử khi tên thư mục đã có (`_2` … `_999`).
const MAX_SUFFIX: u32 = 999;

/// Một lần xuất: thư mục cha, thời điểm (đặt tên, ghi trong Markdown), ngôn ngữ của Markdown, các
/// bước theo thứ tự hiển thị và đường dẫn ảnh nguồn tương ứng.
#[derive(Clone, Debug)]
pub struct ExportJob {
    pub parent: PathBuf,
    pub when: LocalTime,
    pub lang: Lang,
    pub steps: Vec<Step>,
    pub sources: Vec<PathBuf>,
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
    std::io::copy(&mut from, &mut to)?;
    to.sync_all()
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

/// Chép ảnh và ghi `steps.md` vào thư mục `dir` (vừa tạo, rỗng).
fn fill(dir: &Path, job: &ExportJob) -> std::io::Result<()> {
    let names = export_names(job.steps.len());
    for (src, name) in job.sources.iter().zip(&names) {
        copy_new(src, &dir.join(name))?;
    }
    let md = markdown(job.lang, job.when, &job.steps);
    write_new(&dir.join(MARKDOWN_FILE), md.as_bytes())
}

/// Xuất `job`; trả thư mục vừa tạo, hay lý do lỗi (chữ của hệ thống). Lỗi thì không để lại gì.
pub fn export(job: &ExportJob) -> Result<PathBuf, String> {
    if job.steps.len() != job.sources.len() {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput).to_string());
    }
    let dir =
        create_unique_dir(&job.parent, &export_dir_name(job.when)).map_err(|e| e.to_string())?;
    match fill(&dir, job) {
        Ok(()) => Ok(dir),
        Err(e) => {
            // Không nửa vời: bỏ cả thư mục vừa tạo (chỉ chứa tệp của lượt này).
            let _ = std::fs::remove_dir_all(&dir);
            Err(e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        };
        assert!(export(&job).is_err());
        assert!(!parent.exists());
        if let Some(p) = parent.parent() {
            let _ = std::fs::remove_dir_all(p);
        }
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
        };
        assert!(export(&job).is_err());
        assert!(names(&parent).is_empty());
        let _ = std::fs::remove_dir_all(&parent);
    }
}
