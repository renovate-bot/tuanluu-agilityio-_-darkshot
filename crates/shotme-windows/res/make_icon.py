"""Sinh res/shotme.ico (và ảnh xem trước tùy chọn). Cần Pillow.

Chạy:  python make_icon.py [thư_mục_xem_trước]
Cỡ 16 và 20 vẽ tay theo lưới pixel; cỡ khác vẽ ở độ phân giải gấp 8 rồi thu nhỏ.
Cỡ 256 lưu PNG; cỡ còn lại lưu DIB (BMP 32 bit) để rc.exe nhận.
"""

import io
import os
import struct
import sys

from PIL import Image, ImageDraw

SIZES = [16, 20, 24, 32, 40, 48, 64, 256]
TOP = (0x3B, 0x82, 0xF6)
BOTTOM = (0x1D, 0x4E, 0xD8)
SS = 8


def background(size):
    """Hình vuông bo góc, chuyển màu dọc, nền ngoài trong suốt (khử răng cưa bằng siêu mẫu)."""
    big = size * SS
    mask = Image.new("L", (big, big), 0)
    r = round(0.22 * big)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, big - 1, big - 1), radius=r, fill=255)
    grad = Image.new("RGBA", (big, big))
    px = grad.load()
    for y in range(big):
        t = y / (big - 1)
        c = tuple(round(TOP[i] + (BOTTOM[i] - TOP[i]) * t) for i in range(3)) + (255,)
        for x in range(big):
            px[x, y] = c
    grad.putalpha(mask)
    return grad


def glyph_smooth(size):
    big = size * SS
    img = background(size)
    d = ImageDraw.Draw(img)
    m = 0.24 * big
    sw = 0.09 * big
    lo = m + sw / 2
    hi = big - m - sw / 2
    # Mỗi cạnh chữ L dài 15% cạnh icon tính cả đầu nét bo tròn (bán kính sw/2) kể từ mép khung.
    arm = 0.15 * big - sw  # đầu mút = m + 0.15*big, trừ nửa nét ở gốc và nửa nét ở đầu
    white = (255, 255, 255, 255)

    def seg(x0, y0, x1, y1):
        d.line((x0, y0, x1, y1), fill=white, width=round(sw))
        for cx, cy in ((x0, y0), (x1, y1)):
            d.ellipse((cx - sw / 2, cy - sw / 2, cx + sw / 2, cy + sw / 2), fill=white)

    for cx, sx in ((lo, 1), (hi, -1)):
        for cy, sy in ((lo, 1), (hi, -1)):
            seg(cx, cy, cx + sx * arm, cy)
            seg(cx, cy, cx, cy + sy * arm)
    rad = 0.06 * big
    dx = dy = 0.5 * big
    d.ellipse((dx - rad, dy - rad, dx + rad, dy + rad), fill=white)
    return img.resize((size, size), Image.LANCZOS)


def glyph_grid(size):
    """16 và 20: bốn góc chữ L nét 2 px đúng lưới, không chấm tròn."""
    img = background(size).resize((size, size), Image.LANCZOS)
    lo, arm = {16: (3, 4), 20: (4, 5), 24: (5, 4)}[size]
    hi = size - 1 - lo
    px = img.load()
    white = (255, 255, 255, 255)

    def put(x, y):
        px[x, y] = white

    for i in range(arm):
        for t in range(2):
            # cạnh ngang và dọc của bốn góc
            for x in (lo + i, hi - i):
                for y in (lo + t, hi - t):
                    put(x, y)
            for y in (lo + i, hi - i):
                for x in (lo + t, hi - t):
                    put(x, y)
    return img


def make(size):
    return glyph_grid(size) if size in (16, 20, 24) else glyph_smooth(size)


def self_check(images):
    """Khung phải là bốn góc rời nhau: giữa cạnh trên và cạnh trái là nền xanh, góc là nét trắng."""
    for size, img in images:
        px = img.load()
        m = {16: 3, 20: 4, 24: 5}.get(size, round(0.285 * size))
        mid = size // 2
        assert px[mid, m][0] < 128, f"cỡ {size}: giữa cạnh trên bị tô trắng"
        assert px[m, mid][0] < 128, f"cỡ {size}: giữa cạnh trái bị tô trắng"
        assert px[m, m][0] > 200, f"cỡ {size}: góc trên trái không trắng"
        if size >= 24:
            whites = [x for x in range(size) if px[x, m][0] >= 128]
            best = max(b - a - 1 for a, b in zip(whites, whites[1:]))
            assert best >= 0.16 * size, f"cỡ {size}: khe giữa hai góc chỉ {best}px"


def dib(img):
    w, h = img.size
    pix = img.load()
    header = struct.pack("<IiiHHIIiiII", 40, w, h * 2, 1, 32, 0, w * h * 4, 0, 0, 0, 0)
    rows = bytearray()
    for y in range(h - 1, -1, -1):
        for x in range(w):
            r, g, b, a = pix[x, y]
            rows += bytes((b, g, r, a))
    mask_stride = ((w + 31) // 32) * 4
    return header + bytes(rows) + bytes(mask_stride * h)  # mặt nạ AND toàn 0


def build_ico(images):
    blobs = []
    for size, img in images:
        if size >= 256:
            buf = io.BytesIO()
            img.save(buf, "PNG")
            blobs.append(buf.getvalue())
        else:
            blobs.append(dib(img))
    out = bytearray(struct.pack("<HHH", 0, 1, len(images)))
    offset = 6 + 16 * len(images)
    for (size, _), blob in zip(images, blobs):
        b = 0 if size >= 256 else size
        out += struct.pack("<BBBBHHII", b, b, 0, 0, 1, 32, len(blob), offset)
        offset += len(blob)
    for blob in blobs:
        out += blob
    return bytes(out)


def preview(images, path):
    shown = [(s, im) for s, im in images if s in (16, 20, 24, 32, 40, 48, 64, 256)]
    cell = 140
    pad = 12
    width = pad + (cell + pad) * len(shown)
    real_h = 272
    band = pad + cell + pad + 40 + pad
    out = Image.new("RGBA", (width, band * 2), (0, 0, 0, 255))
    for row, bg in enumerate(((0xF3, 0xF4, 0xF6, 255), (0x1F, 0x29, 0x37, 255))):
        ImageDraw.Draw(out).rectangle((0, row * band, width, (row + 1) * band), fill=bg)
        x = pad
        for size, im in shown:
            zoom = im.resize((128, 128), Image.NEAREST)
            out.alpha_composite(zoom, (x + (cell - 128) // 2, row * band + pad))
            real = im if size <= 64 else im.resize((48, 48), Image.LANCZOS)
            out.alpha_composite(real, (x + (cell - real.width) // 2, row * band + pad + 128 + 8))
            x += cell + pad
    out.convert("RGB").save(path)


def main():
    images = [(s, make(s)) for s in SIZES]
    try:
        self_check(images)
    except AssertionError as e:
        print("LỖI tự kiểm:", e)
        sys.exit(1)
    here = os.path.dirname(os.path.abspath(__file__))
    with open(os.path.join(here, "shotme.ico"), "wb") as f:
        f.write(build_ico(images))
    if len(sys.argv) > 1:
        os.makedirs(sys.argv[1], exist_ok=True)
        preview(images, os.path.join(sys.argv[1], "preview.png"))


if __name__ == "__main__":
    main()
