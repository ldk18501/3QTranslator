import { describe, expect, it } from "vitest";
import { dailyFallback } from "./mockData";

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
});
