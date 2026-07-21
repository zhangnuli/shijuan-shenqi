use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use crate::storage::{read_json, unique_id, write_json};

const MAX_DEFAULT: usize = 30;
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryStore {
    schema_version: u32,
    entries: Vec<HistoryEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum HistoryFile {
    Versioned(HistoryStore),
    Legacy(Vec<HistoryEntry>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub created_at: u64,
    pub title: String,
    pub summary: String,
    /// exam | lessonPlan
    #[serde(default = "default_kind")]
    pub kind: String,
    pub paper: Value,
    #[serde(default)]
    pub form_snapshot: Option<Value>,
}

fn default_kind() -> String {
    "exam".into()
}

fn detect_kind(paper: &Value) -> String {
    if let Some(k) = paper.get("kind").and_then(|v| v.as_str()) {
        match k {
            "lessonPlan" | "lessonPlanBundle" | "reviewOutline" | "parallelSet" => {
                return k.into();
            }
            _ => {}
        }
    }
    // 全课时教案包
    if paper.get("plans").and_then(|v| v.as_array()).is_some()
        && paper
            .pointer("/meta/title")
            .and_then(|v| v.as_str())
            .map(|t| t.contains("教案") || t.contains("全课时"))
            .unwrap_or(false)
    {
        return "lessonPlanBundle".into();
    }
    if paper.get("plans").and_then(|v| v.as_array()).is_some()
        && paper.get("sections").is_none()
    {
        return "lessonPlanBundle".into();
    }
    // 讲评提纲
    if paper.get("points").is_some() && paper.get("process").is_some() && paper.get("sections").is_none()
        && paper
            .get("kind")
            .and_then(|v| v.as_str())
            == Some("reviewOutline")
    {
        return "reviewOutline".into();
    }
    if paper.get("knowledgeFocus").is_some() && paper.get("sections").is_none() {
        return "reviewOutline".into();
    }
    if paper.get("process").is_some() && paper.get("objectives").is_some() {
        return "lessonPlan".into();
    }
    if paper.get("sections").is_some() {
        return "exam".into();
    }
    "exam".into()
}

fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("history");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("papers.json"))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn load_all(app: &AppHandle) -> Result<Vec<HistoryEntry>, String> {
    let path = history_path(app)?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let file: Option<HistoryFile> = read_json(&path)?;
    Ok(match file {
        Some(HistoryFile::Versioned(store)) => store.entries,
        Some(HistoryFile::Legacy(entries)) => entries,
        None => vec![],
    })
}

fn save_all(app: &AppHandle, list: &[HistoryEntry]) -> Result<(), String> {
    let path = history_path(app)?;
    write_json(
        &path,
        &HistoryStore {
            schema_version: SCHEMA_VERSION,
            entries: list.to_vec(),
        },
    )
}

pub fn list_history(app: &AppHandle) -> Result<Vec<HistoryEntry>, String> {
    let mut list = load_all(app)?;
    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(list)
}

pub fn add_history(
    app: &AppHandle,
    paper: Value,
    form_snapshot: Option<Value>,
    max_keep: usize,
) -> Result<HistoryEntry, String> {
    let kind = detect_kind(&paper);
    let title = paper
        .pointer("/meta/title")
        .and_then(|v| v.as_str())
        .unwrap_or(match kind.as_str() {
            "lessonPlan" | "lessonPlanBundle" => "未命名教案",
            "reviewOutline" => "未命名讲评稿",
            _ => "未命名试卷",
        })
        .to_string();

    let subject = paper
        .pointer("/meta/subject")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let grade = paper
        .pointer("/meta/grade")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let summary = if kind == "lessonPlan" || kind == "lessonPlanBundle" {
        let unit = paper
            .pointer("/meta/unitName")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let lesson = paper
            .pointer("/meta/lessonName")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if kind == "lessonPlanBundle" {
            let n = paper
                .get("plans")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            format!("{grade}年级 · {subject} · 全课时教案 · {unit} · {n} 课")
        } else {
            format!("{grade}年级 · {subject} · 教案 · {unit} · {lesson}")
        }
    } else if kind == "reviewOutline" {
        format!("{grade}年级 · {subject} · 作业讲评提纲")
    } else {
        let sections = paper
            .get("sections")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        let items: usize = paper
            .get("sections")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|s| {
                        s.get("items")
                            .and_then(|i| i.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0)
                    })
                    .sum()
            })
            .unwrap_or(0);
        let exam = paper
            .pointer("/meta/examType")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        format!("{grade}年级 · {subject} · {exam} · {sections}大题/{items}小题")
    };

    let entry = HistoryEntry {
        id: unique_id("history"),
        created_at: now_secs(),
        title: title.clone(),
        summary,
        kind,
        paper,
        form_snapshot,
    };

    let mut list = load_all(app)?;
    list.insert(0, entry.clone());
    let keep = if max_keep == 0 { MAX_DEFAULT } else { max_keep };
    if list.len() > keep {
        list.truncate(keep);
    }
    save_all(app, &list)?;
    Ok(entry)
}

pub fn get_history(app: &AppHandle, id: &str) -> Result<HistoryEntry, String> {
    load_all(app)?
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| "历史记录不存在".into())
}

pub fn delete_history(app: &AppHandle, id: &str) -> Result<(), String> {
    let mut list = load_all(app)?;
    let before = list.len();
    list.retain(|e| e.id != id);
    if list.len() == before {
        return Err("历史记录不存在".into());
    }
    save_all(app, &list)
}

pub fn clear_history(app: &AppHandle) -> Result<(), String> {
    save_all(app, &[])
}
