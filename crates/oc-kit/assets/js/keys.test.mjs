import test from "node:test";
import assert from "node:assert/strict";
import { shouldIgnore } from "./keys.js";

test("shortcuts never fire while typing in a field", () => {
  for (const tagName of ["INPUT", "TEXTAREA", "SELECT"]) assert.equal(shouldIgnore({ tagName }), true);
  assert.equal(shouldIgnore({ tagName: "DIV", isContentEditable: true }), true);
  assert.equal(shouldIgnore({ tagName: "BUTTON" }), false);
  assert.equal(shouldIgnore(null), false);
});
