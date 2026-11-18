//! Phiên chụp từng bước: danh sách bước (tên tệp ảnh + ghi chú), JSON lưu phiên và tài liệu
//! Markdown khi xuất. Thuần, không đụng hệ điều hành.

use crate::i18n::{self, Lang};
use crate::naming::LocalTime;

pub const MAX_STEPS: usize = 99;
pub const MAX_CAPTION: usize = 2000;
pub const FILE_VERSION: u32 = 1;

/// Một bước: tên tệp ảnh trong thư mục phiên (`NNN.png`) và ghi chú.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub file: String,
    pub caption: String,
}

/// Phiên đã đủ `MAX_STEPS` bước.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Full;

/// Các bước theo thứ tự hiển thị.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StepSession {
    pub steps: Vec<Step>,
}

impl StepSession {
    /// Thêm bước cuối với ghi chú rỗng; trả chỉ số bước mới. Đủ `MAX_STEPS` thì `Err(Full)`.
    pub fn add(&mut self, file: String) -> Result<usize, Full> {
        if self.steps.len() >= MAX_STEPS {
            return Err(Full);
        }
        self.steps.push(Step {
            file,
            caption: String::new(),
        });
        Ok(self.steps.len() - 1)
    }

    /// `false` khi chỉ số ngoài phạm vi.
    pub fn remove(&mut self, i: usize) -> bool {
        if i >= self.steps.len() {
            return false;
        }
        self.steps.remove(i);
        true
    }

    /// Đổi bước `i` lên trước một vị trí; `false` khi `i` ngoài phạm vi hoặc đã ở đầu.
    pub fn move_up(&mut self, i: usize) -> bool {
        if i == 0 || i >= self.steps.len() {
            return false;
        }
        self.steps.swap(i - 1, i);
        true
    }

    /// Đổi bước `i` xuống sau một vị trí; `false` khi `i` ngoài phạm vi hoặc đã ở cuối.
    pub fn move_down(&mut self, i: usize) -> bool {
        if i + 1 >= self.steps.len() {
            return false;
        }
        self.steps.swap(i, i + 1);
        true
    }

    /// Đặt ghi chú, cắt còn `MAX_CAPTION` ký tự Unicode; `false` khi chỉ số ngoài phạm vi.
    pub fn set_caption(&mut self, i: usize, s: &str) -> bool {
        let Some(step) = self.steps.get_mut(i) else {
            return false;
        };
        step.caption = truncate_chars(s, MAX_CAPTION);
        true
    }

    /// Tên tệp kế tiếp (`001.png`...) không trùng tên đang có: số lớn nhất hiện có cộng một,
    /// quay vòng trong 1..=999 khi hết chỗ.
    pub fn next_file_name(&self) -> String {
        let used: Vec<u32> = self
            .steps
            .iter()
            .filter_map(|s| file_number(&s.file))
            .collect();
        let start = used.iter().copied().max().unwrap_or(0);
        let n = (0..999u32)
            .map(|k| (start + k) % 999 + 1)
            .find(|n| !used.contains(n))
            .unwrap_or(1);
        format!("{n:03}.png")
    }

    pub fn to_json(&self) -> String {
        let steps: Vec<serde_json::Value> = self
            .steps
            .iter()
            .map(|s| serde_json::json!({ "file": s.file, "caption": s.caption }))
            .collect();
        serde_json::json!({ "version": FILE_VERSION, "steps": steps }).to_string()
    }

    /// `None` khi JSON hỏng, version khác 1, hoặc có `file` không khớp `^\d{3}\.png$`.
    /// Tên trùng bị bỏ; quá `MAX_STEPS` bước thì bỏ phần thừa; ghi chú dài bị cắt.
    pub fn from_json(s: &str) -> Option<StepSession> {
        let v: serde_json::Value = serde_json::from_str(s).ok()?;
        if v.get("version")?.as_u64()? != u64::from(FILE_VERSION) {
            return None;
        }
        let mut out = StepSession::default();
        for item in v.get("steps")?.as_array()? {
            let file = item.get("file")?.as_str()?;
            file_number(file)?;
            let caption = match item.get("caption") {
                None => "",
                Some(c) => c.as_str()?,
            };
            if out.steps.iter().any(|x| x.file == file) || out.steps.len() >= MAX_STEPS {
                continue;
            }
            out.steps.push(Step {
                file: file.to_string(),
                caption: truncate_chars(caption, MAX_CAPTION),
            });
        }
        Some(out)
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Số của tên tệp dạng `NNN.png` (đúng ba chữ số ASCII, đuôi `.png` chữ thường); None nếu khác.
fn file_number(file: &str) -> Option<u32> {
    let digits = file.strip_suffix(".png")?;
    if digits.len() != 3 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Tên tệp ảnh khi xuất cho `n` bước: `01.png`... (`001.png`... khi n >= 100).
pub fn export_names(n: usize) -> Vec<String> {
    let width = if n >= 100 { 3 } else { 2 };
    (1..=n).map(|k| format!("{k:0width$}.png")).collect()
}

/// Thoát một dòng ghi chú để Markdown giữ nó là chữ thường: bỏ thụt đầu dòng, thoát ký tự
/// cú pháp ở mọi vị trí và dấu hiệu khối (tiêu đề, danh sách, trích dẫn, gạch ngang) ở đầu dòng.
fn escape_line(line: &str) -> String {
    let body = line.trim_start();
    let chars: Vec<char> = body.chars().collect();
    let mut out = String::with_capacity(body.len() + 4);
    // Vị trí (theo ký tự) cần thêm `\` ở đầu dòng; chỉ ký tự ASCII nên so khớp theo chỉ số.
    let digits = chars.iter().take_while(|c| c.is_ascii_digit()).count();
    // Dấu danh sách `-`/`+`/`*` theo sau bởi khoảng trắng (cách hay tab) hoặc đứng một mình.
    let list_marker = matches!(chars.first(), Some('-' | '+' | '*'))
        && chars.get(1).is_none_or(|c| c.is_whitespace());
    let block_at = if body.starts_with('#')
        || body.starts_with("---")
        || body.starts_with('~')
        || list_marker
        || (!chars.is_empty() && chars.iter().all(|&c| c == '='))
        || (!chars.is_empty() && chars.iter().all(|&c| c == '-'))
    {
        Some(0)
    } else if (1..=9).contains(&digits)
        && matches!(chars.get(digits), Some('.') | Some(')'))
        && chars.get(digits + 1).is_none_or(|c| c.is_whitespace())
    {
        Some(digits)
    } else {
        None
    };
    for (k, &c) in chars.iter().enumerate() {
        let inline = matches!(c, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '|')
            || (c == '!' && chars.get(k + 1) == Some(&'['));
        // `>` đã thuộc nhóm inline nên dòng trích dẫn cũng được thoát ở đây.
        if inline || block_at == Some(k) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
/// Nội dung `steps.md` cho các bước (đã theo thứ tự hiển thị).
pub fn markdown(lang: Lang, when: LocalTime, steps: &[Step]) -> String {
    let names = export_names(steps.len());
    let mut out = format!(
        "# {}\n_darkshot, {:04}-{:02}-{:02} {:02}:{:02}_\n",
        i18n::t(lang, i18n::Msg::StepsWindowTitle),
        when.year,
        when.month,
        when.day,
        when.hour,
        when.minute
    );
    for (k, (step, name)) in steps.iter().zip(&names).enumerate() {
        let label = i18n::step_label(lang, k + 1);
        out.push_str(&format!("\n## {label}\n"));
        let caption = step.caption.replace("\r\n", "\n").replace('\r', "\n");
        let caption = caption.trim_matches('\n');
        if !caption.trim().is_empty() {
            let lines: Vec<String> = caption.split('\n').map(escape_line).collect();
            for (n, line) in lines.iter().enumerate() {
                out.push_str(line);
                // Ngắt dòng cứng CommonMark, sau bước thoát: chỉ giữa hai dòng có chữ.
                let next_has_text = lines.get(n + 1).is_some_and(|l| !l.is_empty());
                if !line.is_empty() && next_has_text {
                    out.push('\\');
                }
                out.push('\n');
            }
        }
        out.push_str(&format!("\n![{label}]({name})\n"));
    }
    out
}

/// Tên thư mục xuất: `darkshot-steps-YYYY-MM-DD_HH-mm`.
pub fn export_dir_name(when: LocalTime) -> String {
    format!(
        "darkshot-steps-{:04}-{:02}-{:02}_{:02}-{:02}",
        when.year, when.month, when.day, when.hour, when.minute
    )
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

    fn session(n: usize) -> StepSession {
        let mut s = StepSession::default();
        for _ in 0..n {
            let f = s.next_file_name();
            s.add(f).unwrap();
        }
        s
    }

    fn files(s: &StepSession) -> Vec<&str> {
        s.steps.iter().map(|x| x.file.as_str()).collect()
    }

    #[test]
    fn add_up_to_max_then_full() {
        let mut s = StepSession::default();
        for k in 0..MAX_STEPS {
            let f = s.next_file_name();
            assert_eq!(s.add(f), Ok(k));
        }
        assert_eq!(s.steps.len(), 99);
        assert_eq!(s.add("100.png".to_string()), Err(Full));
        assert_eq!(s.steps.len(), 99);
        assert!(s.steps.iter().all(|x| x.caption.is_empty()));
    }

    #[test]
    fn remove_in_and_out_of_range() {
        let mut s = session(3);
        assert!(!s.remove(3));
        assert_eq!(s.steps.len(), 3);
        assert!(s.remove(1));
        assert_eq!(files(&s), ["001.png", "003.png"]);
        assert!(s.remove(1));
        assert!(s.remove(0));
        assert!(!s.remove(0));
    }

    #[test]
    fn move_up_down_at_edges() {
        let mut s = session(3);
        assert!(!s.move_up(0));
        assert!(!s.move_down(2));
        assert!(!s.move_up(3));
        assert!(!s.move_down(3));
        assert_eq!(files(&s), ["001.png", "002.png", "003.png"]);
        assert!(s.move_up(2));
        assert_eq!(files(&s), ["001.png", "003.png", "002.png"]);
        assert!(s.move_down(0));
        assert_eq!(files(&s), ["003.png", "001.png", "002.png"]);
    }

    #[test]
    fn caption_truncates_by_chars() {
        let mut s = session(1);
        assert!(!s.set_caption(1, "x"));
        let long: String = "ế".repeat(MAX_CAPTION + 50);
        assert!(s.set_caption(0, &long));
        assert_eq!(s.steps[0].caption.chars().count(), MAX_CAPTION);
        assert!(s.steps[0].caption.chars().all(|c| c == 'ế'));
        let exact: String = "đ".repeat(MAX_CAPTION);
        s.set_caption(0, &exact);
        assert_eq!(s.steps[0].caption, exact);
        s.set_caption(0, "ngắn");
        assert_eq!(s.steps[0].caption, "ngắn");
    }

    #[test]
    fn next_file_name_never_collides_after_removal() {
        let mut s = StepSession::default();
        assert_eq!(s.next_file_name(), "001.png");
        for _ in 0..3 {
            let f = s.next_file_name();
            s.add(f).unwrap();
        }
        s.remove(0);
        assert_eq!(s.next_file_name(), "004.png");
        s.remove(2);
        let n = s.next_file_name();
        assert!(!files(&s).contains(&n.as_str()), "{n}");
        // Gần cuối dải: vẫn không trùng.
        let mut t = StepSession::default();
        t.add("999.png".to_string()).unwrap();
        let n = t.next_file_name();
        assert_ne!(n, "999.png");
        assert_eq!(n, "001.png");
    }

    #[test]
    fn json_round_trip() {
        let mut s = session(3);
        s.set_caption(0, "một \"dòng\"\nhai dòng");
        s.set_caption(2, "Ghi chú ế");
        s.move_up(2);
        let back = StepSession::from_json(&s.to_json()).unwrap();
        assert_eq!(back, s);
        let empty = StepSession::default();
        assert_eq!(StepSession::from_json(&empty.to_json()), Some(empty));
    }

    #[test]
    fn from_json_rejects_bad_input() {
        let ok = r#"{"version":1,"steps":[{"file":"001.png","caption":"a"}]}"#;
        assert!(StepSession::from_json(ok).is_some());
        for bad in [
            "",
            "rác",
            "{",
            "[]",
            "null",
            r#"{"steps":[]}"#,
            r#"{"version":2,"steps":[]}"#,
            r#"{"version":1}"#,
            r#"{"version":1,"steps":[{"file":"../x.png","caption":""}]}"#,
            r#"{"version":1,"steps":[{"file":"1.png","caption":""}]}"#,
            r#"{"version":1,"steps":[{"file":"001.PNG.exe","caption":""}]}"#,
            r#"{"version":1,"steps":[{"file":"001.PNG","caption":""}]}"#,
            r#"{"version":1,"steps":[{"file":"0011.png","caption":""}]}"#,
            r#"{"version":1,"steps":[{"file":"١٢٣.png","caption":""}]}"#,
            r#"{"version":1,"steps":[{"file":"001.png\n","caption":""}]}"#,
            r#"{"version":1,"steps":[{"file":"sub/001.png","caption":""}]}"#,
            r#"{"version":1,"steps":[{"file":5,"caption":""}]}"#,
            r#"{"version":1,"steps":[{"caption":"x"}]}"#,
            r#"{"version":1,"steps":[{"file":"001.png","caption":5}]}"#,
        ] {
            assert_eq!(StepSession::from_json(bad), None, "{bad}");
        }
    }

    #[test]
    fn from_json_dedupes_and_truncates() {
        let long = "x".repeat(MAX_CAPTION + 10);
        let j = format!(
            r#"{{"version":1,"steps":[{{"file":"001.png","caption":"{long}"}},{{"file":"001.png","caption":"b"}},{{"file":"002.png","caption":"c"}}]}}"#
        );
        let s = StepSession::from_json(&j).unwrap();
        assert_eq!(files(&s), ["001.png", "002.png"]);
        assert_eq!(s.steps[0].caption.chars().count(), MAX_CAPTION);
        // Quá MAX_STEPS bước: bỏ phần thừa.
        let many: Vec<String> = (1..=120)
            .map(|k| format!(r#"{{"file":"{k:03}.png","caption":""}}"#))
            .collect();
        let j = format!(r#"{{"version":1,"steps":[{}]}}"#, many.join(","));
        assert_eq!(StepSession::from_json(&j).unwrap().steps.len(), MAX_STEPS);
    }

    #[test]
    fn export_names_widths() {
        assert!(export_names(0).is_empty());
        assert_eq!(export_names(3), ["01.png", "02.png", "03.png"]);
        let n = export_names(99);
        assert_eq!((n[0].as_str(), n[98].as_str()), ("01.png", "99.png"));
        let n = export_names(100);
        assert_eq!((n[0].as_str(), n[99].as_str()), ("001.png", "100.png"));
    }

    #[test]
    fn export_dir_name_format() {
        assert_eq!(export_dir_name(when()), "darkshot-steps-2026-11-18_14-05");
        let w = LocalTime {
            year: 2026,
            month: 1,
            day: 2,
            hour: 3,
            minute: 4,
            second: 5,
        };
        assert_eq!(export_dir_name(w), "darkshot-steps-2026-01-02_03-04");
    }

    fn step(file: &str, caption: &str) -> Step {
        Step {
            file: file.to_string(),
            caption: caption.to_string(),
        }
    }

    #[test]
    fn markdown_vi_with_and_without_caption() {
        let steps = [step("001.png", "Mở menu"), step("002.png", "")];
        let md = markdown(Lang::Vi, when(), &steps);
        assert_eq!(
            md,
            "# Các bước\n_darkshot, 2026-11-18 14:05_\n\n\
## Bước 1\nMở menu\n\n![Bước 1](01.png)\n\n\
## Bước 2\n\n![Bước 2](02.png)\n"
        );
    }

    #[test]
    fn markdown_en() {
        let steps = [step("007.png", "Click Save")];
        let md = markdown(Lang::En, when(), &steps);
        assert_eq!(
            md,
            "# Steps\n_darkshot, 2026-11-18 14:05_\n\n\
## Step 1\nClick Save\n\n![Step 1](01.png)\n"
        );
    }

    #[test]
    fn markdown_empty_session_has_header_only() {
        let md = markdown(Lang::En, when(), &[]);
        assert_eq!(md, "# Steps\n_darkshot, 2026-11-18 14:05_\n");
    }

    /// Phần ghi chú đã thoát của bước duy nhất trong tài liệu.
    fn rendered_caption(caption: &str) -> String {
        let md = markdown(Lang::Vi, when(), &[step("001.png", caption)]);
        let body = md.split_once("## Bước 1\n").unwrap().1;
        let body = body.rsplit_once("\n![Bước 1](01.png)").unwrap().0;
        body.to_string()
    }

    #[test]
    fn markdown_multiline_caption_and_heading_escape() {
        let steps = [step(
            "001.png",
            "dòng một\r\n# không phải tiêu đề\n  ## cũng không\nhết",
        )];
        let md = markdown(Lang::Vi, when(), &steps);
        assert!(!md.contains('\r'));
        assert!(
            md.contains("\ndòng một\\\n\\# không phải tiêu đề\\\n\\## cũng không\\\nhết\n"),
            "{md}"
        );
        assert!(md.contains("\nhết\n\n![Bước 1](01.png)"));
        // Chỉ có đúng một tiêu đề cấp 1 và một cấp 2 thật.
        let real: Vec<&str> = md
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .collect();
        assert_eq!(real, ["# Các bước", "## Bước 1"]);
    }

    #[test]
    fn markdown_escapes_block_structure() {
        assert_eq!(rendered_caption("a\n---"), "a\\\n\\---\n");
        assert_eq!(rendered_caption("a\n==="), "a\\\n\\===\n");
        assert_eq!(rendered_caption("- a"), "\\- a\n");
        assert_eq!(rendered_caption("+ a"), "\\+ a\n");
        assert_eq!(rendered_caption("1. a"), "1\\. a\n");
        assert_eq!(rendered_caption("12) a"), "12\\) a\n");
        assert_eq!(rendered_caption("> a"), "\\> a\n");
    }

    #[test]
    fn markdown_escapes_more_block_markers() {
        // Dòng chỉ gồm `-` (từ hai ký tự): gạch ngang/tiêu đề setext.
        assert_eq!(rendered_caption("--"), "\\--\n");
        assert_eq!(rendered_caption("a\n--"), "a\\\n\\--\n");
        assert_eq!(rendered_caption("-----"), "\\-----\n");
        // `~` đầu dòng (khối mã `~~~`).
        assert_eq!(rendered_caption("~~~"), "\\~~~\n");
        assert_eq!(rendered_caption("~ a"), "\\~ a\n");
        // Dấu danh sách theo sau bởi tab.
        assert_eq!(rendered_caption("-\ta"), "\\-\ta\n");
        assert_eq!(rendered_caption("+\tb"), "\\+\tb\n");
        assert_eq!(rendered_caption("*\tc"), "\\*\tc\n");
        // `*` đầu dòng theo sau bởi khoảng trắng: thoát đúng một lần.
        assert_eq!(rendered_caption("* a"), "\\* a\n");
        // Chữ thường không bị thoát thừa.
        assert_eq!(rendered_caption("a~b"), "a~b\n");
        assert_eq!(rendered_caption("-a"), "-a\n");
        assert_eq!(rendered_caption("a -- b"), "a -- b\n");
    }

    #[test]
    fn markdown_escapes_inline_syntax() {
        assert_eq!(rendered_caption("![x](u)"), "\\!\\[x\\](u)\n");
        assert_eq!(rendered_caption("<b>"), "\\<b\\>\n");
        assert_eq!(rendered_caption("| a |"), "\\| a \\|\n");
        assert_eq!(
            rendered_caption("**b** _c_ `d`"),
            "\\*\\*b\\*\\* \\_c\\_ \\`d\\`\n"
        );
        assert_eq!(rendered_caption("[l](u)"), "\\[l\\](u)\n");
    }

    #[test]
    fn markdown_does_not_over_escape_plain_text() {
        assert_eq!(
            rendered_caption("Bấm Lưu, rồi đóng. Xong! 3.5 là 1-2 và a-b # x"),
            "Bấm Lưu, rồi đóng. Xong! 3.5 là 1-2 và a-b # x\n"
        );
        assert_eq!(rendered_caption("2024. Bắt đầu"), "2024\\. Bắt đầu\n");
        assert_eq!(rendered_caption("1.5 lần"), "1.5 lần\n");
    }

    #[test]
    fn markdown_hard_breaks_after_escaping() {
        // Dòng kết thúc bằng `\` của người dùng: `\\` chữ, rồi `\` ngắt dòng.
        assert_eq!(rendered_caption("a\\\nb"), "a\\\\\\\nb\n");
        // Dòng cuối kết thúc bằng `\`: chỉ thoát, không thêm ngắt dòng.
        assert_eq!(rendered_caption("a\\"), "a\\\\\n");
        // Dòng rỗng giữa ghi chú giữ là dòng rỗng, không thêm `\`.
        assert_eq!(rendered_caption("a\n\nb\nc"), "a\n\nb\\\nc\n");
        assert_eq!(rendered_caption("a\n   \nb"), "a\n\nb\n");
    }
    #[test]
    fn markdown_uses_display_order_names() {
        let steps = [step("005.png", "b"), step("002.png", "a")];
        let md = markdown(Lang::En, when(), &steps);
        assert!(md.contains("![Step 1](01.png)") && md.contains("![Step 2](02.png)"));
        assert!(!md.contains("005.png") && !md.contains("002.png"));
    }

    #[test]
    fn i18n_helper_is_wired() {
        assert_eq!(i18n::step_label(Lang::En, 3), "Step 3");
    }
}
