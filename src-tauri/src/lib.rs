use chrono::Local;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};

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
    level: String,
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
    libre_translate_url: String,
    open_ai_base_url: String,
    open_ai_api_key: String,
}

fn default_settings() -> AppSettings {
    AppSettings {
        default_english_target: "zh".into(),
        default_other_target: "en".into(),
        daily_language: "en".into(),
        daily_level: "beginner".into(),
        shortcut_translate: "Ctrl+Alt+Q".into(),
        shortcut_screenshot: "Ctrl+Alt+S".into(),
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
    .map_err(|err| err.to_string())
}

fn detect_language(text: &str) -> String {
    if text.chars().any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch)) {
        "zh".into()
    } else if text.chars().any(|ch| ('\u{3040}'..='\u{30ff}').contains(&ch)) {
        "ja".into()
    } else if text.chars().any(|ch| ('\u{ac00}'..='\u{d7af}').contains(&ch)) {
        "ko".into()
    } else if text.chars().any(|ch| ('\u{0400}'..='\u{04ff}').contains(&ch)) {
        "ru".into()
    } else if text.chars().any(|ch| ('\u{0600}'..='\u{06ff}').contains(&ch)) {
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

fn default_target_for(source_language: &str, settings: &AppSettings, explicit: Option<String>) -> String {
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
        Ok(Some(row.get::<_, String>(0).map_err(|err| err.to_string())?))
    } else {
        Ok(None)
    }
}

fn load_settings_from_db(conn: &Connection) -> Result<AppSettings, String> {
    let mut settings = default_settings();
    if let Some(raw) = setting_value(conn, "app_settings")? {
        settings = serde_json::from_str(&raw).unwrap_or(settings);
    }
    Ok(settings)
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

async fn english_dictionary(text: &str) -> (Option<String>, Vec<Definition>, Vec<String>, Vec<String>) {
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
                .and_then(|items| items.iter().find_map(|item| item.get("text").and_then(Value::as_str)))
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
                    let example = item.get("example").and_then(Value::as_str).map(ToOwned::to_owned);
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

    let translated_text = translate_with_mymemory(&clean_text, &source_language, &target)
        .await
        .unwrap_or_else(|| "免费翻译源暂时不可用，请稍后重试或在设置里配置高级翻译源。".into());

    let (phonetic, definitions, examples, phrases) = if is_word && source_language == "en" {
        english_dictionary(&clean_text).await
    } else {
        (None, vec![], vec![], vec![])
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
        provider: "MyMemory + Free Dictionary API".into(),
        is_word,
    })
}

#[tauri::command]
fn add_to_wordbook(state: State<'_, AppState>, item: Value) -> Result<WordbookEntry, String> {
    let now = Local::now().to_rfc3339();
    let entry = if item.get("sourceText").is_some() {
        let result: TranslationResult = serde_json::from_value(item).map_err(|err| err.to_string())?;
        WordbookEntry {
            id: format!("word-{}", Local::now().timestamp_nanos_opt().unwrap_or_default()),
            text: result.source_text,
            language: result.source_language,
            target_language: result.target_language,
            translation: result.translated_text,
            definitions: result.definitions,
            examples: result.examples,
            source: result.provider,
            created_at: now,
        }
    } else {
        let daily: DailyItem = serde_json::from_value(item).map_err(|err| err.to_string())?;
        WordbookEntry {
            id: format!("daily-{}", Local::now().timestamp_nanos_opt().unwrap_or_default()),
            text: daily.word,
            language: daily.language,
            target_language: "zh".into(),
            translation: daily.translation,
            definitions: vec![],
            examples: daily.examples,
            source: "daily learning".into(),
            created_at: now,
        }
    };

    let conn = state.db.lock().map_err(|err| err.to_string())?;
    conn.execute(
        "insert or replace into wordbook
        (id, text, language, target_language, translation, definitions_json, examples_json, source, created_at)
        values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            entry.id,
            entry.text,
            entry.language,
            entry.target_language,
            entry.translation,
            serde_json::to_string(&entry.definitions).map_err(|err| err.to_string())?,
            serde_json::to_string(&entry.examples).map_err(|err| err.to_string())?,
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
            "select id, text, language, target_language, translation, definitions_json, examples_json, source, created_at
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
                source: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|err| err.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|err| err.to_string())
}

fn daily_fallback(language: &str, level: &str) -> Vec<DailyItem> {
    let raw = match level {
        "zero" => vec![
            ("hello", "你好", ["Hello, my name is Q.", "She said hello with a smile.", "Hello is a friendly first word."]),
            ("book", "书", ["This book is easy.", "I read a book every night.", "Put the book on the desk."]),
            ("water", "水", ["I drink water.", "The water is cold.", "Please bring some water."]),
            ("friend", "朋友", ["He is my friend.", "A good friend listens.", "I met a new friend today."]),
            ("home", "家", ["I am going home.", "Home feels warm.", "She works from home."]),
        ],
        "skilled" => vec![
            ("nuance", "细微差别", ["The nuance matters in translation.", "She explained the nuance clearly.", "Context reveals nuance."]),
            ("fluent", "流利的", ["He became fluent through practice.", "Fluent speech sounds natural.", "She is fluent in three languages."]),
            ("context", "语境", ["Context changes the meaning.", "Check the context before translating.", "The word is formal in this context."]),
            ("retain", "记住；保留", ["Examples help you retain words.", "The app retains your notes.", "Sleep helps learners retain memory."]),
            ("phrase", "短语", ["Learn the whole phrase.", "This phrase sounds natural.", "A phrase can carry culture."]),
        ],
        "advanced" => vec![
            ("idiomatic", "地道的；惯用的", ["The sentence sounds idiomatic.", "Idiomatic English is hard to translate literally.", "She chose an idiomatic expression."]),
            ("ambiguity", "歧义", ["The translator resolved the ambiguity.", "Ambiguity can be useful in poetry.", "Context reduces ambiguity."]),
            ("register", "语域", ["Register affects word choice.", "This register is too formal.", "Learners should notice register."]),
            ("connotation", "隐含意义", ["The word has a warm connotation.", "Connotation differs from definition.", "Good translators track connotation."]),
            ("paraphrase", "改述", ["Paraphrase the idea in simple words.", "A paraphrase can clarify meaning.", "Try to paraphrase after reading."]),
        ],
        _ => vec![
            ("practice", "练习", ["Practice makes speaking easier.", "I practice English after dinner.", "Daily practice builds confidence."]),
            ("curious", "好奇的", ["A curious student asks questions.", "I am curious about this word.", "Curious minds learn faster."]),
            ("useful", "有用的", ["This phrase is useful.", "A notebook is useful for study.", "Useful examples help memory."]),
            ("improve", "提高", ["I want to improve my listening.", "Small habits improve fluency.", "Feedback helps you improve."]),
            ("sentence", "句子", ["Write one sentence.", "This sentence is clear.", "Read the sentence aloud."]),
        ],
    };

    raw.into_iter()
        .enumerate()
        .map(|(index, (word, translation, examples))| DailyItem {
            id: format!("{}-{}-{}", language, level, index),
            word: word.into(),
            language: language.into(),
            translation: translation.into(),
            examples: examples.into_iter().map(String::from).collect(),
            level: level.into(),
        })
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
    let cache_key = format!("{}:{}", language, level);
    let conn = state.db.lock().map_err(|err| err.to_string())?;

    if !force_refresh {
        let mut stmt = conn
            .prepare("select date, items_json from daily_cache where cache_key = ?1")
            .map_err(|err| err.to_string())?;
        let mut rows = stmt.query(params![cache_key]).map_err(|err| err.to_string())?;
        if let Some(row) = rows.next().map_err(|err| err.to_string())? {
            let date: String = row.get(0).map_err(|err| err.to_string())?;
            let items_json: String = row.get(1).map_err(|err| err.to_string())?;
            if date == today {
                return serde_json::from_str(&items_json).map_err(|err| err.to_string());
            }
        }
    }

    let items = daily_fallback(&language, &level);
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
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let _ = app.emit("global-shortcut", "pressed");
                    }
                })
                .build(),
        )
        .setup(|app| {
            let app_dir = app.path().app_data_dir().map_err(|err| err.to_string())?;
            fs::create_dir_all(&app_dir).map_err(|err| err.to_string())?;
            let db_path = app_dir.join("3q-lang-helper.sqlite");
            let conn = Connection::open(db_path).map_err(|err| err.to_string())?;
            init_db(&conn)?;
            app.manage(AppState {
                db: Mutex::new(conn),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            translate_text,
            add_to_wordbook,
            list_wordbook,
            get_daily_items,
            get_settings,
            save_settings,
            capture_and_translate
        ])
        .run(tauri::generate_context!())
        .expect("error while running 3Q语言助手");
}
