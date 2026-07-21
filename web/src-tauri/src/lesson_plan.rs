use crate::ai::{chat_completion, extract_json};
use crate::config::AppConfig;
use crate::contracts::validate_lesson_plan;
use crate::knowledge::KnowledgePack;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonPlanRequest {
    pub subject: String,
    pub edition: String,
    pub grade: u8,
    pub semester: String,
    pub knowledge_path: String,
    pub unit_id: Option<String>,
    /// 可选：指定课时名；空则整单元/首课时
    pub lesson_name: Option<String>,
    /// 课时数，默认 1
    pub periods: u8,
    pub duration_min: u32,
    /// new | practice | review | feedback
    #[serde(default = "default_lesson_type")]
    pub lesson_type: String,
    /// 是否同时生成家长辅导版（默认 true）
    #[serde(default = "default_true")]
    pub include_parent: bool,
}

fn default_true() -> bool {
    true
}

fn default_lesson_type() -> String {
    "new".into()
}

fn lesson_type_label(t: &str) -> &'static str {
    match t {
        "practice" => "练习课",
        "review" => "复习课",
        "feedback" => "讲评课",
        _ => "新授课",
    }
}

fn lesson_type_process_hint(t: &str) -> &'static str {
    match t {
        "practice" => {
            "课型为练习课：以分层练习、反馈矫正为主；导入简短；少讲多练；过程含基础练、综合练、挑战练与讲评。"
        }
        "review" => {
            "课型为复习课：梳理知识网络、易错对比、综合运用；过程含知识回顾、专题突破、综合练习、小结。"
        }
        "feedback" => {
            "课型为讲评课：基于典型错题；过程含错因分析、变式训练、归纳方法、再测巩固；避免简单对答案。"
        }
        _ => {
            "课型为新授课：过程覆盖导入—探究新知—巩固—小结；突出概念建构与活动体验。"
        }
    }
}

fn subject_cn(s: &str) -> &'static str {
    match s {
        "math" => "数学",
        "english" => "英语",
        _ => "语文",
    }
}

fn edition_cn(s: &str) -> &'static str {
    match s {
        "beishida" => "北师大版",
        "sujiao" => "苏教版",
        "renjiao" => "人教版",
        _ => "人教版",
    }
}

fn pick_unit<'a>(
    pack: &'a KnowledgePack,
    unit_id: &Option<String>,
) -> Option<&'a crate::knowledge::UnitInfo> {
    let uid = unit_id.clone().unwrap_or_default();
    pack.units
        .iter()
        .find(|u| u.id == uid || u.name.contains(&uid))
        .or_else(|| pack.units.first())
}

pub fn generate_lesson_plan(
    cfg: &AppConfig,
    req: &LessonPlanRequest,
    pack: &KnowledgePack,
) -> Result<Value, String> {
    let unit = pick_unit(pack, &req.unit_id).ok_or_else(|| "无可用单元".to_string())?;
    let lesson = req
        .lesson_name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| unit.lessons.first().cloned())
        .unwrap_or_else(|| unit.name.clone());

    let periods = if req.periods == 0 { 1 } else { req.periods };
    let duration = if req.duration_min == 0 {
        40
    } else {
        req.duration_min
    };

    let lt = if req.lesson_type.trim().is_empty() {
        "new"
    } else {
        req.lesson_type.as_str()
    };
    let lt_label = lesson_type_label(lt);
    let lt_hint = lesson_type_process_hint(lt);

    let parent_block = if req.include_parent {
        r#",
  "parentGuide": {
    "title": "家长辅导手册（可带回家）",
    "summary": "用一两句大白话说明孩子这节课学什么、回家怎么帮",
    "goalsInPlain": ["孩子回家后应会……（避免术语堆砌）"],
    "previewTips": ["课前 5～10 分钟预习建议，具体可操作"],
    "accompanySteps": [
      {
        "step": "读一读 / 讲一讲",
        "minutes": 10,
        "how": "家长怎么陪（语气、节奏、是否指读）",
        "say": "家长可以说的引导语示例"
      }
    ],
    "keyQuestions": ["家长可随口问的 3～5 个小问题（检验是否听懂）"],
    "commonMistakes": ["常见卡点 + 纠正说法（温柔、不打击）"],
    "homePractice": ["家庭小练习，控制负担，10～15 分钟内"],
    "encourage": "一句鼓励孩子的话"
  }"#
    } else {
        ""
    };

    let system = format!(
        r#"你是资深小学{sub}教师，熟悉{ed}教材与课堂教学，也擅长写给家长看的辅导说明。
请输出【仅一份 JSON】，不要 Markdown 说明。
本课课型：{lt_label}。{lt_hint}

【教师版】要像可直接上课的详案：目标分条、过程分环节，content/teacherActivity/studentActivity 写具体话术与活动，不要空泛套话。
语文课过程可参考：情境导入 → 初读正音 → 再读懂文 → 研读明意 → 小结作业；数学可参考：情境引入 → 探究新知 → 巩固练习 → 小结。
环节名建议带序号或「一、初读——正其音」这类清晰标题。

结构必须如下：
{{
  "kind": "lessonPlan",
  "meta": {{
    "title": "课题名称",
    "edition": "{ed}",
    "subject": "{sub}",
    "grade": {grade},
    "semester": "{sem}",
    "unitName": "单元名",
    "lessonName": "课时名",
    "lessonType": "{lt_label}",
    "periods": {periods},
    "durationMin": {duration},
    "audience": "教师版+家长版",
    "school": "",
    "teacher": ""
  }},
  "objectives": {{
    "knowledge": ["知识与技能目标…"],
    "ability": ["过程与方法…"],
    "emotion": ["情感态度价值观…"]
  }},
  "keyPoints": ["教学重点"],
  "difficultPoints": ["教学难点"],
  "preparation": {{
    "teacher": ["教具/课件"],
    "student": ["学具/预习"]
  }},
  "process": [
    {{
      "stage": "一、情境创设（或：初读——正其音）",
      "minutes": 5,
      "content": "环节教学内容，可含引导语、文本片段提示",
      "teacherActivity": "教师具体做什么、说什么",
      "studentActivity": "学生具体做什么",
      "intent": "设计意图"
    }}
  ],
  "boardDesign": "板书设计（可用文字示意分栏，先板课题再展开）",
  "homework": ["作业1"],
  "reflection": "教学反思提示（可写预设反思点）"{parent_block}
}}
要求：
1. 贴合该年级认知与课标，禁止超纲；必须紧扣本课「{lesson}」
2. process 总时长约 {duration} 分钟，4～7 个环节，符合「{lt_label}」
3. 目标可观察、可检测；过程可操作，有师生互动
4. 数学注重操作与思维；语文注重朗读、字词、语言运用与情感
5. parentGuide 必须通俗，面向家长（非教育学黑话），负担轻、可今晚就用
6. 不要写真实统考原题字样"#,
        sub = subject_cn(&req.subject),
        ed = edition_cn(&req.edition),
        grade = req.grade,
        sem = if req.semester == "shang" {
            "上册"
        } else {
            "下册"
        },
        periods = periods,
        duration = duration,
        lt_label = lt_label,
        lt_hint = lt_hint,
        parent_block = parent_block,
        lesson = lesson,
    );

    let user = format!(
        "请为下列内容写「教师详案」{}：\n课型：{}\n教材：{} {} {}年级{}\n单元：{}\n课时：{}\n课时列表：{}\n知识点：{}\n教材提示：{}\n共 {} 课时，课堂约 {} 分钟。\n请直接输出 JSON。",
        if req.include_parent {
            "与「家长辅导手册」"
        } else {
            ""
        },
        lt_label,
        edition_cn(&req.edition),
        subject_cn(&req.subject),
        req.grade,
        if req.semester == "shang" {
            "上册"
        } else {
            "下册"
        },
        unit.name,
        lesson,
        unit.lessons.join("、"),
        unit.points.join("、"),
        pack.exam_hints.join("；"),
        periods,
        duration,
    );

    let raw = chat_completion(cfg, &system, &user)?;
    let json_str = extract_json(&raw)?;
    let mut value: Value =
        serde_json::from_str(&json_str).map_err(|e| format!("教案 JSON 无效: {e}"))?;
    // 保证 kind
    if let Some(obj) = value.as_object_mut() {
        obj.insert("kind".into(), json!("lessonPlan"));
    }
    validate_lesson_plan(&value)?;
    Ok(value)
}

/// 为本单元全部课时各生成一份教案（AI）
pub fn generate_unit_all_lessons(
    cfg: &AppConfig,
    req: &LessonPlanRequest,
    pack: &KnowledgePack,
) -> Result<Value, String> {
    let unit = pick_unit(pack, &req.unit_id).ok_or_else(|| "无可用单元".to_string())?;
    let mut lessons = unit.lessons.clone();
    if lessons.is_empty() {
        lessons.push(unit.name.clone());
    }
    // 控制上限，避免一次请求过久
    if lessons.len() > 12 {
        lessons.truncate(12);
    }

    let mut plans: Vec<Value> = Vec::new();
    for (i, name) in lessons.iter().enumerate() {
        let mut one = req.clone();
        one.lesson_name = Some(name.clone());
        one.periods = (i as u8) + 1;
        match generate_lesson_plan(cfg, &one, pack) {
            Ok(p) => plans.push(p),
            Err(e) => {
                // 单课失败不整体失败，记录占位
                plans.push(json!({
                    "kind": "lessonPlan",
                    "meta": {
                        "title": format!("{}（生成失败）", name),
                        "lessonName": name,
                        "unitName": unit.name,
                        "error": e
                    },
                    "error": e
                }));
            }
        }
        // 轻微间隔，降低限流概率
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    Ok(json!({
        "kind": "lessonPlanBundle",
        "meta": {
            "title": format!("{} · 全课时教案", unit.name),
            "unitName": unit.name,
            "edition": edition_cn(&req.edition),
            "subject": subject_cn(&req.subject),
            "grade": req.grade,
            "semester": if req.semester == "shang" { "上册" } else { "下册" },
            "count": plans.len(),
            "lessonType": lesson_type_label(if req.lesson_type.trim().is_empty() { "new" } else { &req.lesson_type })
        },
        "plans": plans
    }))
}

/// 离线：全课时结构模板（不调 AI）
pub fn template_unit_all_lessons(req: &LessonPlanRequest, pack: &KnowledgePack) -> Value {
    let unit = pick_unit(pack, &req.unit_id);
    let unit_name = unit
        .map(|u| u.name.clone())
        .unwrap_or_else(|| "本单元".into());
    let mut lessons = unit.map(|u| u.lessons.clone()).unwrap_or_default();
    if lessons.is_empty() {
        lessons.push(unit_name.clone());
    }
    let plans: Vec<Value> = lessons
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let mut one = req.clone();
            one.lesson_name = Some(name.clone());
            one.periods = (i as u8) + 1;
            template_lesson_plan(&one, pack)
        })
        .collect();

    json!({
        "kind": "lessonPlanBundle",
        "meta": {
            "title": format!("{} · 全课时教案（模板）", unit_name),
            "unitName": unit_name,
            "edition": edition_cn(&req.edition),
            "subject": subject_cn(&req.subject),
            "grade": req.grade,
            "semester": if req.semester == "shang" { "上册" } else { "下册" },
            "count": plans.len()
        },
        "plans": plans
    })
}

/// 离线结构模板教案
pub fn template_lesson_plan(req: &LessonPlanRequest, pack: &KnowledgePack) -> Value {
    let unit = pick_unit(pack, &req.unit_id);
    let unit_name = unit.map(|u| u.name.clone()).unwrap_or_else(|| "本单元".into());
    let lessons = unit.map(|u| u.lessons.clone()).unwrap_or_default();
    let points = unit.map(|u| u.points.clone()).unwrap_or_default();
    let lesson = req
        .lesson_name
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| lessons.first().cloned())
        .unwrap_or_else(|| unit_name.clone());
    let duration = if req.duration_min == 0 {
        40
    } else {
        req.duration_min
    };
    let periods = if req.periods == 0 { 1 } else { req.periods };
    let sub = subject_cn(&req.subject);
    let ed = edition_cn(&req.edition);
    let sem = if req.semester == "shang" {
        "上册"
    } else {
        "下册"
    };
    let lt = if req.lesson_type.trim().is_empty() {
        "new"
    } else {
        req.lesson_type.as_str()
    };
    let lt_label = lesson_type_label(lt);
    let mid = duration.saturating_sub(15).max(12);
    let parent_guide = json!({
        "title": format!("{} · 家长辅导手册", lesson),
        "summary": format!("本课围绕「{}」，建议家长用 15～20 分钟陪孩子读一读、问一问、练一练。", lesson),
        "goalsInPlain": [
            format!("能用自己的话说说「{}」讲了什么", lesson),
            "认识本课要求掌握的字词（或关键算理）",
            "愿意大声读、敢提问"
        ],
        "previewTips": [
            "晚饭后找安静 10 分钟，先看课题和插图猜一猜",
            "让孩子自己先读一遍，家长只在卡壳时帮忙"
        ],
        "accompanySteps": [
            {"step":"读一读","minutes":8,"how":"孩子指读或跟读，家长耐心听完不打断","say":"你读得很认真，这句再读一遍会更顺。"},
            {"step":"问一问","minutes":5,"how":"用聊天方式提问，不考试腔","say":"这一段里，你最喜欢哪一句？为什么？"},
            {"step":"练一练","minutes":5,"how":"口头说一说或写一两个关键词","say":"我们用自己的话复述一下好吗？"}
        ],
        "keyQuestions": [
            format!("「{}」主要写了什么？", lesson),
            "有没有不懂的字词？我们一起查一查。",
            "如果让你给同学讲，你先讲哪一点？"
        ],
        "commonMistakes": [
            "读不准就急着纠正：可先听完，再温柔带读一遍",
            "只盯对错：多夸过程，如「你刚才自己想办法了」"
        ],
        "homePractice": [
            "把喜欢的句子读给家人听（或口述算理）",
            "完成老师布置的书面作业，家长只检查是否认真完成"
        ],
        "encourage": "学习不怕慢，一步一步来，今天的努力都算数。"
    });

    let process = match lt {
        "practice" => json!([
            {"stage":"明确目标","minutes":3,"content":"出示练习目标与要求。","teacherActivity":"说明分层任务。","studentActivity":"明确自己层次。","intent":"带着目标练习。"},
            {"stage":"基础练习","minutes":mid/2,"content":format!("围绕「{}」完成基础题。", lesson),"teacherActivity":"巡视、个别辅导。","studentActivity":"独立完成。","intent":"夯实双基。"},
            {"stage":"综合与挑战","minutes":mid - mid/2,"content":"综合题、易错对比。","teacherActivity":"组织展示讲评。","studentActivity":"互评订正。","intent":"提升灵活运用。"},
            {"stage":"反馈小结","minutes":5,"content":"归纳方法，布置延伸练习。","teacherActivity":"点评共性问题。","studentActivity":"整理错题。","intent":"及时巩固。"}
        ]),
        "review" => json!([
            {"stage":"知识回顾","minutes":8,"content":"梳理本单元/本课知识网络。","teacherActivity":"引导梳理板书。","studentActivity":"说一说要点。","intent":"建立结构。"},
            {"stage":"专题突破","minutes":mid,"content":format!("针对「{}」易错点突破。", lesson),"teacherActivity":"典型题精讲。","studentActivity":"对比辨析。","intent":"突破难点。"},
            {"stage":"综合练习","minutes":10,"content":"限时综合练。","teacherActivity":"巡视反馈。","studentActivity":"独立完成。","intent":"检测掌握。"},
            {"stage":"归纳提升","minutes":5,"content":"方法总结与作业。","teacherActivity":"提炼方法。","studentActivity":"记录要点。","intent":"形成策略。"}
        ]),
        "feedback" => json!([
            {"stage":"错题呈现","minutes":5,"content":"出示共性错题与数据。","teacherActivity":"展示典型错误。","studentActivity":"对照自查。","intent":"聚焦问题。"},
            {"stage":"错因分析","minutes":10,"content":"分析概念/审题/计算等错因。","teacherActivity":"引导归类。","studentActivity":"说出错因。","intent":"对症下药。"},
            {"stage":"变式训练","minutes":mid,"content":format!("围绕「{}」变式巩固。", lesson),"teacherActivity":"提供变式题。","studentActivity":"再练再改。","intent":"迁移提升。"},
            {"stage":"再测小结","minutes":5,"content":"微型检测与方法归纳。","teacherActivity":"点评、布置订正。","studentActivity":"完成再测。","intent":"闭环落实。"}
        ]),
        _ => json!([
            {"stage":"导入新课","minutes":5,"content":"创设情境，引出任务。","teacherActivity":"提问导入，出示目标。","studentActivity":"观察思考。","intent":"激发兴趣。"},
            {"stage":"探究新知","minutes":mid,"content":format!("围绕「{}」展开教学。", lesson),"teacherActivity":"讲解示范，组织活动。","studentActivity":"参与探究与交流。","intent":"突破重点。"},
            {"stage":"巩固练习","minutes":10,"content":"分层练习，反馈矫正。","teacherActivity":"巡视讲评。","studentActivity":"独立完成、互评。","intent":"巩固新知。"},
            {"stage":"课堂小结","minutes":5,"content":"回顾收获，布置作业。","teacherActivity":"引导总结。","studentActivity":"谈收获。","intent":"建构网络。"}
        ]),
    };

    let mut plan = json!({
        "kind": "lessonPlan",
        "meta": {
            "title": format!("{}（{}·第{}课时）", lesson, lt_label, periods),
            "edition": ed,
            "subject": sub,
            "grade": req.grade,
            "semester": sem,
            "unitName": unit_name,
            "lessonName": lesson,
            "lessonType": lt_label,
            "periods": periods,
            "durationMin": duration,
            "audience": "教师版+家长版",
            "school": "",
            "teacher": ""
        },
        "objectives": {
            "knowledge": [
                format!("理解并掌握与「{}」相关的核心内容。", points.first().cloned().unwrap_or_else(|| "本课要点".into())),
                "能正确完成对应练习，表述清楚。"
            ],
            "ability": [
                "通过观察、操作或阅读，提升分析与表达能力。",
                "学会与同伴交流，倾听并补充观点。"
            ],
            "emotion": [
                "感受学科乐趣，养成认真思考、主动学习的习惯。"
            ]
        },
        "keyPoints": points.iter().take(3).cloned().collect::<Vec<_>>(),
        "difficultPoints": [
            format!("对「{}」的深入理解与灵活运用。", points.get(1).cloned().unwrap_or_else(|| "重难点".into()))
        ],
        "preparation": {
            "teacher": ["课件", "板书提纲", "课堂练习题"],
            "student": ["课本", "练习本", "学具（如需要）"]
        },
        "process": process,
        "boardDesign": format!("课题：{}（{}）\n一、要点\n二、例题/练习\n三、方法小结", lesson, lt_label),
        "homework": [
            "完成课本相关练习",
            "复习本课要点，整理错题"
        ],
        "reflection": "（课后填写）目标达成情况、学生生成、改进点。"
    });
    if req.include_parent {
        if let Some(obj) = plan.as_object_mut() {
            obj.insert("parentGuide".into(), parent_guide);
        }
    }
    plan
}
