# darkshot website

Trang giới thiệu (landing page) của darkshot. Tách hẳn khỏi ứng dụng Rust: không dùng chung mã, không đụng tới `Cargo.toml`, `crates/` hay `scripts/check.ps1`.

Static site: Vite + TypeScript, không framework, không backend. Công cụ build là devDependency (`vite`, `typescript`, `node-html-parser`); trang chạy ra không kéo thư viện ngoài nào (JS ~15 KB gzip, CSS ~6 KB gzip, không font ngoài, không script bên thứ ba).

## Chạy thử

Yêu cầu Node 20+.

```powershell
cd website
npm install
npm run dev        # http://localhost:5173
```

## Build và xem bản build

```powershell
npm run build      # typecheck + kiểm bản dịch + vite build + sinh trang /vi/
npm run preview    # http://localhost:4173
```

Kết quả nằm ở `website/dist/`. `npm run build` dừng nếu: TypeScript lỗi; khóa i18n ở `index.html`/`src/*.ts` và `src/vi.json` không khớp (thiếu hoặc thừa).

## Cấu trúc

```text
website/
  index.html            Toàn bộ nội dung tiếng Anh (nguồn gốc), thuộc tính data-i18n*
  public/               favicon, robots.txt, ảnh (logo từ darkshot.ico; ảnh chụp thật Lịch sử và Tùy chọn)
  src/
    main.ts             Khởi tạo
    i18n.ts             EN/VI chuyển ngay không tải lại, nhớ lựa chọn (localStorage)
    vi.json             Bản dịch tiếng Việt (khóa "khu.tên")
    release.ts          Lấy bản phát hành mới nhất từ GitHub API
    scene.ts            Minh họa SVG dựng lại theo giao diện thật (không phải ảnh chụp)
    demos.ts            Hero Chụp → Đánh dấu → Chia sẻ, demo bắt cửa sổ/kéo vùng, so sánh trước/sau, che thông tin, tab xuất ảnh, link R2
    ui.ts               Menu, cuộn, scrollspy, hiện dần khi cuộn
    styles/main.css
  scripts/
    check-i18n.mjs      Kiểm khóa dịch
    prerender-vi.mjs    Sau build: sinh dist/vi/index.html (tiếng Việt, lang="vi", meta tiếng Việt)
```

## Cách hoạt động của hai ngôn ngữ

- Tiếng Anh nằm ngay trong HTML (đọc được khi không có JS, công cụ tìm kiếm thấy). Tiếng Việt ở `src/vi.json`.
- Thêm chữ mới: gắn `data-i18n="khu.tên"` (văn bản), `data-i18n-html` (có thẻ `<kbd>`, `<strong>`...) hoặc `data-i18n-attr="aria-label:khu.tên"`, viết bản tiếng Anh vào HTML và thêm khóa vào `vi.json`. Chữ sinh từ script dùng `t("khóa")` kèm bản Anh trong `EN_JS` (`src/i18n.ts`).
- Chữ Việt trong lời hứa kỹ thuật bám theo bảng `darkshot_core::i18n` của ứng dụng (tên công cụ, nút, thông báo).
- `dist/vi/index.html` là bản dựng sẵn tiếng Việt để công cụ tìm kiếm và người không bật JS thấy đúng ngôn ngữ; cả hai trang đều chuyển được qua nút EN/VI. Trang gốc nhớ lựa chọn của khách; `/vi/` luôn mở bằng tiếng Việt.

## Tải về và bản phát hành

`src/release.ts` gọi `https://api.github.com/repos/tuanluu-agilityio/darkshot/releases/latest` (API này không trả bản nháp hay bản thử). Ngoài ra mã còn từ chối `draft`/`prerelease`.

- Phiên bản, ngày phát hành, kích thước và liên kết lấy từ dữ liệu API; mỗi liên kết là `browser_download_url` của chính asset, và chỉ được nhận nếu bắt đầu bằng `https://github.com/tuanluu-agilityio/darkshot/releases/download/`. Không tự ghép URL.
- Tìm asset theo tên: `darkshot-v*-windows-x64.zip`, `darkshot.exe`, `SHA256SUMS.txt`. Asset không có thì không hiện.
- Dung lượng ZIP (tải về) và `darkshot.exe` (trên đĩa) hiện riêng, ghi rõ từng loại.
- Mọi nút Tải về có sẵn `href` là `https://github.com/tuanluu-agilityio/darkshot/releases/latest`. API thành công thì đổi sang URL của ZIP (hoặc exe nếu thiếu ZIP). API lỗi, bị giới hạn tốc độ hoặc quá 8 giây thì giữ nguyên liên kết dự phòng và hiện thông báo.
- Kết quả được cache 10 phút trong `sessionStorage` để tránh chạm giới hạn của API chưa xác thực (60 yêu cầu/giờ/IP).
- Trang không nêu số phiên bản cố định ở bất kỳ chỗ nào; mọi số phiên bản lấy từ API.

## Minh họa và ảnh chụp

- Hai ảnh chụp thật: cửa sổ Lịch sử (dữ liệu mẫu giữ chỗ) và cửa sổ Tùy chọn (tab Thông thường, đã cắt bỏ ô Thư mục lưu vì chứa đường dẫn máy). Nguồn: `target/demo7/screens`, `target/demo8/screens`.
- Mọi hình còn lại (lớp phủ chụp, thanh công cụ, đánh dấu, che thông tin, bắt cửa sổ, thanh hành động) là SVG dựng lại theo bố cục và màu thật trong `crates/darkshot-core/src/theme.rs`, luôn kèm chú thích "Minh họa". Nội dung mẫu (hóa đơn Acme, email `jane.doe@example.com`, khóa `sk_test_EXAMPLE_0000`, link `img.example.com`) đều bịa, không dữ liệu thật.
- Cần ảnh chụp thật cho lớp phủ: chụp bằng bản build thật rồi thay `scene.ts` trong từng mục (không dùng ảnh có màn hình riêng của ai).

## Triển khai (static hosting)

Build dùng đường dẫn tương đối (`base: "./"`) nên chạy được ở thư mục gốc lẫn thư mục con.

- **Netlify:** Build command `npm run build`, base directory `website`, publish directory `website/dist`.
- **Vercel:** Root Directory `website`, Framework preset "Other", Build Command `npm run build`, Output Directory `dist`.
- **GitHub Pages:** build rồi đẩy nội dung `website/dist` lên nhánh/thư mục Pages (hoặc dùng `actions/upload-pages-artifact` với đường dẫn `website/dist`). Trang là project site (`/darkshot/`) vẫn chạy nhờ đường dẫn tương đối.
- Chưa biết tên miền chính thức, nên chưa có: `<link rel="canonical">`, `og:url`, `og:image` (cần URL tuyệt đối), `sitemap.xml`. Khi có tên miền, thêm các mục đó và dòng `Sitemap:` vào `public/robots.txt`.

## Kiểm tra đã làm (2026-11-18)

Chrome headless (puppeteer-core) ở 1440×900, 820×1100, 390×844: không lỗi hay cảnh báo console, không yêu cầu mạng lỗi, không tràn ngang; chuyển EN ↔ VI; trang `/vi/`; demo bắt cửa sổ (rê, bấm, kéo vùng); bật/tắt che thông tin; tab xuất ảnh bằng bàn phím; chọn hạn/định dạng R2; `prefers-reduced-motion`; giả lập API trả 403 (nút dự phòng hoạt động, không hiện asset); asset thật của v0.3.0 khớp danh sách.
