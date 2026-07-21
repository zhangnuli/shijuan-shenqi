use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UnitInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub lessons: Vec<String>,
    #[serde(default)]
    pub points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub platform_url: String,
    #[serde(default)]
    pub classroom_url: String,
    #[serde(default)]
    pub material_url: String,
    #[serde(default)]
    pub elec_edu_url: String,
    #[serde(default)]
    pub catalog_site: String,
    #[serde(default)]
    pub catalog_ref: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub edition_label: String,
    #[serde(default)]
    pub subject_label: String,
    #[serde(default)]
    pub smartedu_path_hint: String,
    #[serde(default)]
    pub entry_count: usize,
    #[serde(default)]
    pub unit_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgePack {
    pub subject: String,
    pub edition: String,
    pub grade: u8,
    pub semester: String,
    pub title: String,
    #[serde(default)]
    pub source: SourceInfo,
    pub units: Vec<UnitInfo>,
    #[serde(default)]
    pub exam_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    pub subject: String,
    pub subject_label: String,
    pub edition: String,
    pub edition_label: String,
    pub grade: u8,
    pub semester: String,
    pub semester_label: String,
    pub title: String,
    pub path: String,
    pub units: Vec<UnitInfo>,
    #[serde(default)]
    pub source: SourceInfo,
    /// bundled | user
    #[serde(default)]
    pub origin: String,
}

/// 用户一键更新后的课标目录（本机同步）
pub fn user_curriculum_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|p| p.join("curriculum"))
}

pub fn bundled_data_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(p) = app.path().resource_dir() {
        v.push(p.join("resources").join("data"));
        v.push(p.join("data"));
    }
    v.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("data"));
    v
}

/// 加载时：用户更新目录优先，再回退内置包
pub fn data_roots(app: &AppHandle) -> Vec<(PathBuf, &'static str)> {
    let mut roots = Vec::new();
    if let Some(user) = user_curriculum_dir(app) {
        if user.exists() {
            roots.push((user, "user"));
        }
    }
    for p in bundled_data_dirs(app) {
        if p.exists() {
            roots.push((p, "bundled"));
        }
    }
    roots
}

pub fn list_catalog(app: &AppHandle) -> Result<Vec<CatalogItem>, String> {
    let roots = data_roots(app);
    if roots.is_empty() {
        return Err("找不到知识点资源目录（内置或已更新课标）".into());
    }

    // path -> item，后写不覆盖先写：用户优先已先扫描
    let mut map: std::collections::BTreeMap<String, CatalogItem> = std::collections::BTreeMap::new();

    for (root, origin) in &roots {
        let mut items = Vec::new();
        walk_json(root, root, *origin, &mut items)?;
        for it in items {
            let key = format!(
                "{}|{}|{}|{}",
                it.subject, it.edition, it.grade, it.semester
            );
            map.entry(key).or_insert(it);
        }
    }

    let mut items: Vec<_> = map.into_values().collect();
    items.sort_by(|a, b| {
        (
            a.subject.as_str(),
            a.edition.as_str(),
            a.grade,
            a.semester.as_str(),
        )
            .cmp(&(
                b.subject.as_str(),
                b.edition.as_str(),
                b.grade,
                b.semester.as_str(),
            ))
    });
    Ok(items)
}

fn walk_json(
    root: &Path,
    dir: &Path,
    origin: &str,
    out: &mut Vec<CatalogItem>,
) -> Result<(), String> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            walk_json(root, &path, origin, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if path.file_name().and_then(|s| s.to_str()) == Some("index.json") {
                continue;
            }
            if path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|n| n.starts_with('_'))
                .unwrap_or(false)
            {
                continue;
            }
            if let Ok(pack) = load_pack_file(&path) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let edition_label = if !pack.source.edition_label.is_empty() {
                    pack.source.edition_label.clone()
                } else {
                    edition_label(&pack.edition)
                };
                let subject_label = if !pack.source.subject_label.is_empty() {
                    pack.source.subject_label.clone()
                } else {
                    subject_label(&pack.subject)
                };
                out.push(CatalogItem {
                    subject: pack.subject.clone(),
                    subject_label,
                    edition: pack.edition.clone(),
                    edition_label,
                    grade: pack.grade,
                    semester: pack.semester.clone(),
                    semester_label: if pack.semester == "shang" {
                        "上册".into()
                    } else {
                        "下册".into()
                    },
                    title: pack.title,
                    path: rel,
                    units: pack.units,
                    source: pack.source,
                    origin: origin.into(),
                });
            }
        }
    }
    Ok(())
}

fn subject_label(s: &str) -> String {
    match s {
        "math" => "数学".into(),
        "chinese" => "语文".into(),
        "english" => "英语".into(),
        _ => s.into(),
    }
}

fn edition_label(s: &str) -> String {
    match s {
        "beishida" => "北师大版".into(),
        "renjiao" => "人教版".into(),
        "sujiao" => "苏教版".into(),
        _ => s.into(),
    }
}

pub fn load_pack(app: &AppHandle, rel_path: &str) -> Result<KnowledgePack, String> {
    // 用户目录优先
    if let Some(user) = user_curriculum_dir(app) {
        let p = user.join(rel_path);
        if p.exists() {
            return load_pack_file(&p);
        }
    }
    for root in bundled_data_dirs(app) {
        let p = root.join(rel_path);
        if p.exists() {
            return load_pack_file(&p);
        }
    }
    Err(format!("找不到知识点包: {rel_path}"))
}

pub fn load_pack_file(path: &Path) -> Result<KnowledgePack, String> {
    let s = fs::read_to_string(path).map_err(|e| format!("读取 {path:?} 失败: {e}"))?;
    serde_json::from_str(&s).map_err(|e| format!("解析知识点包失败 {path:?}: {e}"))
}
