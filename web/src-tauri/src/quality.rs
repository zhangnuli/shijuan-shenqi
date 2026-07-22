//! 试卷质检：空答案、过短题干、选项问题、知识点缺失、超纲粗检、分值合计

use crate::spec_table::build_spec_table;
use crate::verify::verify_paper_math;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityIssue {
    /// error | warn | info
    pub level: String,
    pub code: String,
    pub message: String,
    pub section_index: Option<usize>,
    pub item_index: Option<usize>,
    pub item_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityReport {
    pub score: u8,
    pub summary: String,
    pub error_count: usize,
    pub warn_count: usize,
    pub info_count: usize,
    pub issues: Vec<QualityIssue>,
    /// 细目表摘要（反查）
    pub spec_summary: String,
    pub knowledge_count: usize,
    pub unmarked_count: usize,
    pub math_mismatch: usize,
    pub math_checked: usize,
}

fn issue(
    level: &str,
    code: &str,
    message: impl Into<String>,
    si: Option<usize>,
    ii: Option<usize>,
    id: Option<String>,
) -> QualityIssue {
    QualityIssue {
        level: level.into(),
        code: code.into(),
        message: message.into(),
        section_index: si,
        item_index: ii,
        item_id: id,
    }
}

/// 对试卷做本地质检（无需 AI）
pub fn inspect_paper(paper: &Value) -> QualityReport {
    let mut issues = Vec::new();
    let sections = paper
        .get("sections")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if sections.is_empty() {
        issues.push(issue(
            "error",
            "no_sections",
            "试卷没有任何大题 sections",
            None,
            None,
            None,
        ));
    }

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

    let mut section_score_sum = 0.0f64;
    let mut total_items = 0usize;
    let mut empty_answer = 0usize;
    let mut short_stem = 0usize;
    let mut no_kp = 0usize;
    let mut bad_choice = 0usize;

    // 超纲粗检：高年级词出现在低年级
    let grade = paper
        .pointer("/meta/grade")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let subject = paper
        .pointer("/meta/subject")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let hard_words_g1: &[&str] = &["方程", "百分数", "分数乘", "圆柱", "圆锥", "比例"];
    let hard_words_g3: &[&str] = &["一元一次方程", "二次根式", "函数"];

    for (si, sec) in sections.iter().enumerate() {
        let sec_title = sec.get("title").and_then(|v| v.as_str()).unwrap_or("大题");
        let sec_type = sec.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let sec_score = sec
            .get("score")
            .and_then(|v| v.as_f64())
            .or_else(|| sec.get("score").and_then(|v| v.as_u64()).map(|n| n as f64))
            .unwrap_or(0.0);
        section_score_sum += sec_score;

        let items = sec
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if items.is_empty() {
            issues.push(issue(
                "error",
                "empty_section",
                format!("「{sec_title}」下没有小题"),
                Some(si),
                None,
                None,
            ));
            continue;
        }

        for (ii, item) in items.iter().enumerate() {
            total_items += 1;
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}-{}", si + 1, ii + 1));
            let stem = item.get("stem").and_then(|v| v.as_str()).unwrap_or("").trim();
            let answer = item
                .get("answer")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();

            if stem.is_empty() {
                issues.push(issue(
                    "error",
                    "empty_stem",
                    format!("第 {id} 题题干为空"),
                    Some(si),
                    Some(ii),
                    Some(id.clone()),
                ));
            } else if stem.chars().count() < 4 {
                short_stem += 1;
                issues.push(issue(
                    "warn",
                    "short_stem",
                    format!("第 {id} 题题干过短，可能不完整"),
                    Some(si),
                    Some(ii),
                    Some(id.clone()),
                ));
            }

            if answer.is_empty() || answer == "略" || answer == "（见教师评阅）" {
                empty_answer += 1;
                let level = if answer.is_empty() { "error" } else { "warn" };
                issues.push(issue(
                    level,
                    "weak_answer",
                    format!("第 {id} 题答案为空或占位（{answer}）"),
                    Some(si),
                    Some(ii),
                    Some(id.clone()),
                ));
            }

            let kps = item
                .get("knowledgePoints")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            if kps == 0 {
                no_kp += 1;
                issues.push(issue(
                    "info",
                    "no_knowledge",
                    format!("第 {id} 题未标注知识点，细目表会归入「未标注」"),
                    Some(si),
                    Some(ii),
                    Some(id.clone()),
                ));
            }

            // 选择题
            if sec_type == "choice" || sec_title.contains("选择") {
                let opts = item
                    .get("options")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                if opts.len() < 2 {
                    bad_choice += 1;
                    issues.push(issue(
                        "error",
                        "choice_options",
                        format!("第 {id} 题为选择题但选项不足"),
                        Some(si),
                        Some(ii),
                        Some(id.clone()),
                    ));
                } else {
                    let texts: Vec<String> = opts
                        .iter()
                        .filter_map(|o| o.as_str().map(|s| s.trim().to_string()))
                        .collect();
                    let mut uniq = texts.clone();
                    uniq.sort();
                    uniq.dedup();
                    if uniq.len() < texts.len() {
                        bad_choice += 1;
                        issues.push(issue(
                            "warn",
                            "dup_options",
                            format!("第 {id} 题选项有重复"),
                            Some(si),
                            Some(ii),
                            Some(id.clone()),
                        ));
                    }
                }
            }

            // 超纲粗检
            if subject.contains("数学") || subject.eq_ignore_ascii_case("math") {
                let text = format!("{stem} {answer}");
                if grade > 0 && grade <= 2 {
                    for w in hard_words_g1 {
                        if text.contains(w) {
                            issues.push(issue(
                                "warn",
                                "over_grade",
                                format!("第 {id} 题出现「{w}」，对 {grade} 年级可能超纲"),
                                Some(si),
                                Some(ii),
                                Some(id.clone()),
                            ));
                            break;
                        }
                    }
                } else if grade > 0 && grade <= 4 {
                    for w in hard_words_g3 {
                        if text.contains(w) {
                            issues.push(issue(
                                "warn",
                                "over_grade",
                                format!("第 {id} 题出现「{w}」，对 {grade} 年级可能超纲"),
                                Some(si),
                                Some(ii),
                                Some(id.clone()),
                            ));
                            break;
                        }
                    }
                }
            }
        }
    }

    if meta_total > 0.0 && section_score_sum > 0.0 {
        let diff = (meta_total - section_score_sum).abs();
        if diff > 0.5 {
            issues.push(issue(
                "warn",
                "score_mismatch",
                format!("卷面满分 {meta_total} 与各大题分值合计 {section_score_sum} 不一致（差 {diff}）"),
                None,
                None,
                None,
            ));
        }
    }

    // 细目反查
    let spec = build_spec_table(paper);
    let unmarked = spec
        .knowledge_rows
        .iter()
        .find(|r| r.knowledge_point.contains("未标注"))
        .map(|r| r.item_count)
        .unwrap_or(0);
    let kp_count = spec
        .knowledge_rows
        .iter()
        .filter(|r| !r.knowledge_point.contains("未标注"))
        .count();

    if unmarked > 0 && unmarked * 2 >= total_items.max(1) {
        issues.push(issue(
            "warn",
            "many_unmarked_kp",
            format!("超过一半题目未标注知识点（{unmarked}/{total_items}），细目表参考价值有限"),
            None,
            None,
            None,
        ));
    }

    // 数学验算
    let mut math_mismatch = 0usize;
    let mut math_checked = 0usize;
    if subject.contains("数学") || subject.eq_ignore_ascii_case("math") {
        let vr = verify_paper_math(paper);
        math_checked = vr.checked;
        math_mismatch = vr.mismatch;
        if vr.mismatch > 0 {
            for it in vr.items.iter().filter(|x| x.status == "mismatch").take(12) {
                issues.push(issue(
                    "error",
                    "math_mismatch",
                    format!(
                        "第 {}-{} 题验算不一致：{}",
                        it.section_index + 1,
                        it.item_index + 1,
                        it.message
                    ),
                    Some(it.section_index),
                    Some(it.item_index),
                    None,
                ));
            }
            if vr.mismatch > 12 {
                issues.push(issue(
                    "error",
                    "math_mismatch_more",
                    format!("另有 {} 道验算不一致未全部列出", vr.mismatch - 12),
                    None,
                    None,
                    None,
                ));
            }
        }
    }

    let error_count = issues.iter().filter(|i| i.level == "error").count();
    let warn_count = issues.iter().filter(|i| i.level == "warn").count();
    let info_count = issues.iter().filter(|i| i.level == "info").count();

    // 评分：100 起扣
    let mut score: i32 = 100;
    score -= (error_count as i32) * 12;
    score -= (warn_count as i32) * 4;
    score -= (info_count as i32).min(10); // info 最多扣 10
    if total_items == 0 {
        score = 0;
    }
    let score = score.clamp(0, 100) as u8;

    let summary = if error_count == 0 && warn_count == 0 {
        format!("质检通过（{score} 分）：{total_items} 题，知识点 {kp_count} 个。{empty_answer} 空答案，{short_stem} 短题干，{no_kp} 未标知识点，{bad_choice} 选项问题。")
    } else if error_count == 0 {
        format!("基本可用（{score} 分）：{warn_count} 条警告，建议导出前过一眼。共 {total_items} 题。")
    } else {
        format!("建议修订后再用（{score} 分）：{error_count} 条错误、{warn_count} 条警告。验算不一致 {math_mismatch}/{math_checked}。")
    };

    QualityReport {
        score,
        summary,
        error_count,
        warn_count,
        info_count,
        issues,
        spec_summary: spec.summary,
        knowledge_count: kp_count,
        unmarked_count: unmarked,
        math_mismatch,
        math_checked,
    }
}
