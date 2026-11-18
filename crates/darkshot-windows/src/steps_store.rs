//! Phiên chụp từng bước trên đĩa: thư mục `paths::steps_dir()` gồm ảnh `NNN.png` và `session.json`.

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
        let base = std::env::temp_dir().join(format!(
            "darkshot_steps_{}_{name}",
            std::process::id()
        ));
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
