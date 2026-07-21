//! 校本收藏：收藏题 + 本校卷导入（本机 JSON）

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use crate::storage::{read_json, unique_id, write_json};

const MAX_ITEMS: usize = 500;
const MAX_PAPERS: usize = 80;
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteItem {
    pub id: String,
    pub created_at: u64,
    pub stem: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub answer: String,
    #[serde(default)]
    pub analysis: String,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub knowledge_points: Vec<String>,
    #[serde(default)]
    pub section_type: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub grade: u8,
    #[serde(default)]
    pub edition: String,
    #[serde(default)]
    pub source_title: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankPaper {
    pub id: String,
    pub created_at: u64,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub grade: u8,
    pub paper: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BankStore {
    #[serde(default = "schema_version")]
    schema_version: u32,
    #[serde(default)]
    items: Vec<FavoriteItem>,
    #[serde(default)]
    papers: Vec<BankPaper>,
}

fn schema_version() -> u32 {
    SCHEMA_VERSION
}

impl Default for BankStore {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            items: vec![],
            papers: vec![],
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn bank_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("bank");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("school_bank.json"))
}

fn load_store(app: &AppHandle) -> Result<BankStore, String> {
    let path = bank_path(app)?;
    if !path.exists() {
        return Ok(BankStore::default());
    }
    Ok(read_json(&path)?.unwrap_or_default())
}

fn save_store(app: &AppHandle, store: &BankStore) -> Result<(), String> {
    let path = bank_path(app)?;
    let mut versioned = store.clone();
    versioned.schema_version = SCHEMA_VERSION;
    write_json(&path, &versioned)
}

pub fn list_favorites(app: &AppHandle) -> Result<Vec<FavoriteItem>, String> {
    let mut store = load_store(app)?;
    store.items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(store.items)
}

pub fn list_bank_papers(app: &AppHandle) -> Result<Vec<BankPaper>, String> {
    let mut store = load_store(app)?;
    store.papers.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(store.papers)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddFavoriteRequest {
    pub item: Value,
    #[serde(default)]
    pub section_type: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub grade: u8,
    #[serde(default)]
    pub edition: String,
    #[serde(default)]
    pub source_title: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub fn add_favorite(app: &AppHandle, req: AddFavoriteRequest) -> Result<FavoriteItem, String> {
    let stem = req
        .item
        .get("stem")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if stem.is_empty() {
        return Err("题目内容为空，无法收藏".into());
    }

    let options = req
        .item
        .get("options")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let answer = req
        .item
        .get("answer")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let analysis = req
        .item
        .get("analysis")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let score = req
        .item
        .get("score")
        .and_then(|v| v.as_f64())
        .or_else(|| {
            req.item
                .get("score")
                .and_then(|v| v.as_u64())
                .map(|n| n as f64)
        })
        .unwrap_or(0.0);
    let knowledge_points = req
        .item
        .get("knowledgePoints")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let fav = FavoriteItem {
        id: unique_id("favorite"),
        created_at: now_secs(),
        stem,
        options,
        answer,
        analysis,
        score,
        knowledge_points,
        section_type: req.section_type,
        subject: req.subject,
        grade: req.grade,
        edition: req.edition,
        source_title: req.source_title,
        tags: req.tags,
    };

    let mut store = load_store(app)?;
    // 去重：同 stem + answer 不重复收藏
    if store
        .items
        .iter()
        .any(|x| x.stem == fav.stem && x.answer == fav.answer)
    {
        return Err("该题已在校本收藏中".into());
    }
    store.items.insert(0, fav.clone());
    if store.items.len() > MAX_ITEMS {
        store.items.truncate(MAX_ITEMS);
    }
    save_store(app, &store)?;
    Ok(fav)
}

pub fn delete_favorite(app: &AppHandle, id: &str) -> Result<(), String> {
    let mut store = load_store(app)?;
    let before = store.items.len();
    store.items.retain(|x| x.id != id);
    if store.items.len() == before {
        return Err("收藏不存在".into());
    }
    save_store(app, &store)
}

pub fn clear_favorites(app: &AppHandle) -> Result<(), String> {
    let mut store = load_store(app)?;
    store.items.clear();
    save_store(app, &store)
}

/// 导入本校试卷 JSON（meta+sections）
pub fn import_bank_paper(app: &AppHandle, paper: Value) -> Result<BankPaper, String> {
    if paper.get("sections").is_none() {
        return Err("不是有效试卷 JSON（缺少 sections）".into());
    }
    let title = paper
        .pointer("/meta/title")
        .and_then(|v| v.as_str())
        .unwrap_or("导入试卷")
        .to_string();
    let subject = paper
        .pointer("/meta/subject")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let grade = paper
        .pointer("/meta/grade")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;
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

    let entry = BankPaper {
        id: unique_id("paper"),
        created_at: now_secs(),
        title: title.clone(),
        summary: format!("{grade}年级 · {subject} · {sections}大题/{items}小题"),
        subject,
        grade,
        paper,
    };

    let mut store = load_store(app)?;
    store.papers.insert(0, entry.clone());
    if store.papers.len() > MAX_PAPERS {
        store.papers.truncate(MAX_PAPERS);
    }
    save_store(app, &store)?;
    Ok(entry)
}

/// 导入时可选：把卷内全部小题也加入收藏
pub fn import_paper_and_items(
    app: &AppHandle,
    paper: Value,
    also_items: bool,
) -> Result<(BankPaper, usize), String> {
    let entry = import_bank_paper(app, paper.clone())?;
    let mut added = 0usize;
    if also_items {
        let subject = paper
            .pointer("/meta/subject")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let grade = paper
            .pointer("/meta/grade")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u8;
        let edition = paper
            .pointer("/meta/edition")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let source_title = entry.title.clone();
        if let Some(sections) = paper.get("sections").and_then(|v| v.as_array()) {
            for sec in sections {
                let section_type = sec
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(items) = sec.get("items").and_then(|v| v.as_array()) {
                    for item in items {
                        let req = AddFavoriteRequest {
                            item: item.clone(),
                            section_type: section_type.clone(),
                            subject: subject.clone(),
                            grade,
                            edition: edition.clone(),
                            source_title: source_title.clone(),
                            tags: vec!["导入".into()],
                        };
                        if add_favorite(app, req).is_ok() {
                            added += 1;
                        }
                    }
                }
            }
        }
    }
    Ok((entry, added))
}

pub fn delete_bank_paper(app: &AppHandle, id: &str) -> Result<(), String> {
    let mut store = load_store(app)?;
    let before = store.papers.len();
    store.papers.retain(|x| x.id != id);
    if store.papers.len() == before {
        return Err("校本卷不存在".into());
    }
    save_store(app, &store)
}

pub fn get_bank_paper(app: &AppHandle, id: &str) -> Result<BankPaper, String> {
    load_store(app)?
        .papers
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "校本卷不存在".into())
}

/// 为组卷挑选校本收藏摘要（供 AI 改编）
pub fn pick_favorite_snippets(
    app: &AppHandle,
    subject: &str,
    grade: u8,
    limit: usize,
) -> Result<Vec<String>, String> {
    let items = list_favorites(app)?;
    let subject_cn = match subject {
        "math" => "数学",
        "chinese" => "语文",
        "english" => "英语",
        _ => subject,
    };
    let mut filtered: Vec<_> = items
        .into_iter()
        .filter(|f| {
            let sub_ok = f.subject.is_empty()
                || f.subject == subject
                || f.subject == subject_cn
                || f.subject.contains(subject_cn);
            let grade_ok = f.grade == 0 || grade == 0 || f.grade == grade || (f.grade as i16 - grade as i16).abs() <= 1;
            sub_ok && grade_ok
        })
        .collect();
    if filtered.is_empty() {
        // 放宽：任意学科
        filtered = list_favorites(app)?;
    }
    let limit = limit.max(1).min(20);
    Ok(filtered
        .into_iter()
        .take(limit)
        .map(|f| {
            let kp = if f.knowledge_points.is_empty() {
                String::new()
            } else {
                format!(" [知识点:{}]", f.knowledge_points.join("、"))
            };
            let ans = if f.answer.is_empty() {
                String::new()
            } else {
                format!(" → {}", f.answer.chars().take(40).collect::<String>())
            };
            format!(
                "{}{}{}",
                f.stem.chars().take(120).collect::<String>(),
                kp,
                ans
            )
        })
        .collect())
}
