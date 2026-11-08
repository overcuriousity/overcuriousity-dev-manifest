const supported = () =>
  typeof window.PublicKeyCredential === "function" &&
  typeof PublicKeyCredential.parseCreationOptionsFromJSON === "function";

async function postJSON(url, body) {
  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: body === undefined ? "" : JSON.stringify(body),
    credentials: "same-origin",
  });
  if (!res.ok) throw new Error(await res.text());
  return res;
}

function notify(text) {
  const live = document.getElementById("oc-live");
  if (live) live.textContent = text;
}

async function register(button) {
  const start = await (await postJSON("/auth/passkeys/register/start")).json();
  const options = PublicKeyCredential.parseCreationOptionsFromJSON(start.publicKey);
  const cred = await navigator.credentials.create({ publicKey: options });
  const label = button.dataset.name || "Passkey";
  const res = await postJSON(`/auth/passkeys/register/finish?name=${encodeURIComponent(label)}`, cred.toJSON());
  const list = document.getElementById("oc-passkey-list");
  if (list) {
    list.outerHTML = await res.text();
    // Markup inserted outside htmx needs htmx and this module to bind it.
    const fresh = document.getElementById("oc-passkey-list");
    window.htmx?.process(fresh);
    init(fresh);
  }
  notify("Passkey added");
}

async function login(button) {
  const start = await (await postJSON("/auth/passkeys/login/start")).json();
  const options = PublicKeyCredential.parseRequestOptionsFromJSON(start.publicKey);
  const cred = await navigator.credentials.get({ publicKey: options });
  const next = encodeURIComponent(button.dataset.next || "/");
  const res = await postJSON(`/auth/passkeys/login/finish?next=${next}`, cred.toJSON());
  window.location.assign((await res.json()).next);
}

function init(root = document) {
  for (const b of root.querySelectorAll("[data-oc-passkey]")) {
    if (!supported()) {
      b.hidden = true;
      continue;
    }
    if (b.dataset.ocBound) continue;
    b.dataset.ocBound = "1";
    b.addEventListener("click", async () => {
      b.setAttribute("aria-busy", "true");
      try {
        await (b.dataset.ocPasskey === "register" ? register(b) : login(b));
      } catch {
        notify("The passkey could not be used. Try again.");
      } finally {
        b.removeAttribute("aria-busy");
      }
    });
  }
}

init();
document.addEventListener("htmx:afterSwap", (e) => init(e.target));
