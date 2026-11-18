/**
 * Latest-release integration.
 *
 * Reads the GitHub Releases API at runtime. Download links come only from
 * each asset's browser_download_url; nothing is ever constructed by hand.
 * If the API is unreachable or rate-limited, every download button keeps its
 * static href (the GitHub "latest release" page), so there is always a working path.
 */
import { getLang, onLangChange, t } from "./i18n";

const REPO = "tuanluu-agilityio/darkshot";
const API = `https://api.github.com/repos/${REPO}/releases/latest`;
const DOWNLOAD_PREFIX = `https://github.com/${REPO}/releases/download/`;
const CACHE_KEY = "darkshot-release-v1";
const CACHE_MS = 10 * 60 * 1000;

interface ApiAsset {
  name: string;
  size: number;
  browser_download_url: string;
}
interface ApiRelease {
  tag_name: string;
  html_url: string;
  published_at: string;
  draft: boolean;
  prerelease: boolean;
  assets: ApiAsset[];
}
interface Release {
  tag: string;
  url: string;
  published: string;
  zip?: ApiAsset;
  exe?: ApiAsset;
  sums?: ApiAsset;
}

/** Accept only stable, published releases whose asset URLs really point at this repo's release downloads. */
export function parseRelease(raw: unknown): Release | null {
  const r = raw as Partial<ApiRelease> | null;
  if (!r || r.draft || r.prerelease || typeof r.tag_name !== "string" || !Array.isArray(r.assets)) return null;
  const ok = (a: ApiAsset | undefined) =>
    a && typeof a.browser_download_url === "string" && a.browser_download_url.startsWith(DOWNLOAD_PREFIX) ? a : undefined;
  const find = (re: RegExp) => ok(r.assets!.find((a) => re.test(a.name)));
  const rel: Release = {
    tag: r.tag_name,
    url: typeof r.html_url === "string" && r.html_url.startsWith(`https://github.com/${REPO}/`) ? r.html_url : `https://github.com/${REPO}/releases/latest`,
    published: r.published_at ?? "",
    zip: find(/^darkshot-v[\w.-]+-windows-x64\.zip$/i),
    exe: find(/^darkshot\.exe$/i),
    sums: find(/^SHA256SUMS\.txt$/i),
  };
  return rel.zip || rel.exe ? rel : null;
}

async function load(): Promise<Release | null> {
  try {
    const cached = sessionStorage.getItem(CACHE_KEY);
    if (cached) {
      const { at, data } = JSON.parse(cached) as { at: number; data: unknown };
      if (Date.now() - at < CACHE_MS) return parseRelease(data);
    }
  } catch {
    /* no cache */
  }
  const ctl = new AbortController();
  const timer = setTimeout(() => ctl.abort(), 8000);
  try {
    const res = await fetch(API, { headers: { Accept: "application/vnd.github+json" }, signal: ctl.signal });
    if (!res.ok) return null; // 403/429 rate limit, 404, 5xx → fallback
    const data: unknown = await res.json();
    try {
      sessionStorage.setItem(CACHE_KEY, JSON.stringify({ at: Date.now(), data }));
    } catch {
      /* ignore */
    }
    return parseRelease(data);
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

const fmtSize = (bytes: number): string => `${(bytes / 1048576).toFixed(2)} MiB`;

function fmtDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return new Intl.DateTimeFormat(getLang() === "vi" ? "vi-VN" : "en-US", { year: "numeric", month: "long", day: "numeric" }).format(d);
}

function renderAssets(list: HTMLElement, rel: Release): void {
  list.replaceChildren();
  const rows: [ApiAsset | undefined, string, string, string][] = [
    [rel.zip, "dl.zip", "dl.zipDesc", "zip"],
    [rel.exe, "dl.exe", "dl.exeDesc", "exe"],
    [rel.sums, "dl.sums", "dl.sumsDesc", "sums"],
  ];
  for (const [asset, title, desc, kind] of rows) {
    if (!asset) continue; // never show assets that don't exist
    const li = document.createElement("li");
    const a = document.createElement("a");
    a.href = asset.browser_download_url;
    a.rel = "noopener";
    a.className = `asset asset-${kind}`;
    a.setAttribute("download", "");
    const strong = document.createElement("strong");
    strong.textContent = t(title);
    const small = document.createElement("span");
    small.textContent = t(desc, { size: fmtSize(asset.size) });
    const name = document.createElement("code");
    name.textContent = asset.name;
    a.append(strong, small, name);
    li.append(a);
    list.append(li);
  }
}

export async function initRelease(): Promise<void> {
  const ctas = document.querySelectorAll<HTMLAnchorElement>("[data-download-cta]");
  const status = document.querySelector<HTMLElement>("[data-release-status]");
  const meta = document.querySelector<HTMLElement>("[data-release-meta]");
  const assets = document.querySelector<HTMLElement>("[data-release-assets]");
  const notes = document.querySelector<HTMLAnchorElement>("[data-release-notes]");
  const badge = document.querySelector<HTMLElement>("[data-release-badge]");
  const version = document.querySelector<HTMLElement>("[data-release-version]");
  const date = document.querySelector<HTMLElement>("[data-release-date]");
  if (!status || !meta || !assets) return;

  // Download feedback works for both the direct link and the fallback page.
  ctas.forEach((cta) => {
    cta.addEventListener("click", () => {
      const label = cta.querySelector<HTMLElement>("[data-cta-label]");
      if (!label || cta.classList.contains("is-busy")) return;
      const direct = cta.dataset.direct === "1";
      const original = label.textContent;
      cta.classList.add("is-busy");
      label.textContent = t(direct ? "cta.starting" : "cta.opening");
      setTimeout(() => {
        cta.classList.remove("is-busy");
        label.textContent = original;
      }, 2200);
    });
  });

  const rel = await load();
  if (!rel) {
    status.hidden = false;
    status.dataset.state = "fail";
    status.textContent = t("dl.fail");
    onLangChange(() => {
      if (status.dataset.state === "fail") status.textContent = t("dl.fail");
    });
    return;
  }

  const target = rel.zip ?? rel.exe!;
  ctas.forEach((cta) => {
    cta.href = target.browser_download_url;
    cta.dataset.direct = "1";
    cta.title = target.name;
  });

  const paint = () => {
    status.hidden = true;
    meta.hidden = false;
    if (version) version.textContent = rel.tag;
    if (date) date.textContent = fmtDate(rel.published);
    if (notes) notes.href = rel.url;
    if (badge) {
      badge.hidden = false;
      badge.textContent = t("badge.latest", { v: rel.tag });
    }
    renderAssets(assets, rel);
  };
  paint();
  onLangChange(paint);
}
