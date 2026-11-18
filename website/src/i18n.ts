/**
 * Tiny i18n layer.
 *
 * English is the source of truth and lives in the HTML itself (so it is
 * indexable and works without JavaScript). Vietnamese lives in vi.json.
 * Elements opt in with:
 *   data-i18n="key"            → textContent
 *   data-i18n-html="key"       → innerHTML (trusted, authored by us)
 *   data-i18n-attr="a:key;b:k" → attributes
 * English text is captured from the DOM the first time an element is seen.
 *
 * Strings produced by scripts use t("key"), whose English fallbacks are in EN_JS below.
 */
import vi from "./vi.json";

export type Lang = "en" | "vi";

const STORAGE_KEY = "darkshot-lang";
const viDict = vi as Record<string, string>;

/** English for strings created in TypeScript (the rest is read from the HTML). */
const EN_JS: Record<string, string> = {
  "demo.hint": "Click: window · Drag: region · Esc: cancel",
  "demo.linkCopied": "Link copied",
  "cta.starting": "Starting download…",
  "cta.opening": "Opening GitHub…",
  "badge.latest": "Latest stable: {v}",
  "fa.statusHover": "{name} · {size}. Click or press Enter to select it.",
  "fa.statusWindow": "Selected exactly that window's frame ({size}).",
  "fa.statusRegion": "Dragged a free region ({size}).",
  "fa.winTerminal": "Terminal window",
  "fa.winApp": "App window",
  "fc.undo": "Show the original",
  "fc.s1": "Both fields are covered by an opaque colour. Nothing readable is left.",
  "dl.fail": "Couldn't load release details right now. The button still opens the latest release on GitHub.",
  "dl.zip": "ZIP package",
  "dl.zipDesc": "darkshot.exe and README · {size} download",
  "dl.exe": "Standalone executable",
  "dl.exeDesc": "darkshot.exe · {size} on disk",
  "dl.sums": "SHA256SUMS.txt",
  "dl.sumsDesc": "Checksums to verify your download",
};

const en: Record<string, string> = { ...EN_JS };
const listeners = new Set<(lang: Lang) => void>();
let current: Lang = "en";

const keyOf = (v: string) => v.trim();

function capture(root: ParentNode): void {
  root.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    const k = keyOf(el.dataset.i18n!);
    if (!(k in en)) en[k] = el.textContent ?? "";
  });
  root.querySelectorAll<HTMLElement>("[data-i18n-html]").forEach((el) => {
    const k = keyOf(el.dataset.i18nHtml!);
    if (!(k in en)) en[k] = el.innerHTML;
  });
  root.querySelectorAll<HTMLElement>("[data-i18n-attr]").forEach((el) => {
    for (const [attr, k] of parseAttrs(el.dataset.i18nAttr!)) {
      if (!(k in en)) en[k] = el.getAttribute(attr) ?? "";
    }
  });
}

function parseAttrs(spec: string): [string, string][] {
  return spec
    .split(";")
    .map((p) => p.trim())
    .filter(Boolean)
    .map((p) => {
      const i = p.indexOf(":");
      return [p.slice(0, i), p.slice(i + 1)] as [string, string];
    });
}

const dict = (lang: Lang) => (lang === "vi" ? viDict : en);

/** Translate a script-side string and fill {placeholders}. */
export function t(key: string, vars: Record<string, string | number> = {}): string {
  const raw = dict(current)[key] ?? en[key] ?? key;
  return raw.replace(/\{(\w+)\}/g, (_, n: string) => String(vars[n] ?? `{${n}}`));
}

export function getLang(): Lang {
  return current;
}

/** Apply the current language to everything under root (call again after injecting new markup). */
export function translate(root: ParentNode = document): void {
  capture(root);
  const d = dict(current);
  root.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    const v = d[keyOf(el.dataset.i18n!)];
    if (v !== undefined) el.textContent = v;
  });
  root.querySelectorAll<HTMLElement>("[data-i18n-html]").forEach((el) => {
    const v = d[keyOf(el.dataset.i18nHtml!)];
    if (v !== undefined) el.innerHTML = v;
  });
  root.querySelectorAll<HTMLElement>("[data-i18n-attr]").forEach((el) => {
    for (const [attr, k] of parseAttrs(el.dataset.i18nAttr!)) {
      const v = d[k];
      if (v !== undefined) el.setAttribute(attr, v);
    }
  });
}

function applyLang(lang: Lang): void {
  current = lang;
  const html = document.documentElement;
  html.lang = lang;
  translate(document);
  document.querySelectorAll<HTMLButtonElement>("[data-lang]").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.lang === lang)));
  const locale = document.querySelector('meta[property="og:locale"]');
  const alt = document.querySelector('meta[property="og:locale:alternate"]');
  locale?.setAttribute("content", lang === "vi" ? "vi_VN" : "en_US");
  alt?.setAttribute("content", lang === "vi" ? "en_US" : "vi_VN");
  // Language-specific screenshots keep the real UI in the reader's language.
  document.querySelectorAll<HTMLImageElement>("img[data-shot]").forEach((img) => {
    img.src = img.src.replace(/-(en|vi)\.webp/, `-${lang}.webp`);
  });
  listeners.forEach((fn) => fn(lang));
}

export function onLangChange(fn: (lang: Lang) => void): void {
  listeners.add(fn);
}

export function setLang(lang: Lang): void {
  try {
    localStorage.setItem(STORAGE_KEY, lang);
  } catch {
    /* storage unavailable (private mode): the choice just won't persist */
  }
  applyLang(lang);
}

export function initI18n(): void {
  const pageLang: Lang = document.documentElement.lang === "vi" ? "vi" : "en";
  // A pre-rendered Vietnamese page already contains Vietnamese text, so English
  // must come from the dictionary the prerender step embedded, not from the DOM.
  if (pageLang === "vi") restoreEnglishFromPrerender();
  capture(document);
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(STORAGE_KEY);
  } catch {
    /* ignore */
  }
  // An explicit /vi/ URL wins over a stored preference; the root page honours the stored choice.
  const initial: Lang = pageLang === "vi" ? "vi" : stored === "vi" || stored === "en" ? stored : pageLang;
  applyLang(initial);
  document.querySelectorAll<HTMLButtonElement>("[data-lang]").forEach((b) => {
    b.addEventListener("click", () => setLang(b.dataset.lang as Lang));
  });
}

/**
 * The prerender step stores the original English in data-en* attributes so a
 * Vietnamese page can still switch back to English without a reload.
 */
function restoreEnglishFromPrerender(): void {
  const script = document.getElementById("en-dict");
  if (!script?.textContent) return;
  try {
    Object.assign(en, JSON.parse(script.textContent) as Record<string, string>);
  } catch {
    /* ignore a malformed dictionary; English falls back to keys */
  }
}
