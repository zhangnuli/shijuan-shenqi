//! 作业讲评稿：根据错题/知识点生成讲评提纲

use crate::ai::{chat_completion, extract_json};
use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrongItemRef {
    pub section_index: usize,
    pub item_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRequest {
    pub paper: Value,
    /// 勾选的错题位置；空则使用 knowledge_points
    #[serde(default)]
    pub wrong_items: Vec<WrongItemRef>,
    /// 额外知识点（可手填）
    #[serde(default)]
    pub knowledge_points: Vec<String>,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub grade: u8,
    /// 是否调用 AI；false 则出本地模板
    #[serde(default = "default_true")]
    pub use_ai: bool,
}

fn default_true() -> bool {
    true
}

fn collect_wrong_payload(req: &ReviewRequest) -> (Vec<Value>, Vec<String>) {
    let mut items_out = Vec::new();
    let mut kps: Vec<String> = req.knowledge_points.clone();
    let sections = req
        .paper
        .get("sections")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if !req.wrong_items.is_empty() {
        for r in &req.wrong_items {
            if let Some(sec) = sections.get(r.section_index) {
                let sec_title = sec
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("大题");
                if let Some(item) = sec
                    .get("items")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.get(r.item_index))
                {
                    if let Some(arr) = item.get("knowledgePoints").and_then(|v| v.as_array()) {
                        for kp in arr {
                            if let Some(s) = kp.as_str() {
                                let t = s.trim();
                                if !t.is_empty() && !kps.contains(&t.to_string()) {
                                    kps.push(t.to_string());
                                }
                            }
                        }
                    }
                    items_out.push(json!({
                        "sectionTitle": sec_title,
                        "stem": item.get("stem").and_then(|v| v.as_str()).unwrap_or(""),
                        "answer": item.get("answer").and_then(|v| v.as_str()).unwrap_or(""),
                        "analysis": item.get("analysis").and_then(|v| v.as_str()).unwrap_or(""),
                        "knowledgePoints": item.get("knowledgePoints").cloned().unwrap_or(json!([])),
                    }));
                }
            }
        }
    }

    // 未勾错题时：用知识点从全卷筛相关题（最多 8 道）
    if items_out.is_empty() && !kps.is_empty() {
        for sec in &sections {
            let sec_title = sec
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("大题");
            if let Some(items) = sec.get("items").and_then(|v| v.as_array()) {
                for item in items {
                    let item_kps: Vec<String> = item
                        .get("knowledgePoints")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    let hit = item_kps.iter().any(|k| kps.iter().any(|t| k.contains(t) || t.contains(k)));
                    if hit {
                        items_out.push(json!({
                            "sectionTitle": sec_title,
                            "stem": item.get("stem").and_then(|v| v.as_str()).unwrap_or(""),
                            "answer": item.get("answer").and_then(|v| v.as_str()).unwrap_or(""),
                            "analysis": item.get("analysis").and_then(|v| v.as_str()).unwrap_or(""),
                            "knowledgePoints": item_kps,
                        }));
                        if items_out.len() >= 8 {
                            break;
                        }
                    }
                }
            }
            if items_out.len() >= 8 {
                break;
            }
        }
    }

    (items_out, kps)
}

fn local_template(
    title: &str,
    subject: &str,
    grade: u8,
    kps: &[String],
    items: &[Value],
) -> Value {
    let mut points: Vec<Value> = kps
        .iter()
        .map(|kp| {
            json!({
                "knowledgePoint": kp,
                "errorPattern": format!("学生对「{kp}」掌握不牢固，易出现概念混淆或步骤遗漏。"),
                "keyExplain": format!("回扣定义/算理，板书示范「{kp}」的标准过程。"),
                "boardNote": format!("重点：{kp}"),
                "practice": [
                    format!("再练 2 道与「{kp}」同类变式题"),
                    "口述解题步骤，同伴互查"
                ],
                "minutes": 8
            })
        })
        .collect();
    if points.is_empty() {
        points.push(json!({
            "knowledgePoint": "综合错题",
            "errorPattern": "审题不清、步骤不完整、计算失误。",
            "keyExplain": "先读题找关键词，再列式/列步骤，最后检验。",
            "boardNote": "审题 → 列式 → 检验",
            "practice": ["同型再练 1 题", "错题订正并写一句反思"],
            "minutes": 10
        }));
    }

    let sample_stems: Vec<String> = items
        .iter()
        .filter_map(|it| it.get("stem").and_then(|v| v.as_str()).map(|s| s.chars().take(40).collect::<String>()))
        .take(5)
        .collect();

    json!({
        "kind": "reviewOutline",
        "meta": {
            "title": format!("{title} · 作业讲评提纲"),
            "subject": subject,
            "grade": grade,
            "source": "local-template",
            "durationMin": 40
        },
        "overview": format!(
            "本节针对 {} 等知识点进行集中讲评，建议用时约 40 分钟。",
            if kps.is_empty() { "错题共性问题".to_string() } else { kps.join("、") }
        ),
        "knowledgeFocus": kps,
        "wrongSamples": sample_stems,
        "process": [
            {
                "stage": "导入回顾",
                "minutes": 5,
                "content": "公布共性问题与目标，明确本节要突破的 2～3 个点。",
                "teacherActivity": "展示错因统计（或口述高频错点）",
                "studentActivity": "对照自己错题，标记同类错误"
            },
            {
                "stage": "逐点讲评",
                "minutes": 20,
                "content": "按知识点拆解错因，示范规范解法，学生口述复述。",
                "teacherActivity": "板书关键步骤，对比正确与错误样例",
                "studentActivity": "订正错题，补写步骤"
            },
            {
                "stage": "变式巩固",
                "minutes": 10,
                "content": "每点 1～2 道变式，当堂反馈。",
                "teacherActivity": "巡视、点拨",
                "studentActivity": "限时完成并互批"
            },
            {
                "stage": "小结与作业",
                "minutes": 5,
                "content": "归纳易错清单；布置订正与少量巩固题。",
                "teacherActivity": "总结三句话",
                "studentActivity": "整理错题本"
            }
        ],
        "points": points,
        "homework": [
            "错题完整订正（写清步骤与错因）",
            "每类知识点再练 1～2 道",
            "家长/同桌抽查口述算理或关键词"
        ],
        "reflection": "记录仍未掌握的学生名单，下节课前 5 分钟再测。"
    })
}

/// 根据错题/知识点生成「再练卷」（短卷）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedrillRequest {
    pub paper: Value,
    #[serde(default)]
    pub wrong_items: Vec<WrongItemRef>,
    #[serde(default)]
    pub knowledge_points: Vec<String>,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub grade: u8,
    #[serde(default = "default_score")]
    pub total_score: u32,
    #[serde(default = "default_duration")]
    pub duration_min: u32,
}

fn default_score() -> u32 {
    50
}
fn default_duration() -> u32 {
    20
}

pub fn generate_redrill_paper(cfg: &AppConfig, req: &RedrillRequest) -> Result<Value, String> {
    let review_like = ReviewRequest {
        paper: req.paper.clone(),
        wrong_items: req.wrong_items.clone(),
        knowledge_points: req.knowledge_points.clone(),
        subject: req.subject.clone(),
        grade: req.grade,
        use_ai: true,
    };
    let (items, kps) = collect_wrong_payload(&review_like);
    if items.is_empty() && kps.is_empty() {
        return Err("请先勾选错题或填写知识点，再生成再练卷".into());
    }

    let title = req
        .paper
        .pointer("/meta/title")
        .and_then(|v| v.as_str())
        .unwrap_or("本套试卷");
    let subject = if req.subject.is_empty() {
        req.paper
            .pointer("/meta/subject")
            .and_then(|v| v.as_str())
            .unwrap_or("学科")
            .to_string()
    } else {
        req.subject.clone()
    };
    let grade = if req.grade == 0 {
        req.paper
            .pointer("/meta/grade")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u8
    } else {
        req.grade
    };
    let edition = req
        .paper
        .pointer("/meta/edition")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let semester = req
        .paper
        .pointer("/meta/semester")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // 无密钥：本地简易再练卷
    if cfg.api_key.trim().is_empty() {
        return Ok(local_redrill(
            &title,
            &subject,
            &edition,
            &semester,
            grade,
            req.total_score,
            req.duration_min,
            &kps,
            &items,
        ));
    }

    let system = r#"你是小学命题教师。请根据错题样本与知识点，生成一份「错题再练」短卷 JSON（仅 JSON）：
{
  "meta": {
    "edition": "版本",
    "subject": "学科",
    "grade": 年级,
    "semester": "上册或下册",
    "examType": "错题再练",
    "title": "完整卷名（含再练）",
    "totalScore": 分值,
    "durationMin": 分钟
  },
  "sections": [ /* 2～3 个大题，结构同普通试卷 */ ]
}
规则：
1. 考查点紧扣给定知识点与错题，必须变式（换数/换情境），禁止原题照搬
2. 题量适合 15～25 分钟；答案明确可批改
3. 计算题答案为可验算数值；每题带 knowledgePoints
4. sections 分值之和 = totalScore"#
        .to_string();

    let user = format!(
        "年级：{grade}\n学科：{subject}\n版本：{edition}\n原卷：{title}\n满分：{}\n时长：{} 分钟\n知识点：{}\n错题样本：\n{}\n请直接输出再练卷 JSON。",
        req.total_score,
        req.duration_min,
        if kps.is_empty() {
            "（见错题）".into()
        } else {
            kps.join("、")
        },
        serde_json::to_string(&items).unwrap_or_else(|_| "[]".into())
    );

    match chat_completion(cfg, &system, &user) {
        Ok(raw) => {
            let json_str = extract_json(&raw)?;
            let value: Value = serde_json::from_str(&json_str)
                .map_err(|e| format!("再练卷 JSON 无效: {e}"))?;
            if value.get("meta").is_none() || value.get("sections").is_none() {
                return Ok(local_redrill(
                    &title,
                    &subject,
                    &edition,
                    &semester,
                    grade,
                    req.total_score,
                    req.duration_min,
                    &kps,
                    &items,
                ));
            }
            Ok(value)
        }
        Err(_) => Ok(local_redrill(
            &title,
            &subject,
            &edition,
            &semester,
            grade,
            req.total_score,
            req.duration_min,
            &kps,
            &items,
        )),
    }
}

fn local_redrill(
    title: &str,
    subject: &str,
    edition: &str,
    semester: &str,
    grade: u8,
    total_score: u32,
    duration_min: u32,
    kps: &[String],
    items: &[Value],
) -> Value {
    let focus = if kps.is_empty() {
        "错题共性问题".to_string()
    } else {
        kps.join("、")
    };
    let mut drill_items = Vec::new();
    for (i, it) in items.iter().take(6).enumerate() {
        let stem = it
            .get("stem")
            .and_then(|v| v.as_str())
            .unwrap_or("原题")
            .chars()
            .take(80)
            .collect::<String>();
        let kp = it
            .get("knowledgePoints")
            .cloned()
            .unwrap_or(json!([]));
        drill_items.push(json!({
            "id": format!("1-{}", i + 1),
            "stem": format!("{}. 变式巩固：针对「{}」再练一题（原题参考：{}…）。请独立完成。", i + 1, focus, stem),
            "options": [],
            "answer": "（教师按知识点自拟标准答案，或改用 AI 生成完整再练卷）",
            "analysis": "本地模板：建议配置 API 后重新生成完整变式题",
            "score": 5,
            "knowledgePoints": kp
        }));
    }
    if drill_items.is_empty() {
        for (i, kp) in kps.iter().take(5).enumerate() {
            drill_items.push(json!({
                "id": format!("1-{}", i + 1),
                "stem": format!("{}. 关于「{kp}」：请完成一道同类基础题并写出步骤。", i + 1),
                "options": [],
                "answer": "略",
                "analysis": "",
                "score": 6,
                "knowledgePoints": [kp]
            }));
        }
    }
    let n = drill_items.len().max(1) as u32;
    let each = (total_score / 2) / n;
    for it in &mut drill_items {
        if let Some(obj) = it.as_object_mut() {
            obj.insert("score".into(), json!(each.max(2)));
        }
    }
    let s1 = each.max(2) * n;
    let s2 = total_score.saturating_sub(s1);

    json!({
        "meta": {
            "edition": edition,
            "subject": subject,
            "grade": grade,
            "semester": semester,
            "examType": "错题再练",
            "title": format!("{title} · 错题再练"),
            "totalScore": total_score,
            "durationMin": duration_min,
            "source": "local-redrill"
        },
        "sections": [
            {
                "type": "mixed",
                "title": format!("一、变式巩固（共{s1}分）"),
                "score": s1,
                "items": drill_items
            },
            {
                "type": "problem",
                "title": format!("二、综合一题（共{s2}分）"),
                "score": s2,
                "items": [{
                    "id": "2-1",
                    "stem": format!("结合「{focus}」，独立完成一道稍综合的题，写出完整过程。"),
                    "options": [],
                    "answer": "略",
                    "analysis": "",
                    "score": s2,
                    "knowledgePoints": kps
                }]
            }
        ]
    })
}

pub fn generate_review_outline(cfg: &AppConfig, req: &ReviewRequest) -> Result<Value, String> {
    let (items, kps) = collect_wrong_payload(req);
    if items.is_empty() && kps.is_empty() {
        return Err("请先勾选错题，或填写要讲评的知识点".into());
    }

    let title = req
        .paper
        .pointer("/meta/title")
        .and_then(|v| v.as_str())
        .unwrap_or("本套试卷");
    let subject = if req.subject.is_empty() {
        req.paper
            .pointer("/meta/subject")
            .and_then(|v| v.as_str())
            .unwrap_or("学科")
            .to_string()
    } else {
        req.subject.clone()
    };
    let grade = if req.grade == 0 {
        req.paper
            .pointer("/meta/grade")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u8
    } else {
        req.grade
    };

    if !req.use_ai || cfg.api_key.trim().is_empty() {
        return Ok(local_template(title, &subject, grade, &kps, &items));
    }

    let system = r#"你是小学一线教师，擅长作业讲评。请只输出一份 JSON（不要 markdown）：
{
  "kind": "reviewOutline",
  "meta": {
    "title": "讲评稿标题",
    "subject": "学科",
    "grade": 年级数字,
    "durationMin": 40,
    "source": "ai"
  },
  "overview": "本节讲评总述（2～4 句）",
  "knowledgeFocus": ["知识点1"],
  "wrongSamples": ["错题摘要1"],
  "process": [
    {
      "stage": "环节名",
      "minutes": 8,
      "content": "环节内容",
      "teacherActivity": "教师活动",
      "studentActivity": "学生活动"
    }
  ],
  "points": [
    {
      "knowledgePoint": "知识点",
      "errorPattern": "典型错因",
      "keyExplain": "讲解要点",
      "boardNote": "板书要点",
      "practice": ["变式建议1"],
      "minutes": 8
    }
  ],
  "homework": ["作业1"],
  "reflection": "教后反思提示"
}
要求：贴合小学课堂；过程合计约 40 分钟；每个知识点给出错因+讲法+变式；语言简洁可直接上课用。"#
        .to_string();

    let user = format!(
        "年级：{grade}\n学科：{subject}\n原卷：{title}\n聚焦知识点：{}\n错题样本 JSON：\n{}",
        if kps.is_empty() {
            "（见错题）".into()
        } else {
            kps.join("、")
        },
        serde_json::to_string(&items).unwrap_or_else(|_| "[]".into())
    );

    match chat_completion(cfg, &system, &user) {
        Ok(raw) => {
            let json_str = extract_json(&raw)?;
            let mut value: Value = serde_json::from_str(&json_str)
                .map_err(|e| format!("讲评稿 JSON 无效: {e}"))?;
            if value.get("kind").is_none() {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("kind".into(), json!("reviewOutline"));
                }
            }
            if value.get("process").is_none() && value.get("points").is_none() {
                return Ok(local_template(title, &subject, grade, &kps, &items));
            }
            Ok(value)
        }
        Err(_) => Ok(local_template(title, &subject, grade, &kps, &items)),
    }
}
