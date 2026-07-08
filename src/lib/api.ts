import type { ApiProvider, AppSettings, DailyItem, Level, ProviderTestResult, ScreenshotCapture, ScreenshotRegion, TranslationResult, WordbookEntry } from "./types";
import { fallbackDefinitions } from "./mockData";
import { dailyFallback } from "./dailyWordBank";
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
  dailyCacheLimit: 120,
  shortcutTranslate: "Ctrl+Alt+Q",
  shortcutScreenshot: "Ctrl+Alt+S",
  closeToTray: true,
  launchAtStartup: false,
  activeProviderId: "mymemory",
  apiProviders: [
    { id: "mymemory", name: "MyMemory 免费源", providerType: "mymemory", enabled: true, baseUrl: "", apiKey: "", apiSecret: "", region: "", model: "" },
    { id: "libre-default", name: "LibreTranslate", providerType: "libretranslate", enabled: false, baseUrl: "", apiKey: "", apiSecret: "", region: "", model: "" },
    { id: "openai-default", name: "OpenAI-compatible", providerType: "openai", enabled: false, baseUrl: "", apiKey: "", apiSecret: "", region: "", model: "gpt-4o-mini" },
    { id: "tencent-default", name: "腾讯云机器翻译", providerType: "tencent", enabled: false, baseUrl: "https://tmt.tencentcloudapi.com", apiKey: "", apiSecret: "", region: "ap-guangzhou", model: "" },
    { id: "azure-default", name: "Azure Translator", providerType: "azure", enabled: false, baseUrl: "https://api.cognitive.microsofttranslator.com", apiKey: "", apiSecret: "", region: "", model: "" },
    { id: "deepl-default", name: "DeepL API", providerType: "deepl", enabled: false, baseUrl: "https://api-free.deepl.com/v2", apiKey: "", apiSecret: "", region: "", model: "" },
    { id: "baidu-default", name: "百度翻译开放平台", providerType: "baidu", enabled: false, baseUrl: "https://fanyi-api.baidu.com/api/trans/vip/translate", apiKey: "", apiSecret: "", region: "", model: "" },
  ],
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

function inferDifficulty(text: string, definitions = 0, examples = 0): Level {
  const size = text.trim().length;
  if (size <= 4 && definitions <= 1) return "zero";
  if (size <= 8 && definitions <= 2) return "beginner";
  if (size <= 13 || examples >= 2) return "skilled";
  return "advanced";
}

function activeProvider(settings: AppSettings): ApiProvider {
  return settings.apiProviders.find((item) => item.id === settings.activeProviderId && item.enabled) ?? settings.apiProviders[0];
}

async function translateWithLibre(text: string, source: string, target: string, provider: ApiProvider): Promise<string> {
  if (!provider.baseUrl.trim()) throw new Error("LibreTranslate 地址为空");
  const response = await fetch(`${provider.baseUrl.replace(/\/$/, "")}/translate`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      q: text,
      source,
      target,
      format: "text",
      api_key: provider.apiKey || undefined,
    }),
  });
  const data = await response.json();
  if (!response.ok || !data?.translatedText) throw new Error("LibreTranslate 翻译失败");
  return data.translatedText;
}

async function translateWithOpenAi(text: string, source: string, target: string, provider: ApiProvider): Promise<string> {
  if (!provider.baseUrl.trim() || !provider.apiKey.trim()) throw new Error("OpenAI-compatible 配置不完整");
  const base = provider.baseUrl.replace(/\/$/, "");
  const endpoint = base.endsWith("/chat/completions") ? base : `${base}/chat/completions`;
  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${provider.apiKey}`,
    },
    body: JSON.stringify({
      model: provider.model || "gpt-4o-mini",
      temperature: 0.2,
      messages: [
        { role: "system", content: "You are a concise translation engine. Return only the translated text." },
        { role: "user", content: `Translate from ${source} to ${target}:\n${text}` },
      ],
    }),
  });
  const data = await response.json();
  const translated = data?.choices?.[0]?.message?.content?.trim();
  if (!response.ok || !translated) throw new Error("OpenAI-compatible 翻译失败");
  return translated;
}

async function fallbackTranslate(text: string, targetLanguage?: string): Promise<TranslationResult> {
  const settings = await getSettings();
  const sourceLanguage = detectLanguage(text);
  const target = targetLanguage || defaultTargetFor(sourceLanguage, settings.defaultEnglishTarget, settings.defaultOtherTarget);
  const word = looksLikeWord(text);
  let translatedText = "";
  let provider = activeProvider(settings);
  let providerName = provider.name;

  try {
    if (provider.providerType === "libretranslate") {
      translatedText = await translateWithLibre(text, sourceLanguage, target, provider);
    } else if (provider.providerType === "openai") {
      translatedText = await translateWithOpenAi(text, sourceLanguage, target, provider);
    } else {
      const langPair = `${sourceLanguage}|${target}`;
      const url = `https://api.mymemory.translated.net/get?q=${encodeURIComponent(text)}&langpair=${encodeURIComponent(langPair)}`;
      const response = await fetch(url);
      const data = await response.json();
      translatedText = data?.responseData?.translatedText || "";
      providerName = "MyMemory";
    }
  } catch {
    try {
      const langPair = `${sourceLanguage}|${target}`;
      const url = `https://api.mymemory.translated.net/get?q=${encodeURIComponent(text)}&langpair=${encodeURIComponent(langPair)}`;
      const response = await fetch(url);
      const data = await response.json();
      translatedText = data?.responseData?.translatedText || (sourceLanguage === "en" ? "暂时无法连接翻译源，请稍后重试。" : text);
    } catch {
      translatedText = sourceLanguage === "en" ? "暂时无法连接翻译源，请稍后重试。" : text;
    }
    providerName = provider.providerType === "mymemory" ? "MyMemory" : `${provider.name} → MyMemory fallback`;
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
    exampleTranslations: [],
    phrases,
    provider: providerName,
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
        examples: result.examples.map((example, index) => {
          const translated = result.exampleTranslations[index];
          return translated ? `${example}\n${translated}` : example;
        }),
        level: inferDifficulty(result.sourceText, result.definitions.length, result.examples.length),
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
        examples: result.examples.map((example, index) => {
          const translated = result.exampleTranslations[index];
          return translated ? `${example}\n${translated}` : example;
        }),
        level: result.level,
        source: "daily learning",
        createdAt: new Date().toISOString(),
      };
  const withoutDuplicate = entries.filter((item) => !(item.text === entry.text && item.language === entry.language && item.targetLanguage === entry.targetLanguage));
  writeLocal("wordbook", [entry, ...withoutDuplicate]);
  return entry;
}

export async function listWordbook(): Promise<WordbookEntry[]> {
  if (isTauri) {
    return tauriInvoke("list_wordbook");
  }
  return readLocal<Array<WordbookEntry & { level?: Level }>>("wordbook", []).map((item) => ({
    ...item,
    level: item.level ?? inferDifficulty(item.text, item.definitions.length, item.examples.length),
  }));
}

export async function deleteWordbookEntry(id: string): Promise<void> {
  if (isTauri) {
    return tauriInvoke("delete_wordbook_entry", { id });
  }
  const entries = await listWordbook();
  writeLocal("wordbook", entries.filter((item) => item.id !== id));
}

export async function updateWordbookEntryLevel(id: string, level: Level): Promise<WordbookEntry> {
  if (isTauri) {
    return tauriInvoke("update_wordbook_entry_level", { id, level });
  }
  const entries = await listWordbook();
  const entry = entries.find((item) => item.id === id);
  if (!entry) throw new Error("单词不存在");
  const updated = { ...entry, level };
  writeLocal("wordbook", entries.map((item) => (item.id === id ? updated : item)));
  return updated;
}

export async function getDailyItems(language: string, level: AppSettings["dailyLevel"], forceRefresh = false): Promise<DailyItem[]> {
  if (isTauri) {
    return tauriInvoke("get_daily_items", { language, level, forceRefresh });
  }

  const today = new Date().toISOString().slice(0, 10);
  const key = `daily:v5:${language}:${level}`;
  const cached = readLocal<{ date: string; items: DailyItem[] } | null>(key, null);
  if (!forceRefresh && cached?.date === today) return cached.items;
  const items = dailyFallback(language, level, forceRefresh);
  writeLocal(key, { date: today, items });
  return items;
}

export async function getSettings(): Promise<AppSettings> {
  if (isTauri) {
    return tauriInvoke("get_settings");
  }
  const saved = readLocal<Partial<AppSettings>>("settings", {});
  return {
    ...defaultSettings,
    ...saved,
    apiProviders: normalizeApiProviders(saved.apiProviders),
    activeProviderId: saved.activeProviderId ?? defaultSettings.activeProviderId,
  };
}

function normalizeApiProviders(saved?: ApiProvider[]): ApiProvider[] {
  const providers = (saved?.length ? saved : defaultSettings.apiProviders).map((provider) => ({
    ...provider,
    apiSecret: provider.apiSecret ?? "",
    region: provider.region ?? "",
  }));
  for (const defaultProvider of defaultSettings.apiProviders) {
    if (!providers.some((provider) => provider.id === defaultProvider.id)) {
      providers.push(defaultProvider);
    }
  }
  return providers;
}

export async function saveSettings(settings: AppSettings): Promise<AppSettings> {
  if (isTauri) {
    return tauriInvoke("save_settings", { settings });
  }
  writeLocal("settings", settings);
  return settings;
}

export async function testApiProvider(provider: ApiProvider): Promise<ProviderTestResult> {
  if (isTauri) {
    return tauriInvoke("test_api_provider", { provider });
  }
  try {
    const translatedText = provider.providerType === "libretranslate"
      ? await translateWithLibre("hello", "en", "zh", provider)
      : provider.providerType === "openai"
        ? await translateWithOpenAi("hello", "en", "zh", provider)
        : (await fallbackTranslate("hello", "zh")).translatedText;
    return { ok: true, message: `${provider.name} 返回正常`, translatedText };
  } catch (error) {
    return { ok: false, message: error instanceof Error ? error.message : "连接测试失败" };
  }
}

export async function captureAndTranslate(): Promise<TranslationResult> {
  if (isTauri) {
    return tauriInvoke("capture_and_translate");
  }
  throw new Error("截图翻译需要在 Tauri 桌面端运行。");
}

export async function captureScreenshot(): Promise<ScreenshotCapture> {
  if (isTauri) {
    return tauriInvoke("capture_screenshot");
  }
  throw new Error("截图翻译需要在 Tauri 桌面端运行。");
}

export async function exitScreenshotMode(): Promise<void> {
  if (isTauri) {
    return tauriInvoke("exit_screenshot_mode");
  }
}

export async function translateScreenshotRegion(imageDataUrl: string, region: ScreenshotRegion): Promise<TranslationResult> {
  if (isTauri) {
    return tauriInvoke("translate_screenshot_region", { imageDataUrl, region });
  }
  throw new Error("截图翻译需要在 Tauri 桌面端运行。");
}
