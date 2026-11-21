//! Privacy Shield: dò thông tin nhạy cảm trên chữ OCR (không phụ thuộc hệ điều hành).
//! Kết quả chỉ có vị trí và loại, không có chữ. Module này không ghi log.

pub mod detect;
pub mod geom;
pub mod model;
pub mod prepared;
pub mod redact;
pub mod text;

pub use detect::{Detector, LineCtx, RawMatch, Span};
pub use geom::merge_tile;
pub use model::{
    Category, Confidence, Finding, Mapping, Redaction, RedactionPlan, RedactionSource, RuleId,
    ScanError, ScanReport,
};
pub use prepared::{PreparedOutput, Protection};
pub use redact::RedactError;

use crate::ocr_text::OcrLine;
use text::LineText;

/// Dò trên các dòng OCR (tọa độ px ảnh gốc). Không giữ chữ trong kết quả.
pub fn detect(lines: &[OcrLine]) -> Vec<Finding> {
    // Bộ đệm chữ dòng ghi đè bằng 0 khi thả (Drop của `text::Version`).
    let texts: Vec<LineText> = lines.iter().map(LineText::build).collect();
    let mut raw: Vec<RawMatch> = Vec::new();
    for index in 0..texts.len() {
        for d in detect::ALL {
            let mut passes = [false, true].as_slice();
            if !(d.on_joined() && texts[index].has_join()) {
                passes = &passes[..1];
            }
            for &joined in passes {
                let ctx = LineCtx {
                    lines: &texts,
                    index,
                    joined,
                };
                d.scan(&ctx, &mut raw);
            }
        }
    }
    let mut out: Vec<Finding> = raw.iter().map(|m| geom::to_finding(&texts, m)).collect();
    drop(texts);
    geom::normalize(&mut out);
    out
}

#[cfg(test)]
mod tests;
