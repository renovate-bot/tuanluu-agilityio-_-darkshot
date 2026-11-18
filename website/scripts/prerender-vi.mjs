// Post-build: writes dist/vi/index.html, a Vietnamese pre-rendering of the page,
// so crawlers and no-JS visitors get real Vietnamese content, lang="vi" and
// Vietnamese metadata. The same bundle runs on it; it can still switch to English
// without reloading (the English text is embedded as JSON in #en-dict).
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { parse } from "node-html-parser";

const vi = JSON.parse(readFileSync("src/vi.json", "utf8"));
const src = readFileSync("dist/index.html", "utf8");
const root = parse(src, { comments: true, blockTextElements: { script: true, style: true } });

const en = {};
const attrPairs = (spec) =>
  spec.split(";").map((p) => p.trim()).filter(Boolean).map((p) => [p.slice(0, p.indexOf(":")), p.slice(p.indexOf(":") + 1)]);

for (const el of root.querySelectorAll("[data-i18n]")) {
  const k = el.getAttribute("data-i18n").trim();
  if (!(k in vi)) continue;
  en[k] ??= el.textContent;
  el.set_content(vi[k].replace(/&/g, "&amp;").replace(/</g, "&lt;"));
}
for (const el of root.querySelectorAll("[data-i18n-html]")) {
  const k = el.getAttribute("data-i18n-html").trim();
  if (!(k in vi)) continue;
  en[k] ??= el.innerHTML;
  el.set_content(vi[k]);
}
for (const el of root.querySelectorAll("[data-i18n-attr]")) {
  for (const [attr, k] of attrPairs(el.getAttribute("data-i18n-attr"))) {
    if (!(k in vi)) continue;
    en[k] ??= el.getAttribute(attr) ?? "";
    el.setAttribute(attr, vi[k]);
  }
}

const html = root.querySelector("html");
html.setAttribute("lang", "vi");
root.querySelector('meta[property="og:locale"]')?.setAttribute("content", "vi_VN");
root.querySelector('meta[property="og:locale:alternate"]')?.setAttribute("content", "en_US");
for (const b of root.querySelectorAll("[data-lang]")) b.setAttribute("aria-pressed", String(b.getAttribute("data-lang") === "vi"));
for (const img of root.querySelectorAll("img[data-shot]")) img.setAttribute("src", img.getAttribute("src").replace("-en.webp", "-vi.webp"));

// The page now lives one level deeper: asset URLs written as ./x become ../x.
let out = root.toString().replace(/\b(src|href)="\.\/(?!\/)/g, '$1="../');
// Hash links stay on this page; the page-level EN/VI switch works at runtime.
const dict = JSON.stringify(en).replace(/</g, "\u003c");
out = out.replace("</head>", `<script id="en-dict" type="application/json">${dict}</script>\n</head>`);

mkdirSync("dist/vi", { recursive: true });
writeFileSync("dist/vi/index.html", out);

// Cross-link the two language versions for search engines (relative; add absolute URLs once the domain is known).
let enOut = src.replace("</head>", `<link rel="alternate" hreflang="vi" href="./vi/" />\n</head>`);
writeFileSync("dist/index.html", enOut);
out = out.replace("</head>", `<link rel="alternate" hreflang="en" href="../" />\n</head>`);
writeFileSync("dist/vi/index.html", out);
console.log(`Prerendered dist/vi/index.html (${Object.keys(en).length} translated strings).`);
