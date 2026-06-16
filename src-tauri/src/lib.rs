use chrono::Local;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::sync::Mutex;
use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

struct AppState {
    db: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Definition {
    part_of_speech: String,
    meaning: String,
    example: Option<String>,
    synonyms: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranslationResult {
    source_text: String,
    source_language: String,
    target_language: String,
    translated_text: String,
    phonetic: Option<String>,
    definitions: Vec<Definition>,
    examples: Vec<String>,
    phrases: Vec<String>,
    provider: String,
    is_word: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WordbookEntry {
    id: String,
    text: String,
    language: String,
    target_language: String,
    translation: String,
    definitions: Vec<Definition>,
    examples: Vec<String>,
    level: String,
    source: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DailyItem {
    id: String,
    word: String,
    language: String,
    translation: String,
    examples: Vec<String>,
    #[serde(default)]
    example_translations: Vec<String>,
    level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiProvider {
    id: String,
    name: String,
    provider_type: String,
    enabled: bool,
    base_url: String,
    api_key: String,
    model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    default_english_target: String,
    default_other_target: String,
    daily_language: String,
    daily_level: String,
    shortcut_translate: String,
    shortcut_screenshot: String,
    #[serde(default = "default_close_to_tray")]
    close_to_tray: bool,
    #[serde(default = "default_active_provider_id")]
    active_provider_id: String,
    #[serde(default = "default_api_providers")]
    api_providers: Vec<ApiProvider>,
    #[serde(default)]
    libre_translate_url: String,
    #[serde(default)]
    open_ai_base_url: String,
    #[serde(default)]
    open_ai_api_key: String,
}

fn default_close_to_tray() -> bool {
    true
}

fn default_active_provider_id() -> String {
    "mymemory".into()
}

fn default_api_providers() -> Vec<ApiProvider> {
    vec![
        ApiProvider {
            id: "mymemory".into(),
            name: "MyMemory 免费源".into(),
            provider_type: "mymemory".into(),
            enabled: true,
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
        },
        ApiProvider {
            id: "libre-default".into(),
            name: "LibreTranslate".into(),
            provider_type: "libretranslate".into(),
            enabled: false,
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
        },
        ApiProvider {
            id: "openai-default".into(),
            name: "OpenAI-compatible".into(),
            provider_type: "openai".into(),
            enabled: false,
            base_url: String::new(),
            api_key: String::new(),
            model: "gpt-4o-mini".into(),
        },
    ]
}

fn default_settings() -> AppSettings {
    AppSettings {
        default_english_target: "zh".into(),
        default_other_target: "en".into(),
        daily_language: "en".into(),
        daily_level: "beginner".into(),
        shortcut_translate: "Ctrl+Alt+Q".into(),
        shortcut_screenshot: "Ctrl+Alt+S".into(),
        close_to_tray: default_close_to_tray(),
        active_provider_id: default_active_provider_id(),
        api_providers: default_api_providers(),
        libre_translate_url: String::new(),
        open_ai_base_url: String::new(),
        open_ai_api_key: String::new(),
    }
}

fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        create table if not exists settings (
            key text primary key,
            value text not null
        );
        create table if not exists wordbook (
            id text primary key,
            text text not null,
            language text not null,
            target_language text not null,
            translation text not null,
            definitions_json text not null,
            examples_json text not null,
            source text not null,
            created_at text not null
        );
        create index if not exists idx_wordbook_language on wordbook(language);
        create table if not exists daily_cache (
            cache_key text primary key,
            date text not null,
            items_json text not null
        );
        ",
    )
    .map_err(|err| err.to_string())?;

    if !table_has_column(conn, "wordbook", "level")? {
        conn.execute(
            "alter table wordbook add column level text not null default 'beginner'",
            [],
        )
        .map_err(|err| err.to_string())?;
    }

    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("pragma table_info({})", table))
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| err.to_string())?;

    for row in rows {
        if row.map_err(|err| err.to_string())? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn detect_language(text: &str) -> String {
    if text
        .chars()
        .any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch))
    {
        "zh".into()
    } else if text
        .chars()
        .any(|ch| ('\u{3040}'..='\u{30ff}').contains(&ch))
    {
        "ja".into()
    } else if text
        .chars()
        .any(|ch| ('\u{ac00}'..='\u{d7af}').contains(&ch))
    {
        "ko".into()
    } else if text
        .chars()
        .any(|ch| ('\u{0400}'..='\u{04ff}').contains(&ch))
    {
        "ru".into()
    } else if text
        .chars()
        .any(|ch| ('\u{0600}'..='\u{06ff}').contains(&ch))
    {
        "ar".into()
    } else {
        "en".into()
    }
}

fn looks_like_word(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= 40
        && trimmed
            .chars()
            .all(|ch| ch.is_alphabetic() || ch == '\'' || ch == '-')
}

fn infer_difficulty(text: &str, definitions: usize, examples: usize) -> String {
    let size = text.trim().chars().count();
    if size <= 4 && definitions <= 1 {
        "zero".into()
    } else if size <= 8 && definitions <= 2 {
        "beginner".into()
    } else if size <= 13 || examples >= 2 {
        "skilled".into()
    } else {
        "advanced".into()
    }
}

fn default_target_for(
    source_language: &str,
    settings: &AppSettings,
    explicit: Option<String>,
) -> String {
    explicit.unwrap_or_else(|| {
        if source_language == "en" {
            settings.default_english_target.clone()
        } else {
            settings.default_other_target.clone()
        }
    })
}

fn setting_value(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    let mut stmt = conn
        .prepare("select value from settings where key = ?1")
        .map_err(|err| err.to_string())?;
    let mut rows = stmt.query(params![key]).map_err(|err| err.to_string())?;
    if let Some(row) = rows.next().map_err(|err| err.to_string())? {
        Ok(Some(
            row.get::<_, String>(0).map_err(|err| err.to_string())?,
        ))
    } else {
        Ok(None)
    }
}

fn load_settings_from_db(conn: &Connection) -> Result<AppSettings, String> {
    let mut settings = default_settings();
    if let Some(raw) = setting_value(conn, "app_settings")? {
        settings = serde_json::from_str(&raw).unwrap_or(settings);
    }
    Ok(normalize_settings(settings))
}

fn normalize_settings(mut settings: AppSettings) -> AppSettings {
    if settings.api_providers.is_empty() {
        settings.api_providers = default_api_providers();
    }

    if !settings.libre_translate_url.trim().is_empty() {
        if let Some(provider) = settings
            .api_providers
            .iter_mut()
            .find(|provider| provider.provider_type == "libretranslate")
        {
            if provider.base_url.trim().is_empty() {
                provider.base_url = settings.libre_translate_url.clone();
            }
        }
    }

    if !settings.open_ai_base_url.trim().is_empty() || !settings.open_ai_api_key.trim().is_empty() {
        if let Some(provider) = settings
            .api_providers
            .iter_mut()
            .find(|provider| provider.provider_type == "openai")
        {
            if provider.base_url.trim().is_empty() {
                provider.base_url = settings.open_ai_base_url.clone();
            }
            if provider.api_key.trim().is_empty() {
                provider.api_key = settings.open_ai_api_key.clone();
            }
        }
    }

    if !settings
        .api_providers
        .iter()
        .any(|provider| provider.id == settings.active_provider_id)
    {
        settings.active_provider_id = default_active_provider_id();
    }

    settings
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn should_close_to_tray(window: &tauri::Window) -> bool {
    window
        .app_handle()
        .try_state::<AppState>()
        .and_then(|state| {
            state
                .db
                .lock()
                .ok()
                .and_then(|conn| load_settings_from_db(&conn).ok())
        })
        .map(|settings| settings.close_to_tray)
        .unwrap_or(default_close_to_tray())
}

async fn translate_with_mymemory(text: &str, source: &str, target: &str) -> Option<String> {
    let lang_pair = format!("{}|{}", source, target);
    let url = format!(
        "https://api.mymemory.translated.net/get?q={}&langpair={}",
        urlencoding::encode(text),
        urlencoding::encode(&lang_pair)
    );
    let response = reqwest::get(url).await.ok()?;
    let data = response.json::<Value>().await.ok()?;
    data.pointer("/responseData/translatedText")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

async fn translate_with_libre(
    text: &str,
    source: &str,
    target: &str,
    provider: &ApiProvider,
) -> Option<String> {
    let base_url = provider.base_url.trim().trim_end_matches('/');
    if base_url.is_empty() {
        return None;
    }

    let mut body = json!({
        "q": text,
        "source": source,
        "target": target,
        "format": "text"
    });
    if !provider.api_key.trim().is_empty() {
        body["api_key"] = Value::String(provider.api_key.clone());
    }

    let response = reqwest::Client::new()
        .post(format!("{}/translate", base_url))
        .json(&body)
        .send()
        .await
        .ok()?;
    let data = response.json::<Value>().await.ok()?;
    data.get("translatedText")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

async fn translate_with_openai(
    text: &str,
    source: &str,
    target: &str,
    provider: &ApiProvider,
) -> Option<String> {
    let base_url = provider.base_url.trim().trim_end_matches('/');
    let api_key = provider.api_key.trim();
    if base_url.is_empty() || api_key.is_empty() {
        return None;
    }

    let endpoint = if base_url.ends_with("/chat/completions") {
        base_url.to_string()
    } else {
        format!("{}/chat/completions", base_url)
    };
    let model = if provider.model.trim().is_empty() {
        "gpt-4o-mini"
    } else {
        provider.model.trim()
    };
    let body = json!({
        "model": model,
        "temperature": 0.2,
        "messages": [
            {
                "role": "system",
                "content": "You are a concise translation engine. Return only the translated text."
            },
            {
                "role": "user",
                "content": format!("Translate from {} to {}:\n{}", source, target, text)
            }
        ]
    });

    let response = reqwest::Client::new()
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .ok()?;
    let data = response.json::<Value>().await.ok()?;
    data.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn translate_with_configured_provider(
    text: &str,
    source: &str,
    target: &str,
    settings: &AppSettings,
) -> (String, String) {
    let active = settings
        .api_providers
        .iter()
        .find(|provider| provider.id == settings.active_provider_id && provider.enabled)
        .or_else(|| {
            settings
                .api_providers
                .iter()
                .find(|provider| provider.id == "mymemory")
        })
        .cloned()
        .unwrap_or_else(|| {
            default_api_providers()
                .into_iter()
                .next()
                .expect("default provider")
        });

    let translated = match active.provider_type.as_str() {
        "libretranslate" => translate_with_libre(text, source, target, &active).await,
        "openai" => translate_with_openai(text, source, target, &active).await,
        _ => translate_with_mymemory(text, source, target).await,
    };

    if let Some(translated) = translated {
        return (translated, active.name);
    }

    let fallback = translate_with_mymemory(text, source, target)
        .await
        .unwrap_or_else(|| "翻译源暂时不可用，请稍后重试或检查设置里的 API 配置。".into());
    let provider_name = if active.provider_type == "mymemory" {
        "MyMemory 免费源".into()
    } else {
        format!("{} → MyMemory fallback", active.name)
    };
    (fallback, provider_name)
}

async fn english_dictionary(
    text: &str,
) -> (Option<String>, Vec<Definition>, Vec<String>, Vec<String>) {
    let url = format!(
        "https://api.dictionaryapi.dev/api/v2/entries/en/{}",
        urlencoding::encode(text.trim())
    );
    let Ok(response) = reqwest::get(url).await else {
        return (None, vec![], vec![], vec![]);
    };
    let Ok(data) = response.json::<Value>().await else {
        return (None, vec![], vec![], vec![]);
    };
    let Some(entry) = data.as_array().and_then(|items| items.first()) else {
        return (None, vec![], vec![], vec![]);
    };

    let phonetic = entry
        .get("phonetic")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            entry
                .get("phonetics")
                .and_then(Value::as_array)
                .and_then(|items| {
                    items
                        .iter()
                        .find_map(|item| item.get("text").and_then(Value::as_str))
                })
                .map(ToOwned::to_owned)
        });

    let mut definitions = Vec::new();
    let mut examples = Vec::new();
    let mut phrases = Vec::new();

    if let Some(meanings) = entry.get("meanings").and_then(Value::as_array) {
        for meaning in meanings {
            let part = meaning
                .get("partOfSpeech")
                .and_then(Value::as_str)
                .unwrap_or("word")
                .to_string();
            if let Some(items) = meaning.get("definitions").and_then(Value::as_array) {
                for item in items.iter().take(3) {
                    let meaning_text = item
                        .get("definition")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let example = item
                        .get("example")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned);
                    if let Some(example_text) = &example {
                        examples.push(example_text.clone());
                    }
                    let synonyms = item
                        .get("synonyms")
                        .and_then(Value::as_array)
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(Value::as_str)
                                .take(4)
                                .map(ToOwned::to_owned)
                                .collect::<Vec<_>>()
                        });
                    if let Some(items) = &synonyms {
                        phrases.extend(items.iter().cloned());
                    }
                    if !meaning_text.is_empty() {
                        definitions.push(Definition {
                            part_of_speech: part.clone(),
                            meaning: meaning_text,
                            example,
                            synonyms,
                        });
                    }
                }
            }
        }
    }

    examples.truncate(6);
    phrases.truncate(8);
    (phonetic, definitions, examples, phrases)
}

#[tauri::command]
async fn translate_text(
    state: State<'_, AppState>,
    text: String,
    target_language: Option<String>,
) -> Result<TranslationResult, String> {
    let clean_text = text.trim().to_string();
    if clean_text.is_empty() {
        return Err("请输入要翻译的内容".into());
    }

    let settings = {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        load_settings_from_db(&conn)?
    };
    let source_language = detect_language(&clean_text);
    let target = default_target_for(&source_language, &settings, target_language);
    let is_word = looks_like_word(&clean_text);

    let (translated_text, provider_name) =
        translate_with_configured_provider(&clean_text, &source_language, &target, &settings).await;

    let (phonetic, definitions, examples, phrases) = if is_word && source_language == "en" {
        english_dictionary(&clean_text).await
    } else {
        (None, vec![], vec![], vec![])
    };
    let provider = if is_word && source_language == "en" {
        format!("{} + Free Dictionary API", provider_name)
    } else {
        provider_name
    };

    Ok(TranslationResult {
        source_text: clean_text,
        source_language,
        target_language: target,
        translated_text,
        phonetic,
        definitions,
        examples,
        phrases,
        provider,
        is_word,
    })
}

#[tauri::command]
fn add_to_wordbook(state: State<'_, AppState>, item: Value) -> Result<WordbookEntry, String> {
    let now = Local::now().to_rfc3339();
    let entry = if item.get("sourceText").is_some() {
        let result: TranslationResult =
            serde_json::from_value(item).map_err(|err| err.to_string())?;
        let level = infer_difficulty(
            &result.source_text,
            result.definitions.len(),
            result.examples.len(),
        );
        WordbookEntry {
            id: format!(
                "word-{}",
                Local::now().timestamp_nanos_opt().unwrap_or_default()
            ),
            text: result.source_text,
            language: result.source_language,
            target_language: result.target_language,
            translation: result.translated_text,
            level,
            definitions: result.definitions,
            examples: result.examples,
            source: result.provider,
            created_at: now,
        }
    } else {
        let daily: DailyItem = serde_json::from_value(item).map_err(|err| err.to_string())?;
        let examples = daily
            .examples
            .iter()
            .enumerate()
            .map(|(index, example)| {
                daily
                    .example_translations
                    .get(index)
                    .filter(|translation| !translation.is_empty())
                    .map(|translation| format!("{}\n{}", example, translation))
                    .unwrap_or_else(|| example.clone())
            })
            .collect();
        WordbookEntry {
            id: format!(
                "daily-{}",
                Local::now().timestamp_nanos_opt().unwrap_or_default()
            ),
            text: daily.word,
            language: daily.language,
            target_language: "zh".into(),
            translation: daily.translation,
            definitions: vec![],
            examples,
            level: daily.level,
            source: "daily learning".into(),
            created_at: now,
        }
    };

    let conn = state.db.lock().map_err(|err| err.to_string())?;
    conn.execute(
        "insert or replace into wordbook
        (id, text, language, target_language, translation, definitions_json, examples_json, level, source, created_at)
        values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            entry.id,
            entry.text,
            entry.language,
            entry.target_language,
            entry.translation,
            serde_json::to_string(&entry.definitions).map_err(|err| err.to_string())?,
            serde_json::to_string(&entry.examples).map_err(|err| err.to_string())?,
            entry.level,
            entry.source,
            entry.created_at,
        ],
    )
    .map_err(|err| err.to_string())?;

    Ok(entry)
}

#[tauri::command]
fn list_wordbook(state: State<'_, AppState>) -> Result<Vec<WordbookEntry>, String> {
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    let mut stmt = conn
        .prepare(
            "select id, text, language, target_language, translation, definitions_json, examples_json, level, source, created_at
             from wordbook order by created_at desc",
        )
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let definitions_json: String = row.get(5)?;
            let examples_json: String = row.get(6)?;
            Ok(WordbookEntry {
                id: row.get(0)?,
                text: row.get(1)?,
                language: row.get(2)?,
                target_language: row.get(3)?,
                translation: row.get(4)?,
                definitions: serde_json::from_str(&definitions_json).unwrap_or_default(),
                examples: serde_json::from_str(&examples_json).unwrap_or_default(),
                level: row.get(7)?,
                source: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|err| err.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn delete_wordbook_entry(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    conn.execute("delete from wordbook where id = ?1", params![id])
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn language_level_words(language: &str, level: &str) -> Option<Vec<(&'static str, &'static str)>> {
    let words = match (language, level) {
        ("zh", "zero") => vec![
            ("你好", "hello"),
            ("谢谢", "thank you"),
            ("水", "water"),
            ("书", "book"),
            ("家", "home"),
        ],
        ("zh", "beginner") => vec![
            ("学习", "learn; study"),
            ("朋友", "friend"),
            ("今天", "today"),
            ("喜欢", "like"),
            ("明白", "understand"),
        ],
        ("zh", "skilled") => vec![
            ("语境", "context"),
            ("表达", "expression"),
            ("习惯", "habit"),
            ("记忆", "memory"),
            ("练习", "practice"),
        ],
        ("zh", "advanced") => vec![
            ("歧义", "ambiguity"),
            ("隐喻", "metaphor"),
            ("含义", "connotation"),
            ("语域", "register"),
            ("改述", "paraphrase"),
        ],
        ("ja", "zero") => vec![
            ("こんにちは", "你好"),
            ("ありがとう", "谢谢"),
            ("水", "水"),
            ("本", "书"),
            ("家", "家"),
        ],
        ("ja", "beginner") => vec![
            ("勉強", "学习"),
            ("友達", "朋友"),
            ("便利", "方便"),
            ("意味", "意思"),
            ("今日", "今天"),
        ],
        ("ja", "skilled") => vec![
            ("文脈", "语境"),
            ("表現", "表达"),
            ("習慣", "习惯"),
            ("記憶", "记忆"),
            ("練習", "练习"),
        ],
        ("ja", "advanced") => vec![
            ("曖昧", "歧义"),
            ("比喩", "隐喻"),
            ("含意", "隐含意义"),
            ("語域", "语域"),
            ("言い換え", "改述"),
        ],
        ("ko", "zero") => vec![
            ("안녕하세요", "你好"),
            ("감사합니다", "谢谢"),
            ("물", "水"),
            ("책", "书"),
            ("집", "家"),
        ],
        ("ko", "beginner") => vec![
            ("공부", "学习"),
            ("친구", "朋友"),
            ("필요", "需要"),
            ("의미", "意思"),
            ("오늘", "今天"),
        ],
        ("ko", "skilled") => vec![
            ("맥락", "语境"),
            ("표현", "表达"),
            ("습관", "习惯"),
            ("기억", "记忆"),
            ("연습", "练习"),
        ],
        ("ko", "advanced") => vec![
            ("모호함", "歧义"),
            ("은유", "隐喻"),
            ("함의", "隐含意义"),
            ("어역", "语域"),
            ("바꿔 말하기", "改述"),
        ],
        ("fr", "zero") => vec![
            ("bonjour", "你好"),
            ("merci", "谢谢"),
            ("eau", "水"),
            ("livre", "书"),
            ("maison", "家"),
        ],
        ("fr", "beginner") => vec![
            ("apprendre", "学习"),
            ("ami", "朋友"),
            ("utile", "有用的"),
            ("comprendre", "理解"),
            ("aujourd'hui", "今天"),
        ],
        ("fr", "skilled") => vec![
            ("contexte", "语境"),
            ("expression", "表达"),
            ("habitude", "习惯"),
            ("mémoire", "记忆"),
            ("pratique", "练习"),
        ],
        ("fr", "advanced") => vec![
            ("ambiguïté", "歧义"),
            ("métaphore", "隐喻"),
            ("connotation", "隐含意义"),
            ("registre", "语域"),
            ("paraphrase", "改述"),
        ],
        ("de", "zero") => vec![
            ("hallo", "你好"),
            ("danke", "谢谢"),
            ("Wasser", "水"),
            ("Buch", "书"),
            ("Haus", "家"),
        ],
        ("de", "beginner") => vec![
            ("lernen", "学习"),
            ("Freund", "朋友"),
            ("nützlich", "有用的"),
            ("verstehen", "理解"),
            ("heute", "今天"),
        ],
        ("de", "skilled") => vec![
            ("Kontext", "语境"),
            ("Ausdruck", "表达"),
            ("Gewohnheit", "习惯"),
            ("Gedächtnis", "记忆"),
            ("Übung", "练习"),
        ],
        ("de", "advanced") => vec![
            ("Mehrdeutigkeit", "歧义"),
            ("Metapher", "隐喻"),
            ("Konnotation", "隐含意义"),
            ("Register", "语域"),
            ("Paraphrase", "改述"),
        ],
        ("es", "zero") => vec![
            ("hola", "你好"),
            ("gracias", "谢谢"),
            ("agua", "水"),
            ("libro", "书"),
            ("casa", "家"),
        ],
        ("es", "beginner") => vec![
            ("aprender", "学习"),
            ("amigo", "朋友"),
            ("útil", "有用的"),
            ("entender", "理解"),
            ("hoy", "今天"),
        ],
        ("es", "skilled") => vec![
            ("contexto", "语境"),
            ("expresión", "表达"),
            ("hábito", "习惯"),
            ("memoria", "记忆"),
            ("práctica", "练习"),
        ],
        ("es", "advanced") => vec![
            ("ambigüedad", "歧义"),
            ("metáfora", "隐喻"),
            ("connotación", "隐含意义"),
            ("registro", "语域"),
            ("paráfrasis", "改述"),
        ],
        _ => return None,
    };
    Some(words)
}

fn examples_for_daily_word(language: &str, word: &str) -> (Vec<String>, Vec<String>) {
    match language {
        "zh" => (
            vec![
                format!("我今天学习“{}”。", word),
                "这个词在句子里很常见。".into(),
                format!("请用“{}”造一个句子。", word),
            ],
            vec![
                format!("I study \"{}\" today.", word),
                "This word is common in sentences.".into(),
                format!("Please make a sentence with \"{}\".", word),
            ],
        ),
        "ja" => (
            vec![
                format!("{}を練習します。", word),
                "この単語はよく使います。".into(),
                format!("{}を例文で覚えます。", word),
            ],
            vec![
                format!("练习 {}。", word),
                "这个单词经常使用。".into(),
                format!("用例句记住 {}。", word),
            ],
        ),
        "ko" => (
            vec![
                format!("{}를 연습해요.", word),
                "이 단어는 자주 써요.".into(),
                format!("{}를 예문으로 기억해요.", word),
            ],
            vec![
                format!("练习 {}。", word),
                "这个单词经常使用。".into(),
                format!("用例句记住 {}。", word),
            ],
        ),
        "fr" => (
            vec![
                format!("J'apprends {} aujourd'hui.", word),
                format!("{} apparaît dans des phrases simples.", word),
                format!("Je mémorise {} avec un exemple.", word),
            ],
            vec![
                format!("我今天学习 {}。", word),
                format!("{} 会出现在简单句子里。", word),
                format!("我用例句记住 {}。", word),
            ],
        ),
        "de" => (
            vec![
                format!("Ich lerne {} heute.", word),
                format!("{} passt in einfache Sätze.", word),
                format!("Ich merke mir {} mit einem Beispiel.", word),
            ],
            vec![
                format!("我今天学习 {}。", word),
                format!("{} 适合放进简单句子里。", word),
                format!("我用例句记住 {}。", word),
            ],
        ),
        _ => (
            vec![
                format!("Aprendo {} hoy.", word),
                format!("{} aparece en frases simples.", word),
                format!("Memorizo {} con un ejemplo.", word),
            ],
            vec![
                format!("我今天学习 {}。", word),
                format!("{} 会出现在简单句子里。", word),
                format!("我用例句记住 {}。", word),
            ],
        ),
    }
}

fn daily_fallback(language: &str, level: &str, variant: usize) -> Vec<DailyItem> {
    if let Some(words) = language_level_words(language, level) {
        let offset = if words.is_empty() {
            0
        } else {
            variant % words.len()
        };
        return words
            .iter()
            .cycle()
            .skip(offset)
            .take(5)
            .enumerate()
            .map(|(index, (word, translation))| {
                let (examples, example_translations) = examples_for_daily_word(language, word);
                DailyItem {
                    id: format!("{}-{}-v3-{}", language, level, index),
                    word: (*word).into(),
                    language: language.into(),
                    translation: (*translation).into(),
                    examples,
                    example_translations,
                    level: level.into(),
                }
            })
            .collect();
    }

    let raw: Vec<(&str, &str, [&str; 3], [&str; 3])> = match language {
        "zh" => vec![
            (
                "学习",
                "learn; study",
                [
                    "我每天学习一点新内容。",
                    "学习语言需要耐心。",
                    "她喜欢边听边学习。",
                ],
                [
                    "I learn a little new content every day.",
                    "Learning a language requires patience.",
                    "She likes learning while listening.",
                ],
            ),
            (
                "朋友",
                "friend",
                [
                    "朋友给了我很多帮助。",
                    "他是我的老朋友。",
                    "我们一起练习口语。",
                ],
                [
                    "My friend helped me a lot.",
                    "He is my old friend.",
                    "We practice speaking together.",
                ],
            ),
            (
                "今天",
                "today",
                [
                    "今天我想学五个词。",
                    "今天的天气很好。",
                    "今天先复习昨天的内容。",
                ],
                [
                    "Today I want to learn five words.",
                    "The weather is nice today.",
                    "Review yesterday's content first today.",
                ],
            ),
            (
                "喜欢",
                "like",
                ["我喜欢这门语言。", "她喜欢听慢速音频。", "你喜欢哪个例句？"],
                [
                    "I like this language.",
                    "She likes listening to slow audio.",
                    "Which example sentence do you like?",
                ],
            ),
            (
                "明白",
                "understand",
                [
                    "我明白这个句子的意思。",
                    "他还不太明白语法。",
                    "例句能帮助我明白用法。",
                ],
                [
                    "I understand the meaning of this sentence.",
                    "He does not quite understand the grammar yet.",
                    "Examples help me understand usage.",
                ],
            ),
        ],
        "ja" => vec![
            (
                "こんにちは",
                "你好",
                [
                    "こんにちは、はじめまして。",
                    "彼女は笑顔でこんにちはと言いました。",
                    "こんにちはは便利なあいさつです。",
                ],
                [
                    "你好，初次见面。",
                    "她微笑着说了你好。",
                    "こんにちは 是很实用的问候语。",
                ],
            ),
            (
                "勉強",
                "学习",
                [
                    "毎日日本語を勉強します。",
                    "勉強は少しずつ続けます。",
                    "例文で単語を勉強します。",
                ],
                ["我每天学习日语。", "学习要一点点坚持。", "用例句学习单词。"],
            ),
            (
                "友達",
                "朋友",
                [
                    "友達と会話を練習します。",
                    "彼は大切な友達です。",
                    "新しい友達ができました。",
                ],
                ["我和朋友练习对话。", "他是重要的朋友。", "我交到了新朋友。"],
            ),
            (
                "便利",
                "方便",
                [
                    "この表現は便利です。",
                    "便利なアプリを使います。",
                    "短い例文は便利です。",
                ],
                ["这个表达很方便。", "我使用方便的应用。", "短例句很方便。"],
            ),
            (
                "意味",
                "意思；含义",
                [
                    "この単語の意味は何ですか。",
                    "文脈で意味が変わります。",
                    "意味を確認しましょう。",
                ],
                [
                    "这个单词是什么意思？",
                    "含义会随语境改变。",
                    "我们确认一下意思吧。",
                ],
            ),
        ],
        "ko" => vec![
            (
                "안녕하세요",
                "你好",
                [
                    "안녕하세요, 만나서 반가워요.",
                    "그녀는 안녕하세요라고 말했어요.",
                    "안녕하세요는 기본 인사예요.",
                ],
                [
                    "你好，很高兴见到你。",
                    "她说了你好。",
                    "안녕하세요 是基础问候语。",
                ],
            ),
            (
                "공부",
                "学习",
                [
                    "저는 매일 한국어를 공부해요.",
                    "공부는 꾸준함이 중요해요.",
                    "예문으로 단어를 공부해요.",
                ],
                ["我每天学习韩语。", "学习贵在坚持。", "用例句学习单词。"],
            ),
            (
                "친구",
                "朋友",
                [
                    "친구와 같이 말하기를 연습해요.",
                    "그는 좋은 친구예요.",
                    "새 친구를 만났어요.",
                ],
                ["我和朋友一起练习口语。", "他是好朋友。", "我认识了新朋友。"],
            ),
            (
                "필요",
                "需要",
                [
                    "도움이 필요해요.",
                    "이 단어는 자주 필요해요.",
                    "연습이 필요합니다.",
                ],
                ["我需要帮助。", "这个单词经常需要用到。", "需要练习。"],
            ),
            (
                "의미",
                "意思；含义",
                [
                    "이 문장의 의미를 알아요.",
                    "의미가 조금 달라요.",
                    "문맥이 의미를 설명해요.",
                ],
                [
                    "我知道这个句子的意思。",
                    "意思有点不同。",
                    "语境解释了含义。",
                ],
            ),
        ],
        "fr" => vec![
            (
                "bonjour",
                "你好",
                [
                    "Bonjour, je m'appelle Q.",
                    "Elle dit bonjour avec le sourire.",
                    "Bonjour est une salutation simple.",
                ],
                [
                    "你好，我叫 Q。",
                    "她微笑着打招呼。",
                    "bonjour 是简单的问候语。",
                ],
            ),
            (
                "apprendre",
                "学习",
                [
                    "J'apprends le français chaque jour.",
                    "Apprendre une langue prend du temps.",
                    "Les exemples aident à apprendre.",
                ],
                [
                    "我每天学习法语。",
                    "学习一门语言需要时间。",
                    "例句有助于学习。",
                ],
            ),
            (
                "ami",
                "朋友",
                [
                    "Mon ami m'aide à pratiquer.",
                    "Elle parle avec un ami.",
                    "Un bon ami écoute.",
                ],
                ["我的朋友帮我练习。", "她和一位朋友说话。", "好朋友会倾听。"],
            ),
            (
                "utile",
                "有用的",
                [
                    "Cette phrase est utile.",
                    "Un dictionnaire est utile.",
                    "Les exemples utiles restent en mémoire.",
                ],
                ["这个句子很有用。", "词典很有用。", "有用的例句容易记住。"],
            ),
            (
                "comprendre",
                "理解",
                [
                    "Je comprends cette phrase.",
                    "Il veut comprendre le contexte.",
                    "Comprendre vient avec la pratique.",
                ],
                ["我理解这个句子。", "他想理解语境。", "理解来自练习。"],
            ),
        ],
        "de" => vec![
            (
                "hallo",
                "你好",
                [
                    "Hallo, ich heiße Q.",
                    "Sie sagt hallo.",
                    "Hallo ist ein einfaches Wort.",
                ],
                ["你好，我叫 Q。", "她说你好。", "hallo 是一个简单的词。"],
            ),
            (
                "lernen",
                "学习",
                [
                    "Ich lerne jeden Tag Deutsch.",
                    "Wir lernen mit Beispielen.",
                    "Lernen braucht Geduld.",
                ],
                ["我每天学习德语。", "我们用例句学习。", "学习需要耐心。"],
            ),
            (
                "Freund",
                "朋友",
                [
                    "Mein Freund hilft mir.",
                    "Ein Freund hört zu.",
                    "Ich übe mit einem Freund.",
                ],
                ["我的朋友帮助我。", "朋友会倾听。", "我和朋友一起练习。"],
            ),
            (
                "nützlich",
                "有用的",
                [
                    "Dieser Satz ist nützlich.",
                    "Das Buch ist nützlich.",
                    "Nützliche Beispiele helfen.",
                ],
                ["这个句子很有用。", "这本书很有用。", "有用的例句会有帮助。"],
            ),
            (
                "verstehen",
                "理解",
                [
                    "Ich verstehe das Wort.",
                    "Der Kontext hilft beim Verstehen.",
                    "Sie versteht die Frage.",
                ],
                ["我理解这个词。", "语境有助于理解。", "她理解这个问题。"],
            ),
        ],
        "es" => vec![
            (
                "hola",
                "你好",
                [
                    "Hola, me llamo Q.",
                    "Ella dice hola.",
                    "Hola es un saludo común.",
                ],
                ["你好，我叫 Q。", "她说你好。", "hola 是常见问候语。"],
            ),
            (
                "aprender",
                "学习",
                [
                    "Aprendo español cada día.",
                    "Aprender con ejemplos ayuda.",
                    "Quiero aprender más palabras.",
                ],
                [
                    "我每天学习西班牙语。",
                    "用例句学习有帮助。",
                    "我想学习更多单词。",
                ],
            ),
            (
                "amigo",
                "朋友",
                [
                    "Mi amigo practica conmigo.",
                    "Un amigo bueno escucha.",
                    "Conocí a un amigo nuevo.",
                ],
                [
                    "我的朋友和我一起练习。",
                    "好朋友会倾听。",
                    "我认识了一位新朋友。",
                ],
            ),
            (
                "útil",
                "有用的",
                [
                    "Esta frase es útil.",
                    "Un ejemplo útil ayuda.",
                    "La aplicación es útil para estudiar.",
                ],
                [
                    "这个句子很有用。",
                    "有用的例句会有帮助。",
                    "这个应用对学习有用。",
                ],
            ),
            (
                "entender",
                "理解",
                [
                    "Entiendo la frase.",
                    "El contexto ayuda a entender.",
                    "Ella entiende la palabra.",
                ],
                ["我理解这个句子。", "语境有助于理解。", "她理解这个单词。"],
            ),
        ],
        _ => match level {
            "zero" => vec![
                (
                    "hello",
                    "你好",
                    [
                        "Hello, my name is Q.",
                        "She said hello with a smile.",
                        "Hello is a friendly first word.",
                    ],
                    [
                        "你好，我叫 Q。",
                        "她微笑着打招呼。",
                        "hello 是一个友好的入门词。",
                    ],
                ),
                (
                    "book",
                    "书",
                    [
                        "This book is easy.",
                        "I read a book every night.",
                        "Put the book on the desk.",
                    ],
                    ["这本书很简单。", "我每天晚上读一本书。", "把书放在桌子上。"],
                ),
                (
                    "water",
                    "水",
                    [
                        "I drink water.",
                        "The water is cold.",
                        "Please bring some water.",
                    ],
                    ["我喝水。", "水是冷的。", "请拿一些水来。"],
                ),
                (
                    "friend",
                    "朋友",
                    [
                        "He is my friend.",
                        "A good friend listens.",
                        "I met a new friend today.",
                    ],
                    [
                        "他是我的朋友。",
                        "好朋友会倾听。",
                        "我今天认识了一位新朋友。",
                    ],
                ),
                (
                    "home",
                    "家",
                    [
                        "I am going home.",
                        "Home feels warm.",
                        "She works from home.",
                    ],
                    ["我要回家。", "家让人感到温暖。", "她在家工作。"],
                ),
            ],
            "skilled" => vec![
                (
                    "nuance",
                    "细微差别",
                    [
                        "The nuance matters in translation.",
                        "She explained the nuance clearly.",
                        "Context reveals nuance.",
                    ],
                    [
                        "翻译时细微差别很重要。",
                        "她清楚地解释了这个细微差别。",
                        "语境会揭示细微差别。",
                    ],
                ),
                (
                    "fluent",
                    "流利的",
                    [
                        "He became fluent through practice.",
                        "Fluent speech sounds natural.",
                        "She is fluent in three languages.",
                    ],
                    [
                        "他通过练习变得流利。",
                        "流利的表达听起来自然。",
                        "她能流利使用三种语言。",
                    ],
                ),
                (
                    "context",
                    "语境",
                    [
                        "Context changes the meaning.",
                        "Check the context before translating.",
                        "The word is formal in this context.",
                    ],
                    [
                        "语境会改变含义。",
                        "翻译前先检查语境。",
                        "这个词在此语境中偏正式。",
                    ],
                ),
                (
                    "retain",
                    "记住；保留",
                    [
                        "Examples help you retain words.",
                        "The app retains your notes.",
                        "Sleep helps learners retain memory.",
                    ],
                    [
                        "例句帮助你记住单词。",
                        "应用会保留你的笔记。",
                        "睡眠帮助学习者保持记忆。",
                    ],
                ),
                (
                    "phrase",
                    "短语",
                    [
                        "Learn the whole phrase.",
                        "This phrase sounds natural.",
                        "A phrase can carry culture.",
                    ],
                    [
                        "学习整个短语。",
                        "这个短语听起来很自然。",
                        "短语可以承载文化。",
                    ],
                ),
            ],
            "advanced" => vec![
                (
                    "idiomatic",
                    "地道的；惯用的",
                    [
                        "The sentence sounds idiomatic.",
                        "Idiomatic English is hard to translate literally.",
                        "She chose an idiomatic expression.",
                    ],
                    [
                        "这个句子听起来很地道。",
                        "地道英语很难逐字翻译。",
                        "她选择了一个惯用表达。",
                    ],
                ),
                (
                    "ambiguity",
                    "歧义",
                    [
                        "The translator resolved the ambiguity.",
                        "Ambiguity can be useful in poetry.",
                        "Context reduces ambiguity.",
                    ],
                    [
                        "译者消除了歧义。",
                        "歧义在诗歌中可能有用。",
                        "语境会减少歧义。",
                    ],
                ),
                (
                    "register",
                    "语域",
                    [
                        "Register affects word choice.",
                        "This register is too formal.",
                        "Learners should notice register.",
                    ],
                    [
                        "语域会影响选词。",
                        "这种语域太正式了。",
                        "学习者应该注意语域。",
                    ],
                ),
                (
                    "connotation",
                    "隐含意义",
                    [
                        "The word has a warm connotation.",
                        "Connotation differs from definition.",
                        "Good translators track connotation.",
                    ],
                    [
                        "这个词带有温暖的含义。",
                        "隐含意义不同于定义。",
                        "优秀译者会留意隐含意义。",
                    ],
                ),
                (
                    "paraphrase",
                    "改述",
                    [
                        "Paraphrase the idea in simple words.",
                        "A paraphrase can clarify meaning.",
                        "Try to paraphrase after reading.",
                    ],
                    [
                        "用简单的话改述这个想法。",
                        "改述可以澄清含义。",
                        "阅读后试着改述。",
                    ],
                ),
            ],
            _ => vec![
                (
                    "practice",
                    "练习",
                    [
                        "Practice makes speaking easier.",
                        "I practice English after dinner.",
                        "Daily practice builds confidence.",
                    ],
                    [
                        "练习会让口语更容易。",
                        "我晚饭后练习英语。",
                        "每日练习能建立自信。",
                    ],
                ),
                (
                    "curious",
                    "好奇的",
                    [
                        "A curious student asks questions.",
                        "I am curious about this word.",
                        "Curious minds learn faster.",
                    ],
                    [
                        "好奇的学生会提问。",
                        "我对这个词很好奇。",
                        "好奇的头脑学得更快。",
                    ],
                ),
                (
                    "useful",
                    "有用的",
                    [
                        "This phrase is useful.",
                        "A notebook is useful for study.",
                        "Useful examples help memory.",
                    ],
                    [
                        "这个短语很有用。",
                        "笔记本对学习有帮助。",
                        "有用的例句有助于记忆。",
                    ],
                ),
                (
                    "improve",
                    "提高",
                    [
                        "I want to improve my listening.",
                        "Small habits improve fluency.",
                        "Feedback helps you improve.",
                    ],
                    [
                        "我想提高听力。",
                        "小习惯能提高流利度。",
                        "反馈能帮助你进步。",
                    ],
                ),
                (
                    "sentence",
                    "句子",
                    [
                        "Write one sentence.",
                        "This sentence is clear.",
                        "Read the sentence aloud.",
                    ],
                    ["写一个句子。", "这个句子很清楚。", "把这个句子大声读出来。"],
                ),
            ],
        },
    };

    let offset = if raw.is_empty() {
        0
    } else {
        variant % raw.len()
    };
    raw.iter()
        .cycle()
        .skip(offset)
        .take(5)
        .enumerate()
        .map(
            |(index, (word, translation, examples, example_translations))| DailyItem {
                id: format!("{}-{}-v2-{}", language, level, index),
                word: (*word).into(),
                language: language.into(),
                translation: (*translation).into(),
                examples: examples.iter().map(|value| (*value).into()).collect(),
                example_translations: example_translations
                    .iter()
                    .map(|value| (*value).into())
                    .collect(),
                level: level.into(),
            },
        )
        .collect()
}

#[tauri::command]
fn get_daily_items(
    state: State<'_, AppState>,
    language: String,
    level: String,
    force_refresh: bool,
) -> Result<Vec<DailyItem>, String> {
    let today = Local::now().date_naive().to_string();
    let cache_key = format!("v3:{}:{}", language, level);
    let conn = state.db.lock().map_err(|err| err.to_string())?;

    if !force_refresh {
        let mut stmt = conn
            .prepare("select date, items_json from daily_cache where cache_key = ?1")
            .map_err(|err| err.to_string())?;
        let mut rows = stmt
            .query(params![cache_key])
            .map_err(|err| err.to_string())?;
        if let Some(row) = rows.next().map_err(|err| err.to_string())? {
            let date: String = row.get(0).map_err(|err| err.to_string())?;
            let items_json: String = row.get(1).map_err(|err| err.to_string())?;
            if date == today {
                return serde_json::from_str(&items_json).map_err(|err| err.to_string());
            }
        }
    }

    let variant = if force_refresh {
        Local::now().timestamp() as usize
    } else {
        0
    };
    let items = daily_fallback(&language, &level, variant);
    conn.execute(
        "insert or replace into daily_cache(cache_key, date, items_json) values (?1, ?2, ?3)",
        params![
            cache_key,
            today,
            serde_json::to_string(&items).map_err(|err| err.to_string())?
        ],
    )
    .map_err(|err| err.to_string())?;

    Ok(items)
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    load_settings_from_db(&conn)
}

#[tauri::command]
fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> Result<AppSettings, String> {
    let settings = normalize_settings(settings);
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    conn.execute(
        "insert or replace into settings(key, value) values ('app_settings', ?1)",
        params![serde_json::to_string(&settings).map_err(|err| err.to_string())?],
    )
    .map_err(|err| err.to_string())?;
    Ok(settings)
}

#[tauri::command]
async fn capture_and_translate() -> Result<TranslationResult, String> {
    Err("截图入口已接入；OCR 需要 Windows OCR 绑定在本机 Rust 环境安装后继续验证。".into())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let app_dir = app.path().app_data_dir().map_err(|err| err.to_string())?;
            fs::create_dir_all(&app_dir).map_err(|err| err.to_string())?;
            let db_path = app_dir.join("3q-lang-helper.sqlite");
            let conn = Connection::open(db_path).map_err(|err| err.to_string())?;
            init_db(&conn)?;
            app.manage(AppState {
                db: Mutex::new(conn),
            });
            let _ = app
                .global_shortcut()
                .on_shortcut("ctrl+alt+q", |app, _shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }

                    show_main_window(app);

                    let _ = app.emit("3q-open-translate", ());
                });

            let _ = app
                .global_shortcut()
                .on_shortcut("ctrl+alt+s", |app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }

                    if shortcut.matches(Modifiers::CONTROL | Modifiers::ALT, Code::KeyS) {
                        show_main_window(app);

                        let _ = app.emit("3q-screenshot-translate", ());
                    }
                });

            let tray_menu = MenuBuilder::new(app)
                .text("show", "显示主窗口")
                .text("quit", "退出")
                .build()
                .map_err(|err| err.to_string())?;
            let mut tray_builder = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("3Q语言助手")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                    | TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    } => show_main_window(tray.app_handle()),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            let _ = tray_builder.build(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            translate_text,
            add_to_wordbook,
            list_wordbook,
            delete_wordbook_entry,
            get_daily_items,
            get_settings,
            save_settings,
            capture_and_translate
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if should_close_to_tray(window) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running 3Q语言助手");
}
