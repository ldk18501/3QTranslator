const languageNames: Record<string, string> = {
  en: "英语",
  zh: "中文",
  ja: "日语",
  ko: "韩语",
  fr: "法语",
  de: "德语",
  es: "西班牙语",
  ru: "俄语",
  it: "意大利语",
  pt: "葡萄牙语",
  ar: "阿拉伯语",
};

export const languageOptions = [
  { code: "zh", label: "中文" },
  { code: "en", label: "英语" },
  { code: "ja", label: "日语" },
  { code: "ko", label: "韩语" },
  { code: "fr", label: "法语" },
  { code: "de", label: "德语" },
  { code: "es", label: "西班牙语" },
  { code: "ru", label: "俄语" },
  { code: "it", label: "意大利语" },
  { code: "pt", label: "葡萄牙语" },
];

export function languageLabel(code: string): string {
  return languageNames[code] ?? code.toUpperCase();
}

export function detectLanguage(text: string): string {
  if (/[\u4e00-\u9fff]/.test(text)) return "zh";
  if (/[\u3040-\u30ff]/.test(text)) return "ja";
  if (/[\uac00-\ud7af]/.test(text)) return "ko";
  if (/[\u0400-\u04ff]/.test(text)) return "ru";
  if (/[\u0600-\u06ff]/.test(text)) return "ar";
  return "en";
}

export function looksLikeWord(text: string): boolean {
  return /^[\p{L}'-]{1,40}$/u.test(text.trim());
}

export function defaultTargetFor(sourceLanguage: string, englishTarget = "zh", otherTarget = "en"): string {
  return sourceLanguage === "en" ? englishTarget : otherTarget;
}
