import { test } from "node:test";
import assert from "node:assert/strict";
import { matchShortcut } from "../src/shortcuts.ts";

const key = (key, code, extra = {}) => ({ key, code, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...extra });

test("macOS uses ⌘ and never matches plain or Ctrl keys typed into a terminal", () => {
  assert.equal(matchShortcut(key("1", "Digit1", { metaKey: true }), true), "nav1");
  assert.equal(matchShortcut(key("r", "KeyR", { metaKey: true }), true), "refresh");
  assert.equal(matchShortcut(key("r", "KeyR"), true), null);
  assert.equal(matchShortcut(key("r", "KeyR", { ctrlKey: true }), true), null);
  assert.equal(matchShortcut(key("w", "KeyW", { metaKey: true }), true), null, "⌘W stays the native window close");
});

test("shift distinguishes session shortcuts from back and layout-independent punctuation", () => {
  assert.equal(matchShortcut(key("[", "BracketLeft", { metaKey: true }), true), "back");
  assert.equal(matchShortcut(key("{", "BracketLeft", { metaKey: true, shiftKey: true }), true), "prevSession");
  assert.equal(matchShortcut(key("}", "BracketRight", { metaKey: true, shiftKey: true }), true), "nextSession");
  assert.equal(matchShortcut(key("F", "KeyF", { metaKey: true, shiftKey: true }), true), "fullWindow");
  assert.equal(matchShortcut(key("W", "KeyW", { metaKey: true, shiftKey: true }), true), "closeSession");
});

test("other platforms require Ctrl+Alt so Ctrl keys reach the shell", () => {
  assert.equal(matchShortcut(key("t", "KeyT", { ctrlKey: true }), false), null);
  assert.equal(matchShortcut(key("t", "KeyT", { ctrlKey: true, altKey: true }), false), "connect");
});
