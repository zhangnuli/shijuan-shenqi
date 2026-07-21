use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::storage::{read_json, write_json};
use crate::secret::{clear_api_key as remove_api_key, load_api_key, save_api_key};

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
            models: vec![
                "grok-4.5".into(),
                "grok-3".into(),
                "grok-3-mini".into(),
            ],
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
    match load_api_key() {
        Ok(Some(api_key)) => {
            cfg.api_key = api_key;
            cfg.api_key_configured = true;
        }
        _ if !cfg.api_key.trim().is_empty() => {
            let _ = save_api_key(&cfg.api_key);
            cfg.api_key_configured = true;
            let mut migrated = cfg.clone();
            migrated.api_key.clear();
            let _ = write_json(&path, &migrated);
        }
        _ => {
            cfg.api_key.clear();
            cfg.api_key_configured = false;
        }
    }
    cfg
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let path = config_path()?;
    let mut stored = cfg.clone();
    if !cfg.api_key.trim().is_empty() {
        save_api_key(cfg.api_key.trim())?;
        stored.api_key_configured = true;
    } else {
        stored.api_key_configured = load_api_key()?.is_some();
    }
    stored.api_key.clear();
    write_json(&path, &stored)
}

pub fn load_config_for_frontend() -> AppConfig {
    let mut cfg = load_config();
    cfg.api_key.clear();
    cfg
}

pub fn clear_api_key() -> Result<(), String> {
    remove_api_key()?;
    let mut cfg = load_config();
    cfg.api_key.clear();
    cfg.api_key_configured = false;
    let path = config_path()?;
    write_json(&path, &cfg)
}
