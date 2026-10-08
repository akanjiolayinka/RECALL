import { describe, expect, it } from "vitest";

import { highlightWords } from "./highlight";

describe("highlightWords", () => {
  it("marks whole words case-insensitively, including word endings", () => {
    expect(highlightWords("The Budget and budgets.", ["budget"])).toEqual([
      { text: "The ", highlight: false },
      { text: "Budget", highlight: true },
      { text: " and ", highlight: false },
      { text: "budgets", highlight: true },
      { text: ".", highlight: false },
    ]);
  });

  it("does not mark matches in the middle of a word", () => {
    expect(highlightWords("subproject", ["project"])).toEqual([{ text: "subproject", highlight: false }]);
  });

  it("treats regex characters in words literally", () => {
    expect(highlightWords("Cost (NGN) 2,500,000", ["(ngn)", "2,500,000"]).filter((p) => p.highlight)).toEqual([
      { text: "(NGN)", highlight: true },
      { text: "2,500,000", highlight: true },
    ]);
  });

  it("handles no words and empty text", () => {
    expect(highlightWords("text", [])).toEqual([{ text: "text", highlight: false }]);
    expect(highlightWords("", ["x"])).toEqual([]);
  });

  it("keeps non-English text intact", () => {
    const parts = highlightWords("Ọgbà ilé wa 🌱 garden", ["garden"]);
    expect(parts.map((p) => p.text).join("")).toBe("Ọgbà ilé wa 🌱 garden");
    expect(parts[parts.length - 1]).toEqual({ text: "garden", highlight: true });
  });
});
