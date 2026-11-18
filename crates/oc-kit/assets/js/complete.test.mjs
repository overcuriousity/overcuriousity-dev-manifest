import test from "node:test";
import assert from "node:assert/strict";
import { rest, nextWord, cut } from "./complete.js";

test("rest is the suffix of the first case-insensitive, start-anchored match", () => {
  assert.equal(rest("dis", ["Disk full on /var", "Discord"]), "k full on /var");
  assert.equal(rest("cord", ["Discord"]), "");
});

test("typing past the match, an empty field or a complete match suggest nothing", () => {
  assert.equal(rest("disk x", ["Disk full"]), "");
  assert.equal(rest("", ["a"]), "");
  assert.equal(rest("Disk full on /var", ["Disk full on /var"]), "");
});

test("the next word keeps its leading whitespace", () => {
  assert.equal(nextWord("k full on"), "k");
  assert.equal(nextWord(" full on"), " full");
  assert.equal(nextWord(""), "");
});

test("cut shortens to n characters with an ellipsis", () => {
  const c = cut("a".repeat(100), 80);
  assert.equal(c.length, 80);
  assert.ok(c.endsWith("…"));
  assert.equal(cut("ab", 80), "ab");
});
