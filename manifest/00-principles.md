# 00 · Principles

Nine principles behind every rule in this manifest. When a rule and a principle seem to disagree, the principle wins and the rule gets fixed.

- MUST: **Quiet by default.** Show nothing unless asked for. Chrome is minimal; detail appears on demand.
- MUST: **Few words.** UI copy is a label, a unit, a range, never a paragraph. Explanations belong in docs.
- MUST: **Always obvious what acts.** A link goes somewhere, a button does something, and each looks like what it is.
- MUST: **Calm for long sessions.** Moderate saturation, off-white and off-black, no decorative motion.
- MUST: **Legible for everyone.** Contrast floors, colour-blind-safe status, a 12 px text floor, the user's text size respected.
- MUST: **Offline and private.** No outbound connection unless configured; an airgapped install is always possible.
- MUST: **Lean beats clever.** Few dependencies, one way to do each thing, comments only where needed.
- MUST: **Reversible over confirmed.** Offer Undo instead of asking "are you sure".
- MUST: **The deployer decides.** The defaults are safe; anything that only shapes the deployment (proxy, TLS, bind address, paths, service manager) is a setting, not a rule. Leaving a safe default is explicit, never silent.

Every rule says MUST, SHOULD, NEVER or MAY, then gives a one-line *why* where one exists. Where research is thin, the *why* says so instead of overclaiming.

The manifest is a reference. `oc-kit` implements it and the demo shows it, but an app follows the manifest by following the rules, not by depending on the crates.
