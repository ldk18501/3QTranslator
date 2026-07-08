use base64::{engine::general_purpose, Engine as _};
use chrono::{Local, Utc};
use hmac::{Hmac, Mac};
use image::{imageops::FilterType, GenericImageView};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{
    AppHandle,
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

struct AppState {
    db: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Definition {
    part_of_speech: String,
    meaning: String,
    #[serde(default)]
    meaning_translation: Option<String>,
    example: Option<String>,
    #[serde(default)]
    example_translation: Option<String>,
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
    #[serde(default)]
    example_translations: Vec<String>,
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
struct AiDailyPayload {
    items: Vec<DailyItem>,
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
    #[serde(default)]
    api_secret: String,
    #[serde(default)]
    region: String,
    model: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderTestResult {
    ok: bool,
    message: String,
    translated_text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScreenshotCapture {
    image_data_url: String,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScreenshotRegion {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    default_english_target: String,
    default_other_target: String,
    daily_language: String,
    daily_level: String,
    #[serde(default = "default_daily_cache_limit")]
    daily_cache_limit: i64,
    shortcut_translate: String,
    shortcut_screenshot: String,
    #[serde(default = "default_close_to_tray")]
    close_to_tray: bool,
    #[serde(default)]
    launch_at_startup: bool,
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

fn default_daily_cache_limit() -> i64 {
    120
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
            api_secret: String::new(),
            region: String::new(),
            model: String::new(),
        },
        ApiProvider {
            id: "libre-default".into(),
            name: "LibreTranslate".into(),
            provider_type: "libretranslate".into(),
            enabled: false,
            base_url: String::new(),
            api_key: String::new(),
            api_secret: String::new(),
            region: String::new(),
            model: String::new(),
        },
        ApiProvider {
            id: "openai-default".into(),
            name: "OpenAI-compatible".into(),
            provider_type: "openai".into(),
            enabled: false,
            base_url: String::new(),
            api_key: String::new(),
            api_secret: String::new(),
            region: String::new(),
            model: "gpt-4o-mini".into(),
        },
        ApiProvider {
            id: "tencent-default".into(),
            name: "腾讯云机器翻译".into(),
            provider_type: "tencent".into(),
            enabled: false,
            base_url: "https://tmt.tencentcloudapi.com".into(),
            api_key: String::new(),
            api_secret: String::new(),
            region: "ap-guangzhou".into(),
            model: String::new(),
        },
        ApiProvider {
            id: "azure-default".into(),
            name: "Azure Translator".into(),
            provider_type: "azure".into(),
            enabled: false,
            base_url: "https://api.cognitive.microsofttranslator.com".into(),
            api_key: String::new(),
            api_secret: String::new(),
            region: String::new(),
            model: String::new(),
        },
        ApiProvider {
            id: "deepl-default".into(),
            name: "DeepL API".into(),
            provider_type: "deepl".into(),
            enabled: false,
            base_url: "https://api-free.deepl.com/v2".into(),
            api_key: String::new(),
            api_secret: String::new(),
            region: String::new(),
            model: String::new(),
        },
        ApiProvider {
            id: "baidu-default".into(),
            name: "百度翻译开放平台".into(),
            provider_type: "baidu".into(),
            enabled: false,
            base_url: "https://fanyi-api.baidu.com/api/trans/vip/translate".into(),
            api_key: String::new(),
            api_secret: String::new(),
            region: String::new(),
            model: String::new(),
        },
    ]
}

fn default_settings() -> AppSettings {
    AppSettings {
        default_english_target: "zh".into(),
        default_other_target: "en".into(),
        daily_language: "en".into(),
        daily_level: "beginner".into(),
        daily_cache_limit: default_daily_cache_limit(),
        shortcut_translate: "Ctrl+Alt+Q".into(),
        shortcut_screenshot: "Ctrl+Alt+S".into(),
        close_to_tray: default_close_to_tray(),
        launch_at_startup: false,
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
    let lower = text.trim().to_lowercase();
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
    } else if lower.chars().any(|ch| "àâæçèêëîïôœùûÿ".contains(ch))
        || contains_any_word(
            &lower,
            &["le", "la", "les", "des", "une", "bonjour", "merci", "avec", "pour", "dans"],
        )
    {
        "fr".into()
    } else if lower.chars().any(|ch| "ñ¿¡".contains(ch))
        || contains_any_word(
            &lower,
            &["el", "la", "los", "las", "una", "gracias", "hola", "por", "para", "que", "con"],
        )
    {
        "es".into()
    } else if lower.chars().any(|ch| "äöüß".contains(ch))
        || contains_any_word(
            &lower,
            &["der", "die", "das", "und", "ich", "nicht", "mit", "fur", "für", "ist", "danke", "hallo"],
        )
    {
        "de".into()
    } else if lower.chars().any(|ch| "ãõçáâêíóôú".contains(ch)) {
        "pt".into()
    } else {
        "en".into()
    }
}

fn contains_any_word(text: &str, words: &[&str]) -> bool {
    text.split(|ch: char| !ch.is_alphabetic())
        .any(|part| words.contains(&part))
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

    for default_provider in default_api_providers() {
        if !settings
            .api_providers
            .iter()
            .any(|provider| provider.id == default_provider.id)
        {
            settings.api_providers.push(default_provider);
        }
    }

    for provider in settings.api_providers.iter_mut() {
        if provider.provider_type == "tencent" && provider.base_url.trim().is_empty() {
            provider.base_url = "https://tmt.tencentcloudapi.com".into();
        } else if provider.provider_type == "azure" && provider.base_url.trim().is_empty() {
            provider.base_url = "https://api.cognitive.microsofttranslator.com".into();
        } else if provider.provider_type == "deepl" && provider.base_url.trim().is_empty() {
            provider.base_url = "https://api-free.deepl.com/v2".into();
        } else if provider.provider_type == "baidu" && provider.base_url.trim().is_empty() {
            provider.base_url = "https://fanyi-api.baidu.com/api/trans/vip/translate".into();
        }
        if provider.provider_type == "tencent" && provider.region.trim().is_empty() {
            provider.region = "ap-guangzhou".into();
        }
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

#[cfg(target_os = "windows")]
fn set_launch_at_startup(enabled: bool) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run_key, _) = hkcu
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|err| format!("打开开机启动注册表失败：{}", err))?;
    let app_name = "3Q语言助手";
    if enabled {
        let exe = std::env::current_exe().map_err(|err| format!("读取程序路径失败：{}", err))?;
        run_key
            .set_value(app_name, &format!("\"{}\"", exe.to_string_lossy()))
            .map_err(|err| format!("设置开机启动失败：{}", err))?;
    } else {
        match run_key.delete_value(app_name) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("关闭开机启动失败：{}", err)),
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_launch_at_startup(_enabled: bool) -> Result<(), String> {
    Ok(())
}

fn validate_shortcut(label: &str, shortcut: &str) -> Result<String, String> {
    let trimmed = shortcut.trim();
    if trimmed.is_empty() {
        return Err(format!("{}快捷键不能为空", label));
    }
    if !trimmed.contains('+') {
        return Err(format!("{}快捷键需要包含修饰键，例如 Ctrl+Alt+Q", label));
    }
    Ok(trimmed.to_lowercase())
}

fn register_configured_shortcuts(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let translate_shortcut = validate_shortcut("呼出翻译窗口", &settings.shortcut_translate)?;
    let screenshot_shortcut = validate_shortcut("截图翻译", &settings.shortcut_screenshot)?;
    if translate_shortcut == screenshot_shortcut {
        return Err("两个快捷键不能相同".into());
    }

    app.global_shortcut()
        .unregister_all()
        .map_err(|err| format!("清理旧快捷键失败：{}", err))?;

    app.global_shortcut()
        .on_shortcut(translate_shortcut.as_str(), |app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            show_main_window(app);
            let _ = app.emit("3q-open-translate", ());
        })
        .map_err(|err| format!("注册呼出翻译窗口快捷键失败：{}", err))?;

    app.global_shortcut()
        .on_shortcut(screenshot_shortcut.as_str(), |app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            show_main_window(app);
            let _ = app.emit("3q-screenshot-translate", ());
        })
        .map_err(|err| format!("注册截图翻译快捷键失败：{}", err))?;

    Ok(())
}

async fn translate_with_mymemory(text: &str, source: &str, target: &str) -> Option<String> {
    let lang_pair = format!("{}|{}", source, target);
    let url = format!(
        "https://api.mymemory.translated.net/get?q={}&langpair={}",
        urlencoding::encode(text),
        urlencoding::encode(&lang_pair)
    );
    let response = reqwest::Client::new()
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    let data = response.json::<Value>().await.ok()?;
    data.pointer("/responseData/translatedText")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

async fn translate_with_google_free(text: &str, source: &str, target: &str) -> Option<String> {
    let source = if source.trim().is_empty() { "auto" } else { source };
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        urlencoding::encode(source),
        urlencoding::encode(target),
        urlencoding::encode(text)
    );
    let response = reqwest::Client::new()
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    let data = response.json::<Value>().await.ok()?;
    let translated = data
        .get(0)
        .and_then(Value::as_array)?
        .iter()
        .filter_map(|item| item.get(0).and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");
    Some(translated).filter(|value| !value.trim().is_empty())
}

fn normalized_translation_text(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .flat_map(char::to_lowercase)
        .collect()
}

fn translation_is_useful(source: &str, translated: &str, source_language: &str, target_language: &str) -> bool {
    let translated = translated.trim();
    if translated.is_empty() {
        return false;
    }
    if source_language != target_language
        && normalized_translation_text(source) == normalized_translation_text(translated)
    {
        return false;
    }
    true
}

fn provider_base_url<'a>(provider: &'a ApiProvider, fallback: &'a str) -> String {
    let value = provider.base_url.trim().trim_end_matches('/');
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn azure_language(code: &str) -> &str {
    match code {
        "zh" => "zh-Hans",
        "pt" => "pt-PT",
        value => value,
    }
}

fn deepl_language(code: &str, target: bool) -> &str {
    match code {
        "zh" => "ZH",
        "en" if target => "EN-US",
        "en" => "EN",
        "ja" => "JA",
        "ko" => "KO",
        "fr" => "FR",
        "de" => "DE",
        "es" => "ES",
        "ru" => "RU",
        "it" => "IT",
        "pt" if target => "PT-PT",
        "pt" => "PT",
        value => value,
    }
}

fn baidu_language(code: &str) -> &str {
    match code {
        "zh" => "zh",
        "ja" => "jp",
        "ko" => "kor",
        "fr" => "fra",
        "es" => "spa",
        "pt" => "pt",
        value => value,
    }
}

fn tencent_language(code: &str) -> &str {
    match code {
        "zh" => "zh",
        "ja" => "ja",
        "ko" => "ko",
        "fr" => "fr",
        "de" => "de",
        "es" => "es",
        "ru" => "ru",
        "it" => "it",
        "pt" => "pt",
        value => value,
    }
}

fn hmac_sha256(key: &[u8], message: &str) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(message.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn provider_config_error(provider: &ApiProvider) -> Option<String> {
    match provider.provider_type.as_str() {
        "tencent" => {
            if provider.api_key.trim().is_empty() {
                Some("腾讯云 SecretId 不能为空".into())
            } else if provider.api_secret.trim().is_empty() {
                Some("腾讯云 SecretKey 不能为空".into())
            } else {
                None
            }
        }
        "azure" => {
            if provider.api_key.trim().is_empty() {
                Some("Azure API Key 不能为空".into())
            } else if provider.region.trim().is_empty() {
                Some("Azure 区域不能为空，请填写资源所在区域，例如 eastasia".into())
            } else {
                None
            }
        }
        "deepl" => {
            if provider.api_key.trim().is_empty() {
                Some("DeepL API Key 不能为空".into())
            } else {
                None
            }
        }
        "baidu" => {
            if provider.api_key.trim().is_empty() {
                Some("百度 AppID 不能为空".into())
            } else if provider.api_secret.trim().is_empty() {
                Some("百度密钥不能为空".into())
            } else {
                None
            }
        }
        "openai" => {
            if provider.base_url.trim().is_empty() || provider.api_key.trim().is_empty() {
                Some("OpenAI-compatible 需要填写 Base URL 和 API Key".into())
            } else {
                None
            }
        }
        "libretranslate" => {
            if provider.base_url.trim().is_empty() {
                Some("LibreTranslate Base URL 不能为空".into())
            } else {
                None
            }
        }
        _ => None,
    }
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
        .timeout(Duration::from_secs(10))
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
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    let data = response.json::<Value>().await.ok()?;
    data.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn translate_with_azure(
    text: &str,
    source: &str,
    target: &str,
    provider: &ApiProvider,
) -> Option<String> {
    let api_key = provider.api_key.trim();
    if api_key.is_empty() {
        return None;
    }
    let base_url = provider_base_url(provider, "https://api.cognitive.microsofttranslator.com");
    let endpoint = if source == "auto" {
        format!(
            "{}/translate?api-version=3.0&to={}",
            base_url,
            azure_language(target)
        )
    } else {
        format!(
            "{}/translate?api-version=3.0&from={}&to={}",
            base_url,
            azure_language(source),
            azure_language(target)
        )
    };
    let mut request = reqwest::Client::new()
        .post(endpoint)
        .header("Ocp-Apim-Subscription-Key", api_key)
        .header("Content-Type", "application/json")
        .json(&json!([{ "Text": text }]));
    if !provider.region.trim().is_empty() {
        request = request.header("Ocp-Apim-Subscription-Region", provider.region.trim());
    }
    let data = request
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?
        .json::<Value>()
        .await
        .ok()?;
    data.pointer("/0/translations/0/text")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

async fn translate_with_deepl(
    text: &str,
    source: &str,
    target: &str,
    provider: &ApiProvider,
) -> Option<String> {
    translate_with_deepl_result(text, source, target, provider)
        .await
        .ok()
}

async fn translate_with_deepl_result(
    text: &str,
    source: &str,
    target: &str,
    provider: &ApiProvider,
) -> Result<String, String> {
    let api_key = provider.api_key.trim();
    if api_key.is_empty() {
        return Err("DeepL API Key 不能为空".into());
    }
    let base_url = provider_base_url(provider, "https://api-free.deepl.com/v2");
    let mut body = json!({
        "text": [text],
        "target_lang": deepl_language(target, true),
    });
    if source != "auto" {
        body["source_lang"] = json!(deepl_language(source, false));
    }
    let response = reqwest::Client::new()
        .post(format!("{}/translate", base_url))
        .header("Authorization", format!("DeepL-Auth-Key {}", api_key))
        .json(&body)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|err| format!("DeepL 请求失败：{}", err))?;
    let status = response.status();
    let raw = response
        .text()
        .await
        .map_err(|err| format!("DeepL 响应读取失败：{}", err))?;
    let data = serde_json::from_str::<Value>(&raw).ok();
    if !status.is_success() {
        let detail = data
            .as_ref()
            .and_then(|value| value.pointer("/message").and_then(Value::as_str))
            .unwrap_or_else(|| raw.trim())
            .chars()
            .take(300)
            .collect::<String>();
        return Err(format!(
            "DeepL 返回 HTTP {}：{}",
            status.as_u16(),
            if detail.is_empty() {
                "请检查 API Key、免费/Pro Base URL 和账号额度"
            } else {
                detail.as_str()
            }
        ));
    }
    let data = data.ok_or_else(|| "DeepL 返回内容不是有效 JSON".to_string())?;
    data.pointer("/translations/0/text")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "DeepL 返回中没有译文".to_string())
}

async fn translate_with_baidu(
    text: &str,
    source: &str,
    target: &str,
    provider: &ApiProvider,
) -> Option<String> {
    let app_id = provider.api_key.trim();
    let secret = provider.api_secret.trim();
    if app_id.is_empty() || secret.is_empty() {
        return None;
    }
    let base_url = provider_base_url(provider, "https://fanyi-api.baidu.com/api/trans/vip/translate");
    let salt = Local::now().timestamp_millis().to_string();
    let from = if source == "auto" { "auto" } else { baidu_language(source) };
    let to = baidu_language(target);
    let sign_raw = format!("{}{}{}{}", app_id, text, salt, secret);
    let sign = format!("{:x}", md5::compute(sign_raw.as_bytes()));
    let data = reqwest::Client::new()
        .post(base_url.clone())
        .form(&[
            ("q", text),
            ("from", from),
            ("to", to),
            ("appid", app_id),
            ("salt", &salt),
            ("sign", &sign),
        ])
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?
        .json::<Value>()
        .await
        .ok()?;
    let translated = data.get("trans_result")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(|item| item.get("dst").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    Some(translated).filter(|value| !value.trim().is_empty())
}

async fn translate_with_tencent(
    text: &str,
    source: &str,
    target: &str,
    provider: &ApiProvider,
) -> Option<String> {
    let secret_id = provider.api_key.trim();
    let secret_key = provider.api_secret.trim();
    if secret_id.is_empty() || secret_key.is_empty() {
        return None;
    }
    let base_url = provider_base_url(provider, "https://tmt.tencentcloudapi.com");
    let host = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("tmt.tencentcloudapi.com");
    let region = if provider.region.trim().is_empty() {
        "ap-guangzhou"
    } else {
        provider.region.trim()
    };
    let now = Utc::now();
    let timestamp = now.timestamp();
    let date = now.date_naive().to_string();
    let payload = json!({
        "SourceText": text,
        "Source": if source == "auto" { "auto" } else { tencent_language(source) },
        "Target": tencent_language(target),
        "ProjectId": 0
    })
    .to_string();
    let hashed_payload = hex::encode(Sha256::digest(payload.as_bytes()));
    let canonical_headers = format!(
        "content-type:application/json; charset=utf-8\nhost:{}\nx-tc-action:texttranslate\n",
        host
    );
    let canonical_request = format!(
        "POST\n/\n\n{}{}\n{}",
        canonical_headers,
        "content-type;host;x-tc-action",
        hashed_payload
    );
    let credential_scope = format!("{}/tmt/tc3_request", date);
    let string_to_sign = format!(
        "TC3-HMAC-SHA256\n{}\n{}\n{}",
        timestamp,
        credential_scope,
        hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );
    let secret_date = hmac_sha256(format!("TC3{}", secret_key).as_bytes(), &date);
    let secret_service = hmac_sha256(&secret_date, "tmt");
    let secret_signing = hmac_sha256(&secret_service, "tc3_request");
    let signature = hex::encode(hmac_sha256(&secret_signing, &string_to_sign));
    let authorization = format!(
        "TC3-HMAC-SHA256 Credential={}/{}, SignedHeaders=content-type;host;x-tc-action, Signature={}",
        secret_id, credential_scope, signature
    );
    let data = reqwest::Client::new()
        .post(base_url.clone())
        .header("Authorization", authorization)
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Host", host)
        .header("X-TC-Action", "TextTranslate")
        .header("X-TC-Version", "2018-03-21")
        .header("X-TC-Timestamp", timestamp.to_string())
        .header("X-TC-Region", region)
        .body(payload)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?
        .json::<Value>()
        .await
        .ok()?;
    data.pointer("/Response/TargetText")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
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

    let translated = if provider_config_error(&active).is_some() {
        None
    } else {
        match active.provider_type.as_str() {
            "libretranslate" => translate_with_libre(text, source, target, &active).await,
            "openai" => translate_with_openai(text, source, target, &active).await,
            "tencent" => translate_with_tencent(text, source, target, &active).await,
            "azure" => translate_with_azure(text, source, target, &active).await,
            "deepl" => translate_with_deepl(text, source, target, &active).await,
            "baidu" => translate_with_baidu(text, source, target, &active).await,
            _ => translate_with_mymemory(text, source, target).await,
        }
    };

    if let Some(translated) =
        translated.filter(|value| translation_is_useful(text, value, source, target))
    {
        return (translated, active.name);
    }

    if let Some(translated) = translate_with_google_free(text, source, target)
        .await
        .filter(|value| translation_is_useful(text, value, source, target))
    {
        let provider_name = if active.provider_type == "mymemory" {
            "Google 免费接口 fallback".into()
        } else {
            format!("{} → Google 免费接口 fallback", active.name)
        };
        return (translated, provider_name);
    }

    let fallback = translate_with_mymemory(text, source, target)
        .await
        .filter(|value| translation_is_useful(text, value, source, target))
        .unwrap_or_else(|| "翻译源暂时不可用，请稍后重试或检查设置里的 API 配置。".into());
    let provider_name = if active.provider_type == "mymemory" {
        "MyMemory 免费源".into()
    } else {
        format!("{} → MyMemory fallback", active.name)
    };
    (fallback, provider_name)
}

fn language_name(language: &str) -> &'static str {
    match language {
        "zh" => "中文",
        "en" => "英语",
        "ja" => "日语",
        "ko" => "韩语",
        "fr" => "法语",
        "de" => "德语",
        "es" => "西班牙语",
        "ru" => "俄语",
        "it" => "意大利语",
        "pt" => "葡萄牙语",
        "ar" => "阿拉伯语",
        _ => "目标语言",
    }
}

fn level_name(level: &str) -> &'static str {
    match level {
        "zero" => "完全不会",
        "skilled" => "熟练",
        "advanced" => "精通",
        _ => "入门",
    }
}

fn active_openai_provider(settings: &AppSettings) -> Option<ApiProvider> {
    settings
        .api_providers
        .iter()
        .find(|provider| {
            provider.enabled
                && provider.provider_type == "openai"
                && provider.id == settings.active_provider_id
                && !provider.base_url.trim().is_empty()
                && !provider.api_key.trim().is_empty()
        })
        .or_else(|| {
            settings.api_providers.iter().find(|provider| {
                provider.enabled
                    && provider.provider_type == "openai"
                    && !provider.base_url.trim().is_empty()
                    && !provider.api_key.trim().is_empty()
            })
        })
        .cloned()
}

fn strip_json_markdown(content: &str) -> &str {
    let trimmed = content.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        return rest.trim().trim_end_matches("```").trim();
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        return rest.trim().trim_end_matches("```").trim();
    }
    trimmed
}

fn validate_ai_daily_items(
    mut payload: AiDailyPayload,
    language: &str,
    level: &str,
    date: &str,
) -> Option<Vec<DailyItem>> {
    if payload.items.len() != 5 {
        return None;
    }
    for (index, item) in payload.items.iter_mut().enumerate() {
        if item.word.trim().is_empty()
            || item.translation.trim().is_empty()
            || item.examples.len() != 3
            || item.example_translations.len() != 3
            || item
                .examples
                .iter()
                .any(|example| example.trim().is_empty())
            || item
                .example_translations
                .iter()
                .any(|translation| translation.trim().is_empty())
        {
            return None;
        }
        item.id = format!("{}-{}-ai-{}-{}", language, level, date, index);
        item.language = language.into();
        item.level = level.into();
    }
    Some(payload.items)
}

async fn generate_daily_items_with_openai(
    language: &str,
    level: &str,
    date: &str,
    force_refresh: bool,
    settings: &AppSettings,
) -> Option<Vec<DailyItem>> {
    let provider = active_openai_provider(settings)?;
    let base_url = provider.base_url.trim().trim_end_matches('/');
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
    let translation_language = if language == "zh" { "英语" } else { "中文" };
    let refresh_instruction = if force_refresh {
        "这是用户手动刷新，请避开常规首选词，换一组同难度的新词。"
    } else {
        "这是当天首次生成，请选择适合长期学习的常用词。"
    };
    let prompt = format!(
        r#"为一款外语学习软件生成“每日学习”内容。
学习语言：{language_name}
学习难度：{level_name}
日期：{date}
翻译语言：{translation_language}
额外要求：{refresh_instruction}

请返回 5 组真实、自然、适合该难度的学习词汇。每组必须包含：
1. word：{language_name}中的真实常用词或常用短语，不要生僻词、乱码、词典标题、专有名词。
2. translation：用{translation_language}给出准确释义。
3. examples：3 条自然例句，必须使用 {language_name}，并且每句都要真实表达该词的常见用法。
4. exampleTranslations：3 条与 examples 一一对应的{translation_language}翻译。
5. level：必须是 "{level}"。

难度标准：
- 完全不会：问候、数字、家庭、食物、日常名词和最基础动词。
- 入门：高频日常表达、基础动词、常见形容词。
- 熟练：抽象但常用的学习、工作、交流词汇。
- 精通：语域、隐含意义、抽象表达、地道表达，但仍必须常用。

只返回 JSON，不要 Markdown，不要解释。格式必须完全如下：
{{"items":[{{"word":"...","translation":"...","examples":["...","...","..."],"exampleTranslations":["...","...","..."],"level":"{level}"}}]}}"#,
        language_name = language_name(language),
        level_name = level_name(level),
        date = date,
        translation_language = translation_language,
        refresh_instruction = refresh_instruction,
        level = level
    );

    for attempt in 0..2 {
        let body = json!({
            "model": model,
            "temperature": if force_refresh { 0.75 } else { 0.45 },
            "messages": [
                {
                    "role": "system",
                    "content": "你是严谨的外语老师和结构化 JSON 生成器。你必须只输出合法 JSON。"
                },
                {
                    "role": "user",
                    "content": if attempt == 0 {
                        prompt.clone()
                    } else {
                        format!("{}\n\n上一次输出未通过校验。请严格返回 5 项，每项 3 条例句和 3 条对应翻译，只输出合法 JSON。", prompt)
                    }
                }
            ]
        });
        let response = reqwest::Client::new()
            .post(&endpoint)
            .bearer_auth(provider.api_key.trim())
            .json(&body)
            .send()
            .await
            .ok()?;
        let data = response.json::<Value>().await.ok()?;
        let content = data
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)?;
        if let Ok(payload) = serde_json::from_str::<AiDailyPayload>(strip_json_markdown(content)) {
            if let Some(items) = validate_ai_daily_items(payload, language, level, date) {
                return Some(items);
            }
        }
    }
    None
}

async fn english_dictionary(
    text: &str,
    target_language: &str,
) -> (Option<String>, Vec<Definition>, Vec<String>, Vec<String>, Vec<String>) {
    let url = format!(
        "https://api.dictionaryapi.dev/api/v2/entries/en/{}",
        urlencoding::encode(text.trim())
    );
    let Ok(response) = reqwest::Client::new()
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
    else {
        return (None, vec![], vec![], vec![], vec![]);
    };
    let Ok(data) = response.json::<Value>().await else {
        return (None, vec![], vec![], vec![], vec![]);
    };
    let Some(entry) = data.as_array().and_then(|items| items.first()) else {
        return (None, vec![], vec![], vec![], vec![]);
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
    let mut example_translations = Vec::new();
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
                        let meaning_translation = if target_language != "en" {
                            translate_with_google_free(&meaning_text, "en", target_language).await
                        } else {
                            None
                        };
                        let example_translation = if target_language != "en" {
                            if let Some(example_text) = &example {
                                translate_with_google_free(example_text, "en", target_language).await
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        if let Some(value) = &example_translation {
                            example_translations.push(value.clone());
                        }
                        definitions.push(Definition {
                            part_of_speech: part.clone(),
                            meaning: meaning_text,
                            meaning_translation,
                            example,
                            example_translation,
                            synonyms,
                        });
                    }
                }
            }
        }
    }

    examples.truncate(6);
    example_translations.truncate(6);
    phrases.truncate(8);
    (phonetic, definitions, examples, example_translations, phrases)
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

    let (phonetic, definitions, examples, example_translations, phrases) = if is_word && source_language == "en" {
        english_dictionary(&clean_text, &target).await
    } else {
        (None, vec![], vec![], vec![], vec![])
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
        example_translations,
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
            examples: result
                .examples
                .iter()
                .enumerate()
                .map(|(index, example)| {
                    result
                        .example_translations
                        .get(index)
                        .filter(|translation| !translation.is_empty())
                        .map(|translation| format!("{}\n{}", example, translation))
                        .unwrap_or_else(|| example.clone())
                })
                .collect(),
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
        "delete from wordbook where text = ?1 and language = ?2 and target_language = ?3",
        params![entry.text, entry.language, entry.target_language],
    )
    .map_err(|err| err.to_string())?;
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

#[tauri::command]
fn update_wordbook_entry_level(
    state: State<'_, AppState>,
    id: String,
    level: String,
) -> Result<WordbookEntry, String> {
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    conn.execute(
        "update wordbook set level = ?1 where id = ?2",
        params![level, id],
    )
    .map_err(|err| err.to_string())?;

    let mut stmt = conn
        .prepare(
            "select id, text, language, target_language, translation, definitions_json, examples_json, level, source, created_at
             from wordbook where id = ?1",
        )
        .map_err(|err| err.to_string())?;
    let mut rows = stmt.query(params![id]).map_err(|err| err.to_string())?;
    let Some(row) = rows.next().map_err(|err| err.to_string())? else {
        return Err("单词不存在".into());
    };
    let definitions_json: String = row.get(5).map_err(|err| err.to_string())?;
    let examples_json: String = row.get(6).map_err(|err| err.to_string())?;
    Ok(WordbookEntry {
        id: row.get(0).map_err(|err| err.to_string())?,
        text: row.get(1).map_err(|err| err.to_string())?,
        language: row.get(2).map_err(|err| err.to_string())?,
        target_language: row.get(3).map_err(|err| err.to_string())?,
        translation: row.get(4).map_err(|err| err.to_string())?,
        definitions: serde_json::from_str(&definitions_json).unwrap_or_default(),
        examples: serde_json::from_str(&examples_json).unwrap_or_default(),
        level: row.get(7).map_err(|err| err.to_string())?,
        source: row.get(8).map_err(|err| err.to_string())?,
        created_at: row.get(9).map_err(|err| err.to_string())?,
    })
}

fn language_level_words(language: &str, level: &str) -> Option<Vec<(&'static str, &'static str)>> {
    let mut words = match (language, level) {
        ("en", "zero") => vec![
            ("hello", "你好"),
            ("book", "书"),
            ("water", "水"),
            ("friend", "朋友"),
            ("home", "家"),
        ],
        ("en", "beginner") => vec![
            ("practice", "练习"),
            ("curious", "好奇的"),
            ("useful", "有用的"),
            ("improve", "提高"),
            ("sentence", "句子"),
        ],
        ("en", "skilled") => vec![
            ("nuance", "细微差别"),
            ("fluent", "流利的"),
            ("context", "语境"),
            ("retain", "记住；保留"),
            ("phrase", "短语"),
        ],
        ("en", "advanced") => vec![
            ("idiomatic", "地道的；惯用的"),
            ("ambiguity", "歧义"),
            ("register", "语域"),
            ("connotation", "隐含意义"),
            ("paraphrase", "改述"),
        ],
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
    words.extend(language_level_extensions(language, level));
    Some(words)
}

fn language_level_extensions(language: &str, level: &str) -> Vec<(&'static str, &'static str)> {
    match (language, level) {
        ("en", "zero") => vec![
            ("food", "食物"),
            ("day", "一天；白天"),
            ("name", "名字"),
            ("good", "好的"),
            ("help", "帮助"),
            ("school", "学校"),
            ("family", "家庭"),
        ],
        ("en", "beginner") => vec![
            ("listen", "听"),
            ("remember", "记住"),
            ("question", "问题"),
            ("answer", "回答"),
            ("travel", "旅行"),
            ("morning", "早晨"),
            ("because", "因为"),
        ],
        ("en", "skilled") => vec![
            ("summarize", "总结"),
            ("accurate", "准确的"),
            ("contrast", "对比"),
            ("assume", "假设"),
            ("evidence", "证据"),
            ("specific", "具体的"),
            ("transfer", "迁移；转移"),
        ],
        ("en", "advanced") => vec![
            ("subtle", "微妙的"),
            ("coherence", "连贯性"),
            ("inference", "推断"),
            ("rhetoric", "修辞"),
            ("approximate", "近似的"),
            ("constraint", "约束"),
            ("interpretation", "解释；理解"),
        ],
        ("zh", "zero") => vec![
            ("饭", "meal"),
            ("人", "person"),
            ("名字", "name"),
            ("好", "good"),
            ("学校", "school"),
            ("朋友", "friend"),
            ("今天", "today"),
        ],
        ("zh", "beginner") => vec![
            ("问题", "question"),
            ("回答", "answer"),
            ("早上", "morning"),
            ("旅行", "travel"),
            ("需要", "need"),
            ("帮助", "help"),
            ("句子", "sentence"),
        ],
        ("zh", "skilled") => vec![
            ("准确", "accurate"),
            ("比较", "compare"),
            ("总结", "summarize"),
            ("证据", "evidence"),
            ("具体", "specific"),
            ("假设", "assumption"),
            ("短语", "phrase"),
        ],
        ("zh", "advanced") => vec![
            ("连贯性", "coherence"),
            ("推断", "inference"),
            ("修辞", "rhetoric"),
            ("约束", "constraint"),
            ("诠释", "interpretation"),
            ("近似", "approximation"),
            ("细微差别", "nuance"),
        ],
        ("ja", "zero") => vec![
            ("人", "人"),
            ("名前", "名字"),
            ("学校", "学校"),
            ("友達", "朋友"),
            ("今日", "今天"),
            ("良い", "好的"),
            ("食べ物", "食物"),
        ],
        ("ja", "beginner") => vec![
            ("質問", "问题"),
            ("答え", "回答"),
            ("旅行", "旅行"),
            ("必要", "需要"),
            ("助ける", "帮助"),
            ("文", "句子"),
            ("覚える", "记住"),
        ],
        ("ja", "skilled") => vec![
            ("正確", "准确"),
            ("比較", "比较"),
            ("要約", "总结"),
            ("証拠", "证据"),
            ("具体的", "具体的"),
            ("仮定", "假设"),
            ("変換", "转化"),
        ],
        ("ja", "advanced") => vec![
            ("一貫性", "连贯性"),
            ("推論", "推断"),
            ("修辞", "修辞"),
            ("制約", "约束"),
            ("解釈", "解释"),
            ("近似", "近似"),
            ("微妙", "微妙的"),
        ],
        ("ko", "zero") => vec![
            ("사람", "人"),
            ("이름", "名字"),
            ("학교", "学校"),
            ("친구", "朋友"),
            ("오늘", "今天"),
            ("좋다", "好的"),
            ("음식", "食物"),
        ],
        ("ko", "beginner") => vec![
            ("질문", "问题"),
            ("대답", "回答"),
            ("여행", "旅行"),
            ("도움", "帮助"),
            ("문장", "句子"),
            ("기억하다", "记住"),
            ("좋아하다", "喜欢"),
        ],
        ("ko", "skilled") => vec![
            ("정확한", "准确的"),
            ("비교", "比较"),
            ("요약", "总结"),
            ("증거", "证据"),
            ("구체적", "具体的"),
            ("가정", "假设"),
            ("전환", "转化"),
        ],
        ("ko", "advanced") => vec![
            ("일관성", "连贯性"),
            ("추론", "推断"),
            ("수사", "修辞"),
            ("제약", "约束"),
            ("해석", "解释"),
            ("근사", "近似"),
            ("미묘함", "微妙"),
        ],
        ("fr", "zero") => vec![
            ("personne", "人"),
            ("nom", "名字"),
            ("école", "学校"),
            ("bon", "好的"),
            ("nourriture", "食物"),
            ("jour", "一天"),
            ("famille", "家庭"),
        ],
        ("fr", "beginner") => vec![
            ("question", "问题"),
            ("réponse", "回答"),
            ("pratiquer", "练习"),
            ("voyager", "旅行"),
            ("besoin", "需要"),
            ("aider", "帮助"),
            ("phrase", "句子"),
        ],
        ("fr", "skilled") => vec![
            ("précis", "准确的"),
            ("comparer", "比较"),
            ("résumer", "总结"),
            ("preuve", "证据"),
            ("spécifique", "具体的"),
            ("hypothèse", "假设"),
            ("formulation", "措辞"),
        ],
        ("fr", "advanced") => vec![
            ("cohérence", "连贯性"),
            ("inférence", "推断"),
            ("rhétorique", "修辞"),
            ("contrainte", "约束"),
            ("interprétation", "解释"),
            ("approximation", "近似"),
            ("idiomatique", "地道的"),
        ],
        ("de", "zero") => vec![
            ("Mensch", "人"),
            ("Name", "名字"),
            ("Schule", "学校"),
            ("gut", "好的"),
            ("Essen", "食物"),
            ("Tag", "一天"),
            ("Familie", "家庭"),
        ],
        ("de", "beginner") => vec![
            ("Frage", "问题"),
            ("Antwort", "回答"),
            ("üben", "练习"),
            ("reisen", "旅行"),
            ("brauchen", "需要"),
            ("helfen", "帮助"),
            ("Satz", "句子"),
        ],
        ("de", "skilled") => vec![
            ("genau", "准确的"),
            ("vergleichen", "比较"),
            ("zusammenfassen", "总结"),
            ("Beweis", "证据"),
            ("spezifisch", "具体的"),
            ("Annahme", "假设"),
            ("Wendung", "短语"),
        ],
        ("de", "advanced") => vec![
            ("Kohärenz", "连贯性"),
            ("Schlussfolgerung", "推断"),
            ("Rhetorik", "修辞"),
            ("Einschränkung", "约束"),
            ("Interpretation", "解释"),
            ("Annäherung", "近似"),
            ("idiomatisch", "地道的"),
        ],
        ("es", "zero") => vec![
            ("persona", "人"),
            ("nombre", "名字"),
            ("escuela", "学校"),
            ("bueno", "好的"),
            ("comida", "食物"),
            ("día", "一天"),
            ("familia", "家庭"),
        ],
        ("es", "beginner") => vec![
            ("pregunta", "问题"),
            ("respuesta", "回答"),
            ("practicar", "练习"),
            ("viajar", "旅行"),
            ("necesitar", "需要"),
            ("ayudar", "帮助"),
            ("frase", "句子"),
        ],
        ("es", "skilled") => vec![
            ("preciso", "准确的"),
            ("comparar", "比较"),
            ("resumir", "总结"),
            ("evidencia", "证据"),
            ("específico", "具体的"),
            ("suposición", "假设"),
            ("matiz", "细微差别"),
        ],
        ("es", "advanced") => vec![
            ("coherencia", "连贯性"),
            ("inferencia", "推断"),
            ("retórica", "修辞"),
            ("restricción", "约束"),
            ("interpretación", "解释"),
            ("aproximación", "近似"),
            ("idiomático", "地道的"),
        ],
        _ => vec![],
    }
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

fn daily_variant(language: &str, level: &str, date: &str, force_refresh: bool) -> usize {
    let source = if force_refresh {
        format!(
            "{}:{}:{}:{}",
            date,
            language,
            level,
            Local::now().timestamp()
        )
    } else {
        format!("{}:{}:{}", date, language, level)
    };
    source.chars().fold(0usize, |sum, ch| {
        sum.wrapping_mul(31).wrapping_add(ch as usize)
    })
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
async fn get_daily_items(
    state: State<'_, AppState>,
    language: String,
    level: String,
    force_refresh: bool,
) -> Result<Vec<DailyItem>, String> {
    let today = Local::now().date_naive().to_string();
    let cache_key = format!("v5:{}:{}", language, level);
    if !force_refresh {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
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

    let settings = {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        load_settings_from_db(&conn)?
    };
    let variant = daily_variant(&language, &level, &today, force_refresh);
    let items =
        generate_daily_items_with_openai(&language, &level, &today, force_refresh, &settings)
            .await
            .unwrap_or_else(|| daily_fallback(&language, &level, variant));
    {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        conn.execute(
            "insert or replace into daily_cache(cache_key, date, items_json) values (?1, ?2, ?3)",
            params![
                cache_key,
                today,
                serde_json::to_string(&items).map_err(|err| err.to_string())?
            ],
        )
        .map_err(|err| err.to_string())?;
        trim_daily_cache(&conn, settings.daily_cache_limit)?;
    }

    Ok(items)
}

fn trim_daily_cache(conn: &Connection, limit: i64) -> Result<(), String> {
    let limit = limit.clamp(20, 1000);
    conn.execute(
        "delete from daily_cache
         where rowid not in (
             select rowid from daily_cache order by date desc, rowid desc limit ?1
         )",
        params![limit],
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    load_settings_from_db(&conn)
}

#[tauri::command]
async fn test_api_provider(provider: ApiProvider) -> Result<ProviderTestResult, String> {
    if let Some(message) = provider_config_error(&provider) {
        return Ok(ProviderTestResult {
            ok: false,
            message,
            translated_text: None,
        });
    }
    let translated = match provider.provider_type.as_str() {
        "libretranslate" => translate_with_libre("hello", "en", "zh", &provider).await,
        "openai" => translate_with_openai("hello", "en", "zh", &provider).await,
        "tencent" => translate_with_tencent("hello", "en", "zh", &provider).await,
        "azure" => translate_with_azure("hello", "en", "zh", &provider).await,
        "deepl" => match translate_with_deepl_result("hello", "en", "zh", &provider).await {
            Ok(translated) => Some(translated),
            Err(message) => {
                return Ok(ProviderTestResult {
                    ok: false,
                    message,
                    translated_text: None,
                });
            }
        },
        "baidu" => translate_with_baidu("hello", "en", "zh", &provider).await,
        _ => translate_with_mymemory("hello", "en", "zh").await,
    };
    if let Some(translated) = translated.filter(|value| !value.trim().is_empty()) {
        return Ok(ProviderTestResult {
            ok: true,
            message: format!("{} 返回正常", provider.name),
            translated_text: Some(translated),
        });
    }
    Ok(ProviderTestResult {
        ok: false,
        message: format!(
            "{} 连接测试失败：接口未返回有效译文。请检查 Base URL、密钥、区域、服务是否开通以及免费额度是否可用。",
            provider.name
        ),
        translated_text: None,
    })
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let previous = {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        load_settings_from_db(&conn)?
    };
    let settings = normalize_settings(settings);
    if let Err(err) = register_configured_shortcuts(&app, &settings) {
        let _ = register_configured_shortcuts(&app, &previous);
        return Err(err);
    }
    if let Err(err) = set_launch_at_startup(settings.launch_at_startup) {
        let _ = register_configured_shortcuts(&app, &previous);
        let _ = set_launch_at_startup(previous.launch_at_startup);
        return Err(err);
    }

    let conn = state.db.lock().map_err(|err| err.to_string())?;
    if let Err(err) = conn.execute(
        "insert or replace into settings(key, value) values ('app_settings', ?1)",
        params![serde_json::to_string(&settings).map_err(|err| err.to_string())?],
    ) {
        let _ = register_configured_shortcuts(&app, &previous);
        let _ = set_launch_at_startup(previous.launch_at_startup);
        return Err(err.to_string());
    }
    Ok(settings)
}

#[tauri::command]
async fn capture_and_translate(state: State<'_, AppState>) -> Result<TranslationResult, String> {
    let clean_text = ocr_primary_monitor_text().await?;
    translate_ocr_text(state, clean_text, "Windows OCR").await
}

async fn translate_ocr_text(
    state: State<'_, AppState>,
    clean_text: String,
    source_label: &str,
) -> Result<TranslationResult, String> {
    let settings = {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        load_settings_from_db(&conn)?
    };
    let source_language = detect_language(&clean_text);
    let target = default_target_for(&source_language, &settings, None);
    let is_word = looks_like_word(&clean_text);
    let (translated_text, provider_name) =
        translate_with_configured_provider(&clean_text, &source_language, &target, &settings).await;
    let (phonetic, definitions, examples, example_translations, phrases) = if is_word && source_language == "en" {
        english_dictionary(&clean_text, &target).await
    } else {
        (None, vec![], vec![], vec![], vec![])
    };

    Ok(TranslationResult {
        source_text: clean_text,
        source_language,
        target_language: target,
        translated_text,
        phonetic,
        definitions,
        examples,
        example_translations,
        phrases,
        provider: format!("{} + {}", provider_name, source_label),
        is_word,
    })
}

#[tauri::command]
async fn capture_screenshot(app: AppHandle) -> Result<ScreenshotCapture, String> {
    let window = app.get_webview_window("main");
    if let Some(window) = &window {
        let _ = window.hide();
        std::thread::sleep(Duration::from_millis(180));
    }
    let capture = capture_primary_monitor_png();
    if let Some(window) = &window {
        let _ = window.set_decorations(false);
        let _ = window.set_always_on_top(true);
        let _ = window.set_fullscreen(true);
        let _ = window.show();
        let _ = window.set_focus();
    }
    let (png, width, height) = capture?;
    Ok(ScreenshotCapture {
        image_data_url: format!(
            "data:image/png;base64,{}",
            general_purpose::STANDARD.encode(png)
        ),
        width,
        height,
    })
}

#[tauri::command]
fn exit_screenshot_mode(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_fullscreen(false);
        let _ = window.set_always_on_top(false);
        let _ = window.set_decorations(true);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
async fn translate_screenshot_region(
    state: State<'_, AppState>,
    image_data_url: String,
    region: ScreenshotRegion,
) -> Result<TranslationResult, String> {
    let png = crop_screenshot_region(&image_data_url, &region)?;
    let text = recognize_png_bytes_with_windows_ocr(&png).await?;
    let clean_text = clean_ocr_text(&text);
    if clean_text.is_empty() {
        return Err("选区中没有识别到文字".into());
    }
    translate_ocr_text(state, clean_text, "Windows OCR 选区").await
}

async fn ocr_primary_monitor_text() -> Result<String, String> {
    let (png, _, _) = capture_primary_monitor_png()?;
    let text = recognize_png_bytes_with_windows_ocr(&png).await?;
    let clean_text = clean_ocr_text(&text);
    if clean_text.is_empty() {
        return Err("截图中没有识别到文字".into());
    }
    Ok(clean_text)
}

fn capture_primary_monitor_png() -> Result<(Vec<u8>, u32, u32), String> {
    let monitors = xcap::Monitor::all().map_err(|err| format!("无法读取显示器：{}", err))?;
    let monitor = monitors
        .iter()
        .find(|monitor| monitor.is_primary())
        .or_else(|| monitors.first())
        .ok_or_else(|| "未找到可截图的显示器".to_string())?;
    let image = monitor
        .capture_image()
        .map_err(|err| format!("截图失败：{}", err))?;
    let (width, height) = image.dimensions();
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|err| format!("截图编码失败：{}", err))?;
    Ok((png, width, height))
}

fn clean_ocr_text(text: &str) -> String {
    text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn crop_screenshot_region(image_data_url: &str, region: &ScreenshotRegion) -> Result<Vec<u8>, String> {
    if region.width < 4.0 || region.height < 4.0 {
        return Err("请先框选需要识别的区域".into());
    }
    let encoded = image_data_url
        .split_once(',')
        .map(|(_, data)| data)
        .unwrap_or(image_data_url);
    let bytes = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|err| format!("读取截图数据失败：{}", err))?;
    let image = image::load_from_memory(&bytes).map_err(|err| format!("解析截图失败：{}", err))?;
    let (image_width, image_height) = image.dimensions();
    let margin = 14.0;
    let x = (region.x - margin)
        .max(0.0)
        .min(image_width.saturating_sub(1) as f64)
        .floor() as u32;
    let y = (region.y - margin)
        .max(0.0)
        .min(image_height.saturating_sub(1) as f64)
        .floor() as u32;
    let right = (region.x + region.width + margin)
        .max((x + 1) as f64)
        .min(image_width as f64)
        .ceil() as u32;
    let bottom = (region.y + region.height + margin)
        .max((y + 1) as f64)
        .min(image_height as f64)
        .ceil() as u32;
    let width = right.saturating_sub(x);
    let height = bottom.saturating_sub(y);
    if width < 4 || height < 4 {
        return Err("选区太小，请重新框选".into());
    }
    let cropped = preprocess_ocr_image(image.crop_imm(x, y, width, height));
    let mut png = Vec::new();
    cropped
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|err| format!("选区编码失败：{}", err))?;
    Ok(png)
}

fn preprocess_ocr_image(image: image::DynamicImage) -> image::DynamicImage {
    let (width, height) = image.dimensions();
    let min_width = 720.0;
    let min_height = 180.0;
    let scale = (min_width / width.max(1) as f64)
        .max(min_height / height.max(1) as f64)
        .clamp(1.0, 3.5);
    if scale <= 1.05 {
        return image;
    }
    let target_width = ((width as f64) * scale).round().max(width as f64) as u32;
    let target_height = ((height as f64) * scale).round().max(height as f64) as u32;
    image.resize(target_width, target_height, FilterType::CatmullRom)
}

async fn recognize_png_bytes_with_windows_ocr(png: &[u8]) -> Result<String, String> {
    use windows::core::HSTRING;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapDecoder, BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

    let stream = InMemoryRandomAccessStream::new().map_err(|err| format!("创建 OCR 输入流失败：{}", err))?;
    let writer = DataWriter::CreateDataWriter(&stream).map_err(|err| format!("创建 OCR 写入器失败：{}", err))?;
    writer
        .WriteBytes(png)
        .map_err(|err| format!("写入截图失败：{}", err))?;
    writer
        .StoreAsync()
        .map_err(|err| format!("提交截图失败：{}", err))?
        .get()
        .map_err(|err| format!("提交截图失败：{}", err))?;
    writer
        .FlushAsync()
        .map_err(|err| format!("刷新截图流失败：{}", err))?
        .get()
        .map_err(|err| format!("刷新截图流失败：{}", err))?;
    stream.Seek(0).map_err(|err| format!("重置截图流失败：{}", err))?;

    let decoder = BitmapDecoder::CreateAsync(&stream)
        .map_err(|err| format!("创建截图解码器失败：{}", err))?
        .get()
        .map_err(|err| format!("解码截图失败：{}", err))?;
    let bitmap = decoder
        .GetSoftwareBitmapAsync()
        .map_err(|err| format!("读取截图像素失败：{}", err))?
        .get()
        .map_err(|err| format!("读取截图像素失败：{}", err))?;
    let bitmap = SoftwareBitmap::ConvertWithAlpha(
        &bitmap,
        BitmapPixelFormat::Bgra8,
        BitmapAlphaMode::Premultiplied,
    )
    .map_err(|err| format!("转换截图像素格式失败：{}", err))?;
    let mut engines = Vec::new();
    for tag in ["zh-Hans", "zh-Hant", "en-US"] {
        if let Ok(language) = Language::CreateLanguage(&HSTRING::from(tag)) {
            if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&language) {
                engines.push(engine);
            }
        }
    }
    if let Ok(engine) = OcrEngine::TryCreateFromUserProfileLanguages() {
        engines.push(engine);
    }
    if engines.is_empty() {
        return Err("创建 Windows OCR 引擎失败，请确认系统已安装 OCR 语言包".into());
    }

    let mut best_text = String::new();
    for engine in engines {
        let Ok(result) = engine.RecognizeAsync(&bitmap).and_then(|operation| operation.get()) else {
            continue;
        };
        let Ok(text) = result.Text().map(|value| value.to_string()) else {
            continue;
        };
        if text.chars().filter(|ch| !ch.is_whitespace()).count()
            > best_text.chars().filter(|ch| !ch.is_whitespace()).count()
        {
            best_text = text;
        }
    }
    if best_text.trim().is_empty() {
        return Err("OCR 未识别到文字，请扩大选区或确认系统已安装对应语言的 OCR 包".into());
    }
    Ok(best_text)
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
            let settings = load_settings_from_db(&conn).unwrap_or_else(|_| default_settings());
            app.manage(AppState {
                db: Mutex::new(conn),
            });
            if register_configured_shortcuts(app.handle(), &settings).is_err() {
                let _ = register_configured_shortcuts(app.handle(), &default_settings());
            }
            let _ = set_launch_at_startup(settings.launch_at_startup);

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
            update_wordbook_entry_level,
            get_daily_items,
            get_settings,
            test_api_provider,
            save_settings,
            capture_and_translate,
            capture_screenshot,
            exit_screenshot_mode,
            translate_screenshot_region
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
