use crate::models::{
    AiGenerateRequest, AiGenerateResult, AiTestResult, AppSettings, AppStatePayload, ChatCompletionRequest, ChatCompletionResponse,
    ChatMessage, FeishuSendRequest, FeishuSendResult, FeishuWebhookResponse, UpdateInfo,
};
use crate::operation_log::OperationLogger;
use std::fs;
use std::path::PathBuf;
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

        logger.info("app", "startup", "Tauri Framework started");

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
pub async fn test_feishu_profile(data: State<'_, AppData>) -> Result<FeishuSendResult, CommandError> {
    data.logger.info("feishu", "test_profile", "开始测试飞书配置");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let text = settings.feishu_profile.test_message.trim();
    let message = if text.is_empty() {
        "飞书连通性测试成功。".to_string()
    } else {
        text.to_string()
    };

    match call_feishu(&settings, &message).await {
        Ok(result) => {
            data.logger.info("feishu", "test_profile", "飞书配置测试成功");
            Ok(result)
        }
        Err(error) => {
            data.logger.error("feishu", "test_profile", "飞书配置测试失败", &error.message);
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn send_feishu_message(data: State<'_, AppData>, request: FeishuSendRequest) -> Result<FeishuSendResult, CommandError> {
    data.logger.info("feishu", "send_message", "开始发送飞书消息");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    match call_feishu(&settings, &request.text).await {
        Ok(result) => {
            data.logger.info("feishu", "send_message", "飞书消息发送成功");
            Ok(result)
        }
        Err(error) => {
            data.logger.error("feishu", "send_message", "飞书消息发送失败", &error.message);
            Err(error)
        }
    }
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

async fn call_feishu(settings: &AppSettings, text: &str) -> Result<FeishuSendResult, CommandError> {
    let profile = &settings.feishu_profile;
    let webhook_url = profile.webhook_url.trim();
    if webhook_url.is_empty() {
        return Err(command_error("请先填写飞书 Webhook 地址。"));
    }
    if !webhook_url.starts_with("http://") && !webhook_url.starts_with("https://") {
        return Err(command_error("飞书 Webhook 地址必须以 http:// 或 https:// 开头。"));
    }

    let message = text.trim();
    if message.is_empty() {
        return Err(command_error("请先填写要发送的飞书消息。"));
    }

    let title = profile.title.trim();
    let content = if title.is_empty() {
        message.to_string()
    } else {
        format!("【{title}】\n{message}")
    };

    let response = reqwest::Client::new()
        .post(webhook_url)
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Accept", "application/json")
        .header("User-Agent", "TauriFramework/0.1")
        .json(&serde_json::json!({
            "msg_type": "text",
            "content": {
                "text": content,
            }
        }))
        .send()
        .await
        .map_err(|error| command_error(format!("飞书请求失败：{error}")))?
        .error_for_status()
        .map_err(|error| command_error(format!("飞书服务返回错误：{error}")))?;

    let body = response
        .json::<FeishuWebhookResponse>()
        .await
        .map_err(|error| command_error(format!("飞书响应解析失败：{error}")))?;

    let code = body.code.unwrap_or(-1);
    if code != 0 {
        let msg = body.msg.unwrap_or_else(|| "未知错误".to_string());
        return Err(command_error(format!("飞书机器人返回错误：code={code}，msg={msg}")));
    }

    Ok(FeishuSendResult {
        ok: true,
        message: "飞书连通性测试成功，消息已发送。".to_string(),
    })
}

fn command_error(message: impl Into<String>) -> CommandError {
    CommandError {
        message: message.into(),
    }
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> CommandError {
    CommandError {
        message: error.to_string(),
    }
}
