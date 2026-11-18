// Fails the build if the Vietnamese dictionary and the page disagree.
// - every key used in index.html or src/*.ts must exist in src/vi.json
// - every key in src/vi.json must be used somewhere
import { readFileSync, readdirSync } from "node:fs";

const html = readFileSync("index.html", "utf8");
const ts = readdirSync("src")
  .filter((f) => f.endsWith(".ts"))
  .map((f) => readFileSync(`src/${f}`, "utf8"))
  .join("\n");
const vi = JSON.parse(readFileSync("src/vi.json", "utf8"));

const used = new Set();
const add = (k) => k && k.includes(".") && used.add(k.trim()); // keys are "area.name"; skips doc-comment placeholders
for (const src of [html, ts]) {
  for (const m of src.matchAll(/data-i18n(?:-html)?="([^"]+)"/g)) add(m[1]);
  for (const m of src.matchAll(/data-i18n-attr="([^"]+)"/g)) for (const p of m[1].split(";")) add(p.slice(p.indexOf(":") + 1));
}
// script-side keys: any string literal shaped like "area.name" that vi.json knows or EN_JS declares
const enJs = ts.match(/const EN_JS[^{]*\{([\s\S]*?)\n\};/)?.[1] ?? "";
for (const m of enJs.matchAll(/^\s*"([^"]+)":/gm)) add(m[1]);
for (const m of ts.matchAll(/(?:t\(|dataset\.i18n = [^;]*?)"([a-z]+\.[A-Za-z0-9]+)"/g)) add(m[1]);
for (const m of ts.matchAll(/"((?:fc|fa|dl|cta|badge)\.[A-Za-z0-9]+)"/g)) add(m[1]);

const missing = [...used].filter((k) => !(k in vi));
const unused = Object.keys(vi).filter((k) => !used.has(k));
if (missing.length) console.error("Missing in vi.json:\n  " + missing.join("\n  "));
if (unused.length) console.error("Unused in vi.json:\n  " + unused.join("\n  "));
if (missing.length || unused.length) process.exit(1);
console.log(`i18n OK: ${used.size} keys, all translated.`);
