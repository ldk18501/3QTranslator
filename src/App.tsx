import { useEffect, useMemo, useState } from "react";
import { BookOpen, Camera, Check, GraduationCap, Languages, Plus, RotateCw, Search, Settings, Sparkles } from "lucide-react";
import clsx from "clsx";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { addToWordbook, captureAndTranslate, getDailyItems, getSettings, listWordbook, saveSettings, translateText } from "./lib/api";
import { defaultTargetFor, languageLabel, languageOptions } from "./lib/language";
import type { AppSettings, DailyItem, TranslationResult, ViewKey } from "./lib/types";

const navItems: Array<{ key: ViewKey; label: string; icon: typeof Languages }> = [
  { key: "translate", label: "翻译", icon: Languages },
  { key: "wordbook", label: "单词本", icon: BookOpen },
  { key: "daily", label: "每日学习", icon: GraduationCap },
  { key: "settings", label: "设置", icon: Settings },
];

function App() {
  const [view, setView] = useState<ViewKey>("translate");
  const settingsQuery = useQuery({ queryKey: ["settings"], queryFn: getSettings });

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
        {view === "translate" && <TranslateView settings={settingsQuery.data} />}
        {view === "wordbook" && <WordbookView />}
        {view === "daily" && <DailyView settings={settingsQuery.data} />}
        {view === "settings" && <SettingsView settings={settingsQuery.data} />}
      </section>
    </main>
  );
}

function TranslateView({ settings }: { settings?: AppSettings }) {
  const queryClient = useQueryClient();
  const [text, setText] = useState("salt");
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
    if (text.trim()) translateMutation.mutate();
    // Run once as a useful first screen.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
  const wordbookQuery = useQuery({ queryKey: ["wordbook"], queryFn: listWordbook });
  const entries = wordbookQuery.data ?? [];
  const filtered = entries.filter((item) => `${item.text} ${item.translation} ${item.language}`.toLowerCase().includes(filter.toLowerCase()));
  const groups = useMemo(() => {
    return filtered.reduce<Record<string, typeof filtered>>((acc, item) => {
      acc[item.language] = acc[item.language] ?? [];
      acc[item.language].push(item);
      return acc;
    }, {});
  }, [filtered]);

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>单词本</h1>
          <p>按语言自动分组，保存释义、翻译和例句。</p>
        </div>
        <div className="search-box">
          <Search size={16} />
          <input value={filter} onChange={(event) => setFilter(event.target.value)} placeholder="搜索单词本" />
        </div>
      </header>

      {Object.keys(groups).length === 0 ? (
        <div className="empty-state">
          <BookOpen size={36} />
          <p>还没有收藏内容。</p>
        </div>
      ) : (
        <div className="group-list">
          {Object.entries(groups).map(([language, items]) => (
            <section key={language} className="word-group">
              <h2>{languageLabel(language)} · {items.length}</h2>
              <div className="word-grid">
                {items.map((item) => (
                  <article className="word-card" key={item.id}>
                    <div className="card-title">
                      <strong>{item.text}</strong>
                      <span>{new Date(item.createdAt).toLocaleDateString()}</span>
                    </div>
                    <p>{item.translation}</p>
                    {item.examples[0] && <em>{item.examples[0]}</em>}
                  </article>
                ))}
              </div>
            </section>
          ))}
        </div>
      )}
    </div>
  );
}

function DailyView({ settings }: { settings?: AppSettings }) {
  const queryClient = useQueryClient();
  const [language, setLanguage] = useState(settings?.dailyLanguage ?? "en");
  const [level, setLevel] = useState<AppSettings["dailyLevel"]>(settings?.dailyLevel ?? "beginner");

  useEffect(() => {
    if (settings) {
      setLanguage(settings.dailyLanguage);
      setLevel(settings.dailyLevel);
    }
  }, [settings]);

  const dailyQuery = useQuery({
    queryKey: ["daily", language, level],
    queryFn: () => getDailyItems(language, level),
  });

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
          <select value={language} onChange={(event) => setLanguage(event.target.value)}>
            {languageOptions.map((item) => <option key={item.code} value={item.code}>{item.label}</option>)}
          </select>
          <select value={level} onChange={(event) => setLevel(event.target.value as AppSettings["dailyLevel"])}>
            <option value="zero">完全不会</option>
            <option value="beginner">入门</option>
            <option value="skilled">熟练</option>
            <option value="advanced">精通</option>
          </select>
          <button className="icon-button" onClick={() => dailyQuery.refetch()} title="刷新">
            <RotateCw size={18} />
          </button>
        </div>
      </header>

      <div className="daily-grid">
        {(dailyQuery.data ?? []).map((item) => (
          <article className="daily-card" key={item.id}>
            <div className="card-title">
              <strong>{item.word}</strong>
              <span>{item.translation}</span>
            </div>
            <ul className="example-list">
              {item.examples.map((example) => <li key={example}>{example}</li>)}
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
          呼出翻译窗口
          <input value={draft.shortcutTranslate} onChange={(event) => set("shortcutTranslate", event.target.value)} />
        </label>
        <label>
          截图翻译
          <input value={draft.shortcutScreenshot} onChange={(event) => set("shortcutScreenshot", event.target.value)} />
        </label>
        <label>
          LibreTranslate 地址
          <input value={draft.libreTranslateUrl} onChange={(event) => set("libreTranslateUrl", event.target.value)} placeholder="https://libretranslate.example.com" />
        </label>
        <label>
          OpenAI-compatible Base URL
          <input value={draft.openAiBaseUrl} onChange={(event) => set("openAiBaseUrl", event.target.value)} placeholder="https://api.example.com/v1" />
        </label>
        <label className="wide-field">
          OpenAI-compatible API Key
          <input type="password" value={draft.openAiApiKey} onChange={(event) => set("openAiApiKey", event.target.value)} />
        </label>
      </section>

      <div className="settings-note">
        <Sparkles size={18} />
        <span>免费翻译源会优先使用，配置高级源后可在后续版本切换为更稳定的质量优先模式。</span>
      </div>
    </div>
  );
}

export default App;
