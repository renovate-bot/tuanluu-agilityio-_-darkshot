/**
 * Interactive product illustrations. All of them are drawn from scene.ts and
 * are honest reconstructions, labelled as illustrations in the page.
 */
import { onLangChange, t, translate } from "./i18n";
import {
  SCENE_H,
  SCENE_W,
  SNAP_H,
  SNAP_W,
  SNAP_WINDOWS,
  appScene,
  dimPath,
  heroSvg,
  horizontalBar,
  sceneSvg,
  selectionFrame,
  sizeLabel,
  snapSvg,
} from "./scene";

const reduceMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Fill every [data-scene] placeholder. */
export function injectScenes(): void {
  const redactSvg = `<svg viewBox="0 0 ${SCENE_W} ${SCENE_H}" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" focusable="false" class="scene-svg">
    ${appScene()}
    <rect class="rd" x="296" y="153" width="168" height="22" fill="#C5C9CF"/>
    <rect class="rd" x="296" y="187" width="168" height="22" fill="#C5C9CF"/>
  </svg>`;
  const builders: Record<string, () => string> = {
    hero: heroSvg,
    snap: snapSvg,
    before: () => sceneSvg({ withAnnotations: false }),
    after: () => sceneSvg({ withAnnotations: true }),
    redact: () => redactSvg,
  };
  document.querySelectorAll<HTMLElement>("[data-scene]").forEach((el) => {
    const build = builders[el.dataset.scene ?? ""];
    if (build) el.innerHTML = build();
  });
  const bar = document.querySelector<SVGSVGElement>("[data-bar]");
  if (bar) {
    const hb = horizontalBar(1, 1);
    bar.setAttribute("viewBox", `0 0 ${hb.w + 2} ${hb.h + 2}`);
    bar.innerHTML = hb.svg;
  }
}

/* ---------- Hero: Capture → Annotate → Share ---------- */

const STAGES = ["capture", "annotate", "share"] as const;
type Stage = (typeof STAGES)[number];
const STAGE_MS: Record<Stage, number> = { capture: 3200, annotate: 5200, share: 3800 };

export function initHero(): void {
  const root = document.querySelector<HTMLElement>("[data-hero]");
  if (!root) return;
  const buttons = root.querySelectorAll<HTMLButtonElement>("[data-stage-btn]");
  let stage: Stage = reduceMotion() ? "annotate" : "capture";
  let timer = 0;
  let auto = !reduceMotion();
  let visible = true;

  const show = (s: Stage) => {
    stage = s;
    root.dataset.stage = s;
    buttons.forEach((b) => b.setAttribute("aria-selected", String(b.dataset.stageBtn === s)));
  };
  const schedule = () => {
    window.clearTimeout(timer);
    if (!auto || !visible || document.hidden) return;
    timer = window.setTimeout(() => {
      show(STAGES[(STAGES.indexOf(stage) + 1) % STAGES.length]);
      schedule();
    }, STAGE_MS[stage]);
  };

  buttons.forEach((b) =>
    b.addEventListener("click", () => {
      auto = false; // a deliberate choice stops the loop
      window.clearTimeout(timer);
      show(b.dataset.stageBtn as Stage);
    }),
  );
  new IntersectionObserver((entries) => {
    visible = entries[0]?.isIntersecting ?? true;
    schedule();
  }).observe(root);
  document.addEventListener("visibilitychange", schedule);
  show(stage);
  schedule();
}

/* ---------- A. Snap demo: click a window, drag a region ---------- */

export function initSnap(): void {
  const root = document.querySelector<HTMLElement>("[data-snap]");
  const svg = root?.querySelector<SVGSVGElement>("svg");
  const status = document.querySelector<HTMLElement>("[data-snap-status]");
  const resetBtn = document.querySelector<HTMLButtonElement>("[data-snap-reset]");
  if (!root || !svg || !status) return;

  const dim = svg.querySelector<SVGPathElement>(".snap-dim")!;
  const frame = svg.querySelector<SVGGElement>(".snap-frame")!;
  const label = svg.querySelector<SVGGElement>(".snap-label")!;
  const hint = svg.querySelector<SVGGElement>(".hint-pill")!;

  type Box = { x: number; y: number; w: number; h: number; size: string };
  let hover = -1;
  let selected: Box | null = null;
  let drag: { x0: number; y0: number; box: Box | null } | null = null;
  let statusKey: [string, Record<string, string>] = ["fa.status0", {}];

  const nameOf = (id: string) => t(id === "terminal" ? "fa.winTerminal" : "fa.winApp");
  const setStatus = (key: string, vars: Record<string, string> = {}) => {
    statusKey = [key, vars];
    status.textContent = t(key, vars);
  };
  onLangChange(() => (status.textContent = t(...statusKey)));

  const toSvg = (e: PointerEvent): [number, number] => {
    const r = svg.getBoundingClientRect();
    return [((e.clientX - r.left) * SNAP_W) / r.width, ((e.clientY - r.top) * SNAP_H) / r.height];
  };
  const hit = (x: number, y: number) => SNAP_WINDOWS.findIndex((w) => x >= w.x && x <= w.x + w.w && y >= w.y && y <= w.y + w.h);
  const boxOfWindow = (i: number): Box => {
    const w = SNAP_WINDOWS[i];
    return { x: w.x, y: w.y, w: w.w, h: w.h, size: w.label };
  };

  const draw = () => {
    const box = selected ?? drag?.box ?? (hover >= 0 ? boxOfWindow(hover) : null);
    dim.setAttribute("d", dimPath(SNAP_W, SNAP_H, box ?? undefined));
    if (box) {
      frame.innerHTML = selectionFrame(box.x, box.y, box.w, box.h, !!selected || !!drag?.box);
      label.innerHTML = sizeLabel(box.x, Math.max(2, box.y - 26), box.size);
    } else {
      frame.innerHTML = "";
      label.innerHTML = "";
    }
    hint.style.opacity = selected || drag?.box ? "0" : "1";
  };

  const reset = () => {
    hover = -1;
    selected = null;
    drag = null;
    setStatus("fa.status0");
    draw();
  };

  const setHover = (i: number) => {
    if (i === hover) return;
    hover = i;
    if (i >= 0) setStatus("fa.statusHover", { name: nameOf(SNAP_WINDOWS[i].id), size: SNAP_WINDOWS[i].label });
    else setStatus("fa.status0");
    draw();
  };
  const selectWindow = (i: number) => {
    selected = boxOfWindow(i);
    setStatus("fa.statusWindow", { size: selected.size });
    draw();
  };

  root.addEventListener("pointermove", (e) => {
    const [x, y] = toSvg(e);
    if (drag) {
      const w = Math.abs(x - drag.x0);
      const h = Math.abs(y - drag.y0);
      if (drag.box || Math.hypot(x - drag.x0, y - drag.y0) > 5) {
        drag.box = { x: Math.min(x, drag.x0), y: Math.min(y, drag.y0), w, h, size: `${Math.round(w * 2.2)} × ${Math.round(h * 2.2)}` };
        draw();
      }
      return;
    }
    if (!selected) setHover(hit(x, y));
  });
  root.addEventListener("pointerleave", () => {
    if (!drag && !selected) setHover(-1);
  });
  root.addEventListener("pointerdown", (e) => {
    const [x, y] = toSvg(e);
    if (selected) {
      selected = null; // a new gesture starts over, like the real overlay before a region exists
    }
    hover = hit(x, y);
    drag = { x0: x, y0: y, box: null };
    root.setPointerCapture(e.pointerId);
    root.focus({ preventScroll: true });
    draw();
  });
  const finish = (e: PointerEvent, cancelled: boolean) => {
    if (!drag) return;
    const d = drag;
    drag = null;
    if (root.hasPointerCapture(e.pointerId)) root.releasePointerCapture(e.pointerId);
    if (cancelled) return draw();
    if (d.box && d.box.w > 4 && d.box.h > 4) {
      selected = d.box;
      setStatus("fa.statusRegion", { size: d.box.size });
      draw();
    } else if (hover >= 0) {
      selectWindow(hover);
    } else {
      draw();
    }
  };
  root.addEventListener("pointerup", (e) => finish(e, false));
  root.addEventListener("pointercancel", (e) => finish(e, true));

  // Keyboard: arrows cycle windows, Enter selects, Esc resets.
  root.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      reset();
    } else if (e.key === "Enter" && hover >= 0 && !selected) {
      selectWindow(hover);
      e.preventDefault();
    } else if (/^Arrow/.test(e.key) && !selected) {
      setHover(hover < 0 ? 0 : (hover + 1) % SNAP_WINDOWS.length);
      e.preventDefault();
    }
  });
  resetBtn?.addEventListener("click", reset);
  draw();
}

/* ---------- B. Before / after slider ---------- */

export function initCompare(): void {
  const root = document.querySelector<HTMLElement>("[data-compare]");
  const input = root?.querySelector<HTMLInputElement>("input[type=range]");
  if (!root || !input) return;
  const update = () => root.style.setProperty("--split", `${input.value}%`);
  input.addEventListener("input", update);
  update();
}

/* ---------- C. Redaction toggle ---------- */

export function initRedact(): void {
  const root = document.querySelector<HTMLElement>("[data-redact]");
  const btn = document.querySelector<HTMLButtonElement>("[data-redact-toggle]");
  const label = document.querySelector<HTMLElement>("[data-redact-label]");
  const status = document.querySelector<HTMLElement>("[data-redact-status]");
  if (!root || !btn || !label || !status) return;
  let on = false;
  const render = () => {
    root.dataset.state = on ? "on" : "off";
    btn.setAttribute("aria-pressed", String(on));
    label.dataset.i18n = on ? "fc.undo" : "fc.apply";
    status.dataset.i18n = on ? "fc.s1" : "fc.s0";
    translate(btn);
    translate(status.parentElement!);
  };
  btn.addEventListener("click", () => {
    on = !on;
    render();
  });
}

/* ---------- D. Tabs (export actions) ---------- */

export function initTabs(): void {
  document.querySelectorAll<HTMLElement>("[data-tabs]").forEach((root) => {
    const tabs = Array.from(root.querySelectorAll<HTMLButtonElement>("[role=tab]"));
    const select = (i: number, focus = false) => {
      tabs.forEach((tab, j) => {
        const on = i === j;
        tab.setAttribute("aria-selected", String(on));
        tab.tabIndex = on ? 0 : -1;
        const panel = document.getElementById(tab.getAttribute("aria-controls")!);
        if (panel) panel.hidden = !on;
      });
      if (focus) tabs[i].focus();
    };
    tabs.forEach((tab, i) => {
      tab.addEventListener("click", () => select(i));
      tab.addEventListener("keydown", (e) => {
        const n = tabs.length;
        const next: Record<string, number> = {
          ArrowRight: (i + 1) % n,
          ArrowDown: (i + 1) % n,
          ArrowLeft: (i + n - 1) % n,
          ArrowUp: (i + n - 1) % n,
          Home: 0,
          End: n - 1,
        };
        if (e.key in next) {
          e.preventDefault();
          select(next[e.key], true);
        }
      });
    });
  });
}

/* ---------- E. R2 link preview ---------- */

export function initR2(): void {
  const root = document.querySelector<HTMLElement>("[data-r2]");
  const out = root?.querySelector<HTMLElement>("[data-r2-link]");
  if (!root || !out) return;
  const update = () => {
    const folder = root.querySelector<HTMLInputElement>("input[name=expiry]:checked")?.value ?? "d30";
    const ext = root.querySelector<HTMLInputElement>("input[name=fmt]:checked")?.value ?? "png";
    out.textContent = `https://img.example.com/${folder}/k3X9f2abQ7mZp1Lw.${ext}`;
  };
  root.addEventListener("change", update);
  update();
}
