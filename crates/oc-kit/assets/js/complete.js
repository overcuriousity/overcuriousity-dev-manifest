// Inline suggestions from the app's own data. Ranking is the caller's job: the first match wins.
// attach() expects the markup of the kit's complete-field component around the input.
// Attaching an already attached input returns its existing detach and ignores the new options.

export function rest(text, candidates) {
  if (!text) return "";
  const t = text.toLowerCase();
  const hit = candidates.find((c) => c.length > text.length && c.toLowerCase().startsWith(t));
  return hit ? hit.slice(text.length) : "";
}

export function nextWord(rest) {
  return rest.match(/^\s*\S*/)[0];
}

export function cut(s, n) {
  return s.length > n ? `${s.slice(0, n - 1)}…` : s;
}

const attached = new WeakMap();

function shared(id, setup) {
  let el = document.getElementById(id);
  if (!el) {
    el = document.createElement("span");
    el.id = id;
    el.className = "oc-sr-only";
    setup(el);
    document.body.append(el);
  }
  return el;
}

// Through execCommand so undo works and the view sees an ordinary input event.
function insert(input, text, onFallback) {
  if (document.execCommand("insertText", false, text)) return;
  const max = input.maxLength >= 0 ? input.maxLength : Infinity;
  input.value = (input.value.slice(0, input.selectionStart) + text).slice(0, max);
  input.dispatchEvent(new Event("input", { bubbles: true }));
  onFallback();
}

export function attach(input, { candidates, recent }) {
  if (attached.has(input)) return attached.get(input);
  const wrap = input.closest(".oc-complete");
  const ghost = wrap.querySelector(".oc-complete__ghost");
  const typed = ghost.querySelector(".oc-complete__typed");
  const tail = ghost.querySelector(".oc-complete__rest");
  const take = wrap.querySelector(".oc-complete__take");
  const row = wrap.querySelector(".oc-complete__recent");
  shared("oc-complete-help", (el) => (el.textContent = "Tab takes the suggested word. Escape hides it."));
  const live = shared("oc-complete-live", (el) => el.setAttribute("aria-live", "polite"));

  let dismissed = false;
  let pendingChange = false;
  let timer;
  let current = "";
  let visible = false;
  let shownRecent;

  const atEnd = () => input.selectionStart === input.value.length && input.selectionEnd === input.value.length;

  // Empty field: recent row; text: take button. Both share one slot, so only visibility changes.
  const renderRecent = () => {
    row.inert = input.value !== "";
    const values = (recent?.() ?? []).slice(0, 5);
    const key = JSON.stringify(values);
    if (key === shownRecent) return;
    shownRecent = key;
    row.replaceChildren(
      ...values.map((v) => {
        const b = document.createElement("button");
        b.type = "button";
        b.className = "oc-btn";
        const t = document.createElement("span");
        t.textContent = v;
        b.append(t);
        b.addEventListener("click", () => {
          input.focus();
          input.select();
          insert(input, v, () => (pendingChange = true));
        });
        return b;
      }),
    );
  };

  const update = () => {
    const focused = document.activeElement === input;
    current = focused && !dismissed && atEnd() ? rest(input.value, candidates()) : "";
    // Overflowing text leaves no room after the caret; the take button still works.
    visible = Boolean(current) && input.scrollWidth <= input.clientWidth;
    typed.textContent = visible ? input.value : "";
    tail.textContent = visible ? current : "";
    take.inert = !current;
    renderRecent();
    // The input scrolls to the caret only after layout.
    requestAnimationFrame(() => (ghost.scrollLeft = input.scrollLeft));
  };

  const takeWord = () => {
    if (!current) return;
    insert(input, nextWord(current), () => (pendingChange = true));
  };

  const onInput = () => {
    dismissed = false;
    update();
    clearTimeout(timer);
    timer = setTimeout(() => {
      if (current) live.textContent = cut(input.value + current, 80);
    }, 500);
  };
  const onKey = (e) => {
    // keyCode 229: Safari's composition keydowns.
    if (e.isComposing || e.keyCode === 229 || !visible) return;
    if (e.key === "Tab" && !e.shiftKey && !e.ctrlKey && !e.altKey && !e.metaKey) {
      e.preventDefault();
      takeWord();
    } else if (e.key === "Escape") {
      // Keeps keys.js and the browser from also closing a menu or dialog.
      e.preventDefault();
      e.stopPropagation();
      dismissed = true;
      update();
    }
  };
  const onBlur = () => {
    if (pendingChange) {
      pendingChange = false;
      input.dispatchEvent(new Event("change", { bubbles: true }));
    }
    update();
  };
  const keepFocus = (e) => e.preventDefault();
  const listeners = [
    [input, "input", onInput],
    [input, "keydown", onKey],
    [input, "keyup", update],
    [input, "click", update],
    [input, "focus", update],
    [input, "scroll", update],
    [input, "blur", onBlur],
    [take, "mousedown", keepFocus],
    [take, "click", takeWord],
  ];
  for (const [el, type, fn] of listeners) el.addEventListener(type, fn);
  update();

  const detach = () => {
    for (const [el, type, fn] of listeners) el.removeEventListener(type, fn);
    clearTimeout(timer);
    typed.textContent = tail.textContent = "";
    take.inert = true;
    row.replaceChildren();
    attached.delete(input);
  };
  attached.set(input, detach);
  return detach;
}

// Apps may rewrite data-oc-complete after their own writes; it is re-parsed only when it changes.
function wire(wrap) {
  const input = wrap.querySelector("input");
  if (!input || attached.has(input)) return;
  let raw;
  let list = [];
  const candidates = () => {
    if (wrap.dataset.ocComplete !== raw) {
      raw = wrap.dataset.ocComplete;
      try {
        const parsed = JSON.parse(raw);
        list = Array.isArray(parsed) ? parsed.filter((s) => typeof s === "string") : [];
      } catch {
        list = [];
      }
    }
    return list;
  };
  attach(input, { candidates, recent: () => candidates().slice(0, 5) });
}

function within(root, selector) {
  return [...(root.matches?.(selector) ? [root] : []), ...(root.querySelectorAll?.(selector) ?? [])];
}

function scan(root) {
  for (const w of within(root, "[data-oc-complete]")) wire(w);
}

if (typeof document !== "undefined") {
  scan(document);
  document.addEventListener("htmx:afterSwap", (e) => scan(e.target));
  // A restored history snapshot holds fresh, unwired inputs.
  document.addEventListener("htmx:historyRestore", () => scan(document));
  // htmx fires this on every element it removes, descendants included.
  document.addEventListener("htmx:beforeCleanupElement", (e) => {
    if (e.target.matches?.("input")) attached.get(e.target)?.();
  });
}
