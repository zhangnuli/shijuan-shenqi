//! 课标对照：内置包 vs 本机同步包

use crate::knowledge::{bundled_data_dirs, load_pack_file, user_curriculum_dir, UnitInfo};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitDiff {
    pub unit_id: String,
    pub unit_name: String,
    /// same | onlyBundled | onlyUser | changed
    pub status: String,
    pub bundled_lessons: Vec<String>,
    pub user_lessons: Vec<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurriculumDiffReport {
    pub path: String,
    pub subject: String,
    pub edition: String,
    pub grade: u8,
    pub semester: String,
    pub has_bundled: bool,
    pub has_user: bool,
    pub summary: String,
    pub units: Vec<UnitDiff>,
    pub added_units: Vec<String>,
    pub removed_units: Vec<String>,
}

fn find_bundled_file(app: &AppHandle, rel: &str) -> Option<PathBuf> {
    for root in bundled_data_dirs(app) {
        let p = root.join(rel);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn find_user_file(app: &AppHandle, rel: &str) -> Option<PathBuf> {
    user_curriculum_dir(app).map(|d| d.join(rel)).filter(|p| p.exists())
}

fn unit_key(u: &UnitInfo) -> String {
    if !u.id.is_empty() {
        u.id.clone()
    } else {
        u.name.clone()
    }
}

fn lessons_of(u: &UnitInfo) -> BTreeSet<String> {
    u.lessons.iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// 对比某册课标：内置 vs 用户同步
pub fn diff_curriculum_pack(app: &AppHandle, rel_path: &str) -> Result<CurriculumDiffReport, String> {
    let rel = rel_path.trim().trim_start_matches('/').trim_start_matches('\\');
    let bundled_path = find_bundled_file(app, rel);
    let user_path = find_user_file(app, rel);

    let bundled = bundled_path
        .as_ref()
        .map(|p| load_pack_file(p))
        .transpose()?;
    let user = user_path.as_ref().map(|p| load_pack_file(p)).transpose()?;

    let has_bundled = bundled.is_some();
    let has_user = user.is_some();

    if !has_bundled && !has_user {
        return Err(format!("找不到课标包: {rel}"));
    }

    let b_units = bundled.as_ref().map(|p| &p.units).cloned().unwrap_or_default();
    let u_units = user.as_ref().map(|p| &p.units).cloned().unwrap_or_default();

    let b_map: std::collections::BTreeMap<String, UnitInfo> =
        b_units.into_iter().map(|u| (unit_key(&u), u)).collect();
    let u_map: std::collections::BTreeMap<String, UnitInfo> =
        u_units.into_iter().map(|u| (unit_key(&u), u)).collect();

    let all_keys: BTreeSet<String> = b_map.keys().chain(u_map.keys()).cloned().collect();
    let mut units = Vec::new();
    let mut added_units = Vec::new();
    let mut removed_units = Vec::new();

    for k in all_keys {
        match (b_map.get(&k), u_map.get(&k)) {
            (Some(bu), Some(uu)) => {
                let bl = lessons_of(bu);
                let ul = lessons_of(uu);
                let added: Vec<_> = ul.difference(&bl).cloned().collect();
                let removed: Vec<_> = bl.difference(&ul).cloned().collect();
                let status = if added.is_empty() && removed.is_empty() && bu.name == uu.name {
                    "same"
                } else {
                    "changed"
                };
                units.push(UnitDiff {
                    unit_id: k,
                    unit_name: uu.name.clone(),
                    status: status.into(),
                    bundled_lessons: bl.into_iter().collect(),
                    user_lessons: ul.into_iter().collect(),
                    added,
                    removed,
                });
            }
            (Some(bu), None) => {
                removed_units.push(bu.name.clone());
                let bl = lessons_of(bu);
                units.push(UnitDiff {
                    unit_id: k,
                    unit_name: bu.name.clone(),
                    status: "onlyBundled".into(),
                    bundled_lessons: bl.into_iter().collect(),
                    user_lessons: vec![],
                    added: vec![],
                    removed: vec![],
                });
            }
            (None, Some(uu)) => {
                added_units.push(uu.name.clone());
                let ul = lessons_of(uu);
                units.push(UnitDiff {
                    unit_id: k,
                    unit_name: uu.name.clone(),
                    status: "onlyUser".into(),
                    bundled_lessons: vec![],
                    user_lessons: ul.into_iter().collect(),
                    added: vec![],
                    removed: vec![],
                });
            }
            _ => {}
        }
    }

    let changed = units.iter().filter(|u| u.status == "changed").count();
    let summary = match (has_bundled, has_user) {
        (true, true) => format!(
            "对照完成：{} 个单元有差异，同步多 {} 个单元，内置多 {} 个单元",
            changed,
            added_units.len(),
            removed_units.len()
        ),
        (true, false) => "仅有内置课标，尚未同步本机包（同步后可对照）".into(),
        (false, true) => "仅有同步课标，无对应内置包可对照".into(),
        _ => "无数据".into(),
    };

    let meta = user.as_ref().or(bundled.as_ref()).unwrap();
    Ok(CurriculumDiffReport {
        path: rel.into(),
        subject: meta.subject.clone(),
        edition: meta.edition.clone(),
        grade: meta.grade,
        semester: meta.semester.clone(),
        has_bundled,
        has_user,
        summary,
        units,
        added_units,
        removed_units,
    })
}
