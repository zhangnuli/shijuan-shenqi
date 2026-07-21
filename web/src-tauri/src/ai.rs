use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};

/// 用户点击「取消生成」时置位；下一次请求前清除
static CANCEL_FLAG: AtomicBool = AtomicBool::new(false);

pub fn clear_cancel_flag() {
    CANCEL_FLAG.store(false, Ordering::SeqCst);
}

pub fn request_cancel() {
    CANCEL_FLAG.store(true, Ordering::SeqCst);
}

pub fn is_cancelled() -> bool {
    CANCEL_FLAG.load(Ordering::SeqCst)
}

fn check_cancel() -> Result<(), String> {
    if is_cancelled() {
        Err("已取消生成".into())
    } else {
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Option<Vec<Choice>>,
    error: Option<ApiErrorBody>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Option<ChatMessage>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    message: Option<String>,
}

/// 规范化 API Base：
/// - 去掉末尾 /
/// - 若误填了 .../chat/completions 则剥掉
/// - 若明显是官网首页且无 /v1，给出提示（调用处处理）
pub fn normalize_api_base(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    // 去掉查询串
    if let Some(i) = s.find('?') {
        s = s[..i].to_string();
    }
    s = s.trim_end_matches('/').to_string();

    // 用户有时会把完整 completions 地址填进 Base
    for suffix in [
        "/chat/completions",
        "/v1/chat/completions",
        "/completions",
    ] {
        if s.to_lowercase().ends_with(suffix) {
            s = s[..s.len() - suffix.len()].trim_end_matches('/').to_string();
            break;
        }
    }
    s
}

fn looks_like_html(text: &str) -> bool {
    let t = text.trim_start().to_lowercase();
    t.starts_with("<!doctype html")
        || t.starts_with("<html")
        || t.contains("<title>") && t.contains("</html>")
        || (t.contains("<!doctype") && t.contains("html"))
}

fn truncate(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        return t.to_string();
    }
    t.chars().take(max).collect::<String>() + "…"
}

fn html_response_hint(api_base: &str, request_url: &str) -> String {
    format!(
        "API 返回了网页 HTML，而不是 JSON。说明「API Base」填成了网站首页/控制台地址，而不是接口地址。\n\
        \n\
        当前 Base：{api_base}\n\
        实际请求：POST {request_url}\n\
        \n\
        正确示例（OpenAI 兼容）：\n\
        · https://api.openai.com/v1\n\
        · https://api.deepseek.com/v1\n\
        · https://api.x.ai/v1\n\
        · 第三方网关：https://你的域名/v1  （一般以 /v1 结尾）\n\
        \n\
        不要填：\n\
        · https://xxx.com/ （网站首页）\n\
        · 管理后台登录页 / 购买页\n\
        \n\
        请到「AI 与 API 设置」把 API Base 改成文档里的 Endpoint（含 /v1），并确认 Key、模型名正确。"
    )
}

/// 调用 OpenAI 兼容 Chat Completions
pub fn chat_completion(cfg: &AppConfig, system: &str, user: &str) -> Result<String, String> {
    check_cancel()?;
    // 本地 Ollama 等常不需要真实 Key
    let base_l0 = cfg.api_base.to_lowercase();
    let local = base_l0.contains("127.0.0.1") || base_l0.contains("localhost");
    if cfg.api_key.trim().is_empty() && !local {
        return Err("请先在设置中填写 API Key".into());
    }
    if cfg.api_base.trim().is_empty() {
        return Err("请填写 API Base URL".into());
    }
    if cfg.model.trim().is_empty() {
        return Err("请填写模型名称".into());
    }

    let base = normalize_api_base(&cfg.api_base);
    if base.is_empty() {
        return Err("API Base 无效".into());
    }

    // 常见误填：只有域名没有 /v1
    let base_l = base.to_lowercase();
    if !base_l.contains("/v1")
        && !base_l.contains("/api")
        && (base_l.ends_with(".com")
            || base_l.ends_with(".cn")
            || base_l.ends_with(".asia")
            || base_l.matches('.').count() <= 2)
    {
        // 不强制失败，但多数网关需要 /v1；若后面拿到 HTML 会再提示
    }

    let url = format!("{}/chat/completions", base);

    let body = json!({
        "model": cfg.model,
        "temperature": cfg.temperature,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ]
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .user_agent("shijuan-shenqi/0.1")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    check_cancel()?;
    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body);
    let key = cfg.api_key.trim();
    if !key.is_empty() {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    let resp = req.send().map_err(|e| {
        if is_cancelled() {
            return "已取消生成".to_string();
        }
        format!("请求失败: {e}\nURL: {url}\n请检查网络，以及 API Base 是否可访问。")
    })?;

    check_cancel()?;
    let status = resp.status();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let text = resp
        .text()
        .map_err(|e| format!("读取响应失败: {e}"))?;

    if looks_like_html(&text) || content_type.contains("text/html") {
        return Err(html_response_hint(&base, &url));
    }

    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(msg) = err
                .pointer("/error/message")
                .and_then(|v| v.as_str())
                .or_else(|| err.get("message").and_then(|v| v.as_str()))
            {
                return Err(format!(
                    "API 错误 ({status}): {msg}\n请求: POST {url}\n模型: {}",
                    cfg.model
                ));
            }
        }
        return Err(format!(
            "API 错误 ({status})\n请求: POST {url}\n响应: {}",
            truncate(&text, 400)
        ));
    }

    // 有些网关 200 但 body 仍是空或非 JSON
    if text.trim().is_empty() {
        return Err(format!(
            "API 返回空内容。请检查模型名「{}」是否在该网关可用。\n请求: POST {url}",
            cfg.model
        ));
    }

    let parsed: ChatCompletionResponse = serde_json::from_str(&text).map_err(|e| {
        if looks_like_html(&text) {
            html_response_hint(&base, &url)
        } else {
            format!(
                "解析响应失败: {e}\n请求: POST {url}\nContent-Type: {content_type}\n响应摘要: {}",
                truncate(&text, 300)
            )
        }
    })?;

    if let Some(e) = parsed.error {
        return Err(format!(
            "API 返回错误: {}",
            e.message.unwrap_or_else(|| "unknown".into())
        ));
    }

    let content = parsed
        .choices
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.message)
        .map(|m| m.content)
        .ok_or_else(|| {
            format!(
                "API 未返回 choices 内容。请确认模型「{}」可用，且接口为 OpenAI 兼容 /chat/completions。\n请求: POST {url}\n响应摘要: {}",
                cfg.model,
                truncate(&text, 300)
            )
        })?;

    if content.trim().is_empty() {
        return Err("模型返回内容为空，请换模型或提高 max tokens 后重试".into());
    }

    Ok(content)
}

/// 从模型输出中提取 JSON（支持 ```json 代码块）
pub fn extract_json(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.starts_with('{') {
        return Ok(trimmed.to_string());
    }
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        let after = after
            .strip_prefix("json")
            .or_else(|| after.strip_prefix("JSON"))
            .unwrap_or(after);
        let after = after.trim_start_matches('\n').trim_start_matches('\r');
        if let Some(end) = after.find("```") {
            return Ok(after[..end].trim().to_string());
        }
    }
    if let (Some(s), Some(e)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if e > s {
            return Ok(trimmed[s..=e].to_string());
        }
    }
    Err(format!(
        "未能从模型输出中提取 JSON。模型原文摘要：{}",
        truncate(raw, 400)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_completions() {
        assert_eq!(
            normalize_api_base("https://api.openai.com/v1/chat/completions"),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            normalize_api_base("https://hello.vangularcode.asia/v1/"),
            "https://hello.vangularcode.asia/v1"
        );
    }
}
