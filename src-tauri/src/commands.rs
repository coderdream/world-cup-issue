use crate::{
    database::{default_license, AppError, Database},
    ensure_scorebar,
    http_client::build_clients,
    match_status::normalize_matches,
    models::*,
    open_data::{merge_matches_with_football_data, refresh_football_data, refresh_open_football, test_football_data_token as run_football_data_token_test},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub message: String,
}

impl From<AppError> for CommandError {
    fn from(value: AppError) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

fn sample_teams() -> Vec<Team> {
    vec![
        team("mex", "MEX", "墨西哥", "Mexico", "A", "MX", 1950),
        team("rsa", "RSA", "南非", "South Africa", "A", "ZA", 1690),
        team("kor", "KOR", "韩国", "South Korea", "A", "KR", 1840),
        team("cze", "CZE", "捷克", "Czech Republic", "A", "CZ", 1770),
        team("fra", "FRA", "法国", "France", "I", "FR", 2085),
        team("sen", "SEN", "塞内加尔", "Senegal", "I", "SN", 1825),
        team("irq", "IRQ", "伊拉克", "Iraq", "I", "IQ", 1680),
        team("nor", "NOR", "挪威", "Norway", "I", "NO", 1860),
        team("arg", "ARG", "阿根廷", "Argentina", "J", "AR", 2055),
        team("alg", "ALG", "阿尔及利亚", "Algeria", "J", "DZ", 1785),
        team("aut", "AUT", "奥地利", "Austria", "J", "AT", 1845),
        team("jor", "JOR", "约旦", "Jordan", "J", "JO", 1600),
    ]
}

fn sample_matches() -> Vec<Match> {
    normalize_matches(vec![
        game("m001", "A", "2026-06-12", "03:00", "mex", "rsa", Some(2), Some(0), "finished", "Mexico City"),
        game("m002", "A", "2026-06-12", "10:00", "kor", "cze", Some(2), Some(1), "finished", "Guadalajara (Zapopan)"),
        game("m017", "I", "2026-06-17", "03:00", "fra", "sen", Some(3), Some(1), "finished", "New York/New Jersey (East Rutherford)"),
        game("m018", "I", "2026-06-17", "06:00", "irq", "nor", Some(1), Some(4), "finished", "Boston (Foxborough)"),
        game("m019", "J", "2026-06-17", "09:00", "arg", "alg", None, None, "scheduled", "Kansas City"),
        game("m020", "J", "2026-06-17", "12:00", "aut", "jor", None, None, "scheduled", "San Francisco Bay Area (Santa Clara)"),
    ])
}

#[tauri::command]
pub fn get_app_state(db: State<'_, Database>) -> Result<AppStatePayload, CommandError> {
    let cached_matches = db.get_cached_matches()?;
    Ok(AppStatePayload {
        teams: sample_teams(),
        matches: normalize_matches(if cached_matches.len() >= 104 { cached_matches } else { sample_matches() }),
        settings: db.get_settings()?,
        predictions: db.get_predictions()?,
        license: default_license(),
        last_updated: db.get_latest_refresh_label()?,
    })
}

#[tauri::command]
pub async fn refresh_matches(db: State<'_, Database>) -> Result<RefreshMatchesResult, CommandError> {
    match refresh_open_football().await {
        Ok(refresh) => {
            let settings = db.get_settings()?;
            let mut source = refresh.source;
            let mut payload = refresh.payload;
            let mut matches = normalize_matches(refresh.matches);
            let mut updated_at = Some(format_beijing_minute_now());

            match refresh_football_data(&settings.football_data_token).await {
                Ok(Some(football_data)) => {
                    log::info!(
                        "merged {} matches from {} into openfootball refresh",
                        football_data.matches.len(),
                        football_data.source
                    );
                    matches = merge_matches_with_football_data(matches, football_data.matches);
                    source = format!("{source}+{}", football_data.source);
                    payload = format!(
                        "{{\"openfootball\":{},\"footballData\":{}}}",
                        payload, football_data.payload
                    );
                }
                Ok(None) => {
                    log::info!("football-data token is empty; using openfootball refresh only");
                }
                Err(err) => {
                    log::warn!("football-data incremental refresh failed: {err}");
                }
            }

            let matches = normalize_matches(matches);
            log::info!("refreshed {} matches from {}", matches.len(), source);
            db.cache_matches(&source, &payload, &matches)?;
            Ok(RefreshMatchesResult {
                matches,
                last_updated: updated_at.take(),
            })
        }
        Err(err) => {
            log::warn!("open data refresh failed: {err}");
            let cached = db.get_cached_matches()?;
            if cached.len() >= 16 {
                Ok(RefreshMatchesResult {
                    matches: normalize_matches(cached),
                    last_updated: None,
                })
            } else {
                Ok(RefreshMatchesResult {
                    matches: sample_matches(),
                    last_updated: None,
                })
            }
        }
    }
}

#[tauri::command]
pub fn get_matches(db: State<'_, Database>) -> Result<Vec<Match>, CommandError> {
    let cached = db.get_cached_matches()?;
    Ok(normalize_matches(if cached.len() >= 16 { cached } else { sample_matches() }))
}

#[tauri::command]
pub fn get_standings() -> Vec<serde_json::Value> {
    Vec::new()
}

#[tauri::command]
pub fn get_bracket() -> Vec<BracketNode> {
    vec![BracketNode {
        id: "final".to_string(),
        round: "final".to_string(),
        slot_a: "W101".to_string(),
        slot_b: "W102".to_string(),
        venue: Some("New York/New Jersey (East Rutherford)".to_string()),
        winner: None,
    }]
}

#[tauri::command]
pub fn get_teams() -> Vec<Team> {
    sample_teams()
}

#[tauri::command]
pub fn toggle_favorite_team(db: State<'_, Database>, team_id: String) -> Result<Vec<String>, CommandError> {
    Ok(db.toggle_favorite(&team_id)?)
}

#[tauri::command]
pub fn save_prediction(db: State<'_, Database>, prediction: Prediction) -> Result<Vec<Prediction>, CommandError> {
    Ok(db.save_prediction(&prediction)?)
}

#[tauri::command]
pub fn get_predictions(db: State<'_, Database>) -> Result<Vec<Prediction>, CommandError> {
    Ok(db.get_predictions()?)
}

#[tauri::command]
pub fn get_settings(db: State<'_, Database>) -> Result<AppSettings, CommandError> {
    Ok(db.get_settings()?)
}

#[tauri::command]
pub fn set_settings(db: State<'_, Database>, settings: AppSettings) -> Result<AppSettings, CommandError> {
    Ok(db.set_settings(&settings)?)
}

#[tauri::command]
pub async fn test_football_data_token(token: String) -> Result<ConnectivityTestResult, CommandError> {
    Ok(run_football_data_token_test(&token).await)
}

#[tauri::command]
pub async fn test_ai_model_config(config: AiModelConfig) -> Result<ConnectivityTestResult, CommandError> {
    Ok(run_ai_model_config_test(config).await)
}

#[tauri::command]
pub async fn generate_ai_evaluation(config: AiModelConfig, context: AiEvaluationContext) -> Result<AiGenerationResult, CommandError> {
    Ok(run_ai_evaluation(config, context).await)
}

#[tauri::command]
pub fn toggle_spoiler_mode(db: State<'_, Database>) -> Result<AppSettings, CommandError> {
    let mut settings = db.get_settings()?;
    settings.spoiler_mode = !settings.spoiler_mode;
    Ok(db.set_settings(&settings)?)
}

#[tauri::command]
pub fn open_floating_scorebar(app: AppHandle, db: State<'_, Database>) -> Result<AppSettings, CommandError> {
    ensure_scorebar(&app).map_err(|err| CommandError { message: err.to_string() })?;
    let mut settings = db.get_settings()?;
    settings.scorebar_enabled = true;
    Ok(db.set_settings(&settings)?)
}

#[tauri::command]
pub fn close_floating_scorebar(app: AppHandle, db: State<'_, Database>) -> Result<AppSettings, CommandError> {
    if let Some(window) = app.get_webview_window("scorebar") {
        let _ = window.close();
    }
    let mut settings = db.get_settings()?;
    settings.scorebar_enabled = false;
    Ok(db.set_settings(&settings)?)
}

#[tauri::command]
pub fn check_update_mock() -> UpdateInfo {
    UpdateInfo {
        current_version: "0.1.11".to_string(),
        latest_version: "0.1.11".to_string(),
        available: false,
        notes: "当前已是 WorldCupIssue（世界杯组手）本地复刻版最新版本。".to_string(),
    }
}

fn team(id: &str, code: &str, zh: &str, en: &str, group: &str, flag: &str, elo: i64) -> Team {
    Team {
        id: id.to_string(),
        code: code.to_string(),
        name_zh: zh.to_string(),
        name_en: en.to_string(),
        group: group.to_string(),
        flag: flag.to_string(),
        elo,
    }
}

#[allow(clippy::too_many_arguments)]
fn game(
    id: &str,
    group: &str,
    date: &str,
    time: &str,
    home: &str,
    away: &str,
    score_home: Option<i64>,
    score_away: Option<i64>,
    status: &str,
    venue: &str,
) -> Match {
    Match {
        id: id.to_string(),
        group: group.to_string(),
        stage: "小组赛".to_string(),
        date: date.to_string(),
        time: time.to_string(),
        utc_offset: "UTC+8".to_string(),
        home_team_id: home.to_string(),
        away_team_id: away.to_string(),
        score: Score {
            home: score_home,
            away: score_away,
        },
        status: status.to_string(),
        venue: venue.to_string(),
        city: venue.to_string(),
        updated_at: Some("15:42:01".to_string()),
    }
}

fn format_beijing_minute_now() -> String {
    let beijing = chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).expect("valid offset"));
    beijing.format("%H:%M").to_string()
}

async fn run_ai_evaluation(config: AiModelConfig, context: AiEvaluationContext) -> AiGenerationResult {
    let api_key = config.api_key.trim();
    let model = config.model.trim();
    let base_url = config.base_url.trim();

    if api_key.is_empty() || model.is_empty() || base_url.is_empty() {
        return AiGenerationResult {
            ok: false,
            content: String::new(),
            message: Some("请先完成 AI 模型配置".to_string()),
        };
    }

    let endpoint = ai_chat_endpoint(base_url);
    let clients = build_clients(Duration::from_secs(45));
    let prompt = build_ai_evaluation_prompt(&context);
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": "你是世界杯赛事资讯解读助手。只做公开赛事信息分析，不提供投注、赔率、下注或博彩建议。"
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "stream": false
    });
    let mut errors = Vec::new();

    for client in &clients {
        match client
            .post(&endpoint)
            .bearer_auth(api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                if !status.is_success() {
                    log::warn!("AI evaluation failed with status {status}: {text}");
                    return AiGenerationResult {
                        ok: false,
                        content: String::new(),
                        message: Some(format!("AI 评估生成失败：HTTP {status}")),
                    };
                }
                match serde_json::from_str::<ChatCompletionResponse>(&text) {
                    Ok(parsed) => {
                        let content = parsed
                            .choices
                            .first()
                            .and_then(|choice| choice.message.content.as_deref())
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        if content.is_empty() {
                            return AiGenerationResult {
                                ok: false,
                                content,
                                message: Some("AI 已响应，但没有返回分析内容".to_string()),
                            };
                        }
                        return AiGenerationResult {
                            ok: true,
                            content,
                            message: None,
                        };
                    }
                    Err(error) => {
                        log::warn!("AI evaluation response parse failed: {error}; body={text}");
                        return AiGenerationResult {
                            ok: false,
                            content: String::new(),
                            message: Some("AI 返回内容不是 OpenAI 兼容格式".to_string()),
                        };
                    }
                }
            }
            Err(error) => {
                log::warn!("AI evaluation request failed: {error}");
                errors.push(error.to_string());
            }
        }
    }

    AiGenerationResult {
        ok: false,
        content: String::new(),
        message: Some(if errors.is_empty() {
            "AI 评估生成失败：无法请求接口地址".to_string()
        } else {
            format!("AI 评估生成失败：{}", errors.join("; "))
        }),
    }
}

fn build_ai_evaluation_prompt(context: &AiEvaluationContext) -> String {
    format!(
        "请基于公开赛事信息，生成一段中文赛前/赛中资讯解读。\n比赛：{} vs {}\n时间：{}\n地点：{}\n状态：{}\n比分：{}\nElo 概率估算：主胜 {}%，平局 {}%，客胜 {}%。\n要求：3-5 句，语气克制，不要给投注建议，不要出现赔率、下注、盘口相关表达。",
        context.home_team,
        context.away_team,
        context.kickoff,
        context.venue,
        context.status,
        context.score,
        context.odds_home,
        context.odds_draw,
        context.odds_away
    )
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

async fn run_ai_model_config_test(config: AiModelConfig) -> ConnectivityTestResult {
    let api_key = config.api_key.trim();
    let model = config.model.trim();
    let base_url = config.base_url.trim();

    if api_key.is_empty() {
        return ConnectivityTestResult {
            ok: false,
            message: "请先填写 API Key".to_string(),
            details: None,
        };
    }
    if model.is_empty() {
        return ConnectivityTestResult {
            ok: false,
            message: "请先填写模型名".to_string(),
            details: None,
        };
    }
    if base_url.is_empty() {
        return ConnectivityTestResult {
            ok: false,
            message: "请先填写接口地址 baseURL".to_string(),
            details: None,
        };
    }

    let endpoint = ai_chat_endpoint(base_url);
    let clients = build_clients(Duration::from_secs(25));
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": "你好，请只回复 ok"
            }
        ],
        "stream": false
    });
    let mut errors = Vec::new();

    for client in &clients {
        match client
            .post(&endpoint)
            .bearer_auth(api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                if !status.is_success() {
                    log::warn!("AI model config test failed with status {status}: {text}");
                    return ConnectivityTestResult {
                        ok: false,
                        message: format!("模型连接失败：HTTP {status}"),
                        details: if text.is_empty() { None } else { Some(text) },
                    };
                }

                match serde_json::from_str::<ChatCompletionResponse>(&text) {
                    Ok(parsed) => {
                        let content = parsed
                            .choices
                            .first()
                            .and_then(|choice| choice.message.content.as_deref())
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        let ok = content.eq_ignore_ascii_case("ok");
                        return ConnectivityTestResult {
                            ok,
                            message: if ok {
                                "模型连接成功，返回 ok".to_string()
                            } else {
                                "模型已响应，但未按测试提示返回 ok".to_string()
                            },
                            details: if content.is_empty() { Some(text) } else { Some(content) },
                        };
                    }
                    Err(error) => {
                        log::warn!("AI model config test response parse failed: {error}; body={text}");
                        return ConnectivityTestResult {
                            ok: false,
                            message: "模型连接失败：返回内容不是 OpenAI 兼容格式".to_string(),
                            details: Some(text),
                        };
                    }
                }
            }
            Err(error) => {
                log::warn!("AI model config test request failed: {error}");
                errors.push(error.to_string());
            }
        }
    }

    ConnectivityTestResult {
        ok: false,
        message: "模型连接失败：无法请求接口地址".to_string(),
        details: if errors.is_empty() { None } else { Some(errors.join("; ")) },
    }
}

fn ai_chat_endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if let Some(prefix) = trimmed.strip_suffix("/openai/v1") {
        return format!("{}/api/v1/chat/completions", prefix.trim_end_matches('/'));
    }
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    }
}

