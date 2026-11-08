const KEY = "oc.pane-w";

function rem() {
  return parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
}

function setWidth(pane, px) {
  const w = Math.round(Math.min(Math.max(px, 16 * rem()), 48 * rem()));
  pane.style.setProperty("--pane-w", `${w}px`);
  return w;
}

if (typeof document !== "undefined") {
  const pane = document.getElementById("pane");
  if (pane) {
    try {
      const saved = Number(localStorage.getItem(KEY));
      if (saved) setWidth(pane, saved);
    } catch {
      /* storage blocked */
    }
  }

  document.addEventListener("pointerdown", (e) => {
    const handle = e.target.closest?.("[data-oc-pane-resize]");
    if (!handle) return;
    const p = handle.closest(".oc-pane");
    const right = p.getBoundingClientRect().right;
    handle.setPointerCapture(e.pointerId);
    const move = (m) => setWidth(p, right - m.clientX);
    const up = (u) => {
      handle.removeEventListener("pointermove", move);
      try {
        localStorage.setItem(KEY, String(setWidth(p, right - u.clientX)));
      } catch {
        /* storage blocked */
      }
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", up, { once: true });
  });

  // Rail toggles open and close the shell's optional regions.
  document.addEventListener("click", (e) => {
    const b = e.target.closest?.("[data-oc-toggle]");
    if (!b) return;
    const shell = document.getElementById("oc-shell");
    const region = b.dataset.ocToggle;
    const el = document.getElementById(region);
    const shown = el ? getComputedStyle(el).display !== "none" : false;
    shell.dataset[region] = shown ? "closed" : "open";
    b.setAttribute("aria-pressed", String(!shown));
  });
}
