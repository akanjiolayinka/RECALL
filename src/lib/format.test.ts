import { describe, expect, it } from "vitest";

import { formatBytes, plural } from "./format";

describe("format", () => {
  it("formats sizes", () => {
    expect(formatBytes(10)).toBe("10 B");
    expect(formatBytes(2_400_000)).toBe("2.3 MB");
  });

  it("pluralises", () => {
    expect(plural(1, "file")).toBe("1 file");
    expect(plural(2, "file")).toBe("2 files");
    expect(plural(3, "passage")).toBe("3 passages");
  });
});
