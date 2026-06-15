import type { AppSettings, DailyItem, TranslationResult, WordbookEntry } from "./types";
import { dailyFallback, fallbackDefinitions } from "./mockData";
import { defaultTargetFor, detectLanguage, looksLikeWord } from "./language";

const isTauri = "__TAURI_INTERNALS__" in window;

async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

const defaultSettings: AppSettings = {
  defaultEnglishTarget: "zh",
  defaultOtherTarget: "en",
  dailyLanguage: "en",
  dailyLevel: "beginner",
  shortcutTranslate: "Ctrl+Alt+Q",
  shortcutScreenshot: "Ctrl+Alt+S",
  libreTranslateUrl: "",
  openAiBaseUrl: "",
  openAiApiKey: "",
};

function readLocal<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

function writeLocal<T>(key: string, value: T): void {
  localStorage.setItem(key, JSON.stringify(value));
}

async function fallbackTranslate(text: string, targetLanguage?: string): Promise<TranslationResult> {
  const settings = await getSettings();
  const sourceLanguage = detectLanguage(text);
  const target = targetLanguage || defaultTargetFor(sourceLanguage, settings.defaultEnglishTarget, settings.defaultOtherTarget);
  const word = looksLikeWord(text);
  let translatedText = "";
  let provider = "local fallback";

  try {
    const langPair = `${sourceLanguage}|${target}`;
    const url = `https://api.mymemory.translated.net/get?q=${encodeURIComponent(text)}&langpair=${encodeURIComponent(langPair)}`;
    const response = await fetch(url);
    const data = await response.json();
    translatedText = data?.responseData?.translatedText || "";
    provider = "MyMemory";
  } catch {
    translatedText = sourceLanguage === "en" ? "暂时无法连接免费翻译源，请稍后重试。" : text;
  }

  let definitions = fallbackDefinitions[text.trim().toLowerCase()] ?? [];
  let phonetic = "";
  let examples = definitions.map((item) => item.example).filter(Boolean) as string[];
  let phrases: string[] = [];

  if (word && sourceLanguage === "en") {
    try {
      const response = await fetch(`https://api.dictionaryapi.dev/api/v2/entries/en/${encodeURIComponent(text.trim())}`);
      const data = await response.json();
      const entry = Array.isArray(data) ? data[0] : undefined;
      phonetic = entry?.phonetic ?? entry?.phonetics?.find((item: { text?: string }) => item.text)?.text ?? "";
      definitions = entry?.meanings?.flatMap((meaning: { partOfSpeech: string; definitions: Array<{ definition: string; example?: string; synonyms?: string[] }> }) =>
        meaning.definitions.slice(0, 3).map((definition) => ({
          partOfSpeech: meaning.partOfSpeech,
          meaning: definition.definition,
          example: definition.example,
          synonyms: definition.synonyms,
        })),
      ) ?? definitions;
      examples = definitions.map((item) => item.example).filter(Boolean) as string[];
      phrases = definitions.flatMap((item) => item.synonyms ?? []).slice(0, 6);
    } catch {
      // Keep the offline-like fallback result.
    }
  }

  return {
    sourceText: text,
    sourceLanguage,
    targetLanguage: target,
    translatedText,
    phonetic,
    definitions,
    examples: examples.slice(0, 6),
    phrases,
    provider,
    isWord: word,
  };
}

export async function translateText(text: string, targetLanguage?: string): Promise<TranslationResult> {
  if (isTauri) {
    return tauriInvoke("translate_text", { text, targetLanguage });
  }
  return fallbackTranslate(text, targetLanguage);
}

export async function addToWordbook(result: TranslationResult | DailyItem): Promise<WordbookEntry> {
  if (isTauri) {
    return tauriInvoke("add_to_wordbook", { item: result });
  }

  const entries = await listWordbook();
  const entry: WordbookEntry = "sourceText" in result
    ? {
        id: crypto.randomUUID(),
        text: result.sourceText,
        language: result.sourceLanguage,
        targetLanguage: result.targetLanguage,
        translation: result.translatedText,
        definitions: result.definitions,
        examples: result.examples,
        source: result.provider,
        createdAt: new Date().toISOString(),
      }
    : {
        id: crypto.randomUUID(),
        text: result.word,
        language: result.language,
        targetLanguage: "zh",
        translation: result.translation,
        definitions: [],
        examples: result.examples,
        source: "daily learning",
        createdAt: new Date().toISOString(),
      };
  writeLocal("wordbook", [entry, ...entries]);
  return entry;
}

export async function listWordbook(): Promise<WordbookEntry[]> {
  if (isTauri) {
    return tauriInvoke("list_wordbook");
  }
  return readLocal<WordbookEntry[]>("wordbook", []);
}

export async function getDailyItems(language: string, level: AppSettings["dailyLevel"], forceRefresh = false): Promise<DailyItem[]> {
  if (isTauri) {
    return tauriInvoke("get_daily_items", { language, level, forceRefresh });
  }

  const today = new Date().toISOString().slice(0, 10);
  const key = `daily:${language}:${level}`;
  const cached = readLocal<{ date: string; items: DailyItem[] } | null>(key, null);
  if (!forceRefresh && cached?.date === today) return cached.items;
  const items = dailyFallback(language, level);
  writeLocal(key, { date: today, items });
  return items;
}

export async function getSettings(): Promise<AppSettings> {
  if (isTauri) {
    return tauriInvoke("get_settings");
  }
  return { ...defaultSettings, ...readLocal<Partial<AppSettings>>("settings", {}) };
}

export async function saveSettings(settings: AppSettings): Promise<AppSettings> {
  if (isTauri) {
    return tauriInvoke("save_settings", { settings });
  }
  writeLocal("settings", settings);
  return settings;
}

export async function captureAndTranslate(): Promise<TranslationResult> {
  if (isTauri) {
    return tauriInvoke("capture_and_translate");
  }
  throw new Error("截图翻译需要在 Tauri 桌面端运行。");
}
