use crate::secret::{clear_api_key as remove_api_key, load_api_key, save_api_key};
use crate::storage::{read_json, write_json};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// 保存后立即供本进程内的请求使用，避免 Windows 凭据存储在特殊启动环境下暂时不可读。
static RUNTIME_API_KEY: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn runtime_api_key() -> &'static Mutex<Option<String>> {
    RUNTIME_API_KEY.get_or_init(|| Mutex::new(None))
}

fn cached_api_key() -> Option<String> {
    runtime_api_key().lock().ok().and_then(|key| key.clone())
}

fn cache_api_key(api_key: Option<String>) {
    if let Ok(mut cached) = runtime_api_key().lock() {
        *cached = api_key.filter(|value| !value.trim().is_empty());
    }
}

/// 预置厂商（可改 base_url / model，也可完全自定义）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPreset {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub default_model: String,
    pub models: Vec<String>,
    /// OpenAI 兼容 chat/completions
    pub api_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// 当前选用的厂商 id，custom 表示完全自定义
    pub provider_id: String,
    pub api_base: String,
    pub api_key: String,
    /// 仅供前端判断；密钥本体保存在系统用户加密文件中。
    #[serde(default)]
    pub api_key_configured: bool,
    pub model: String,
    /// 温度
    pub temperature: f32,
    /// 默认导出目录（可空）
    pub export_dir: String,

    // —— 组卷偏好 ——
    #[serde(default = "default_subject")]
    pub default_subject: String,
    #[serde(default = "default_edition")]
    pub default_edition: String,
    #[serde(default = "default_grade")]
    pub default_grade: u8,
    #[serde(default = "default_semester")]
    pub default_semester: String,
    #[serde(default = "default_exam_type")]
    pub default_exam_type: String,
    #[serde(default = "default_difficulty")]
    pub default_difficulty: String,
    /// 导出学生卷时是否附带参考答案页
    #[serde(default = "default_true")]
    pub export_attach_answers: bool,
    /// 导出档位：student | with_answers | both
    #[serde(default = "default_export_mode")]
    pub export_mode: String,
    /// 导出文件名模板，可用 {school}{grade}{subject}{title}{date}{variant}{type}
    #[serde(default = "default_filename_pattern")]
    pub export_filename_pattern: String,
    /// 历史记录保留条数
    #[serde(default = "default_history_max")]
    pub history_max: u32,

    // —— 卷头 / 校名 ——
    #[serde(default)]
    pub school_name: String,
    /// 如 2025—2026 学年度
    #[serde(default)]
    pub academic_year: String,
    /// 上学期 / 下学期
    #[serde(default)]
    pub school_term: String,
    /// 默认显示在卷头的班级（可空）
    #[serde(default)]
    pub default_class_name: String,
}

/// 规范化用户填写的 OpenAI 兼容 API Base。
///
/// 请求路径由客户端统一追加 `/chat/completions`。
pub fn normalize_api_base(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if let Some(i) = s.find('?') {
        s = s[..i].to_string();
    }
    s = s.trim_end_matches('/').to_string();

    for suffix in ["/chat/completions", "/v1/chat/completions", "/completions"] {
        if s.to_lowercase().ends_with(suffix) {
            s = s[..s.len() - suffix.len()]
                .trim_end_matches('/')
                .to_string();
            break;
        }
    }

    s
}

fn default_subject() -> String {
    "math".into()
}
fn default_edition() -> String {
    "beishida".into()
}
fn default_grade() -> u8 {
    3
}
fn default_semester() -> String {
    "shang".into()
}
fn default_exam_type() -> String {
    "unit".into()
}
fn default_difficulty() -> String {
    "标准".into()
}
fn default_true() -> bool {
    true
}
fn default_history_max() -> u32 {
    30
}
fn default_export_mode() -> String {
    "with_answers".into()
}
fn default_filename_pattern() -> String {
    "{school}{grade}年级-{subject}-{title}-{date}".into()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider_id: "xai".into(),
            api_base: "https://api.x.ai/v1".into(),
            api_key: String::new(),
            api_key_configured: false,
            model: "grok-4.5".into(),
            temperature: 0.4,
            export_dir: String::new(),
            default_subject: default_subject(),
            default_edition: default_edition(),
            default_grade: default_grade(),
            default_semester: default_semester(),
            default_exam_type: default_exam_type(),
            default_difficulty: default_difficulty(),
            export_attach_answers: true,
            export_mode: default_export_mode(),
            export_filename_pattern: default_filename_pattern(),
            history_max: 30,
            school_name: String::new(),
            academic_year: String::new(),
            school_term: String::new(),
            default_class_name: String::new(),
        }
    }
}

pub fn provider_presets() -> Vec<ProviderPreset> {
    vec![
        ProviderPreset {
            id: "xai".into(),
            name: "SpaceXAI / xAI".into(),
            base_url: "https://api.x.ai/v1".into(),
            default_model: "grok-4.5".into(),
            models: vec!["grok-4.5".into(), "grok-3".into(), "grok-3-mini".into()],
            api_style: "openai".into(),
        },
        ProviderPreset {
            id: "openai".into(),
            name: "OpenAI".into(),
            base_url: "https://api.openai.com/v1".into(),
            default_model: "gpt-4o".into(),
            models: vec![
                "gpt-4o".into(),
                "gpt-4o-mini".into(),
                "gpt-4.1".into(),
                "o3-mini".into(),
            ],
            api_style: "openai".into(),
        },
        ProviderPreset {
            id: "deepseek".into(),
            name: "DeepSeek".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            default_model: "deepseek-chat".into(),
            models: vec!["deepseek-chat".into(), "deepseek-reasoner".into()],
            api_style: "openai".into(),
        },
        ProviderPreset {
            id: "siliconflow".into(),
            name: "硅基流动".into(),
            base_url: "https://api.siliconflow.cn/v1".into(),
            default_model: "deepseek-ai/DeepSeek-V3".into(),
            models: vec![
                "deepseek-ai/DeepSeek-V3".into(),
                "Qwen/Qwen2.5-72B-Instruct".into(),
                "moonshotai/Kimi-K2-Instruct".into(),
            ],
            api_style: "openai".into(),
        },
        ProviderPreset {
            id: "moonshot".into(),
            name: "月之暗面 Kimi".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            default_model: "moonshot-v1-128k".into(),
            models: vec![
                "moonshot-v1-8k".into(),
                "moonshot-v1-32k".into(),
                "moonshot-v1-128k".into(),
            ],
            api_style: "openai".into(),
        },
        ProviderPreset {
            id: "ollama".into(),
            name: "本地 Ollama（离线/局域网）".into(),
            base_url: "http://127.0.0.1:11434/v1".into(),
            default_model: "qwen2.5:7b".into(),
            models: vec![
                "qwen2.5:7b".into(),
                "qwen2.5:14b".into(),
                "llama3.2".into(),
                "deepseek-r1:8b".into(),
            ],
            api_style: "openai".into(),
        },
        ProviderPreset {
            id: "qwen".into(),
            name: "通义千问（兼容模式）".into(),
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(),
            default_model: "qwen-plus".into(),
            models: vec!["qwen-plus".into(), "qwen-max".into(), "qwen-turbo".into()],
            api_style: "openai".into(),
        },
        ProviderPreset {
            id: "qianfan".into(),
            name: "百度千帆（兼容模式）".into(),
            base_url: "https://qianfan.baidubce.com/v2".into(),
            default_model: "deepseek-v3.2".into(),
            models: vec![
                "deepseek-v3.2".into(),
                "deepseek-v4-flash".into(),
                "deepseek-v4-pro".into(),
                "ernie-4.5-turbo-20260402".into(),
                "kimi-k2.6".into(),
                "glm-5".into(),
            ],
            api_style: "openai".into(),
        },
        ProviderPreset {
            id: "qianfan-coding".into(),
            name: "百度千帆 Coding Plan".into(),
            base_url: "https://qianfan.baidubce.com/v2/coding".into(),
            default_model: "qianfan-code-latest".into(),
            models: vec![
                "qianfan-code-latest".into(),
                "kimi-k2.5".into(),
                "deepseek-v3.2".into(),
                "glm-5".into(),
                "minimax-m2.5".into(),
                "ernie-4.5-turbo-20260402".into(),
                "deepseek-v4-flash".into(),
                "glm-5.1".into(),
            ],
            api_style: "openai".into(),
        },
        ProviderPreset {
            id: "qianfan-tokenplan".into(),
            name: "百度千帆 Token Plan 个人版（Coding Plan 已迁移）".into(),
            base_url: "https://qianfan.baidubce.com/v2/tokenplan/personal".into(),
            default_model: "deepseek-v4-flash".into(),
            models: vec![
                "deepseek-v4-pro".into(),
                "deepseek-v4-flash".into(),
                "deepseek-v4-flash-0731".into(),
                "glm-5.2".into(),
                "glm-5.1".into(),
                "kimi-k2.6".into(),
                "ernie-5.1".into(),
            ],
            api_style: "openai".into(),
        },
        ProviderPreset {
            id: "custom".into(),
            name: "自定义（OpenAI 兼容）".into(),
            base_url: "https://api.example.com/v1".into(),
            default_model: "your-model".into(),
            models: vec![],
            api_style: "openai".into(),
        },
    ]
}

fn config_path() -> Result<PathBuf, String> {
    let dirs = ProjectDirs::from("com", "shijuan", "shenqi")
        .ok_or_else(|| "无法定位配置目录".to_string())?;
    let dir = dirs.config_dir();
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    Ok(dir.join("config.json"))
}

pub fn load_config() -> AppConfig {
    let Ok(path) = config_path() else {
        return AppConfig::default();
    };
    let mut cfg: AppConfig = read_json(&path).ok().flatten().unwrap_or_default();
    cfg.api_base = normalize_api_base(&cfg.api_base);
    match load_api_key() {
        Ok(Some(api_key)) => {
            cache_api_key(Some(api_key.clone()));
            cfg.api_key = api_key;
            cfg.api_key_configured = true;
        }
        Ok(None) if !cfg.api_key.trim().is_empty() => {
            let _ = save_api_key(&cfg.api_key);
            cache_api_key(Some(cfg.api_key.clone()));
            cfg.api_key_configured = true;
            let mut migrated = cfg.clone();
            migrated.api_key.clear();
            let _ = write_json(&path, &migrated);
        }
        Ok(None) => {
            if let Some(api_key) = cached_api_key() {
                cfg.api_key = api_key;
                cfg.api_key_configured = true;
            } else {
                cfg.api_key.clear();
                cfg.api_key_configured = false;
            }
        }
        Err(error) => {
            log::warn!("读取 API Key 失败: {error}");
            if let Some(api_key) = cached_api_key() {
                cfg.api_key = api_key;
                cfg.api_key_configured = true;
            } else {
                cfg.api_key.clear();
                cfg.api_key_configured = false;
            }
        }
    }
    cfg
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let path = config_path()?;
    let mut stored = cfg.clone();
    stored.api_base = normalize_api_base(&stored.api_base);
    if !cfg.api_key.trim().is_empty() {
        save_api_key(cfg.api_key.trim())?;
        cache_api_key(Some(cfg.api_key.trim().to_string()));
        stored.api_key_configured = true;
    } else {
        // 前端只回传“已配置”标志时，重新校验本机密钥，避免状态标志过期。
        stored.api_key_configured = match load_api_key() {
            Ok(Some(api_key)) if !api_key.trim().is_empty() => {
                cache_api_key(Some(api_key));
                true
            }
            _ => cached_api_key().is_some(),
        };
    }
    stored.api_key.clear();
    write_json(&path, &stored)
}

pub fn load_config_for_frontend() -> AppConfig {
    let mut cfg = load_config();
    cfg.api_key.clear();
    cfg
}

/// 读取请求时使用的本机密钥，供前端未回传密钥本体时兜底。
pub fn load_saved_api_key() -> Result<Option<String>, String> {
    if let Some(api_key) = cached_api_key() {
        return Ok(Some(api_key));
    }
    load_api_key().map(|value| {
        let value = value.filter(|api_key| !api_key.trim().is_empty());
        if let Some(api_key) = value.as_ref() {
            cache_api_key(Some(api_key.clone()));
        }
        value
    })
}

pub fn clear_api_key() -> Result<(), String> {
    remove_api_key()?;
    cache_api_key(None);
    let mut cfg = load_config();
    cfg.api_key.clear();
    cfg.api_key_configured = false;
    let path = config_path()?;
    write_json(&path, &cfg)
}
