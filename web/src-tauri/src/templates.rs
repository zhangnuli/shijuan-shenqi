//! 本地模板市集：内置结构模板 + 用户模板（本机 JSON）

use crate::knowledge::KnowledgePack;
use crate::storage::{read_json, unique_id, write_json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TemplateStore {
    schema_version: u32,
    templates: Vec<PaperTemplate>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TemplateFile {
    Versioned(TemplateStore),
    Legacy(Vec<PaperTemplate>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSectionSpec {
    pub r#type: String,
    pub title: String,
    pub score: u32,
    #[serde(default = "default_item_count")]
    pub item_count: u32,
    #[serde(default)]
    pub hint: String,
}

fn default_item_count() -> u32 {
    4
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperTemplate {
    /// paperTemplate | lessonTemplate
    #[serde(default = "default_kind")]
    pub kind: String,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub subject: String,
    /// 空表示全学科
    #[serde(default)]
    pub grades: Vec<u8>,
    #[serde(default)]
    pub exam_type: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_duration")]
    pub duration_min: u32,
    #[serde(default = "default_total")]
    pub total_score: u32,
    #[serde(default)]
    pub sections: Vec<TemplateSectionSpec>,
    /// 教案环节（lessonTemplate）
    #[serde(default)]
    pub process_stages: Vec<String>,
    #[serde(default)]
    pub prompt_hints: Vec<String>,
    /// bundled | user
    #[serde(default = "default_origin")]
    pub origin: String,
    #[serde(default)]
    pub created_at: u64,
}

fn default_kind() -> String {
    "paperTemplate".into()
}
fn default_duration() -> u32 {
    40
}
fn default_total() -> u32 {
    100
}
fn default_origin() -> String {
    "bundled".into()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn user_templates_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("templates");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("user_templates.json"))
}

fn load_user_templates(app: &AppHandle) -> Result<Vec<PaperTemplate>, String> {
    let path = user_templates_path(app)?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let file: Option<TemplateFile> = read_json(&path)?;
    Ok(match file {
        Some(TemplateFile::Versioned(store)) => store.templates,
        Some(TemplateFile::Legacy(templates)) => templates,
        None => vec![],
    })
}

fn save_user_templates(app: &AppHandle, list: &[PaperTemplate]) -> Result<(), String> {
    let path = user_templates_path(app)?;
    write_json(
        &path,
        &TemplateStore {
            schema_version: SCHEMA_VERSION,
            templates: list.to_vec(),
        },
    )
}

fn sec(
    typ: &str,
    title: &str,
    score: u32,
    item_count: u32,
    hint: &str,
) -> TemplateSectionSpec {
    TemplateSectionSpec {
        r#type: typ.into(),
        title: title.into(),
        score,
        item_count,
        hint: hint.into(),
    }
}

/// 内置结构模板
pub fn bundled_templates() -> Vec<PaperTemplate> {
    vec![
        PaperTemplate {
            kind: "paperTemplate".into(),
            id: "math-unit-classic".into(),
            name: "数学单元测·经典五段".into(),
            description: "填空/判断/选择/计算/解决问题，适合常规单元检测".into(),
            subject: "math".into(),
            grades: vec![1, 2, 3, 4, 5, 6],
            exam_type: "unit".into(),
            tags: vec!["单元".into(), "经典".into(), "数学".into()],
            duration_min: 40,
            total_score: 100,
            sections: vec![
                sec("fill", "一、填空题（共20分）", 20, 5, "每空2～4分"),
                sec("judge", "二、判断题（共10分）", 10, 5, "√或×"),
                sec("choice", "三、选择题（共10分）", 10, 5, "四选一"),
                sec("calc", "四、计算题（共30分）", 30, 2, "口算+脱式"),
                sec("problem", "五、解决问题（共30分）", 30, 3, "应用题"),
            ],
            process_stages: vec![],
            prompt_hints: vec!["符合小学单元测节奏".into(), "计算题答案可验算".into()],
            origin: "bundled".into(),
            created_at: 0,
        },
        PaperTemplate {
            kind: "paperTemplate".into(),
            id: "math-oral-dense".into(),
            name: "数学口算专项·高密度".into(),
            description: "短时大量口算，适合课前/课后".into(),
            subject: "math".into(),
            grades: vec![1, 2, 3, 4],
            exam_type: "oral".into(),
            tags: vec!["口算".into(), "专项".into(), "日常".into()],
            duration_min: 15,
            total_score: 100,
            sections: vec![
                sec("calc", "一、直接写得数（共60分）", 60, 1, "20～30 小题口算"),
                sec("calc", "二、脱式/简便（共25分）", 25, 4, "混合运算"),
                sec("problem", "三、小小应用（共15分）", 15, 2, "一句话应用"),
            ],
            process_stages: vec![],
            prompt_hints: vec!["题干短".into(), "题量大".into(), "避免长阅读".into()],
            origin: "bundled".into(),
            created_at: 0,
        },
        PaperTemplate {
            kind: "paperTemplate".into(),
            id: "math-lesson-quick".into(),
            name: "数学课时练·一课时".into(),
            description: "约 20 分钟课堂巩固".into(),
            subject: "math".into(),
            grades: vec![1, 2, 3, 4, 5, 6],
            exam_type: "lesson".into(),
            tags: vec!["课时".into(), "课堂".into()],
            duration_min: 20,
            total_score: 50,
            sections: vec![
                sec("fill", "一、填空（共10分）", 10, 4, ""),
                sec("calc", "二、计算（共25分）", 25, 2, ""),
                sec("problem", "三、解决问题（共15分）", 15, 2, ""),
            ],
            process_stages: vec![],
            prompt_hints: vec!["一课时可完成".into()],
            origin: "bundled".into(),
            created_at: 0,
        },
        PaperTemplate {
            kind: "paperTemplate".into(),
            id: "math-homework-light".into(),
            name: "数学课后作业·轻量".into(),
            description: "基础+巩固+1 道拓展".into(),
            subject: "math".into(),
            grades: vec![1, 2, 3, 4, 5, 6],
            exam_type: "homework".into(),
            tags: vec!["作业".into(), "轻量".into()],
            duration_min: 25,
            total_score: 50,
            sections: vec![
                sec("mixed", "一、基础练（共20分）", 20, 5, "填空或选择"),
                sec("calc", "二、巩固练（共20分）", 20, 2, "计算"),
                sec("problem", "三、想一想（共10分）", 10, 1, "稍难"),
            ],
            process_stages: vec![],
            prompt_hints: vec!["控制书写负担".into()],
            origin: "bundled".into(),
            created_at: 0,
        },
        PaperTemplate {
            kind: "paperTemplate".into(),
            id: "math-midterm-full".into(),
            name: "数学期中·综合六段".into(),
            description: "期中模拟常用结构".into(),
            subject: "math".into(),
            grades: vec![2, 3, 4, 5, 6],
            exam_type: "midterm".into(),
            tags: vec!["期中".into(), "综合".into()],
            duration_min: 60,
            total_score: 100,
            sections: vec![
                sec("fill", "一、填空题（共20分）", 20, 5, ""),
                sec("judge", "二、判断题（共10分）", 10, 5, ""),
                sec("choice", "三、选择题（共10分）", 10, 5, ""),
                sec("calc", "四、计算题（共30分）", 30, 2, ""),
                sec("operation", "五、操作题（共10分）", 10, 2, "画图/测量"),
                sec("problem", "六、解决问题（共20分）", 20, 2, ""),
            ],
            process_stages: vec![],
            prompt_hints: vec!["覆盖前半学期".into()],
            origin: "bundled".into(),
            created_at: 0,
        },
        PaperTemplate {
            kind: "paperTemplate".into(),
            id: "chinese-unit-classic".into(),
            name: "语文单元测·经典".into(),
            description: "字词+选择+积累+阅读+小练笔".into(),
            subject: "chinese".into(),
            grades: vec![1, 2, 3, 4, 5, 6],
            exam_type: "unit".into(),
            tags: vec!["单元".into(), "语文".into()],
            duration_min: 45,
            total_score: 100,
            sections: vec![
                sec("pinyin", "一、拼音字词（共20分）", 20, 3, ""),
                sec("choice", "二、选择填空（共20分）", 20, 5, ""),
                sec("mixed", "三、课内积累（共20分）", 20, 3, ""),
                sec("reading", "四、阅读理解（共25分）", 25, 2, "课内+课外可合并出"),
                sec("writing", "五、小练笔（共15分）", 15, 1, ""),
            ],
            process_stages: vec![],
            prompt_hints: vec!["阅读适龄".into()],
            origin: "bundled".into(),
            created_at: 0,
        },
        PaperTemplate {
            kind: "paperTemplate".into(),
            id: "chinese-oral-words".into(),
            name: "语文字词专项".into(),
            description: "拼音字词听写式练习".into(),
            subject: "chinese".into(),
            grades: vec![1, 2, 3, 4],
            exam_type: "oral".into(),
            tags: vec!["字词".into(), "专项".into()],
            duration_min: 15,
            total_score: 50,
            sections: vec![
                sec("pinyin", "一、看拼音写词语（共20分）", 20, 1, ""),
                sec("fill", "二、选字填空/近反义（共15分）", 15, 4, ""),
                sec("mixed", "三、形近字组词（共15分）", 15, 5, ""),
            ],
            process_stages: vec![],
            prompt_hints: vec!["无长阅读".into()],
            origin: "bundled".into(),
            created_at: 0,
        },
        PaperTemplate {
            kind: "paperTemplate".into(),
            id: "english-unit-basic".into(),
            name: "英语单元测·基础".into(),
            description: "词汇句型阅读写话".into(),
            subject: "english".into(),
            grades: vec![3, 4, 5, 6],
            exam_type: "unit".into(),
            tags: vec!["英语".into(), "单元".into()],
            duration_min: 40,
            total_score: 100,
            sections: vec![
                sec("fill", "一、词汇（共20分）", 20, 5, ""),
                sec("choice", "二、选择（共20分）", 20, 5, ""),
                sec("mixed", "三、句型转换/填空（共20分）", 20, 4, ""),
                sec("reading", "四、阅读（共25分）", 25, 1, "短文适龄"),
                sec("writing", "五、写话（共15分）", 15, 1, ""),
            ],
            process_stages: vec![],
            prompt_hints: vec!["题干英文为主，说明可用中文".into()],
            origin: "bundled".into(),
            created_at: 0,
        },
        PaperTemplate {
            kind: "paperTemplate".into(),
            id: "english-lesson-vocab".into(),
            name: "英语课时练·词汇句型".into(),
            description: "短时巩固".into(),
            subject: "english".into(),
            grades: vec![3, 4, 5, 6],
            exam_type: "lesson".into(),
            tags: vec!["课时".into(), "英语".into()],
            duration_min: 20,
            total_score: 50,
            sections: vec![
                sec("fill", "一、词汇（共20分）", 20, 6, ""),
                sec("choice", "二、选择（共15分）", 15, 5, ""),
                sec("mixed", "三、完成句子（共15分）", 15, 4, ""),
            ],
            process_stages: vec![],
            prompt_hints: vec![],
            origin: "bundled".into(),
            created_at: 0,
        },
        PaperTemplate {
            kind: "paperTemplate".into(),
            id: "math-redrill-compact".into(),
            name: "错题再练·紧凑".into(),
            description: "变式巩固 + 综合一题".into(),
            subject: "math".into(),
            grades: vec![1, 2, 3, 4, 5, 6],
            exam_type: "redrill".into(),
            tags: vec!["再练".into(), "讲评后".into()],
            duration_min: 20,
            total_score: 50,
            sections: vec![
                sec("mixed", "一、同类基础（共20分）", 20, 4, ""),
                sec("calc", "二、变式巩固（共20分）", 20, 3, ""),
                sec("problem", "三、综合一题（共10分）", 10, 1, ""),
            ],
            process_stages: vec![],
            prompt_hints: vec!["紧扣错因知识点".into(), "禁止照搬原题数字".into()],
            origin: "bundled".into(),
            created_at: 0,
        },
        // 教案结构
        PaperTemplate {
            kind: "lessonTemplate".into(),
            id: "lesson-new-classic".into(),
            name: "新授课·经典七段".into(),
            description: "导入—探究—新知—巩固—小结—作业—板书".into(),
            subject: "".into(),
            grades: vec![1, 2, 3, 4, 5, 6],
            exam_type: "new".into(),
            tags: vec!["教案".into(), "新授".into()],
            duration_min: 40,
            total_score: 0,
            sections: vec![],
            process_stages: vec![
                "导入激趣".into(),
                "探究新知".into(),
                "讲解示范".into(),
                "巩固练习".into(),
                "课堂小结".into(),
                "布置作业".into(),
                "板书设计".into(),
            ],
            prompt_hints: vec!["突出概念建构与活动体验".into()],
            origin: "bundled".into(),
            created_at: 0,
        },
        PaperTemplate {
            kind: "lessonTemplate".into(),
            id: "lesson-practice".into(),
            name: "练习课·讲练结合".into(),
            description: "回顾—分层练—点评—拓展".into(),
            subject: "".into(),
            grades: vec![1, 2, 3, 4, 5, 6],
            exam_type: "practice".into(),
            tags: vec!["教案".into(), "练习".into()],
            duration_min: 40,
            total_score: 0,
            sections: vec![],
            process_stages: vec![
                "旧知回顾".into(),
                "基础练习".into(),
                "变式提升".into(),
                "展示点评".into(),
                "小结与作业".into(),
            ],
            prompt_hints: vec![],
            origin: "bundled".into(),
            created_at: 0,
        },
    ]
}

pub fn list_all_templates(app: &AppHandle) -> Result<Vec<PaperTemplate>, String> {
    let mut all = bundled_templates();
    let mut user = load_user_templates(app)?;
    for t in &mut user {
        if t.origin.is_empty() {
            t.origin = "user".into();
        }
    }
    all.extend(user);
    Ok(all)
}

pub fn get_template(app: &AppHandle, id: &str) -> Result<PaperTemplate, String> {
    list_all_templates(app)?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("模板不存在: {id}"))
}

/// 结构描述字符串（注入 AI prompt）
pub fn template_structure_line(t: &PaperTemplate) -> String {
    if t.kind == "lessonTemplate" {
        return format!(
            "教案环节：{}",
            t.process_stages.join(" → ")
        );
    }
    t.sections
        .iter()
        .map(|s| {
            if s.hint.is_empty() {
                format!("{}（{}分，约{}题，type={}）", s.title, s.score, s.item_count, s.r#type)
            } else {
                format!(
                    "{}（{}分，约{}题，type={}，{}）",
                    s.title, s.score, s.item_count, s.r#type, s.hint
                )
            }
        })
        .collect::<Vec<_>>()
        .join("；")
}

/// 套用模板生成空壳试卷
pub fn apply_paper_template(
    t: &PaperTemplate,
    req_subject: &str,
    req_edition: &str,
    grade: u8,
    semester: &str,
    unit_name: &str,
    pack: Option<&KnowledgePack>,
) -> Result<Value, String> {
    if t.kind == "lessonTemplate" {
        return Err("这是教案模板，请在教案模式套用".into());
    }
    let subject_cn = match req_subject {
        "math" => "数学",
        "english" => "英语",
        "chinese" => "语文",
        s if !s.is_empty() => s,
        _ => match t.subject.as_str() {
            "math" => "数学",
            "english" => "英语",
            _ => "语文",
        },
    };
    let edition_cn = match req_edition {
        "beishida" => "北师大版",
        "sujiao" => "苏教版",
        "renjiao" => "人教版",
        e if !e.is_empty() => e,
        _ => "人教版",
    };
    let sem = if semester == "shang" || semester.contains("上") {
        "上册"
    } else if semester == "xia" || semester.contains("下") {
        "下册"
    } else {
        semester
    };
    let exam = crate::generate::exam_type_label(if t.exam_type.is_empty() {
        "unit"
    } else {
        &t.exam_type
    });
    let title = if !unit_name.is_empty() {
        format!(
            "小学{}{}{}年级{}{}·{}（{}）",
            edition_cn, subject_cn, grade, sem, exam, unit_name, t.name
        )
    } else {
        format!(
            "小学{}{}{}年级{}{}（{}）",
            edition_cn, subject_cn, grade, sem, exam, t.name
        )
    };

    let points: Vec<String> = pack
        .map(|p| {
            p.units
                .iter()
                .flat_map(|u| u.points.clone())
                .take(8)
                .collect()
        })
        .unwrap_or_default();

    let mut sections = Vec::new();
    for (si, spec) in t.sections.iter().enumerate() {
        let n = spec.item_count.max(1) as usize;
        let each = if n > 0 {
            (spec.score as f64 / n as f64).round().max(1.0) as u32
        } else {
            2
        };
        let mut items = Vec::new();
        for i in 0..n {
            let kp = points
                .get(i % points.len().max(1))
                .cloned()
                .unwrap_or_else(|| "本课知识点".into());
            let stem = match spec.r#type.as_str() {
                "calc" if i == 0 && n <= 3 => {
                    format!(
                        "{}. 计算（模板占位，可用智能组卷填充）\n6×7＝　　  48÷6＝　　  25+36＝",
                        i + 1
                    )
                }
                "judge" => format!("{}. 下列说法正确。（　　）", i + 1),
                "choice" => format!("{}. 请选择正确答案。（　　）", i + 1),
                "problem" => format!(
                    "{}.（约{}分）结合「{}」完成应用题。（模板：请用 AI 填充完整叙述）",
                    i + 1,
                    each,
                    kp
                ),
                _ => format!(
                    "{}. 与「{}」相关：请完成练习。（　　）",
                    i + 1, kp
                ),
            };
            let options = if spec.r#type == "choice" {
                json!(["A. 选项一", "B. 选项二", "C. 选项三", "D. 选项四"])
            } else {
                json!([])
            };
            let answer = if spec.r#type == "judge" {
                "√"
            } else if spec.r#type == "choice" {
                "A"
            } else if spec.r#type == "calc" && i == 0 {
                "42；8；61"
            } else {
                "（模板占位）"
            };
            items.push(json!({
                "id": format!("{}-{}", si + 1, i + 1),
                "stem": stem,
                "options": options,
                "answer": answer,
                "analysis": "结构模板占位，正式使用请「按模板智能组卷」或换题",
                "score": each,
                "knowledgePoints": [kp]
            }));
        }
        sections.push(json!({
            "type": spec.r#type,
            "title": spec.title,
            "score": spec.score,
            "items": items
        }));
    }

    Ok(json!({
        "meta": {
            "edition": edition_cn,
            "subject": subject_cn,
            "grade": grade,
            "semester": sem,
            "examType": exam,
            "title": title,
            "totalScore": t.total_score,
            "durationMin": t.duration_min,
            "source": "template-market",
            "templateId": t.id,
            "templateName": t.name
        },
        "sections": sections
    }))
}

// Fix the title format bug - I need to rewrite apply_paper_template title properly
// Let me fix in a search_replace after write

/// 从试卷另存为用户模板
pub fn save_template_from_paper(
    app: &AppHandle,
    paper: &Value,
    name: Option<String>,
) -> Result<PaperTemplate, String> {
    let sections_v = paper
        .get("sections")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "试卷无 sections".to_string())?;
    let mut sections = Vec::new();
    for sec in sections_v {
        let items_n = sec
            .get("items")
            .and_then(|v| v.as_array())
            .map(|a| a.len() as u32)
            .unwrap_or(1);
        sections.push(TemplateSectionSpec {
            r#type: sec
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("mixed")
                .to_string(),
            title: sec
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("大题")
                .to_string(),
            score: sec
                .get("score")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            item_count: items_n.max(1),
            hint: String::new(),
        });
    }
    if sections.is_empty() {
        return Err("没有大题，无法保存模板".into());
    }
    let meta_title = paper
        .pointer("/meta/title")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名");
    let tpl_name = name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("我的模板·{}", meta_title.chars().take(20).collect::<String>()));
    let subject_raw = paper
        .pointer("/meta/subject")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let subject = match subject_raw {
        "数学" => "math",
        "语文" => "chinese",
        "英语" => "english",
        s => s,
    }
    .to_string();
    let grade = paper
        .pointer("/meta/grade")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;
    let total = paper
        .pointer("/meta/totalScore")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as u32;
    let duration = paper
        .pointer("/meta/durationMin")
        .and_then(|v| v.as_u64())
        .unwrap_or(40) as u32;

    let tpl = PaperTemplate {
        kind: "paperTemplate".into(),
        id: unique_id("user-template"),
        name: tpl_name,
        description: format!("从试卷「{meta_title}」另存"),
        subject,
        grades: if grade > 0 { vec![grade] } else { vec![] },
        exam_type: "unit".into(),
        tags: vec!["我的".into(), "自定义".into()],
        duration_min: duration,
        total_score: total,
        sections,
        process_stages: vec![],
        prompt_hints: vec![],
        origin: "user".into(),
        created_at: now_secs(),
    };

    let mut list = load_user_templates(app)?;
    list.insert(0, tpl.clone());
    if list.len() > 100 {
        list.truncate(100);
    }
    save_user_templates(app, &list)?;
    Ok(tpl)
}

pub fn import_template(app: &AppHandle, value: Value) -> Result<PaperTemplate, String> {
    let mut t: PaperTemplate =
        serde_json::from_value(value).map_err(|e| format!("模板 JSON 无效: {e}"))?;
    if t.id.is_empty() {
        t.id = unique_id("import-template");
    }
    // 避免与内置 id 冲突
    if bundled_templates().iter().any(|b| b.id == t.id) {
        t.id = unique_id(&format!("{}-user", t.id));
    }
    t.origin = "user".into();
    t.created_at = now_secs();
    if t.name.is_empty() {
        t.name = "导入模板".into();
    }
    if t.kind.is_empty() {
        t.kind = if t.process_stages.is_empty() {
            "paperTemplate".into()
        } else {
            "lessonTemplate".into()
        };
    }
    let mut list = load_user_templates(app)?;
    list.retain(|x| x.id != t.id);
    list.insert(0, t.clone());
    save_user_templates(app, &list)?;
    Ok(t)
}

pub fn delete_user_template(app: &AppHandle, id: &str) -> Result<(), String> {
    if bundled_templates().iter().any(|b| b.id == id) {
        return Err("内置模板不能删除".into());
    }
    let mut list = load_user_templates(app)?;
    let before = list.len();
    list.retain(|t| t.id != id);
    if list.len() == before {
        return Err("用户模板不存在".into());
    }
    save_user_templates(app, &list)
}

pub fn export_template_json(app: &AppHandle, id: &str) -> Result<Value, String> {
    let t = get_template(app, id)?;
    serde_json::to_value(t).map_err(|e| e.to_string())
}
