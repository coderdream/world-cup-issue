use crate::models::{
    AiBookMaterialsPayload, AiGenerateRequest, AiGenerateResult, AiTestResult, AppSettings, AppStatePayload, BookMaterials,
    BookMaterialsRequest, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, EpubBook, UpdateInfo,
};
use crate::epub::{count_han_chars, read_epub, truncate_chars};
use crate::operation_log::OperationLogger;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct AppData {
    settings: Mutex<AppSettings>,
    settings_path: PathBuf,
    logger: OperationLogger,
}

impl AppData {
    pub fn load(app: &tauri::AppHandle) -> Self {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let app_local_data_dir = app
            .path()
            .app_local_data_dir()
            .unwrap_or_else(|_| app_data_dir.clone());

        let settings_path = app_data_dir.join("settings.json");
        let db_path = app_data_dir.join("app.db");
        let log_dir = app_local_data_dir.join("logs");
        let logger = OperationLogger::new(db_path, log_dir);

        let settings = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|content| serde_json::from_str::<AppSettings>(&content).ok())
            .unwrap_or_default();

        logger.info("app", "startup", "A Book in 30 Minutes started");

        Self {
            settings: Mutex::new(settings),
            settings_path,
            logger,
        }
    }

    fn save_settings(&self, settings: &AppSettings) -> Result<(), CommandError> {
        if let Some(parent) = self.settings_path.parent() {
            fs::create_dir_all(parent).map_err(|error| command_error(format!("创建配置目录失败：{error}")))?;
        }
        let content = serde_json::to_string_pretty(settings).map_err(|error| command_error(format!("序列化配置失败：{error}")))?;
        fs::write(&self.settings_path, content).map_err(|error| command_error(format!("保存配置失败：{error}")))?;
        self.logger.info("settings", "save", "配置已保存");
        Ok(())
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub message: String,
}

#[tauri::command]
pub fn get_app_state(data: State<'_, AppData>) -> Result<AppStatePayload, CommandError> {
    data.logger.info("app", "get_app_state", "读取应用状态");
    Ok(AppStatePayload {
        settings: data.settings.lock().map_err(lock_error)?.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command]
pub fn get_settings(data: State<'_, AppData>) -> Result<AppSettings, CommandError> {
    data.logger.info("settings", "get", "读取配置");
    Ok(data.settings.lock().map_err(lock_error)?.clone())
}

#[tauri::command]
pub fn set_settings(data: State<'_, AppData>, settings: AppSettings) -> Result<AppSettings, CommandError> {
    let mut current = data.settings.lock().map_err(lock_error)?;
    *current = settings.clone();
    data.save_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn check_update_mock(data: State<'_, AppData>) -> UpdateInfo {
    data.logger.info("update", "check", "执行更新检查");
    UpdateInfo {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        latest_version: env!("CARGO_PKG_VERSION").to_string(),
        available: false,
        notes: "当前已经是最新框架版本。".to_string(),
    }
}

#[tauri::command]
pub async fn test_ai_profile(data: State<'_, AppData>) -> Result<AiTestResult, CommandError> {
    data.logger.info("ai", "test_profile", "开始测试 AI 配置");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let content = match call_ai(
        &settings,
        vec![ChatMessage {
            role: "user".to_string(),
            content: "你好，请只回复 ok".to_string(),
        }],
    )
    .await
    {
        Ok(content) => content,
        Err(error) => {
            data.logger.error("ai", "test_profile", "AI 配置测试失败", &error.message);
            return Err(error);
        }
    };

    data.logger.info("ai", "test_profile", "AI 配置测试成功");
    Ok(AiTestResult {
        ok: true,
        message: "AI 配置测试成功。".to_string(),
        content: Some(content),
    })
}

#[tauri::command]
pub async fn generate_ai_text(data: State<'_, AppData>, request: AiGenerateRequest) -> Result<AiGenerateResult, CommandError> {
    data.logger.info("ai", "generate_text", "开始生成 AI 文本");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let mut messages = Vec::new();
    if let Some(system_prompt) = request.system_prompt.filter(|value| !value.trim().is_empty()) {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        });
    }
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: request.prompt,
    });

    let content = match call_ai(&settings, messages).await {
        Ok(content) => content,
        Err(error) => {
            data.logger.error("ai", "generate_text", "AI 文本生成失败", &error.message);
            return Err(error);
        }
    };
    data.logger.info("ai", "generate_text", "AI 文本生成成功");
    Ok(AiGenerateResult {
        content,
        model: settings.ai_profile.model,
    })
}

#[tauri::command]
pub async fn generate_book_materials(data: State<'_, AppData>, request: BookMaterialsRequest) -> Result<BookMaterials, CommandError> {
    data.logger.info("materials", "generate", "开始生成 YouTube 听书素材");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let epub_path = Path::new(request.epub_path.trim());
    if request.epub_path.trim().is_empty() {
        return Err(command_error("请先填写 EPUB 文件路径。"));
    }
    if !epub_path.exists() {
        return Err(command_error("EPUB 文件不存在，请检查路径。"));
    }

    let book = read_epub(epub_path)?;
    let prompt = build_book_materials_prompt(&book, &request);
    let system_prompt = "你是一个中文 YouTube 听书视频策划和旁白稿作者。你只输出严格 JSON，不输出 Markdown。".to_string();
    let content = match call_ai(
        &settings,
        vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt.clone(),
            },
        ],
    )
    .await
    {
        Ok(content) => content,
        Err(error) => {
            data.logger.error("materials", "generate", "素材生成失败", &error.message);
            return Err(error);
        }
    };

    let mut payload = parse_book_materials_payload(&content)?;
    let min_chars = request.target_min_chars.max(1000);
    let max_chars = request.target_max_chars.max(min_chars + 1);
    let narration_chars = count_han_chars(&payload.narration);
    if narration_chars < min_chars || narration_chars > max_chars {
        let repair_prompt = build_repair_prompt(&payload, min_chars, max_chars);
        if let Ok(repaired) = call_ai(
            &settings,
            vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: repair_prompt,
                },
            ],
        )
        .await
        {
            if let Ok(next_payload) = parse_book_materials_payload(&repaired) {
                payload = next_payload;
            }
        }
    }

    let subtitles = split_subtitles(&payload.narration);
    data.logger.info("materials", "generate", "YouTube 听书素材生成成功");
    Ok(BookMaterials {
        video_title: payload.video_title,
        description: payload.description,
        tags: payload.tags,
        narration: payload.narration,
        subtitles,
        prompt,
        model: settings.ai_profile.model,
        overview: book.overview,
    })
}

async fn call_ai(settings: &AppSettings, messages: Vec<ChatMessage>) -> Result<String, CommandError> {
    let profile = &settings.ai_profile;
    if profile.api_key.trim().is_empty() {
        return Err(command_error("请先填写 AI API Key。"));
    }
    if profile.base_url.trim().is_empty() {
        return Err(command_error("请先填写 AI Base URL。"));
    }
    if profile.model.trim().is_empty() {
        return Err(command_error("请先填写 AI 模型名称。"));
    }

    let base = profile.base_url.trim().trim_end_matches('/');
    let url = if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{base}/chat/completions")
    };

    let response = reqwest::Client::new()
        .post(url)
        .bearer_auth(profile.api_key.trim())
        .json(&ChatCompletionRequest {
            model: profile.model.clone(),
            messages,
            stream: false,
        })
        .send()
        .await
        .map_err(|error| command_error(format!("AI 请求失败：{error}")))?
        .error_for_status()
        .map_err(|error| command_error(format!("AI 服务返回错误：{error}")))?
        .json::<ChatCompletionResponse>()
        .await
        .map_err(|error| command_error(format!("AI 响应解析失败：{error}")))?;

    response
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| command_error("AI 服务没有返回内容。"))
}

pub(crate) fn command_error(message: impl Into<String>) -> CommandError {
    CommandError {
        message: message.into(),
    }
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> CommandError {
    CommandError {
        message: error.to_string(),
    }
}

fn build_book_materials_prompt(book: &EpubBook, request: &BookMaterialsRequest) -> String {
    let target_min = request.target_min_chars.max(1000);
    let target_max = request.target_max_chars.max(target_min + 1);
    let channel_name = if request.channel_name.trim().is_empty() {
        "半小时听完一本书"
    } else {
        request.channel_name.trim()
    };
    let extra_direction = request.extra_direction.trim();
    let chapter_list = book
        .overview
        .chapters
        .iter()
        .map(|chapter| format!("- {}：{} 字", chapter.title, chapter.chars))
        .collect::<Vec<_>>()
        .join("\n");
    let source_packet = build_source_packet(book);

    format!(
        r#"请基于下面这本 EPUB 的目录与章节摘录，生成 YouTube 听书视频素材。

频道名：{channel_name}
目标语言：中文
目标字数：旁白稿必须在 {target_min}-{target_max} 个中文字之间，最佳约为两者中间值。

书籍信息：
- 书名：{title}
- 作者：{creator}
- 出版方：{publisher}
- EPUB 总中文字数：{total_chars}

章节目录：
{chapter_list}

章节素材包：
{source_packet}

写作要求：
1. 这不是逐字朗读，也不是完整替代原书。请写成原创转述、摘要、评论和解读。
2. 不要照抄原文长句，不要出现大段原书内容。
3. 不要平均覆盖所有章节，只选择 5-7 个最有戏剧性、最适合听书视频的故事节点。
4. 结构接近“睡前听完一本书”：开场陪伴感、作者/地点引入、轻盈生活趣味、人物创造力、残酷现实、命运悲剧、主题升华、晚安式结尾。
5. 旁白稿要口语化、短句多、适合 AI 朗读和字幕切分。每句话尽量 8-18 个汉字。
6. 标题要适合 YouTube 中文视频，能包含书名和情绪价值。
7. 简介要适合 YouTube description，包含频道定位、视频内容概述和版权/解读性质提示。
8. 标签输出 12-20 个，包含中文关键词和少量英文品牌词。
9. 只返回严格 JSON，不要 Markdown，不要解释。

额外方向：
{extra_direction}

返回 JSON 格式：
{{
  "videoTitle": "视频标题",
  "description": "视频简介",
  "tags": ["标签1", "标签2"],
  "narration": "完整旁白稿"
}}"#,
        title = book.overview.title,
        creator = book.overview.creator,
        publisher = book.overview.publisher,
        total_chars = book.overview.total_chars,
    )
}

fn build_source_packet(book: &EpubBook) -> String {
    let priority_titles = ["结婚", "悬壶", "娃娃", "白手", "芳邻", "素人", "哑奴", "哭泣", "沙漠"];
    let mut packet = Vec::new();
    for chapter in &book.chapters {
        let has_priority = priority_titles.iter().any(|keyword| chapter.title.contains(keyword));
        let max_chars = if has_priority { 900 } else { 420 };
        let excerpt = truncate_chars(&chapter.text.replace('\n', " "), max_chars);
        packet.push(format!("【{}】\n{}", chapter.title, excerpt));
        if packet.join("\n\n").chars().count() > 24000 {
            break;
        }
    }
    packet.join("\n\n")
}

fn build_repair_prompt(payload: &AiBookMaterialsPayload, min_chars: usize, max_chars: usize) -> String {
    let current_chars = count_han_chars(&payload.narration);
    let json = serde_json::to_string(payload).unwrap_or_default();
    format!(
        r#"上一版 JSON 中 narration 的中文字数为 {current_chars}，不在 {min_chars}-{max_chars} 范围内。
请保留同样的频道定位、题材和结构，重写 JSON，使 narration 严格落在 {min_chars}-{max_chars} 个中文字之间。
只返回严格 JSON，不要 Markdown。

上一版 JSON：
{json}"#
    )
}

fn parse_book_materials_payload(content: &str) -> Result<AiBookMaterialsPayload, CommandError> {
    let json = extract_json_object(content).ok_or_else(|| command_error("AI 没有返回可解析的 JSON。"))?;
    let payload = serde_json::from_str::<AiBookMaterialsPayload>(&json)
        .map_err(|error| command_error(format!("AI JSON 解析失败：{error}")))?;
    if payload.video_title.trim().is_empty() {
        return Err(command_error("AI 返回的视频标题为空。"));
    }
    if payload.narration.trim().is_empty() {
        return Err(command_error("AI 返回的旁白稿为空。"));
    }
    Ok(payload)
}

fn extract_json_object(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }
    let fence_re = Regex::new(r#"(?s)```(?:json)?\s*(\{.*?\})\s*```"#).ok()?;
    if let Some(captures) = fence_re.captures(trimmed) {
        return captures.get(1).map(|value| value.as_str().to_string());
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end > start {
        Some(trimmed[start..=end].to_string())
    } else {
        None
    }
}

fn split_subtitles(narration: &str) -> Vec<String> {
    let punctuation_re = Regex::new(r#"[。！？!?；;，,、\n]+"#).expect("valid regex");
    let clean_re = Regex::new(r#"[“”"‘’'（）()《》<>【】\[\]：:]"#).expect("valid regex");
    let mut lines = Vec::new();
    for part in punctuation_re.split(narration) {
        let cleaned = clean_re.replace_all(part.trim(), "");
        let cleaned = cleaned.trim();
        if cleaned.is_empty() {
            continue;
        }
        push_subtitle_chunks(&mut lines, cleaned, 16);
    }
    lines
}

fn push_subtitle_chunks(lines: &mut Vec<String>, text: &str, max_chars: usize) {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        lines.push(text.to_string());
        return;
    }
    let mut start = 0usize;
    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        lines.push(chars[start..end].iter().collect::<String>());
        start = end;
    }
}
