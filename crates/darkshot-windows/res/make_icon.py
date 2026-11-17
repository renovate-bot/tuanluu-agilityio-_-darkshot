"""Sinh res/darkshot.ico (và ảnh xem trước tùy chọn). Cần Pillow.

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
BG = (0x20, 0x23, 0x28)
BORDER = (0x3A, 0x3E, 0x45)
ACCENT = (0x4C, 0x8D, 0xFF)
DOT = (0xE7, 0xE9, 0xEC)
SS = 8


def background(size):
    """Nền than chì phẳng bo góc có viền xám nhạt 1/52 cạnh (tối thiểu 1 px), ngoài trong suốt (siêu mẫu)."""
    big = size * SS
    bw = max(SS, round(big / 52))
    r = round(0.22 * big)
    outer = Image.new("L", (big, big), 0)
    ImageDraw.Draw(outer).rounded_rectangle((0, 0, big - 1, big - 1), radius=r, fill=255)
    inner = Image.new("L", (big, big), 0)
    ImageDraw.Draw(inner).rounded_rectangle((bw, bw, big - 1 - bw, big - 1 - bw), radius=max(r - bw, 0), fill=255)
    img = Image.new("RGBA", (big, big), BORDER + (255,))
    img.putalpha(outer)
    img.paste(Image.new("RGBA", (big, big), BG + (255,)), (0, 0), inner)
    return img


def background_grid(size):
    """16/20/24: nền đúng lưới, viền 1 px, góc bo vẽ tay (không khử răng cưa) để sạch nét."""
    r = {16: 3, 20: 4, 24: 5}[size]
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    px = img.load()

    def inside(x, y, off):
        # điểm giữa pixel nằm trong hình chữ nhật bo góc co vào off px
        lo, hi = off, size - off
        rad = max(r - off, 0)
        cx = min(max(x + 0.5, lo + rad), hi - rad)
        cy = min(max(y + 0.5, lo + rad), hi - rad)
        return lo <= x + 0.5 <= hi and lo <= y + 0.5 <= hi and (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 <= rad * rad + 0.01

    for y in range(size):
        for x in range(size):
            if inside(x, y, 1):
                px[x, y] = BG + (255,)
            elif inside(x, y, 0):
                px[x, y] = BORDER + (255,)
    return img


def glyph_smooth(size):
    big = size * SS
    img = background(size)
    d = ImageDraw.Draw(img)
    m = 0.23 * big
    sw = 0.085 * big
    # Mỗi cạnh chữ L dài 18% cạnh icon tính từ mép ngoài của khung, đầu vuông bo rất nhẹ:
    # khe giữa hai góc liền kề là 18%, nét đủ dài để đọc ra chữ L chứ không thành một đốm.
    arm = 0.18 * big
    rr = sw / 2  # đầu bo tròn như prototype
    white = ACCENT + (255,)

    def bar(x0, y0, x1, y1):
        d.rounded_rectangle((min(x0, x1), min(y0, y1), max(x0, x1), max(y0, y1)), radius=rr, fill=white)

    for cx, sx in ((m, 1), (big - m, -1)):
        for cy, sy in ((m, 1), (big - m, -1)):
            bar(cx, cy, cx + sx * arm, cy + sy * sw)
            bar(cx, cy, cx + sx * sw, cy + sy * arm)
    rad = 0.055 * big
    dx = dy = 0.5 * big
    d.ellipse((dx - rad, dy - rad, dx + rad, dy + rad), fill=DOT + (255,))
    return img.resize((size, size), Image.LANCZOS)


def glyph_grid(size):
    """16 và 20: bốn góc chữ L nét 2 px đúng lưới, không chấm tròn."""
    img = background_grid(size)
    lo, arm = {16: (3, 4), 20: (4, 5), 24: (5, 4)}[size]
    hi = size - 1 - lo
    px = img.load()
    white = ACCENT + (255,)

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
    """Khung là bốn góc rời nhau: giữa cạnh trên/trái là nền, góc là màu nhấn, mép ngoài có viền."""
    for size, img in images:
        px = img.load()
        m = {16: 3, 20: 4, 24: 5}.get(size, round(0.285 * size))
        mid = size // 2
        assert px[mid, m][2] < 0x60, f"cỡ {size}: giữa cạnh trên bị tô màu nhấn"
        assert px[m, mid][2] < 0x60, f"cỡ {size}: giữa cạnh trái bị tô màu nhấn"
        assert px[m, m][2] > 200, f"cỡ {size}: góc trên trái không phải màu nhấn"
        assert px[mid, 0][3] == 255 and px[mid, 0][0] >= BG[0] + 8, f"cỡ {size}: mép trên không có viền"
        assert px[0, mid][3] == 255 and px[0, mid][0] >= BG[0] + 8, f"cỡ {size}: mép trái không có viền"
        if size >= 24:
            acc = [x for x in range(size) if px[x, m][2] >= 0x90]
            best = max(b - a - 1 for a, b in zip(acc, acc[1:]))
            assert best >= 0.14 * size, f"cỡ {size}: khe giữa hai góc chỉ {best}px"


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
    with open(os.path.join(here, "darkshot.ico"), "wb") as f:
        f.write(build_ico(images))
    if len(sys.argv) > 1:
        os.makedirs(sys.argv[1], exist_ok=True)
        preview(images, os.path.join(sys.argv[1], "preview.png"))


if __name__ == "__main__":
    main()
