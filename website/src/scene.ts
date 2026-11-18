/**
 * SVG builders for the product illustrations.
 *
 * Everything here is a reconstruction drawn from the real app's layout and
 * colours (see crates/darkshot-core/src/theme.rs). It is an illustration, not a
 * screenshot, and the page says so next to every use. All sample text is
 * invented; none of it is real personal data.
 */

export const SCENE_W = 640;
export const SCENE_H = 400;

const FONT = `font-family="Segoe UI Variable, Segoe UI, system-ui, sans-serif"`;
const MONO = `font-family="Cascadia Code, Consolas, ui-monospace, monospace"`;
const RED = "#F2262B";

/** A generic, invented web-app window used as the "thing being captured". */
export function appScene(): string {
  return `
  <rect width="640" height="400" fill="#EEF0F3"/>
  <rect width="640" height="28" fill="#FFFFFF"/>
  <circle cx="14" cy="14" r="4" fill="#E5484D"/><circle cx="28" cy="14" r="4" fill="#F5A524"/><circle cx="42" cy="14" r="4" fill="#3DD68C"/>
  <text x="240" y="18.5" ${FONT} font-size="11" fill="#6B7280">Acme Billing — Invoice #1042</text>
  <rect y="28" width="120" height="372" fill="#F7F8FA"/>
  <rect x="8" y="76" width="104" height="26" rx="6" fill="#E4EAF7"/>
  <text x="20" y="64" ${FONT} font-size="12" fill="#6B7280">Overview</text>
  <text x="20" y="93" ${FONT} font-size="12" font-weight="600" fill="#2F6FE4">Invoices</text>
  <text x="20" y="122" ${FONT} font-size="12" fill="#6B7280">Customers</text>
  <text x="20" y="150" ${FONT} font-size="12" fill="#6B7280">Reports</text>
  <rect x="140" y="48" width="480" height="332" rx="8" fill="#FFFFFF" stroke="#E3E6EA"/>
  <text x="164" y="84" ${FONT} font-size="19" font-weight="700" fill="#1B1F24">Invoice #1042</text>
  <rect x="480" y="66" width="118" height="26" rx="13" fill="#FDECEA"/>
  <text x="539" y="83" text-anchor="middle" ${FONT} font-size="11.5" font-weight="600" fill="#B42318">Payment failed</text>
  <g ${FONT} font-size="13">
    <text x="164" y="134" fill="#6B7280">Customer</text><text x="300" y="134" fill="#1B1F24">Jane Doe</text>
    <text x="164" y="168" fill="#6B7280">Email</text><text x="300" y="168" fill="#1B1F24">jane.doe@example.com</text>
    <text x="164" y="202" fill="#6B7280">API key</text><text x="300" y="202" fill="#1B1F24" ${MONO}>sk_test_EXAMPLE_0000</text>
    <text x="164" y="236" fill="#6B7280">Amount</text><text x="300" y="236" font-weight="700" fill="#1B1F24">$1,240.00</text>
  </g>
  <path d="M164 146H596M164 180H596M164 214H596M164 248H596" stroke="#EDEFF2"/>
  <rect x="164" y="292" width="130" height="36" rx="6" fill="#2F6FE4"/>
  <text x="229" y="314.5" text-anchor="middle" ${FONT} font-size="13" font-weight="600" fill="#fff">Retry payment</text>
  <rect x="306" y="292" width="84" height="36" rx="6" fill="#F1F3F5" stroke="#DDE1E6"/>
  <text x="348" y="314.5" text-anchor="middle" ${FONT} font-size="13" fill="#3B4350">Cancel</text>
  <text x="164" y="360" ${FONT} font-size="11.5" fill="#9AA1AB">Last attempt 2 minutes ago</text>`;
}

/** Invented terminal window, used as a second window in the snap demo. */
export function terminalScene(): string {
  return `
  <rect width="640" height="400" fill="#12151A"/>
  <rect width="640" height="28" fill="#1C1F25"/>
  <text x="14" y="18.5" ${FONT} font-size="11" fill="#A3A9B2">Terminal</text>
  <g ${MONO} font-size="13" fill="#A3A9B2">
    <text x="18" y="62" fill="#4ADE80">$ <tspan fill="#E7E9EC">cargo build --release</tspan></text>
    <text x="18" y="88">   Compiling darkshot-core</text>
    <text x="18" y="112">   Compiling darkshot-windows</text>
    <text x="18" y="136" fill="#4C8DFF">    Finished release in 41.3s</text>
    <text x="18" y="176" fill="#4ADE80">$ <tspan fill="#E7E9EC">.\\target\\release\\darkshot.exe</tspan></text>
    <text x="18" y="200">   ready — press PrintScreen</text>
    <text x="18" y="240" fill="#4ADE80">$ <tspan fill="#E7E9EC">_</tspan></text>
  </g>`;
}

/** Annotation layer in scene coordinates. Each mark carries a class so CSS can reveal it step by step. */
export function annotations(): string {
  return `
  <rect class="ann ann-hl" x="296" y="219" width="88" height="23" rx="3" fill="#FFE14D" fill-opacity="0.45"/>
  <rect class="ann ann-redact" x="296" y="153" width="168" height="22" fill="#C5C9CF"/>
  <rect class="ann ann-redact" x="296" y="187" width="168" height="22" fill="#C5C9CF"/>
  <rect class="ann ann-rect" pathLength="1" x="157" y="285" width="144" height="50" rx="2" fill="none" stroke="${RED}" stroke-width="3"/>
  <path class="ann ann-arrow" pathLength="1" d="M476 124 L505 99" fill="none" stroke="${RED}" stroke-width="3" stroke-linecap="round"/>
  <path class="ann ann-arrowhead" d="M512 93 L493 99 L504 111 Z" fill="${RED}"/>
  <text class="ann ann-text" x="396" y="132" ${FONT} font-size="17" font-weight="700" fill="${RED}" stroke="#fff" stroke-width="3.5" paint-order="stroke" stroke-linejoin="round">Fails here</text>
  <g class="ann ann-marker"><circle cx="268" cy="164" r="11" fill="${RED}"/><text x="268" y="168.5" text-anchor="middle" ${FONT} font-size="13" font-weight="700" fill="#fff">1</text></g>
  <g class="ann ann-marker"><circle cx="268" cy="232" r="11" fill="${RED}"/><text x="268" y="236.5" text-anchor="middle" ${FONT} font-size="13" font-weight="700" fill="#fff">2</text></g>`;
}

/** Standalone captured image, optionally with the annotation layer. */
export function sceneSvg(opts: { withAnnotations: boolean }): string {
  return `<svg viewBox="0 0 ${SCENE_W} ${SCENE_H}" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" focusable="false" class="scene-svg">
  ${appScene()}
  ${opts.withAnnotations ? `<g class="ann-layer">${annotations()}</g>` : ""}
</svg>`;
}

const icon = (id: string, x: number, y: number, size = 18, cls = ""): string =>
  `<use href="#${id}" x="${x}" y="${y}" width="${size}" height="${size}" class="${cls}"/>`;

interface Bar {
  svg: string;
  w: number;
  h: number;
}

/** The vertical annotation toolbar: 9 items, as in the real overlay. */
export function verticalToolbar(x: number, y: number): Bar {
  const items = ["select", "|", "pen", "arrow", "marker", "blur", "text", "step", "|", "size", "color"];
  const IS = 30;
  const SEP = 9;
  const pad = 4;
  let cy = pad;
  let body = "";
  for (const it of items) {
    if (it === "|") {
      body += `<path d="M7 ${cy + SEP / 2}H${IS + pad * 2 - 7}" stroke="#3F434A"/>`;
      cy += SEP;
      continue;
    }
    if (it === "select") body += `<rect x="${pad}" y="${cy}" width="${IS}" height="${IS}" rx="5" fill="#363A41"/>`;
    if (it === "color") {
      body += `<rect x="${pad + 8}" y="${cy + 8}" width="14" height="14" rx="2.5" fill="#FF0000"/>`;
    } else if (it === "size") {
      body += `<circle cx="${pad + IS / 2}" cy="${cy + IS / 2}" r="3.5" fill="#D5D9DF"/>`;
    } else {
      body += icon(`i-${it}`, pad + 6, cy + 6, 18, "tb-icon");
    }
    cy += IS;
  }
  const h = cy + pad;
  const w = IS + pad * 2;
  return {
    svg: `<g class="tb tb-v" transform="translate(${x} ${y})"><rect width="${w}" height="${h}" rx="8" fill="#2B2D31" stroke="#3F434A"/>${body}</g>`,
    w,
    h,
  };
}

/** The horizontal action bar: Undo, Redo | Upload, Save, Copy, Cancel. */
export function horizontalBar(x: number, y: number): Bar {
  const items = ["undo", "redo", "|", "upload", "save", "copy", "close"];
  const IS = 30;
  const SEP = 9;
  const pad = 4;
  let cx = pad;
  let body = "";
  for (const it of items) {
    if (it === "|") {
      body += `<path d="M${cx + SEP / 2} 7V${IS + pad * 2 - 7}" stroke="#3F434A"/>`;
      cx += SEP;
      continue;
    }
    if (it === "upload") body += `<rect class="tb-upload-bg" x="${cx}" y="${pad}" width="${IS}" height="${IS}" rx="5" fill="#2F6FE4"/>`;
    body += icon(`i-${it}`, cx + 6, pad + 6, 18, it === "redo" ? "tb-icon tb-dim" : "tb-icon");
    cx += IS;
  }
  const w = cx + pad;
  const h = IS + pad * 2;
  return {
    svg: `<g class="tb tb-h" transform="translate(${x} ${y})"><rect width="${w}" height="${h}" rx="8" fill="#2B2D31" stroke="#3F434A"/>${body}</g>`,
    w,
    h,
  };
}

export const handles = (x: number, y: number, w: number, h: number): string => {
  const pts: [number, number][] = [
    [x, y], [x + w / 2, y], [x + w, y],
    [x, y + h / 2], [x + w, y + h / 2],
    [x, y + h], [x + w / 2, y + h], [x + w, y + h],
  ];
  return pts
    .map(([px, py]) => `<rect x="${px - 3.5}" y="${py - 3.5}" width="7" height="7" fill="#fff" stroke="#16181B"/>`)
    .join("");
};

/** The two-layer selection border the real overlay draws (inner accent, outer near-black). */
export const selectionFrame = (x: number, y: number, w: number, h: number, withHandles: boolean): string => `
  <rect x="${x - 1}" y="${y - 1}" width="${w + 2}" height="${h + 2}" fill="none" stroke="#16181B"/>
  <rect x="${x}" y="${y}" width="${w}" height="${h}" fill="none" stroke="#4C8DFF" stroke-width="1.5"/>
  ${withHandles ? handles(x, y, w, h) : ""}`;

/** Dim path with a rectangular hole. */
export const dimPath = (vw: number, vh: number, hole?: { x: number; y: number; w: number; h: number }): string =>
  `M0 0H${vw}V${vh}H0Z${hole ? ` M${hole.x} ${hole.y}H${hole.x + hole.w}V${hole.y + hole.h}H${hole.x}Z` : ""}`;

export function sizeLabel(x: number, y: number, text: string): string {
  const w = Math.round(text.length * 6.4 + 16);
  return `<g transform="translate(${x} ${y})"><rect width="${w}" height="20" rx="4" fill="#16181B" stroke="#3F434A"/><text x="${w / 2}" y="14" text-anchor="middle" ${FONT} font-size="11" fill="#E7E9EC">${text}</text></g>`;
}

/** Windows-11-like desktop backdrop with a taskbar. */
export function desktopBackdrop(w: number, h: number): string {
  const tb = 32;
  const colors = ["#4C8DFF", "#3DD68C", "#F5A524", "#8B7CF6", "#3B4350"];
  const icons = colors
    .map((c, i) => `<rect x="${w / 2 - 80 + i * 34}" y="${h - tb + 6}" width="20" height="20" rx="5" fill="${c}" fill-opacity="0.85"/>`)
    .join("");
  return `
  <defs><linearGradient id="wall-${w}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#22324f"/><stop offset="1" stop-color="#10151d"/></linearGradient></defs>
  <rect width="${w}" height="${h}" fill="url(#wall-${w})"/>
  <rect y="${h - tb}" width="${w}" height="${tb}" fill="#0D0F12" fill-opacity="0.92"/>${icons}`;
}

const HINT = (w: number, pillW: number): string =>
  `<g class="hint-pill"><rect x="${w / 2 - pillW / 2}" y="8" width="${pillW}" height="22" rx="11" fill="#2B2D31" stroke="#3F434A"/><text x="${w / 2}" y="23" text-anchor="middle" ${FONT} font-size="11" fill="#E7E9EC" data-i18n="demo.hint">Click: window · Drag: region · Esc: cancel</text></g>`;

/** Hero scene: desktop + overlay + toolbars + toast. Visibility per stage is handled by CSS on [data-stage]. */
export function heroSvg(): string {
  const W = 760;
  const H = 470;
  const wx = 50, wy = 22, ws = 0.875; // window drawn at 560×350
  const ww = SCENE_W * ws, wh = SCENE_H * ws;
  const vt = verticalToolbar(wx + ww + 8, wy + wh - 322);
  const hb = horizontalBar(wx + ww - 197, wy + wh + 6);
  return `<svg viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg" class="hero-svg" aria-hidden="true" focusable="false">
  ${desktopBackdrop(W, H)}
  <g transform="translate(${wx} ${wy}) scale(${ws})">${appScene()}<g class="ann-layer">${annotations()}</g></g>
  <path class="dim" fill="#000" fill-opacity="0.55" fill-rule="evenodd" d="${dimPath(W, H, { x: wx, y: wy, w: ww, h: wh })}"/>
  <g class="sel">${selectionFrame(wx, wy, ww, wh, true)}</g>
  <g class="size-label">${sizeLabel(wx, wy - 26, "1120 × 700")}</g>
  ${vt.svg}
  ${hb.svg}
  ${HINT(W, 300)}
  <g class="toast" transform="translate(${W - 322} ${H - 32 - 84})">
    <rect width="306" height="72" rx="10" fill="#202327" stroke="#3F434A"/>
    <rect x="14" y="14" width="22" height="22" rx="5" fill="#16181B" stroke="#4C8DFF"/>
    <path d="M18 22v-4h4M32 18h4v4M36 28v4h-4M22 32h-4v-4" stroke="#4C8DFF" stroke-width="1.6" fill="none"/>
    <text x="46" y="24" ${FONT} font-size="11" fill="#A3A9B2">darkshot</text>
    <text x="46" y="42" ${FONT} font-size="13" font-weight="600" fill="#E7E9EC" data-i18n="demo.linkCopied">Link copied</text>
    <text x="14" y="62" ${MONO} font-size="10.5" fill="#4C8DFF" textLength="276" lengthAdjust="spacingAndGlyphs">https://img.example.com/d30/k3X9f2abQ7mZp1Lw.png</text>
  </g>
</svg>`;
}

export interface SnapWindow {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  label: string;
}

export const SNAP_W = 760;
export const SNAP_H = 470;
/** Front-most first, so hit-testing can stop at the first match. */
export const SNAP_WINDOWS: SnapWindow[] = [
  { id: "terminal", x: 372, y: 150, w: 360, h: 225, label: "1280 × 800" },
  { id: "app", x: 36, y: 40, w: 440, h: 275, label: "1560 × 975" },
];

/** Snap demo: two overlapping windows under a dim layer; the script draws the hole and frame. */
export function snapSvg(): string {
  const [term, app] = SNAP_WINDOWS;
  return `<svg viewBox="0 0 ${SNAP_W} ${SNAP_H}" xmlns="http://www.w3.org/2000/svg" class="snap-svg" aria-hidden="true" focusable="false">
  ${desktopBackdrop(SNAP_W, SNAP_H)}
  <g transform="translate(${app.x} ${app.y}) scale(${app.w / SCENE_W})">${appScene()}</g>
  <g transform="translate(${term.x} ${term.y}) scale(${term.w / SCENE_W})">${terminalScene()}</g>
  <path class="snap-dim" fill="#000" fill-opacity="0.55" fill-rule="evenodd" d="${dimPath(SNAP_W, SNAP_H)}"/>
  <g class="snap-frame"></g>
  <g class="snap-label"></g>
  ${HINT(SNAP_W, 320)}
</svg>`;
}
