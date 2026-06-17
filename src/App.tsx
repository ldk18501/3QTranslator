import { useEffect, useMemo, useState } from "react";
import {
  BookOpen,
  Camera,
  Check,
  GraduationCap,
  Languages,
  Plus,
  RotateCw,
  Search,
  Settings,
  Trash2,
} from "lucide-react";
import clsx from "clsx";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { addToWordbook, captureAndTranslate, deleteWordbookEntry, getDailyItems, getSettings, listWordbook, saveSettings, translateText } from "./lib/api";
import { defaultTargetFor, languageLabel, languageOptions } from "./lib/language";
import type { ApiProvider, AppSettings, DailyItem, Level, TranslationResult, ViewKey } from "./lib/types";

const navItems: Array<{ key: ViewKey; label: string; icon: typeof Languages }> = [
  { key: "translate", label: "翻译", icon: Languages },
  { key: "wordbook", label: "单词本", icon: BookOpen },
  { key: "daily", label: "每日学习", icon: GraduationCap },
  { key: "settings", label: "设置", icon: Settings },
];

const levelOptions: Array<{ value: Level; label: string }> = [
  { value: "zero", label: "完全不会" },
  { value: "beginner", label: "入门" },
  { value: "skilled", label: "熟练" },
  { value: "advanced", label: "精通" },
];

const levelOrder: Record<Level, number> = {
  zero: 0,
  beginner: 1,
  skilled: 2,
  advanced: 3,
};

function levelLabel(level: Level): string {
  return levelOptions.find((item) => item.value === level)?.label ?? level;
}

function App() {
  const [view, setView] = useState<ViewKey>("translate");
  const [screenshotRequest, setScreenshotRequest] = useState(0);
  const settingsQuery = useQuery({ queryKey: ["settings"], queryFn: getSettings });

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;

    const unlisteners: Array<() => void> = [];
    void import("@tauri-apps/api/event").then(async ({ listen }) => {
      unlisteners.push(
        await listen("3q-open-translate", () => {
          setView("translate");
        }),
      );
      unlisteners.push(
        await listen("3q-screenshot-translate", () => {
          setView("translate");
          setScreenshotRequest((value) => value + 1);
        }),
      );
    });

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">3Q</div>
          <div>
            <strong>3Q语言助手</strong>
            <span>免费翻译与学习</span>
          </div>
        </div>
        <nav className="nav-list">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button key={item.key} className={clsx("nav-button", view === item.key && "active")} onClick={() => setView(item.key)}>
                <Icon size={18} />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
      </aside>

      <section className="workspace">
        {view === "translate" && <TranslateView settings={settingsQuery.data} screenshotRequest={screenshotRequest} />}
        {view === "wordbook" && <WordbookView />}
        {view === "daily" && <DailyView settings={settingsQuery.data} />}
        {view === "settings" && <SettingsView settings={settingsQuery.data} />}
      </section>
    </main>
  );
}

function TranslateView({ settings, screenshotRequest }: { settings?: AppSettings; screenshotRequest: number }) {
  const queryClient = useQueryClient();
  const [text, setText] = useState("");
  const [targetLanguage, setTargetLanguage] = useState("");
  const [result, setResult] = useState<TranslationResult | null>(null);
  const [notice, setNotice] = useState("");

  const translateMutation = useMutation({
    mutationFn: () => translateText(text.trim(), targetLanguage || undefined),
    onSuccess: setResult,
  });

  const addMutation = useMutation({
    mutationFn: (item: TranslationResult) => addToWordbook(item),
    onSuccess: () => {
      setNotice("已加入单词本");
      queryClient.invalidateQueries({ queryKey: ["wordbook"] });
    },
  });

  const captureMutation = useMutation({
    mutationFn: captureAndTranslate,
    onSuccess: (value) => {
      setResult(value);
      setText(value.sourceText);
    },
    onError: (error) => setNotice(error instanceof Error ? error.message : "截图翻译暂不可用"),
  });

  useEffect(() => {
    if (screenshotRequest > 0) captureMutation.mutate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [screenshotRequest]);

  const detectedTarget = result
    ? result.targetLanguage
    : defaultTargetFor("en", settings?.defaultEnglishTarget, settings?.defaultOtherTarget);

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>翻译</h1>
          <p>查单词、翻译长文本，结果可以直接加入单词本。</p>
        </div>
        <div className="header-actions">
          <select value={targetLanguage || detectedTarget} onChange={(event) => setTargetLanguage(event.target.value)}>
            {languageOptions.map((item) => (
              <option key={item.code} value={item.code}>
                译为{item.label}
              </option>
            ))}
          </select>
          <button className="icon-button" onClick={() => captureMutation.mutate()} title="截图翻译">
            <Camera size={18} />
          </button>
        </div>
      </header>

      <section className="translator-layout">
        <div className="input-panel">
          <textarea value={text} onChange={(event) => setText(event.target.value)} placeholder="输入单词、句子或长文本" />
          <div className="panel-footer">
            <span>{text.length} 字符</span>
            <button className="primary-button" disabled={!text.trim() || translateMutation.isPending} onClick={() => translateMutation.mutate()}>
              <Search size={16} />
              {translateMutation.isPending ? "翻译中" : "翻译"}
            </button>
          </div>
        </div>

        <div className="result-panel">
          {result ? (
            <TranslationCard result={result} onAdd={() => addMutation.mutate(result)} adding={addMutation.isPending} />
          ) : (
            <div className="empty-state">
              <Languages size={36} />
              <p>输入内容后开始翻译。</p>
            </div>
          )}
          {notice && <div className="toast">{notice}</div>}
          {translateMutation.error && <div className="error-line">{String(translateMutation.error)}</div>}
        </div>
      </section>
    </div>
  );
}

function TranslationCard({ result, onAdd, adding }: { result: TranslationResult; onAdd: () => void; adding: boolean }) {
  return (
    <article className="translation-card">
      <div className="result-meta">
        <span>{languageLabel(result.sourceLanguage)} → {languageLabel(result.targetLanguage)}</span>
        <span>{result.provider}</span>
      </div>
      <h2>{result.sourceText}</h2>
      {result.phonetic && <div className="phonetic">{result.phonetic}</div>}
      <p className="translated-text">{result.translatedText}</p>

      {result.definitions.length > 0 && (
        <section className="result-section">
          <h3>词典解释</h3>
          {result.definitions.map((item, index) => (
            <div className="definition-row" key={`${item.partOfSpeech}-${index}`}>
              <span>{item.partOfSpeech}</span>
              <p>{item.meaning}</p>
              {item.example && <em>{item.example}</em>}
            </div>
          ))}
        </section>
      )}

      {result.examples.length > 0 && (
        <section className="result-section">
          <h3>例句</h3>
          <ul className="example-list">
            {result.examples.slice(0, 4).map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </section>
      )}

      {result.phrases.length > 0 && (
        <section className="result-section">
          <h3>相关词</h3>
          <div className="chips">
            {result.phrases.map((item) => <span key={item}>{item}</span>)}
          </div>
        </section>
      )}

      <button className="secondary-button" onClick={onAdd} disabled={adding}>
        <Plus size={16} />
        {adding ? "保存中" : "加入单词本"}
      </button>
    </article>
  );
}

function WordbookView() {
  const [filter, setFilter] = useState("");
  const [selectedLanguage, setSelectedLanguage] = useState("");
  const [selectedLevel, setSelectedLevel] = useState<Level | "all">("all");
  const [sortMode, setSortMode] = useState<"created" | "alpha" | "level">("created");
  const queryClient = useQueryClient();
  const wordbookQuery = useQuery({ queryKey: ["wordbook"], queryFn: listWordbook });
  const entries = wordbookQuery.data ?? [];
  const languages = useMemo(() => Array.from(new Set(entries.map((item) => item.language))).sort(), [entries]);

  useEffect(() => {
    if (!selectedLanguage && languages[0]) {
      setSelectedLanguage(languages[0]);
    } else if (selectedLanguage && languages.length > 0 && !languages.includes(selectedLanguage)) {
      setSelectedLanguage(languages[0]);
    }
  }, [languages, selectedLanguage]);

  const deleteMutation = useMutation({
    mutationFn: (id: string) => deleteWordbookEntry(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["wordbook"] }),
  });

  const filtered = useMemo(() => {
    const keyword = filter.trim().toLowerCase();
    return entries
      .filter((item) => !selectedLanguage || item.language === selectedLanguage)
      .filter((item) => selectedLevel === "all" || item.level === selectedLevel)
      .filter((item) => `${item.text} ${item.translation} ${item.language}`.toLowerCase().includes(keyword))
      .sort((a, b) => {
        if (sortMode === "alpha") return a.text.localeCompare(b.text);
        if (sortMode === "level") return levelOrder[a.level] - levelOrder[b.level] || a.text.localeCompare(b.text);
        return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime();
      });
  }, [entries, filter, selectedLanguage, selectedLevel, sortMode]);

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>单词本</h1>
          <p>按语言和难度整理，保存释义、翻译和例句。</p>
        </div>
        <div className="wordbook-tools">
          <div className="search-box">
            <Search size={16} />
            <input value={filter} onChange={(event) => setFilter(event.target.value)} placeholder="搜索单词本" />
          </div>
          <select value={sortMode} onChange={(event) => setSortMode(event.target.value as typeof sortMode)}>
            <option value="created">按加入时间</option>
            <option value="alpha">按首字母 A-Z</option>
            <option value="level">按难易度</option>
          </select>
        </div>
      </header>

      {entries.length > 0 && (
        <div className="filter-stack">
          <div className="segmented-row">
            {languages.map((language) => (
              <button
                key={language}
                className={clsx("segment-button", selectedLanguage === language && "active")}
                onClick={() => setSelectedLanguage(language)}
              >
                {languageLabel(language)}
                <span>{entries.filter((item) => item.language === language).length}</span>
              </button>
            ))}
          </div>
          <div className="segmented-row">
            <button className={clsx("segment-button", selectedLevel === "all" && "active")} onClick={() => setSelectedLevel("all")}>
              全部难度
            </button>
            {levelOptions.map((item) => (
              <button
                key={item.value}
                className={clsx("segment-button", selectedLevel === item.value && "active")}
                onClick={() => setSelectedLevel(item.value)}
              >
                {item.label}
              </button>
            ))}
          </div>
        </div>
      )}

      {entries.length === 0 ? (
        <div className="empty-state">
          <BookOpen size={36} />
          <p>还没有收藏内容。</p>
        </div>
      ) : filtered.length === 0 ? (
        <div className="empty-state">
          <Search size={36} />
          <p>当前筛选下没有匹配的单词。</p>
        </div>
      ) : (
        <section className="word-group">
          <h2>{languageLabel(selectedLanguage)} · {filtered.length}</h2>
          <div className="word-grid">
            {filtered.map((item) => (
              <article className="word-card" key={item.id}>
                <div className="card-title">
                  <strong>{item.text}</strong>
                  <span>{new Date(item.createdAt).toLocaleDateString()}</span>
                </div>
                <div className="word-card-meta">
                  <span>{levelLabel(item.level)}</span>
                  <span>{item.source}</span>
                </div>
                <p>{item.translation}</p>
                {item.examples[0] && <em>{item.examples[0]}</em>}
                <button
                  className="danger-button"
                  disabled={deleteMutation.isPending}
                  onClick={() => deleteMutation.mutate(item.id)}
                >
                  <Trash2 size={15} />
                  删除
                </button>
              </article>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function DailyView({ settings }: { settings?: AppSettings }) {
  const queryClient = useQueryClient();
  const [language, setLanguage] = useState(settings?.dailyLanguage ?? "en");
  const [level, setLevel] = useState<AppSettings["dailyLevel"]>(settings?.dailyLevel ?? "beginner");
  const [forceRefresh, setForceRefresh] = useState(0);

  useEffect(() => {
    if (settings) {
      setLanguage(settings.dailyLanguage);
      setLevel(settings.dailyLevel);
    }
  }, [settings]);

  const dailyQuery = useQuery({
    queryKey: ["daily", language, level, forceRefresh],
    queryFn: () => getDailyItems(language, level, forceRefresh > 0),
  });

  const settingsMutation = useMutation({
    mutationFn: (value: AppSettings) => saveSettings(value),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["settings"] }),
  });

  const changeLanguage = (value: string) => {
    setLanguage(value);
    if (settings) settingsMutation.mutate({ ...settings, dailyLanguage: value, dailyLevel: level });
  };

  const changeLevel = (value: AppSettings["dailyLevel"]) => {
    setLevel(value);
    if (settings) settingsMutation.mutate({ ...settings, dailyLanguage: language, dailyLevel: value });
  };

  const addMutation = useMutation({
    mutationFn: (item: DailyItem) => addToWordbook(item),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["wordbook"] }),
  });

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>每日学习</h1>
          <p>每天 5 组单词与例句，适合打开软件后先热身。</p>
        </div>
        <div className="header-actions">
          <select value={language} onChange={(event) => changeLanguage(event.target.value)}>
            {languageOptions.map((item) => <option key={item.code} value={item.code}>{item.label}</option>)}
          </select>
          <select value={level} onChange={(event) => changeLevel(event.target.value as AppSettings["dailyLevel"])}>
            {levelOptions.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
          </select>
          <button className="icon-button" onClick={() => setForceRefresh((value) => value + 1)} title="刷新">
            <RotateCw size={18} />
          </button>
        </div>
      </header>

      {dailyQuery.isFetching && <div className="inline-status">正在生成今日学习内容，首次生成会稍慢...</div>}

      <div className="daily-grid">
        {(dailyQuery.data ?? []).map((item) => (
          <article className="daily-card" key={item.id}>
            <div className="card-title">
              <strong>{item.word}</strong>
              <span>{item.translation}</span>
            </div>
            <ul className="example-list daily-examples">
              {item.examples.map((example, index) => (
                <li key={example}>
                  <span>{example}</span>
                  {item.exampleTranslations[index] && <em>{item.exampleTranslations[index]}</em>}
                </li>
              ))}
            </ul>
            <button className="secondary-button" onClick={() => addMutation.mutate(item)}>
              <Plus size={16} />
              加入单词本
            </button>
          </article>
        ))}
      </div>
    </div>
  );
}

function SettingsView({ settings }: { settings?: AppSettings }) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<AppSettings | null>(settings ?? null);

  useEffect(() => {
    if (settings) setDraft(settings);
  }, [settings]);

  const saveMutation = useMutation({
    mutationFn: (value: AppSettings) => saveSettings(value),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["settings"] }),
  });

  if (!draft) return <div className="empty-state">加载设置中</div>;

  const set = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => setDraft({ ...draft, [key]: value });
  const updateProvider = <K extends keyof ApiProvider>(id: string, key: K, value: ApiProvider[K]) => {
    set("apiProviders", draft.apiProviders.map((provider) => (provider.id === id ? { ...provider, [key]: value } : provider)));
  };
  const addProvider = () => {
    const id = `provider-${Date.now()}`;
    setDraft({
      ...draft,
      activeProviderId: id,
      apiProviders: [
        ...draft.apiProviders,
        {
          id,
          name: "新的翻译源",
          providerType: "openai",
          enabled: true,
          baseUrl: "",
          apiKey: "",
          model: "gpt-4o-mini",
        },
      ],
    });
  };
  const removeProvider = (id: string) => {
    const providers = draft.apiProviders.filter((provider) => provider.id !== id || provider.id === "mymemory");
    setDraft({
      ...draft,
      apiProviders: providers,
      activeProviderId: draft.activeProviderId === id ? providers[0]?.id ?? "mymemory" : draft.activeProviderId,
    });
  };

  return (
    <div className="page settings-page">
      <header className="page-header">
        <div>
          <h1>设置</h1>
          <p>调整默认语言、快捷键和可选高级翻译源。</p>
        </div>
        <button className="primary-button" onClick={() => saveMutation.mutate(draft)}>
          <Check size={16} />
          保存
        </button>
      </header>

      <section className="settings-grid">
        <label>
          英语默认译为
          <select value={draft.defaultEnglishTarget} onChange={(event) => set("defaultEnglishTarget", event.target.value)}>
            {languageOptions.map((item) => <option key={item.code} value={item.code}>{item.label}</option>)}
          </select>
        </label>
        <label>
          非英语默认译为
          <select value={draft.defaultOtherTarget} onChange={(event) => set("defaultOtherTarget", event.target.value)}>
            {languageOptions.map((item) => <option key={item.code} value={item.code}>{item.label}</option>)}
          </select>
        </label>
        <label>
          每日学习缓存上限
          <input
            type="number"
            min={20}
            max={1000}
            value={draft.dailyCacheLimit}
            onChange={(event) => set("dailyCacheLimit", Number(event.target.value) || 120)}
          />
        </label>
        <label>
          呼出翻译窗口
          <input value={draft.shortcutTranslate} onChange={(event) => set("shortcutTranslate", event.target.value)} />
        </label>
        <label>
          截图翻译
          <input value={draft.shortcutScreenshot} onChange={(event) => set("shortcutScreenshot", event.target.value)} />
        </label>
        <label className="toggle-field wide-field">
          <span>
            <strong>关闭按钮最小化到通知区域</strong>
            <em>开启后点击关闭会隐藏到系统托盘，可从托盘菜单退出。</em>
          </span>
          <input
            type="checkbox"
            checked={draft.closeToTray}
            onChange={(event) => set("closeToTray", event.target.checked)}
          />
        </label>
        <label className="wide-field">
          当前使用翻译源
          <select value={draft.activeProviderId} onChange={(event) => set("activeProviderId", event.target.value)}>
            {draft.apiProviders.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.enabled ? "" : "已停用 · "}{provider.name}
              </option>
            ))}
          </select>
        </label>
      </section>

      <section className="api-section">
        <div className="section-title-row">
          <div>
            <h2>翻译源配置</h2>
            <p>高级源会优先使用；调用失败时自动回落到 MyMemory 免费源。</p>
          </div>
          <button className="secondary-button" onClick={addProvider}>
            <Plus size={16} />
            新增 API
          </button>
        </div>

        <div className="provider-list">
          {draft.apiProviders.map((provider) => (
            <article className="provider-card" key={provider.id}>
              <div className="provider-card-header">
                <label className="toggle-inline">
                  <input
                    type="checkbox"
                    checked={provider.enabled}
                    onChange={(event) => updateProvider(provider.id, "enabled", event.target.checked)}
                  />
                  启用
                </label>
                <button
                  className="icon-button danger-icon"
                  disabled={provider.id === "mymemory"}
                  onClick={() => removeProvider(provider.id)}
                  title="删除翻译源"
                >
                  <Trash2 size={16} />
                </button>
              </div>
              <div className="provider-fields">
                <label>
                  名称
                  <input value={provider.name} onChange={(event) => updateProvider(provider.id, "name", event.target.value)} />
                </label>
                <label>
                  类型
                  <select
                    value={provider.providerType}
                    disabled={provider.id === "mymemory"}
                    onChange={(event) => updateProvider(provider.id, "providerType", event.target.value as ApiProvider["providerType"])}
                  >
                    <option value="mymemory">MyMemory 免费源</option>
                    <option value="libretranslate">LibreTranslate</option>
                    <option value="openai">OpenAI-compatible</option>
                  </select>
                </label>
                {provider.providerType !== "mymemory" && (
                  <label className="wide-field">
                    Base URL
                    <input
                      value={provider.baseUrl}
                      onChange={(event) => updateProvider(provider.id, "baseUrl", event.target.value)}
                      placeholder={provider.providerType === "libretranslate" ? "https://libretranslate.example.com" : "https://api.example.com/v1"}
                    />
                  </label>
                )}
                {provider.providerType !== "mymemory" && (
                  <label>
                    API Key
                    <input type="password" value={provider.apiKey} onChange={(event) => updateProvider(provider.id, "apiKey", event.target.value)} />
                  </label>
                )}
                {provider.providerType === "openai" && (
                  <label>
                    模型
                    <input value={provider.model} onChange={(event) => updateProvider(provider.id, "model", event.target.value)} placeholder="gpt-4o-mini" />
                  </label>
                )}
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}

export default App;
