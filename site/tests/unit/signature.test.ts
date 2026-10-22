import assert from "node:assert/strict";
import { test } from "node:test";
import { breakableSignature } from "../../src/lib/signature.ts";

test("paths get a break opportunity after each separator", () => {
  assert.equal(
    breakableSignature("data_table::default_keymap() / action.event(&state)"),
    "data_table::<wbr>default_keymap() / action.event(&amp;state)",
  );
});

test("method calls and arguments stay whole", () => {
  assert.equal(breakableSignature("state.page_rows()"), "state.page_rows()");
  assert.equal(breakableSignature(".empty(text)"), ".empty(text)");
});

test("markup is escaped", () => {
  assert.equal(breakableSignature("Keymap<Action>"), "Keymap&lt;Action&gt;");
});
