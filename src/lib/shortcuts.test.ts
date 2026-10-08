import { describe, expect, it } from "vitest";

import { isSearchShortcut } from "./shortcuts";

const key = (init: Partial<KeyboardEventInit>) => ({ altKey: false, ctrlKey: false, metaKey: false, key: "", ...init }) as KeyboardEvent;

describe("isSearchShortcut", () => {
  it("matches Ctrl+K and Cmd+K in either case", () => {
    expect(isSearchShortcut(key({ ctrlKey: true, key: "k" }))).toBe(true);
    expect(isSearchShortcut(key({ metaKey: true, key: "K" }))).toBe(true);
  });

  it("ignores plain K and other combinations", () => {
    expect(isSearchShortcut(key({ key: "k" }))).toBe(false);
    expect(isSearchShortcut(key({ ctrlKey: true, altKey: true, key: "k" }))).toBe(false);
    expect(isSearchShortcut(key({ ctrlKey: true, key: "j" }))).toBe(false);
  });
});
