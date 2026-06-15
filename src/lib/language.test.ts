import { describe, expect, it } from "vitest";
import { defaultTargetFor, detectLanguage, looksLikeWord } from "./language";

describe("language helpers", () => {
  it("detects common scripts", () => {
    expect(detectLanguage("hello")).toBe("en");
    expect(detectLanguage("你好")).toBe("zh");
    expect(detectLanguage("こんにちは")).toBe("ja");
    expect(detectLanguage("안녕하세요")).toBe("ko");
  });

  it("uses Chinese as the default target for English", () => {
    expect(defaultTargetFor("en", "zh", "en")).toBe("zh");
    expect(defaultTargetFor("ja", "zh", "en")).toBe("en");
  });

  it("separates words from long text", () => {
    expect(looksLikeWord("salt")).toBe(true);
    expect(looksLikeWord("mother-in-law")).toBe(true);
    expect(looksLikeWord("hello world")).toBe(false);
  });
});
