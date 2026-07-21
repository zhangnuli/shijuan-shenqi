use crate::ai::{chat_completion, extract_json};
use crate::config::AppConfig;
use crate::contracts::validate_exam_paper;
use crate::knowledge::KnowledgePack;
use crate::quality::inspect_paper;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DifficultyRatio {
    #[serde(default = "ratio_basic")]
    pub basic: u8,
    #[serde(default = "ratio_medium")]
    pub medium: u8,
    #[serde(default = "ratio_hard")]
    pub hard: u8,
}

fn ratio_basic() -> u8 {
    40
}
fn ratio_medium() -> u8 {
    40
}
fn ratio_hard() -> u8 {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub subject: String,
    pub edition: String,
    pub grade: u8,
    pub semester: String,
    /// unit | midterm | final
    pub exam_type: String,
    pub unit_id: Option<String>,
    pub difficulty: String,
    pub total_score: u32,
    pub duration_min: u32,
    pub knowledge_path: String,
    /// 单元测勾选的课时名
    #[serde(default)]
    pub selected_lessons: Vec<String>,
    /// 难度配比（百分比，和约为 100）
    #[serde(default)]
    pub difficulty_ratio: DifficultyRatio,
    /// 是否题库混组：优先可验算骨架 + AI 情境
    #[serde(default)]
    pub mix_bank: bool,
    /// 是否掺入校本收藏题作参考
    #[serde(default)]
    pub use_school_bank: bool,
    /// 校本收藏题摘要（前端/命令层注入，供 AI 参考改写）
    #[serde(default)]
    pub school_bank_snippets: Vec<String>,
    /// 模板市集：选用的模板 id
    #[serde(default)]
    pub template_id: Option<String>,
    /// 模板市集：强制结构行（覆盖默认建议题型结构）
    #[serde(default)]
    pub structure_override: Option<String>,
    /// 模板附加命题提示
    #[serde(default)]
    pub template_hints: Vec<String>,
}

pub fn exam_type_label(t: &str) -> &'static str {
    match t {
        "unit" => "单元测试",
        "midterm" => "期中模拟",
        "final" => "期末模拟",
        "oral" => "口算专项",
        "lesson" => "课时练习",
        "homework" => "课后作业",
        "redrill" => "错题再练",
        _ => "模拟考试",
    }
}

/// 是否按「单单元/课时」收窄命题范围
fn is_unit_scoped(exam_type: &str) -> bool {
    matches!(
        exam_type,
        "unit" | "oral" | "lesson" | "homework" | "redrill"
    )
}

fn format_unit_block(u: &crate::knowledge::UnitInfo) -> Vec<String> {
    let mut v = vec![format!("【单元】{}", u.name)];
    if !u.lessons.is_empty() {
        v.push(format!("课时：{}", u.lessons.join("、")));
    }
    if !u.points.is_empty() {
        v.push(format!("知识点：{}", u.points.join("、")));
    }
    v
}

pub fn build_prompt(req: &GenerateRequest, pack: &KnowledgePack) -> (String, String) {
    let subject_cn = match req.subject.as_str() {
        "math" => "数学",
        "english" => "英语",
        _ => "语文",
    };
    let edition_cn = match req.edition.as_str() {
        "beishida" => "北师大版",
        "sujiao" => "苏教版",
        _ => "人教版",
    };
    let sem = if req.semester == "shang" { "上册" } else { "下册" };
    let exam = exam_type_label(&req.exam_type);

    let mut focus_points: Vec<String> = match req.exam_type.as_str() {
        t if is_unit_scoped(t) => {
            let uid = req.unit_id.clone().unwrap_or_default();
            pack.units
                .iter()
                .find(|u| u.id == uid || u.name.contains(&uid))
                .map(format_unit_block)
                .unwrap_or_else(|| {
                    pack.units
                        .first()
                        .map(format_unit_block)
                        .unwrap_or_default()
                })
        }
        "midterm" => {
            let half = (pack.units.len() + 1) / 2;
            pack.units
                .iter()
                .take(half.max(1))
                .flat_map(format_unit_block)
                .collect()
        }
        _ => pack.units.iter().flat_map(format_unit_block).collect(),
    };

    if !req.selected_lessons.is_empty() {
        focus_points.insert(
            0,
            format!("【重点课时（仅覆盖这些课时）】{}", req.selected_lessons.join("、")),
        );
    }

    let mut r = req.difficulty_ratio.clone();
    let sum = (r.basic as u16) + (r.medium as u16) + (r.hard as u16);
    if sum == 0 {
        r = DifficultyRatio {
            basic: 40,
            medium: 40,
            hard: 20,
        };
    }
    let ratio_hint = format!(
        "难度配比：基础约{}%、中等约{}%、拔高约{}%（按题量大致分配）",
        r.basic, r.medium, r.hard
    );

    let mix_hint = if req.mix_bank {
        "组卷策略：题库混组——计算题尽量给出可验算的明确算式与唯一数值答案；应用题数量关系清晰；避免无法判定对错的开放表述。"
    } else {
        "组卷策略：常规 AI 命题，答案明确。"
    };

    let structure = if let Some(ov) = req.structure_override.as_ref().filter(|s| !s.trim().is_empty()) {
        ov.clone()
    } else if req.subject == "math" {
        match req.exam_type.as_str() {
            "oral" => "一、直接写得数·口算(60) 二、脱式/简便(25) 三、小小应用(15)——以口算为主，题量大、题干短".into(),
            "lesson" => "一、填空(15) 二、选择或判断(15) 三、计算(40) 四、解决问题(30)——一课时量，约 20～30 分钟".into(),
            "homework" => "一、基础练(40) 二、巩固练(35) 三、拓展想一想(25)——课后作业量，控制书写负担".into(),
            "redrill" => "一、错因同类基础(40) 二、变式巩固(40) 三、综合一题(20)——针对错题知识点再练".into(),
            "unit" => "一、填空(20) 二、判断(10) 三、选择(10) 四、计算(30) 五、解决问题(30)".into(),
            "midterm" => {
                "一、填空(20) 二、判断(10) 三、选择(10) 四、计算(30) 五、操作(10) 六、解决问题(20)".into()
            }
            _ => "一、填空(20) 二、判断(10) 三、选择(10) 四、计算(30) 五、操作(10) 六、解决问题(20)".into(),
        }
    } else if req.subject == "english" {
        match req.exam_type.as_str() {
            "oral" | "lesson" => "一、词汇(30) 二、选词/选择(30) 三、句型(25) 四、小阅读(15)——短时专项".into(),
            "homework" => "一、抄写/单词(30) 二、完成句子(40) 三、小对话或阅读(30)".into(),
            "redrill" => "一、错词错句巩固(50) 二、变式选择(30) 三、仿写一句(20)".into(),
            "unit" => "一、词汇(20) 二、选择(20) 三、句型转换/填空(20) 四、阅读(25) 五、写话(15)".into(),
            _ => "一、听力笔试替代·词汇句型(30) 二、选择填空(20) 三、阅读(25) 四、写作(25)".into(),
        }
    } else {
        match req.exam_type.as_str() {
            "oral" => "一、拼音(30) 二、字词听写式填空(40) 三、选字/近反义(30)——字词专项，无长阅读".into(),
            "lesson" => "一、字词(25) 二、课内填空(30) 三、小练笔或课内阅读(45)——一课时".into(),
            "homework" => "一、字词巩固(30) 二、积累运用(40) 三、小练笔(30)".into(),
            "redrill" => "一、错字错词(40) 二、同类句式/阅读题(40) 三、订正仿写(20)".into(),
            "unit" => {
                "一、拼音字词(20) 二、选择填空(20) 三、课内积累(20) 四、阅读理解(25) 五、小练笔(15)".into()
            }
            "midterm" => {
                "一、积累与运用(30) 二、课内阅读(20) 三、课外阅读(25) 四、习作(25)".into()
            }
            _ => "一、积累与运用(30) 二、课内阅读(20) 三、课外阅读(25) 四、习作(25)".into(),
        }
    };

    let source_hint = if pack.source.smartedu_path_hint.is_empty() {
        format!(
            "对齐国家中小学智慧教育平台同步课程：https://basic.smartedu.cn/syncClassroom"
        )
    } else {
        format!(
            "课程来源对齐：国家中小学智慧教育平台（https://basic.smartedu.cn/）。路径提示：{}",
            pack.source.smartedu_path_hint
        )
    };

    let system = format!(
        r#"你是资深小学{subject_cn}命题教师，精通{edition_cn}教材，熟悉国家中小学智慧教育平台同步课程与基础性作业风格。
请严格按用户要求输出【仅一份 JSON】，不要 Markdown 说明，不要多余文字。
难度：{difficulty}。必须符合该年级认知，题目必须落在给定单元/课时/知识点范围内，禁止超纲。
{source_hint}
JSON 结构必须如下：
{{
  "meta": {{
    "edition": "{edition_cn}",
    "subject": "{subject_cn}",
    "grade": {grade},
    "semester": "{sem}",
    "examType": "{exam}",
    "title": "完整卷名",
    "totalScore": {score},
    "durationMin": {duration},
    "curriculumSource": "国家中小学智慧教育平台对齐·模拟卷"
  }},
  "sections": [
    {{
      "type": "fill|judge|choice|calc|operation|problem|pinyin|reading|writing|mixed",
      "title": "一、xxx（xx分）",
      "score": 20,
      "items": [
        {{
          "id": "1-1",
          "stem": "题干，可用（　　）表示填空",
          "options": ["A. ..", "B. .."],
          "answer": "参考答案",
          "analysis": "简要解析",
          "score": 2,
          "knowledgePoints": ["知识点"]
        }}
      ]
    }}
  ]
}}
规则：
1. sections 分值之和 = totalScore
2. 判断题 answer 用 √ 或 ×；题干末尾用（　　）
3. 选择题必须有 options，写成 [\"A. …\",\"B. …\",\"C. …\",\"D. …\"]
4. 计算题分条：口算小题 stem 只写算式如「6×7＝」；竖式/脱式可多行，每行一题；答案必须为可验算的唯一数值
5. 应用题 stem 只写题目叙述，不要写「解：」「答：」（排版会自动留答题区）
6. 语文阅读材料字数适龄；习作只出题目与要求；英语题目用英文题干，说明用中文括号提示
7. 卷面为「模拟卷」，不要写真实地区统考原题字样
8. 大题 title 格式统一为「一、填空题（每空2分，共20分）」这种小学卷常见写法
9. 题号从 1 连续编号，stem 不要重复写大题号
10. {ratio_hint}；各难度在全卷大致均匀分布，拔高题优先放计算/解决问题末尾
11. {mix_hint}
12. 若用户指定了「重点课时」，命题范围必须优先覆盖这些课时，不得偏离"#,
        subject_cn = subject_cn,
        edition_cn = edition_cn,
        difficulty = req.difficulty,
        source_hint = source_hint,
        grade = req.grade,
        sem = sem,
        exam = exam,
        score = req.total_score,
        duration = req.duration_min,
        ratio_hint = ratio_hint,
        mix_hint = mix_hint,
    );

    let unit_line = if is_unit_scoped(&req.exam_type) {
        format!(
            "单元：{}",
            req.unit_id.clone().unwrap_or_else(|| "指定单元".into())
        )
    } else {
        String::new()
    };

    let lesson_line = if !req.selected_lessons.is_empty() {
        format!("重点课时：{}", req.selected_lessons.join("、"))
    } else {
        String::new()
    };

    let skeleton_block = if req.mix_bank && req.subject == "math" {
        format!(
            "\n本地题库骨架（可改数字/情境，答案须可验算）：\n{}",
            math_skeleton_bank(req.grade).join("\n")
        )
    } else if req.mix_bank && req.subject == "english" {
        "\n英语骨架提示：词汇题给词意或选词填空；句型题给中文提示；阅读短文 50–120 词适龄。".into()
    } else {
        String::new()
    };

    let school_block = if req.use_school_bank && !req.school_bank_snippets.is_empty() {
        format!(
            "\n校本收藏参考题（可改编数字/情境后入卷，勿原样照抄超过 30% 题量）：\n{}",
            req.school_bank_snippets
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {}", i + 1, s))
                .collect::<Vec<_>>()
                .join("\n")
        )
    } else {
        String::new()
    };

    let special_hint = match req.exam_type.as_str() {
        "oral" => "本卷为口算/字词专项：题量宜多、书写负担适中，避免长阅读与大作文。",
        "lesson" => "本卷为一课时课堂练习：控制在一节课可完成。",
        "homework" => "本卷为课后作业：题量适中，基础为主，可有 1～2 道稍难。",
        "redrill" => "本卷为错题再练：紧扣给定错因/知识点，出同类变式，勿简单重复原题数字。",
        _ => "",
    };

    let template_line = if req.template_id.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
        format!(
            "【必须严格按下列题型结构出卷，大题顺序与分值不可擅自合并或删减】\n模板：{}\n结构：{}",
            req.template_id.as_deref().unwrap_or(""),
            structure
        )
    } else {
        format!("建议题型结构：{structure}")
    };

    let extra_tpl_hints = if req.template_hints.is_empty() {
        String::new()
    } else {
        format!("模板命题提示：{}", req.template_hints.join("；"))
    };

    let user = format!(
        r#"请生成：小学{edition_cn}{subject_cn}{grade}年级{sem}{exam}卷。
{unit_line}
{lesson_line}
{special_hint}
{template_line}
{extra_tpl_hints}
难度配比：{ratio_hint}
命题范围与知识点：
{points}

教材提示：
{hints}
{skeleton_block}
{school_block}

请直接输出 JSON。"#,
        edition_cn = edition_cn,
        subject_cn = subject_cn,
        grade = req.grade,
        sem = sem,
        exam = exam,
        unit_line = unit_line,
        lesson_line = lesson_line,
        special_hint = special_hint,
        template_line = template_line,
        extra_tpl_hints = extra_tpl_hints,
        ratio_hint = ratio_hint,
        points = focus_points.join("\n- "),
        hints = pack.exam_hints.join("\n- "),
        skeleton_block = skeleton_block,
        school_block = school_block,
    );

    (system, user)
}

/// 本地可验算计算题骨架（题库混组用）
fn math_skeleton_bank(grade: u8) -> Vec<String> {
    match grade {
        1..=2 => vec![
            "口算骨架：6+7＝ / 15-8＝ / 9+6＝ / 14-5＝（改数字，答案唯一）".into(),
            "口算骨架：2×5＝ / 3×4＝ / 18÷3＝ / 20÷5＝".into(),
            "脱式骨架：12+8-5 / 9×2+6 / 20-4×3".into(),
            "应用题骨架：买○个○，每个○元，共多少元？（数量关系清晰）".into(),
        ],
        3..=4 => vec![
            "口算骨架：36+48＝ / 72-29＝ / 7×8＝ / 56÷7＝".into(),
            "脱式骨架：36+18÷6 / (45-9)÷6 / 100-6×9".into(),
            "竖式骨架：三位数加减 / 两位数乘一位数（步骤清晰）".into(),
            "应用题骨架：路程=速度×时间；单价×数量=总价（改情境与数字）".into(),
        ],
        _ => vec![
            "口算骨架：125+375＝ / 3/4+1/4＝ / 0.6×5＝ / 2.4÷0.6＝".into(),
            "脱式骨架：1.2×(3.5+1.5) / 36÷0.4-15 / (2/3+1/6)×4".into(),
            "应用题骨架：面积/体积/百分数/比例分配（数字可验算）".into(),
            "综合骨架：先求中间量再求结果，答案唯一数值".into(),
        ],
    }
}

pub fn generate_with_ai(
    cfg: &AppConfig,
    req: &GenerateRequest,
    pack: &KnowledgePack,
) -> Result<Value, String> {
    let (system, user) = build_prompt(req, pack);
    let raw = chat_completion(cfg, &system, &user)?;
    let json_str = extract_json(&raw)?;
    let value: Value =
        serde_json::from_str(&json_str).map_err(|e| format!("试卷 JSON 无效: {e}\n{json_str}"))?;
    validate_exam_paper(&value)?;
    let quality = inspect_paper(&value);
    if quality.error_count > 0 {
        let details = quality
            .issues
            .iter()
            .filter(|issue| issue.level == "error")
            .take(3)
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>()
            .join("；");
        return Err(format!("试卷未通过自动质检: {details}"));
    }
    Ok(value)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegenItemRequest {
    pub paper: Value,
    pub section_index: usize,
    pub item_index: usize,
    pub knowledge_path: String,
    pub subject: String,
    pub grade: u8,
}

/// 仅重出一道题，返回新的 item JSON
pub fn regenerate_item(cfg: &AppConfig, req: &RegenItemRequest) -> Result<Value, String> {
    let sections = req
        .paper
        .get("sections")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "试卷无 sections".to_string())?;
    let sec = sections
        .get(req.section_index)
        .ok_or_else(|| "大题索引无效".to_string())?;
    let items = sec
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "大题无 items".to_string())?;
    let old = items
        .get(req.item_index)
        .ok_or_else(|| "小题索引无效".to_string())?;

    let sec_title = sec
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("题目");
    let sec_type = sec.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let old_stem = old.get("stem").and_then(|v| v.as_str()).unwrap_or("");
    let kps = old
        .get("knowledgePoints")
        .cloned()
        .unwrap_or(Value::Array(vec![]));

    let system = r#"你是小学命题教师。请只输出一道题的 JSON 对象（不要数组、不要 markdown）：
{
  "id": "题号",
  "stem": "题干",
  "options": ["A. .."],
  "answer": "答案",
  "analysis": "解析",
  "score": 分值数字,
  "knowledgePoints": ["知识点"]
}
要求：与原题考查点相同、难度相当，但题干数字/情境/表述必须明显不同；禁止超纲。"#
        .to_string();

    let user = format!(
        "年级：{} 学科：{}\n大题：{}（type={}）\n原题：{}\n知识点：{}\n请生成替换题。",
        req.grade,
        match req.subject.as_str() {
            "math" => "数学",
            "english" => "英语",
            _ => "语文",
        },
        sec_title,
        sec_type,
        old_stem,
        kps
    );

    let raw = chat_completion(cfg, &system, &user)?;
    let json_str = extract_json(&raw)?;
    let item: Value =
        serde_json::from_str(&json_str).map_err(|e| format!("换题 JSON 无效: {e}"))?;
    if item.get("stem").is_none() {
        return Err("换题结果缺少 stem".into());
    }
    Ok(item)
}

/// 生成平行卷（B/C 等）：同结构同知识点，换数换情境
pub fn generate_variant_labeled(
    cfg: &AppConfig,
    paper: &Value,
    label: &str,
) -> Result<Value, String> {
    let label = label.trim();
    let label = if label.is_empty() { "B" } else { label };
    let system = format!(
        r#"你是小学命题教师。用户给出一套完整试卷 JSON，请生成「{label} 卷」：
1. 大题结构、分值、题型完全一致
2. 考查知识点一致，难度相当
3. 数字、人名、情境、选项内容尽量全部更换
4. 标题：去掉原有（A卷）（B卷）（C卷）后缀后，再加「（{label}卷）」
5. 只输出完整试卷 JSON（含 meta、sections），不要 markdown"#
    );
    let user = format!(
        "请生成 {label} 卷。原卷 JSON：\n{}",
        serde_json::to_string(paper).unwrap_or_default()
    );
    let raw = chat_completion(cfg, &system, &user)?;
    let json_str = extract_json(&raw)?;
    let mut value: Value =
        serde_json::from_str(&json_str).map_err(|e| format!("{label}卷 JSON 无效: {e}"))?;
    if value.get("meta").is_none() || value.get("sections").is_none() {
        return Err(format!("{label}卷缺少 meta 或 sections"));
    }
    // 确保标题带卷别
    if let Some(meta) = value.get_mut("meta").and_then(|m| m.as_object_mut()) {
        if let Some(t) = meta.get("title").and_then(|v| v.as_str()) {
            let mut base = t
                .replace("（A卷）", "")
                .replace("（B卷）", "")
                .replace("（C卷）", "")
                .replace("(A卷)", "")
                .replace("(B卷)", "")
                .replace("(C卷)", "")
                .trim()
                .to_string();
            let tag = format!("（{label}卷）");
            if !base.contains(&tag) {
                base.push_str(&tag);
            }
            meta.insert("title".into(), Value::String(base));
        }
        meta.insert("variant".into(), Value::String(label.to_string()));
    }
    Ok(value)
}

/// 兼容旧接口
pub fn generate_variant_b(cfg: &AppConfig, paper: &Value) -> Result<Value, String> {
    generate_variant_labeled(cfg, paper, "B")
}

/// 一键生成平行套卷 A/B/C（A 为原卷，B/C 由 AI 换情境）
pub fn generate_parallel_set(cfg: &AppConfig, paper: &Value) -> Result<Value, String> {
    let mut paper_a = paper.clone();
    if let Some(meta) = paper_a.get_mut("meta").and_then(|m| m.as_object_mut()) {
        if let Some(t) = meta.get("title").and_then(|v| v.as_str()) {
            let mut base = t
                .replace("（A卷）", "")
                .replace("（B卷）", "")
                .replace("（C卷）", "")
                .trim()
                .to_string();
            if !base.contains("（A卷）") {
                base.push_str("（A卷）");
            }
            meta.insert("title".into(), Value::String(base));
        }
        meta.insert("variant".into(), Value::String("A".into()));
    }

    let paper_b = generate_variant_labeled(cfg, &paper_a, "B")?;
    let paper_c = generate_variant_labeled(cfg, &paper_a, "C")?;

    let title = paper
        .pointer("/meta/title")
        .and_then(|v| v.as_str())
        .unwrap_or("平行卷")
        .replace("（A卷）", "")
        .replace("（B卷）", "")
        .replace("（C卷）", "")
        .trim()
        .to_string();

    Ok(serde_json::json!({
        "kind": "parallelSet",
        "meta": {
            "title": format!("{title} · 平行卷 A/B/C"),
            "count": 3,
            "variants": ["A", "B", "C"]
        },
        "papers": [paper_a, paper_b, paper_c]
    }))
}
