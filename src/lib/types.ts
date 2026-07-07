export type ViewKey = "translate" | "wordbook" | "settings";

export type Level = "zero" | "beginner" | "skilled" | "advanced";
export type ProviderType = "mymemory" | "libretranslate" | "openai";

export interface Definition {
  partOfSpeech: string;
  meaning: string;
  example?: string;
  synonyms?: string[];
}

export interface TranslationResult {
  sourceText: string;
  sourceLanguage: string;
  targetLanguage: string;
  translatedText: string;
  phonetic?: string;
  definitions: Definition[];
  examples: string[];
  phrases: string[];
  provider: string;
  isWord: boolean;
}

export interface WordbookEntry {
  id: string;
  text: string;
  language: string;
  targetLanguage: string;
  translation: string;
  definitions: Definition[];
  examples: string[];
  level: Level;
  source: string;
  createdAt: string;
}

export interface ProviderTestResult {
  ok: boolean;
  message: string;
  translatedText?: string;
}

export interface ScreenshotCapture {
  imageDataUrl: string;
  width: number;
  height: number;
}

export interface ScreenshotRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface DailyItem {
  id: string;
  word: string;
  language: string;
  translation: string;
  examples: string[];
  exampleTranslations: string[];
  level: Level;
}

export interface ApiProvider {
  id: string;
  name: string;
  providerType: ProviderType;
  enabled: boolean;
  baseUrl: string;
  apiKey: string;
  model: string;
}

export interface AppSettings {
  defaultEnglishTarget: string;
  defaultOtherTarget: string;
  dailyLanguage: string;
  dailyLevel: Level;
  dailyCacheLimit: number;
  shortcutTranslate: string;
  shortcutScreenshot: string;
  closeToTray: boolean;
  launchAtStartup: boolean;
  activeProviderId: string;
  apiProviders: ApiProvider[];
  libreTranslateUrl: string;
  openAiBaseUrl: string;
  openAiApiKey: string;
}
