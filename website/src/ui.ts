/** Navigation, scroll reveals, feature scrollspy and small page chrome. */

export function initNav(): void {
  const header = document.querySelector<HTMLElement>(".nav");
  const toggle = document.querySelector<HTMLButtonElement>(".nav-toggle");
  const links = document.getElementById("nav-links");
  if (!header || !toggle || !links) return;

  const setOpen = (open: boolean) => {
    header.classList.toggle("open", open);
    toggle.setAttribute("aria-expanded", String(open));
  };
  toggle.addEventListener("click", () => setOpen(toggle.getAttribute("aria-expanded") !== "true"));
  links.addEventListener("click", (e) => {
    if ((e.target as HTMLElement).closest("a")) setOpen(false);
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && header.classList.contains("open")) {
      setOpen(false);
      toggle.focus();
    }
  });
  const onScroll = () => header.classList.toggle("scrolled", window.scrollY > 8);
  window.addEventListener("scroll", onScroll, { passive: true });
  onScroll();

  // Highlight the current section in the primary nav.
  const map = new Map<string, HTMLAnchorElement>();
  links.querySelectorAll<HTMLAnchorElement>("a[href^='#']").forEach((a) => map.set(a.hash.slice(1), a));
  const io = new IntersectionObserver(
    (entries) => {
      for (const en of entries) {
        if (!en.isIntersecting) continue;
        // Sections without a nav entry (download, install) clear the highlight.
        map.forEach((a) => a.removeAttribute("aria-current"));
        map.get(en.target.id)?.setAttribute("aria-current", "true");
      }
    },
    { rootMargin: "-45% 0px -50% 0px" },
  );
  document.querySelectorAll("main section[id]").forEach((sec) => io.observe(sec));
}

export function initReveal(): void {
  const items = document.querySelectorAll<HTMLElement>(".feature, .hl-grid li, .timeline li, .panel, .dl-panel, .notice, .install li");
  if (!("IntersectionObserver" in window) || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  items.forEach((el) => el.classList.add("reveal"));
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting) {
          e.target.classList.add("in");
          io.unobserve(e.target);
        }
      }
    },
    { rootMargin: "0px 0px -8% 0px", threshold: 0.08 },
  );
  items.forEach((el) => io.observe(el));
}

export function initFeatureNav(): void {
  const links = document.querySelectorAll<HTMLAnchorElement>(".feat-nav a");
  const map = new Map<string, HTMLAnchorElement>();
  links.forEach((a) => map.set(a.hash.slice(1), a));
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        links.forEach((a) => a.removeAttribute("aria-current"));
        map.get(e.target.id)?.setAttribute("aria-current", "true");
      }
    },
    { rootMargin: "-40% 0px -55% 0px" },
  );
  map.forEach((_, id) => {
    const el = document.getElementById(id);
    if (el) io.observe(el);
  });
}

export function initYear(): void {
  document.querySelectorAll<HTMLElement>("[data-year]").forEach((el) => (el.textContent = String(new Date().getFullYear())));
}
