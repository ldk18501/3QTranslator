import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, MouseEvent as ReactMouseEvent } from "react";
import {
  BookOpen,
  Camera,
  Check,
  Clipboard,
  Copy,
  ExternalLink,
  Languages,
  Plus,
  RefreshCw,
  Search,
  Settings,
  Trash2,
  X,
} from "lucide-react";
import clsx from "clsx";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  addToWordbook,
  captureScreenshot,
  deleteWordbookEntry,
  exitScreenshotMode,
  getSettings,
  listWordbook,
  saveSettings,
  testApiProvider,
  translateText,
  translateScreenshotRegion,
  updateWordbookEntryLevel,
} from "./lib/api";
import { defaultTargetFor, languageLabel, languageOptions } from "./lib/language";
import type { ApiProvider, AppSettings, Level, ScreenshotCapture, ScreenshotRegion, TranslationResult, ViewKey, WordbookEntry } from "./lib/types";

const navItems: Array<{ key: ViewKey; label: string; icon: typeof Languages }> = [
  { key: "translate", label: "翻译", icon: Languages },
  { key: "wordbook", label: "单词本", icon: BookOpen },
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

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

function shortcutFromKeyboardEvent(event: ReactKeyboardEvent<HTMLInputElement>): string | null {
  const key = event.key;
  if (["Control", "Shift", "Alt", "Meta"].includes(key)) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Meta");
  const normalizedKey = key.length === 1 ? key.toUpperCase() : key.replace(/^Arrow/, "");
  parts.push(normalizedKey);
  return parts.length >= 2 ? parts.join("+") : null;
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
        <section hidden={view !== "translate"}>
          <TranslateView settings={settingsQuery.data} screenshotRequest={screenshotRequest} />
        </section>
        <section hidden={view !== "wordbook"}>
          <WordbookView onGoTranslate={() => setView("translate")} />
        </section>
        <section hidden={view !== "settings"}>
          <SettingsView settings={settingsQuery.data} />
        </section>
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
  const [screenshot, setScreenshot] = useState<ScreenshotCapture | null>(null);
  const translateRequestIdRef = useRef(0);

  const translateMutation = useMutation({
    mutationFn: ({ requestId }: { requestId: number }) => translateText(text.trim(), targetLanguage || undefined).then((value) => ({ value, requestId })),
    onSuccess: ({ value, requestId }) => {
      if (requestId !== translateRequestIdRef.current) return;
      setNotice("");
      setResult(value);
    },
    onError: (error, variables) => {
      if (variables.requestId !== translateRequestIdRef.current) return;
      setNotice(errorMessage(error));
    },
  });

  const addMutation = useMutation({
    mutationFn: (item: TranslationResult) => addToWordbook(item),
    onSuccess: () => {
      setNotice("已加入单词本");
      queryClient.invalidateQueries({ queryKey: ["wordbook"] });
    },
  });

  const captureMutation = useMutation({
    mutationFn: captureScreenshot,
    onSuccess: (value) => {
      setNotice("");
      setScreenshot(value);
    },
    onError: (error) => setNotice(errorMessage(error) || "截图翻译暂不可用"),
  });

  const regionMutation = useMutation({
    mutationFn: async ({ imageDataUrl, region }: { imageDataUrl: string; region: ScreenshotRegion }) => {
      await exitScreenshotMode();
      return translateScreenshotRegion(imageDataUrl, region);
    },
    onSuccess: (value) => {
      setResult(value);
      setText(value.sourceText);
      setScreenshot(null);
      setNotice("");
    },
    onError: (error) => setNotice(errorMessage(error) || "选区识别失败"),
  });

  useEffect(() => {
    if (screenshotRequest > 0) captureMutation.mutate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [screenshotRequest]);

  const detectedTarget = result
    ? result.targetLanguage
    : defaultTargetFor("en", settings?.defaultEnglishTarget, settings?.defaultOtherTarget);
  const startTranslate = () => {
    const requestId = translateRequestIdRef.current + 1;
    translateRequestIdRef.current = requestId;
    translateMutation.mutate({ requestId });
  };
  const cancelTranslate = () => {
    translateRequestIdRef.current += 1;
    translateMutation.reset();
    setNotice("已取消本次翻译");
  };

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
            {captureMutation.isPending ? <RefreshCw size={18} /> : <Camera size={18} />}
          </button>
        </div>
      </header>

      <section className="translator-layout">
        <div className="input-panel">
          <textarea value={text} onChange={(event) => setText(event.target.value)} placeholder="输入单词、句子或长文本" />
          <div className="panel-footer">
            <span>{text.length} 字符</span>
            {translateMutation.isPending ? (
              <button className="secondary-button" onClick={cancelTranslate}>
                <X size={16} />
                取消
              </button>
            ) : (
              <button className="primary-button" disabled={!text.trim()} onClick={startTranslate}>
                <Search size={16} />
                翻译
              </button>
            )}
          </div>
        </div>

        <div className="result-panel">
          {result ? (
            <TranslationCard result={result} onAdd={() => addMutation.mutate(result)} adding={addMutation.isPending} onNotice={setNotice} />
          ) : (
            <div className="empty-state">
              <Languages size={36} />
              <p>输入内容后开始翻译。</p>
            </div>
          )}
          {notice && <div className="toast">{notice}</div>}
          {translateMutation.error && <div className="error-line">{errorMessage(translateMutation.error)}</div>}
        </div>
      </section>

      {screenshot && (
        <ScreenshotSelector
          screenshot={screenshot}
          pending={regionMutation.isPending}
          onClose={() => {
            setScreenshot(null);
            void exitScreenshotMode();
          }}
          onRetry={() => captureMutation.mutate()}
          onSubmit={(region) => {
            const imageDataUrl = screenshot.imageDataUrl;
            setScreenshot(null);
            regionMutation.mutate({ imageDataUrl, region });
          }}
        />
      )}
    </div>
  );
}

function ScreenshotSelector({
  screenshot,
  pending,
  onClose,
  onRetry,
  onSubmit,
}: {
  screenshot: ScreenshotCapture;
  pending: boolean;
  onClose: () => void;
  onRetry: () => void;
  onSubmit: (region: ScreenshotRegion) => void;
}) {
  const imageRef = useRef<HTMLImageElement | null>(null);
  const [selection, setSelection] = useState<ScreenshotRegion | null>(null);
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);

  useEffect(() => {
    setSelection(null);
    setDragStart(null);
  }, [screenshot.imageDataUrl]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const pointFromEvent = (event: ReactMouseEvent): { x: number; y: number } => {
    const rect = imageRef.current?.getBoundingClientRect();
    if (!rect) return { x: 0, y: 0 };
    const x = Math.min(Math.max(event.clientX - rect.left, 0), rect.width);
    const y = Math.min(Math.max(event.clientY - rect.top, 0), rect.height);
    return {
      x: (x / rect.width) * screenshot.width,
      y: (y / rect.height) * screenshot.height,
    };
  };

  const beginSelection = (event: ReactMouseEvent) => {
    const point = pointFromEvent(event);
    setDragStart(point);
    setSelection({ x: point.x, y: point.y, width: 0, height: 0 });
  };

  const updateSelection = (event: ReactMouseEvent) => {
    if (!dragStart) return;
    const point = pointFromEvent(event);
    setSelection({
      x: Math.min(dragStart.x, point.x),
      y: Math.min(dragStart.y, point.y),
      width: Math.abs(point.x - dragStart.x),
      height: Math.abs(point.y - dragStart.y),
    });
  };

  const finishSelection = () => {
    setDragStart(null);
  };

  const selectionStyle = selection
    ? {
        left: `${(selection.x / screenshot.width) * 100}%`,
        top: `${(selection.y / screenshot.height) * 100}%`,
        width: `${(selection.width / screenshot.width) * 100}%`,
        height: `${(selection.height / screenshot.height) * 100}%`,
      }
    : undefined;
  const canSubmit = Boolean(selection && selection.width >= 8 && selection.height >= 8);

  return (
    <div className="modal-backdrop">
      <section className="screenshot-modal">
        <header className="modal-header">
          <div>
            <h2>框选截图翻译区域</h2>
            <p>拖动鼠标选择需要识别的文字区域，选错后直接重新框选。</p>
          </div>
          <button className="icon-button" onClick={onClose} title="关闭">
            <X size={18} />
          </button>
        </header>
        <div
          className="screenshot-stage"
          onMouseDown={beginSelection}
          onMouseMove={updateSelection}
          onMouseUp={finishSelection}
          onMouseLeave={finishSelection}
        >
          <img ref={imageRef} src={screenshot.imageDataUrl} alt="截图预览" draggable={false} />
          {selectionStyle && <div className="selection-rect" style={selectionStyle} />}
        </div>
        <footer className="modal-footer">
          <span>{selection ? `选区 ${Math.round(selection.width)} × ${Math.round(selection.height)}` : "尚未选择区域"}</span>
          <div className="action-row">
            <button className="secondary-button" onClick={onRetry}>
              <Camera size={16} />
              重新截图
            </button>
            <button className="primary-button" disabled={!canSubmit || pending} onClick={() => selection && onSubmit(selection)}>
              <Search size={16} />
              {pending ? "识别中" : "识别并翻译"}
            </button>
          </div>
        </footer>
      </section>
    </div>
  );
}

function TranslationCard({
  result,
  onAdd,
  adding,
  onNotice,
}: {
  result: TranslationResult;
  onAdd: () => void;
  adding: boolean;
  onNotice: (message: string) => void;
}) {
  const copyTranslation = async () => {
    try {
      await navigator.clipboard.writeText(result.translatedText);
      onNotice("译文已复制");
    } catch {
      onNotice("复制失败，请手动选择译文复制");
    }
  };

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
              <p>
                {item.meaning}
                {item.meaningTranslation && <small>{item.meaningTranslation}</small>}
              </p>
              {item.example && (
                <em>
                  {item.example}
                  {item.exampleTranslation && <small>{item.exampleTranslation}</small>}
                </em>
              )}
            </div>
          ))}
        </section>
      )}

      {result.examples.length > 0 && (
        <section className="result-section">
          <h3>例句</h3>
          <ul className="example-list">
            {result.examples.slice(0, 4).map((item, index) => (
              <li key={item}>
                <span>{item}</span>
                {result.exampleTranslations[index] && <em>{result.exampleTranslations[index]}</em>}
              </li>
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

      <div className="action-row">
        <button className="secondary-button" onClick={copyTranslation}>
          <Copy size={16} />
          复制译文
        </button>
        <button className="secondary-button" onClick={onAdd} disabled={adding}>
          <Plus size={16} />
          {adding ? "保存中" : "加入单词本"}
        </button>
      </div>
    </article>
  );
}

function WordbookView({ onGoTranslate }: { onGoTranslate: () => void }) {
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
  const updateLevelMutation = useMutation({
    mutationFn: ({ id, level }: { id: string; level: Level }) => updateWordbookEntryLevel(id, level),
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
          <button className="secondary-button" onClick={onGoTranslate}>
            <ExternalLink size={16} />
            去翻译添加
          </button>
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
              <WordbookCard
                key={item.id}
                item={item}
                deleting={deleteMutation.isPending}
                updatingLevel={updateLevelMutation.isPending}
                onDelete={() => {
                  if (window.confirm(`确定删除“${item.text}”？`)) deleteMutation.mutate(item.id);
                }}
                onLevelChange={(level) => updateLevelMutation.mutate({ id: item.id, level })}
              />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function WordbookCard({
  item,
  deleting,
  updatingLevel,
  onDelete,
  onLevelChange,
}: {
  item: WordbookEntry;
  deleting: boolean;
  updatingLevel: boolean;
  onDelete: () => void;
  onLevelChange: (level: Level) => void;
}) {
  return (
    <article className="word-card">
      <div className="card-title">
        <strong>{item.text}</strong>
        <span>{new Date(item.createdAt).toLocaleDateString()}</span>
      </div>
      <div className="word-card-meta">
        <span>{languageLabel(item.language)} → {languageLabel(item.targetLanguage)}</span>
        <span>{item.source}</span>
      </div>
      <label className="compact-field">
        难度
        <select value={item.level} disabled={updatingLevel} onChange={(event) => onLevelChange(event.target.value as Level)}>
          {levelOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </label>
      <p>{item.translation}</p>
      {(item.definitions.length > 0 || item.examples.length > 0) && (
        <details className="word-details">
          <summary>完整内容</summary>
          {item.definitions.map((definition, index) => (
            <div className="definition-row compact-definition" key={`${definition.partOfSpeech}-${index}`}>
              <span>{definition.partOfSpeech}</span>
              <p>
                {definition.meaning}
                {definition.meaningTranslation && <small>{definition.meaningTranslation}</small>}
              </p>
              {definition.example && (
                <em>
                  {definition.example}
                  {definition.exampleTranslation && <small>{definition.exampleTranslation}</small>}
                </em>
              )}
            </div>
          ))}
          {item.examples.length > 0 && (
            <ul className="example-list">
              {item.examples.map((example) => (
                <li key={example}>{example}</li>
              ))}
            </ul>
          )}
        </details>
      )}
      <button className="danger-button" disabled={deleting} onClick={onDelete}>
        <Trash2 size={15} />
        删除
      </button>
    </article>
  );
}

function ShortcutInput({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const [recording, setRecording] = useState(false);
  return (
    <input
      className={clsx(recording && "recording-shortcut")}
      readOnly
      value={value}
      onFocus={() => setRecording(true)}
      onBlur={() => setRecording(false)}
      onKeyDown={(event) => {
        event.preventDefault();
        const shortcut = shortcutFromKeyboardEvent(event);
        if (shortcut) {
          onChange(shortcut);
          setRecording(false);
          event.currentTarget.blur();
        }
      }}
      placeholder="点击后按组合键"
    />
  );
}

function SettingsView({ settings }: { settings?: AppSettings }) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<AppSettings | null>(settings ?? null);
  const [notice, setNotice] = useState("");
  const [noticeError, setNoticeError] = useState(false);
  const [testingProviderId, setTestingProviderId] = useState("");

  useEffect(() => {
    if (settings) setDraft(settings);
  }, [settings]);

  const saveMutation = useMutation({
    mutationFn: (value: AppSettings) => saveSettings(value),
    onSuccess: () => {
      setNotice("设置已保存");
      setNoticeError(false);
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
    onError: (error) => {
      setNotice(errorMessage(error));
      setNoticeError(true);
    },
  });
  const providerTestMutation = useMutation({
    mutationFn: (provider: ApiProvider) => testApiProvider(provider),
    onMutate: (provider) => setTestingProviderId(provider.id),
    onSuccess: (result) => {
      setNotice(result.ok ? `连接可用：${result.translatedText ?? result.message}` : `连接失败：${result.message}`);
      setNoticeError(!result.ok);
    },
    onError: (error) => {
      setNotice(errorMessage(error));
      setNoticeError(true);
    },
    onSettled: () => setTestingProviderId(""),
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
          apiSecret: "",
          region: "",
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
          {saveMutation.isPending ? "保存中" : "保存"}
        </button>
      </header>
      {notice && <div className={clsx("inline-status", noticeError && "error-text")}>{notice}</div>}

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
          <ShortcutInput value={draft.shortcutTranslate} onChange={(value) => set("shortcutTranslate", value)} />
          <em className="field-hint">点击输入框后直接按组合键，例如 Alt+L，保存后立即重新注册。</em>
        </label>
        <label>
          截图翻译
          <ShortcutInput value={draft.shortcutScreenshot} onChange={(value) => set("shortcutScreenshot", value)} />
          <em className="field-hint">点击输入框后直接按组合键，若冲突会提示保存失败。</em>
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
        <label className="toggle-field wide-field">
          <span>
            <strong>开机自动启动</strong>
            <em>开启后登录 Windows 时自动启动 3Q 语言助手。</em>
          </span>
          <input
            type="checkbox"
            checked={draft.launchAtStartup}
            onChange={(event) => set("launchAtStartup", event.target.checked)}
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
                    <option value="tencent">腾讯云机器翻译</option>
                    <option value="azure">Azure Translator</option>
                    <option value="deepl">DeepL API</option>
                    <option value="baidu">百度翻译开放平台</option>
                  </select>
                </label>
                {provider.providerType !== "mymemory" && (
                  <label className="wide-field">
                    Base URL
                    <input
                      value={provider.baseUrl}
                      onChange={(event) => updateProvider(provider.id, "baseUrl", event.target.value)}
                      placeholder={
                        provider.providerType === "libretranslate"
                          ? "https://libretranslate.example.com"
                          : provider.providerType === "deepl"
                            ? "https://api-free.deepl.com/v2"
                            : provider.providerType === "azure"
                              ? "https://api.cognitive.microsofttranslator.com"
                              : provider.providerType === "baidu"
                                ? "https://fanyi-api.baidu.com/api/trans/vip/translate"
                                : provider.providerType === "tencent"
                                  ? "https://tmt.tencentcloudapi.com"
                                  : "https://api.example.com/v1"
                      }
                    />
                  </label>
                )}
                {provider.providerType !== "mymemory" && provider.providerType !== "libretranslate" && (
                  <label>
                    {provider.providerType === "tencent"
                      ? "SecretId"
                      : provider.providerType === "baidu"
                        ? "AppID"
                        : "API Key"}
                    <input type="password" value={provider.apiKey} onChange={(event) => updateProvider(provider.id, "apiKey", event.target.value)} />
                  </label>
                )}
                {["tencent", "baidu"].includes(provider.providerType) && (
                  <label>
                    {provider.providerType === "tencent" ? "SecretKey" : "密钥"}
                    <input type="password" value={provider.apiSecret} onChange={(event) => updateProvider(provider.id, "apiSecret", event.target.value)} />
                  </label>
                )}
                {["tencent", "azure"].includes(provider.providerType) && (
                  <label>
                    区域
                    <input
                      value={provider.region}
                      onChange={(event) => updateProvider(provider.id, "region", event.target.value)}
                      placeholder={provider.providerType === "tencent" ? "ap-guangzhou" : "eastasia"}
                    />
                  </label>
                )}
                {provider.providerType === "libretranslate" && (
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
              <div className="provider-actions">
                <button
                  className="secondary-button"
                  disabled={providerTestMutation.isPending && testingProviderId === provider.id}
                  onClick={() => providerTestMutation.mutate(provider)}
                >
                  <Clipboard size={16} />
                  {providerTestMutation.isPending && testingProviderId === provider.id ? "测试中" : "测试连接"}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}

export default App;
