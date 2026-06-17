import { describe, expect, it } from "vitest";
import { dailyFallback } from "./dailyWordBank";

describe("daily fallback", () => {
  it("returns five learning items with three examples", () => {
    const items = dailyFallback("en", "beginner");
    expect(items).toHaveLength(5);
    expect(items.every((item) => item.examples.length === 3)).toBe(true);
  });

  it("falls back to English content for languages without a local pool", () => {
    const items = dailyFallback("fr", "advanced");
    expect(items).toHaveLength(5);
    expect(items[0].language).toBe("fr");
  });

  it("changes non-English vocabulary by level", () => {
    const beginner = dailyFallback("ja", "beginner").map((item) => item.word);
    const advanced = dailyFallback("ja", "advanced").map((item) => item.word);
    expect(beginner).not.toEqual(advanced);
  });
});
