//! 双向细目表：从试卷 JSON 统计知识点覆盖与分值分布

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecKpRow {
    pub knowledge_point: String,
    pub item_count: usize,
    pub total_score: f64,
    pub score_ratio: f64,
    pub item_ids: Vec<String>,
    pub section_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecSectionRow {
    pub title: String,
    pub section_type: String,
    pub item_count: usize,
    pub score: f64,
    pub score_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecTable {
    pub title: String,
    pub subject: String,
    pub grade: u64,
    pub total_score: f64,
    pub total_items: usize,
    pub scored_items: usize,
    pub knowledge_rows: Vec<SpecKpRow>,
    pub section_rows: Vec<SpecSectionRow>,
    pub uncovered_note: String,
    pub summary: String,
}

fn item_score(item: &Value, section_score: f64, item_count: usize) -> f64 {
    if let Some(s) = item.get("score").and_then(|v| v.as_f64()) {
        return s;
    }
    if let Some(s) = item.get("score").and_then(|v| v.as_u64()) {
        return s as f64;
    }
    if item_count > 0 && section_score > 0.0 {
        return section_score / item_count as f64;
    }
    0.0
}

fn collect_kps(item: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = item.get("knowledgePoints").and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = v.as_str() {
                let t = s.trim();
                if !t.is_empty() {
                    out.push(t.to_string());
                }
            }
        }
    }
    if out.is_empty() {
        out.push("\u{ff08}\u{672a}\u{6807}\u{6ce8}\u{77e5}\u{8bc6}\u{70b9}\u{ff09}".into());
    }
    out
}

/// 从试卷生成双向细目表（本地统计，无需 AI）
pub fn build_spec_table(paper: &Value) -> SpecTable {
    let unmarked_label = "\u{ff08}\u{672a}\u{6807}\u{6ce8}\u{77e5}\u{8bc6}\u{70b9}\u{ff09}";
    let title = paper
        .pointer("/meta/title")
        .and_then(|v| v.as_str())
        .unwrap_or("\u{672a}\u{547d}\u{540d}\u{8bd5}\u{5377}")
        .to_string();
    let subject = paper
        .pointer("/meta/subject")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let grade = paper
        .pointer("/meta/grade")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let meta_total = paper
        .pointer("/meta/totalScore")
        .and_then(|v| v.as_f64())
        .or_else(|| {
            paper
                .pointer("/meta/totalScore")
                .and_then(|v| v.as_u64())
                .map(|n| n as f64)
        })
        .unwrap_or(0.0);

    let sections = paper
        .get("sections")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    struct Acc {
        count: usize,
        score: f64,
        ids: Vec<String>,
        sections: Vec<String>,
    }
    let mut kp_map: BTreeMap<String, Acc> = BTreeMap::new();
    let mut section_rows = Vec::new();
    let mut total_items = 0usize;
    let mut scored_items = 0usize;
    let mut sum_item_score = 0.0f64;

    for (si, sec) in sections.iter().enumerate() {
        let sec_title = sec
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("\u{5927}\u{9898}")
            .to_string();
        let sec_type = sec
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let sec_score = sec
            .get("score")
            .and_then(|v| v.as_f64())
            .or_else(|| sec.get("score").and_then(|v| v.as_u64()).map(|n| n as f64))
            .unwrap_or(0.0);
        let items = sec
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let n = items.len();
        total_items += n;

        let mut sec_item_score_sum = 0.0f64;
        for (ii, item) in items.iter().enumerate() {
            let sc = item_score(item, sec_score, n.max(1));
            sec_item_score_sum += sc;
            sum_item_score += sc;
            if sc > 0.0 {
                scored_items += 1;
            }
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}-{}", si + 1, ii + 1));
            for kp in collect_kps(item) {
                let e = kp_map.entry(kp).or_insert_with(|| Acc {
                    count: 0,
                    score: 0.0,
                    ids: vec![],
                    sections: vec![],
                });
                e.count += 1;
                e.score += sc;
                if !e.ids.contains(&id) {
                    e.ids.push(id.clone());
                }
                if !e.sections.contains(&sec_title) {
                    e.sections.push(sec_title.clone());
                }
            }
        }

        let display_score = if sec_score > 0.0 {
            sec_score
        } else {
            sec_item_score_sum
        };
        section_rows.push(SpecSectionRow {
            title: sec_title,
            section_type: sec_type,
            item_count: n,
            score: display_score,
            score_ratio: 0.0,
        });
    }

    let total_score = if meta_total > 0.0 {
        meta_total
    } else {
        section_rows.iter().map(|r| r.score).sum()
    };
    let denom = if total_score > 0.0 {
        total_score
    } else if sum_item_score > 0.0 {
        sum_item_score
    } else {
        1.0
    };

    for r in &mut section_rows {
        r.score_ratio = ((r.score / denom) * 1000.0).round() / 10.0;
    }

    let mut knowledge_rows: Vec<SpecKpRow> = kp_map
        .into_iter()
        .map(|(kp, acc)| SpecKpRow {
            knowledge_point: kp,
            item_count: acc.count,
            total_score: (acc.score * 10.0).round() / 10.0,
            score_ratio: ((acc.score / denom) * 1000.0).round() / 10.0,
            item_ids: acc.ids,
            section_titles: acc.sections,
        })
        .collect();
    knowledge_rows.sort_by(|a, b| {
        b.total_score
            .partial_cmp(&a.total_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.knowledge_point.cmp(&b.knowledge_point))
    });

    let unmarked = knowledge_rows
        .iter()
        .find(|r| r.knowledge_point == unmarked_label)
        .map(|r| r.item_count)
        .unwrap_or(0);
    let uncovered_note = if unmarked > 0 {
        format!(
            "\u{6709} {unmarked} \u{9053}\u{9898}\u{672a}\u{6807}\u{6ce8} knowledgePoints\u{ff0c}\u{5efa}\u{8bae}\u{8865}\u{5168}\u{540e}\u{7ec6}\u{76ee}\u{8868}\u{66f4}\u{51c6}\u{786e}\u{3002}"
        )
    } else if knowledge_rows.is_empty() {
        "\u{8bd5}\u{5377}\u{4e2d}\u{6682}\u{65e0}\u{9898}\u{76ee}\u{3002}".into()
    } else {
        "\u{5404}\u{9898}\u{5747}\u{5df2}\u{6807}\u{6ce8}\u{77e5}\u{8bc6}\u{70b9}\u{3002}".into()
    };

    let kp_n = knowledge_rows
        .iter()
        .filter(|r| r.knowledge_point != unmarked_label)
        .count();
    let summary = format!(
        "\u{5171} {total_items} \u{9898} \u{00b7} \u{6ee1}\u{5206} {total_score} \u{5206} \u{00b7} \u{8986}\u{76d6} {kp_n} \u{4e2a}\u{77e5}\u{8bc6}\u{70b9} \u{00b7} {} \u{4e2a}\u{5927}\u{9898}",
        section_rows.len()
    );

    SpecTable {
        title,
        subject,
        grade,
        total_score,
        total_items,
        scored_items,
        knowledge_rows,
        section_rows,
        uncovered_note,
        summary,
    }
}
