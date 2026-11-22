//! Ảnh đã qua Privacy Shield, sẵn sàng xuất. Chỉ tạo được qua các hàm ở đây;
//! `scripts/check.ps1` chặn lời gọi chúng ngoài lõi `privacy/` và `privacy_gate.rs`.
//! Module này không ghi log.

use std::fmt;

use super::model::RedactionPlan;
use super::redact::{self, RedactError};
use crate::image::Image;

/// Trạng thái bảo vệ của một ảnh xuất.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protection {
    /// Shield tắt lúc xuất.
    Disabled,
    /// Quét xong, không phát hiện.
    NoFindings,
    /// Đã che ≥ 1 vùng (tự động hoặc tự thêm).
    Redacted { regions: u32 },
    /// Có phát hiện, người dùng xác nhận không che.
    ReviewedNoneApplied,
    /// OCR lỗi, người dùng xác nhận đi tiếp (có thể tự che).
    SkippedAfterError { regions: u32 },
}

impl Protection {
    /// Đúng khi ảnh đã được quét xong: `NoFindings`, `Redacted`, `ReviewedNoneApplied`.
    pub fn scanned(&self) -> bool {
        matches!(
            self,
            Protection::NoFindings | Protection::Redacted { .. } | Protection::ReviewedNoneApplied
        )
    }
}

/// Ảnh xuất kèm trạng thái bảo vệ. Trường riêng tư, không `Clone`.
pub struct PreparedOutput {
    image: Image,
    protection: Protection,
}

/// Số vùng của plan còn phủ ít nhất một điểm ảnh (trước khi gộp).
fn regions(image: &Image, plan: &RedactionPlan) -> u32 {
    let n = redact::covered_count(&plan.rects, image.width, image.height);
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl PreparedOutput {
    /// Shield tắt: ảnh giữ nguyên, `Protection::Disabled`.
    pub fn unscanned(image: Image) -> Self {
        PreparedOutput {
            image,
            protection: Protection::Disabled,
        }
    }

    pub fn no_findings(image: Image) -> Self {
        PreparedOutput {
            image,
            protection: Protection::NoFindings,
        }
    }

    pub fn reviewed_none(image: Image) -> Self {
        PreparedOutput {
            image,
            protection: Protection::ReviewedNoneApplied,
        }
    }

    /// Làm phẳng plan; plan rỗng (hoặc không vùng nào chạm ảnh) là lỗi lập trình:
    /// trả `Err(RedactError::InvalidImage)` (dùng `reviewed_none`/`skipped_after_error` thay).
    pub fn redacted(image: Image, plan: &RedactionPlan) -> Result<Self, RedactError> {
        let regions = regions(&image, plan);
        if regions == 0 {
            return Err(RedactError::InvalidImage);
        }
        let image = redact::flatten(&image, &plan.rects)?;
        Ok(PreparedOutput {
            image,
            protection: Protection::Redacted { regions },
        })
    }

    /// OCR lỗi, người dùng xác nhận đi tiếp; plan có thể rỗng (regions = 0).
    pub fn skipped_after_error(image: Image, plan: &RedactionPlan) -> Result<Self, RedactError> {
        let regions = regions(&image, plan);
        let image = if regions == 0 {
            if !image.is_valid() {
                return Err(RedactError::InvalidImage);
            }
            image
        } else {
            redact::flatten(&image, &plan.rects)?
        };
        Ok(PreparedOutput {
            image,
            protection: Protection::SkippedAfterError { regions },
        })
    }

    pub fn image(&self) -> &Image {
        &self.image
    }

    pub fn protection(&self) -> Protection {
        self.protection
    }

    /// Chỉ cho đầu ra cuối.
    pub fn into_image(self) -> Image {
        self.image
    }
}

impl fmt::Debug for PreparedOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedOutput")
            .field("width", &self.image.width)
            .field("height", &self.image.height)
            .field("protection", &self.protection)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    fn img(w: u32, h: u32) -> Image {
        let mut bgra = Vec::new();
        for i in 0..w * h {
            bgra.extend_from_slice(&[(i * 7) as u8 | 1, (i * 3) as u8 | 1, 0x80, 0xC0]);
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    fn plan(rects: &[Rect]) -> RedactionPlan {
        RedactionPlan {
            rects: rects.to_vec(),
        }
    }

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn scanned_states() {
        assert!(!Protection::Disabled.scanned());
        assert!(Protection::NoFindings.scanned());
        assert!(Protection::Redacted { regions: 1 }.scanned());
        assert!(Protection::ReviewedNoneApplied.scanned());
        assert!(!Protection::SkippedAfterError { regions: 0 }.scanned());
        assert!(!Protection::SkippedAfterError { regions: 2 }.scanned());
    }

    #[test]
    fn plain_constructors_keep_image() {
        let src = img(10, 8);
        let p = PreparedOutput::unscanned(src.clone());
        assert_eq!(p.protection(), Protection::Disabled);
        assert_eq!(p.image(), &src);
        let p = PreparedOutput::no_findings(src.clone());
        assert_eq!(p.protection(), Protection::NoFindings);
        assert_eq!(p.into_image(), src);
        let p = PreparedOutput::reviewed_none(src.clone());
        assert_eq!(p.protection(), Protection::ReviewedNoneApplied);
        assert_eq!(p.into_image(), src);
    }

    #[test]
    fn redacted_flattens_and_counts_regions() {
        let src = img(40, 40);
        let pl = plan(&[r(5, 5, 4, 4), r(25, 25, 4, 4), r(100, 100, 4, 4)]);
        let p = PreparedOutput::redacted(src.clone(), &pl).unwrap();
        assert_eq!(p.protection(), Protection::Redacted { regions: 2 });
        assert!(p.protection().scanned());
        let want = redact::flatten(&src, &pl.rects).unwrap();
        assert_eq!(p.image(), &want);
        let i = (6 * 40 + 6) * 4;
        assert_eq!(&p.image().bgra[i..i + 4], &[0, 0, 0, 255]);
        assert_eq!(p.into_image(), want);
    }

    #[test]
    fn redacted_with_empty_plan_is_error() {
        let src = img(10, 10);
        assert_eq!(
            PreparedOutput::redacted(src.clone(), &plan(&[])).unwrap_err(),
            RedactError::InvalidImage
        );
        let outside = plan(&[r(50, 50, 3, 3), r(0, 0, 0, 5)]);
        assert!(PreparedOutput::redacted(src, &outside).is_err());
    }

    #[test]
    fn redacted_invalid_image_is_error() {
        let bad = Image {
            width: 3,
            height: 3,
            bgra: vec![0; 5],
        };
        assert_eq!(
            PreparedOutput::redacted(bad, &plan(&[r(0, 0, 1, 1)])).unwrap_err(),
            RedactError::InvalidImage
        );
    }

    #[test]
    fn skipped_after_error_with_and_without_regions() {
        let src = img(20, 20);
        let p = PreparedOutput::skipped_after_error(src.clone(), &plan(&[])).unwrap();
        assert_eq!(p.protection(), Protection::SkippedAfterError { regions: 0 });
        assert!(!p.protection().scanned());
        assert_eq!(p.image(), &src);

        let pl = plan(&[r(2, 2, 2, 2)]);
        let p = PreparedOutput::skipped_after_error(src.clone(), &pl).unwrap();
        assert_eq!(p.protection(), Protection::SkippedAfterError { regions: 1 });
        assert_eq!(p.into_image(), redact::flatten(&src, &pl.rects).unwrap());

        let bad = Image {
            width: 2,
            height: 2,
            bgra: Vec::new(),
        };
        assert!(PreparedOutput::skipped_after_error(bad, &plan(&[])).is_err());
    }

    #[test]
    fn debug_shows_only_size_and_protection() {
        let p = PreparedOutput::redacted(img(30, 20), &plan(&[r(1, 1, 2, 2)])).unwrap();
        let s = format!("{p:?}");
        assert_eq!(
            s,
            "PreparedOutput { width: 30, height: 20, protection: Redacted { regions: 1 } }"
        );
        assert!(!s.contains("bgra"));
        assert_eq!(format!("{:?}", RedactError::OutOfMemory), "OutOfMemory");
    }
}
