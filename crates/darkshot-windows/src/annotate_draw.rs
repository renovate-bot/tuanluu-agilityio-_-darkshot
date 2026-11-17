//! Vẽ các hình đánh dấu bằng GDI+: một hàm vẽ dùng chung cho lớp phủ và cho ảnh ra (`burn`).

// Lớp phủ (Task 36, 37) nối các hàm này vào; tới lúc đó chỉ kiểm thử dùng.
#![allow(dead_code)]

use darkshot_core::annotate::{Shape, TextEdit};
use darkshot_core::geometry::Rect;
use darkshot_core::image::Image;
use windows::Win32::Graphics::Gdi::HDC;

/// Vẽ `shapes` lên `dc`: điểm ảnh (x, y) của ảnh chụp nằm ở (x - ox, y - oy) của dc; chỉ vẽ trong `clip`
/// (tọa độ ảnh chụp). GDI+ phải đang chạy. false khi không tạo được đối tượng GDI+ (đã vẽ được gì thì thôi).
pub fn draw_shapes(dc: HDC, shapes: &[Shape], ox: i32, oy: i32, clip: Rect) -> bool {
    let _ = (dc, shapes, ox, oy, clip);
    todo!()
}

/// Hộp bao (tọa độ ảnh chụp) của một hình Text, đo bằng GDI+ cùng phông với lúc vẽ; cũng dùng cho mọi hình khác (gọi Shape::bounds).
pub fn shape_bounds(s: &Shape) -> Rect {
    let _ = s;
    todo!()
}

/// Vị trí và chiều cao con trỏ nháy (tọa độ ảnh chụp) của ô chữ đang soạn (Task 37 dùng).
pub fn caret_rect(edit: &TextEdit) -> Rect {
    let _ = edit;
    todo!()
}

/// In `shapes` vào ảnh đã cắt từ vùng `crop` (tọa độ ảnh chụp). Tự khởi động GDI+. Ảnh giữ alpha 255.
pub fn burn(img: &mut Image, shapes: &[Shape], crop: Rect) -> Result<(), String> {
    let _ = (img, shapes, crop);
    todo!()
}

/// Chép `img` vào một DIB 32 bit, gọi `paint` với DC của nó, chép kết quả về `img`.
fn with_dib(img: &mut Image, paint: impl FnOnce(HDC) -> bool) -> Result<(), String> {
    let _ = (img, paint);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gdiplus::GdiPlus;
    use darkshot_core::annotate::{arrow_min_head, Pt, PALETTE};
    use darkshot_core::theme::Rgb;

    const RED: Rgb = Rgb(255, 0, 0);

    fn pt(x: f32, y: f32) -> Pt {
        Pt { x, y }
    }

    fn solid(w: u32, h: u32, v: u8) -> Image {
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            bgra.extend_from_slice(&[v, v, v, 255]);
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    /// Ảnh giả ngẫu nhiên (LCG), alpha 255.
    fn noise(w: u32, h: u32, seed: u32) -> Image {
        let mut s = seed;
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            for _ in 0..3 {
                s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                bgra.push((s >> 24) as u8);
            }
            bgra.push(255);
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    /// [B, G, R, A] tại (x, y).
    fn px(img: &Image, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * img.width + x) * 4) as usize;
        [
            img.bgra[i],
            img.bgra[i + 1],
            img.bgra[i + 2],
            img.bgra[i + 3],
        ]
    }

    fn near(a: [u8; 4], b: [u8; 4], tol: u8) -> bool {
        a.iter().zip(b.iter()).all(|(x, y)| x.abs_diff(*y) <= tol)
    }

    fn full(img: &Image) -> Rect {
        Rect {
            x: 0,
            y: 0,
            w: img.width as i32,
            h: img.height as i32,
        }
    }

    fn all_opaque(img: &Image) -> bool {
        img.bgra.chunks_exact(4).all(|p| p[3] == 255)
    }

    #[test]
    fn empty_burn_is_identity() {
        let mut img = noise(64, 40, 7);
        let before = img.clone();
        let crop = Rect {
            x: 13,
            y: 9,
            w: 64,
            h: 40,
        };
        assert_eq!(burn(&mut img, &[], crop), Ok(()));
        assert_eq!(img, before);
    }

    #[test]
    fn line_pixels() {
        let mut img = solid(40, 20, 0);
        let line = Shape::Line {
            a: pt(5.0, 10.0),
            b: pt(35.0, 10.0),
            width: 3.0,
            color: RED,
        };
        let crop = full(&img);
        assert_eq!(burn(&mut img, &[line], crop), Ok(()));
        assert!(
            near(px(&img, 20, 10), [0, 0, 255, 255], 2),
            "{:?}",
            px(&img, 20, 10)
        );
        assert_eq!(px(&img, 20, 14), [0, 0, 0, 255]);
        assert!(all_opaque(&img));
    }

    #[test]
    fn rect_outline_not_filled() {
        let mut img = solid(60, 50, 0);
        let r = Shape::Rect {
            a: pt(50.0, 40.0),
            b: pt(10.0, 10.0),
            width: 3.0,
            color: RED,
        };
        let crop = full(&img);
        assert_eq!(burn(&mut img, &[r], crop), Ok(()));
        assert_eq!(px(&img, 30, 25), [0, 0, 0, 255]);
        assert!(
            near(px(&img, 30, 10), [0, 0, 255, 255], 2),
            "{:?}",
            px(&img, 30, 10)
        );
        assert!(all_opaque(&img));
    }

    #[test]
    fn arrow_head_filled() {
        assert_eq!(arrow_min_head(4.5), 18.0);
        let mut img = solid(100, 60, 0);
        let arrow = Shape::Arrow {
            a: pt(10.0, 30.0),
            b: pt(90.0, 30.0),
            width: 3.0,
            color: RED,
        };
        let crop = full(&img);
        assert_eq!(burn(&mut img, &[arrow], crop), Ok(()));
        // Trong tam giác, ngoài bề rộng của thân.
        assert!(
            near(px(&img, 82, 32), [0, 0, 255, 255], 2),
            "{:?}",
            px(&img, 82, 32)
        );
        assert!(
            near(px(&img, 84, 30), [0, 0, 255, 255], 2),
            "{:?}",
            px(&img, 84, 30)
        );
        // Thân.
        assert!(
            near(px(&img, 40, 30), [0, 0, 255, 255], 2),
            "{:?}",
            px(&img, 40, 30)
        );
        assert!(all_opaque(&img));
    }

    #[test]
    fn short_arrow_draws_head_only() {
        let mut img = solid(40, 20, 0);
        let arrow = Shape::Arrow {
            a: pt(20.0, 10.0),
            b: pt(25.0, 10.0),
            width: 3.0,
            color: RED,
        };
        let crop = full(&img);
        assert_eq!(burn(&mut img, &[arrow], crop), Ok(()));
        assert_eq!(px(&img, 17, 10), [0, 0, 0, 255]);
        // Không có đầu tròn của thân ở `a` (đầu tròn bán kính 1.5 sẽ phủ pixel 19).
        assert_eq!(px(&img, 19, 10), [0, 0, 0, 255]);
        // Đầu mũi tên vẫn có.
        assert!(px(&img, 23, 10)[2] > 200, "{:?}", px(&img, 23, 10));
    }

    #[test]
    fn marker_is_translucent() {
        let mut img = solid(60, 40, 0);
        let m = Shape::Marker {
            points: vec![pt(10.0, 20.0), pt(30.0, 22.0), pt(50.0, 20.0)],
            width: 16.0,
            color: Rgb(255, 255, 255),
        };
        let crop = full(&img);
        assert_eq!(burn(&mut img, &[m], crop), Ok(()));
        let p = px(&img, 30, 21);
        assert!(p[2].abs_diff(102) <= 6, "{p:?}");
        assert!(all_opaque(&img));
    }

    #[test]
    fn marker_self_overlap_not_darker() {
        let mut img = solid(80, 80, 255);
        // Hình X trong một nét: chéo xuống, lên thẳng, chéo xuống ngược lại.
        let m = Shape::Marker {
            points: vec![
                pt(10.0, 10.0),
                pt(70.0, 70.0),
                pt(70.0, 10.0),
                pt(10.0, 70.0),
            ],
            width: 16.0,
            color: Rgb(0, 0, 0),
        };
        let crop = full(&img);
        assert_eq!(burn(&mut img, &[m], crop), Ok(()));
        let cross = px(&img, 40, 40);
        let branch = px(&img, 25, 25);
        assert!(near(cross, branch, 3), "giao {cross:?} nhánh {branch:?}");
        assert!(branch[0] < 200, "nhánh không được vẽ: {branch:?}");
    }

    #[test]
    fn text_draws_outline_and_fill() {
        let _gp = GdiPlus::start();
        let mut img = solid(90, 80, 128);
        let t = Shape::Text {
            origin: pt(10.0, 10.0),
            text: "H".into(),
            size: 40.0,
            color: RED,
        };
        let b = shape_bounds(&t);
        let crop = full(&img);
        assert_eq!(burn(&mut img, std::slice::from_ref(&t), crop), Ok(()));
        let mut red = false;
        let mut black = false;
        for y in 0..img.height {
            for x in 0..img.width {
                let p = px(&img, x, y);
                if near(p, [0, 0, 255, 255], 2) {
                    red = true;
                }
                if near(p, [0, 0, 0, 255], 2) {
                    black = true;
                }
                if p != [128, 128, 128, 255] {
                    let (xi, yi) = (x as i32, y as i32);
                    assert!(
                        xi >= b.x && yi >= b.y && xi < b.right() && yi < b.bottom(),
                        "({x},{y}) ngoài {b:?}"
                    );
                }
            }
        }
        assert!(red && black, "đỏ {red}, đen {black}");
        assert!(all_opaque(&img));
    }

    fn sample_shapes() -> Vec<Shape> {
        vec![
            Shape::Pen {
                points: vec![pt(40.0, 30.0), pt(70.0, 50.0), pt(90.0, 35.0)],
                width: 3.0,
                color: PALETTE[3],
            },
            Shape::Pen {
                points: vec![pt(120.0, 60.0)],
                width: 4.5,
                color: PALETTE[4],
            },
            Shape::Line {
                a: pt(45.5, 75.25),
                b: pt(160.0, 25.0),
                width: 3.0,
                color: PALETTE[0],
            },
            Shape::Arrow {
                a: pt(60.0, 70.0),
                b: pt(140.0, 40.0),
                width: 4.5,
                color: PALETTE[1],
            },
            Shape::Rect {
                a: pt(55.0, 25.0),
                b: pt(130.0, 75.0),
                width: 3.0,
                color: PALETTE[5],
            },
            Shape::Marker {
                points: vec![pt(50.0, 50.0), pt(100.0, 60.0), pt(150.0, 30.0)],
                width: 16.0,
                color: PALETTE[2],
            },
            Shape::Text {
                origin: pt(70.0, 40.0),
                text: "Ab\nc".into(),
                size: 18.0,
                color: PALETTE[6],
            },
        ]
    }

    #[test]
    fn burn_offsets_by_crop_origin() {
        let shapes = sample_shapes();
        let base = noise(200, 100, 3);
        let crop = Rect {
            x: 50,
            y: 20,
            w: 100,
            h: 60,
        };
        let mut whole = base.clone();
        assert_eq!(burn(&mut whole, &shapes, full(&base)), Ok(()));
        let expected = whole.crop(crop);
        let mut part = base.crop(crop);
        assert_eq!(burn(&mut part, &shapes, crop), Ok(()));
        assert_eq!(part, expected);
        assert_ne!(part, base.crop(crop));
    }

    #[test]
    fn clip_limits_drawing() {
        let _gp = GdiPlus::start();
        let shapes = sample_shapes();
        let base = noise(200, 100, 5);
        let clip = Rect {
            x: 80,
            y: 30,
            w: 30,
            h: 20,
        };
        let mut img = base.clone();
        let mut ok = false;
        assert_eq!(
            with_dib(&mut img, |dc| {
                ok = draw_shapes(dc, &shapes, 0, 0, clip);
                ok
            }),
            Ok(())
        );
        assert!(ok);
        let mut changed_inside = false;
        for y in 0..img.height {
            for x in 0..img.width {
                let inside = clip.contains(darkshot_core::geometry::Point {
                    x: x as i32,
                    y: y as i32,
                });
                let (a, b) = (px(&img, x, y), px(&base, x, y));
                if inside {
                    changed_inside |= a != b;
                } else {
                    assert_eq!(a, b, "({x},{y}) ngoài clip bị đổi");
                }
            }
        }
        assert!(changed_inside);
    }

    #[test]
    fn caret_follows_line_and_column() {
        let _gp = GdiPlus::start();
        let mut e = TextEdit::new(pt(20.0, 30.0), 18.0, RED);
        let c0 = caret_rect(&e);
        assert_eq!((c0.x, c0.y), (20, 30));
        assert!(c0.h >= 18, "{c0:?}");
        e.insert("Hello");
        let c1 = caret_rect(&e);
        assert!(c1.x > c0.x + 20, "{c1:?}");
        assert_eq!(c1.y, c0.y);
        e.insert("\n");
        let c2 = caret_rect(&e);
        assert_eq!(c2.x, c0.x);
        assert!(c2.y >= c0.y + 18, "{c2:?}");
        // Hộp bao của chữ chứa con trỏ ở cuối dòng đầu.
        e.insert("x");
        let b = shape_bounds(&e.to_shape().unwrap());
        assert!(
            b.x <= c1.x && b.right() >= c1.x && b.y <= c0.y,
            "{b:?} {c1:?}"
        );
    }

    #[test]
    fn non_text_bounds_match_core() {
        for s in sample_shapes() {
            if let Some(b) = s.bounds() {
                assert_eq!(shape_bounds(&s), b);
            }
        }
    }

    /// `burn` nhiều lần đủ loại hình: số đối tượng GDI của tiến trình không tăng.
    /// Bỏ qua mặc định vì kiểm thử khác chạy song song cũng tạo đối tượng GDI.
    #[test]
    #[ignore = "đếm GDI của cả tiến trình; chạy tay: cargo test -p darkshot-windows -- --ignored render_no_gdi_leak --test-threads=1"]
    fn render_no_gdi_leak() {
        use crate::capture::GDI_TEST_LOCK;
        use windows::Win32::System::Threading::{
            GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS,
        };
        let _gdi = GDI_TEST_LOCK.lock();
        // SAFETY: chỉ đọc số đối tượng GDI của tiến trình này.
        let gdi = || unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
        let shapes = sample_shapes();
        let base = noise(200, 100, 9);
        let crop = Rect {
            x: 10,
            y: 5,
            w: 180,
            h: 90,
        };
        let mut warm = 0;
        for i in 1..=200 {
            let mut img = base.crop(crop);
            assert_eq!(burn(&mut img, &shapes, crop), Ok(()));
            if i == 2 {
                warm = gdi();
            }
        }
        let last = gdi();
        assert!(last <= warm, "GDI tăng từ {warm} lên {last}");
    }
}
