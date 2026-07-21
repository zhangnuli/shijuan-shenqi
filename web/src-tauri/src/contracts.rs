use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExamMeta {
    title: String,
    subject: String,
    grade: u8,
    total_score: f64,
    duration_min: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExamItem {
    #[serde(default)]
    id: String,
    stem: String,
    #[serde(default)]
    answer: String,
    #[serde(default)]
    score: f64,
}

#[derive(Debug, Deserialize)]
struct ExamSection {
    title: String,
    score: f64,
    items: Vec<ExamItem>,
}

#[derive(Debug, Deserialize)]
struct ExamPaper {
    meta: ExamMeta,
    sections: Vec<ExamSection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LessonMeta {
    title: String,
    subject: String,
    grade: u8,
    duration_min: u32,
}

#[derive(Debug, Deserialize, Default)]
struct LessonObjectives {
    #[serde(default)]
    knowledge: Vec<String>,
    #[serde(default)]
    ability: Vec<String>,
    #[serde(default)]
    emotion: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LessonStep {
    stage: String,
    #[serde(default)]
    minutes: u32,
}

#[derive(Debug, Deserialize)]
struct LessonPlan {
    meta: LessonMeta,
    #[serde(default)]
    objectives: LessonObjectives,
    process: Vec<LessonStep>,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

pub fn validate_exam_paper(value: &Value) -> Result<(), String> {
    let paper: ExamPaper =
        serde_json::from_value(value.clone()).map_err(|e| format!("试卷结构不符合合同: {e}"))?;

    if !non_empty(&paper.meta.title) || !non_empty(&paper.meta.subject) {
        return Err("试卷标题或学科为空".into());
    }
    if !(1..=12).contains(&paper.meta.grade) {
        return Err("试卷年级超出有效范围".into());
    }
    if paper.meta.total_score <= 0.0 || paper.meta.duration_min == 0 {
        return Err("试卷总分或考试时长无效".into());
    }
    if paper.sections.is_empty() {
        return Err("试卷没有大题".into());
    }

    for (section_index, section) in paper.sections.iter().enumerate() {
        if !non_empty(&section.title) || section.items.is_empty() || section.score <= 0.0 {
            return Err(format!(
                "第 {} 大题标题、分值或题目列表无效",
                section_index + 1
            ));
        }
        for (item_index, item) in section.items.iter().enumerate() {
            if !non_empty(&item.stem) {
                return Err(format!(
                    "第 {} 大题第 {} 小题缺少题干",
                    section_index + 1,
                    item_index + 1
                ));
            }
            if !non_empty(&item.answer) {
                return Err(format!(
                    "第 {} 大题第 {} 小题缺少答案",
                    section_index + 1,
                    item_index + 1
                ));
            }
            if item.score < 0.0 {
                return Err(format!(
                    "第 {} 大题第 {} 小题分值无效",
                    section_index + 1,
                    item_index + 1
                ));
            }
            let _ = &item.id;
        }
    }
    Ok(())
}

pub fn validate_lesson_plan(value: &Value) -> Result<(), String> {
    let plan: LessonPlan =
        serde_json::from_value(value.clone()).map_err(|e| format!("教案结构不符合合同: {e}"))?;
    if !non_empty(&plan.meta.title) || !non_empty(&plan.meta.subject) {
        return Err("教案标题或学科为空".into());
    }
    if !(1..=12).contains(&plan.meta.grade) || plan.meta.duration_min == 0 {
        return Err("教案年级或课时长度无效".into());
    }
    let objective_count = plan.objectives.knowledge.len()
        + plan.objectives.ability.len()
        + plan.objectives.emotion.len();
    if objective_count == 0 {
        return Err("教案缺少教学目标".into());
    }
    if plan.process.is_empty() {
        return Err("教案缺少教学过程".into());
    }
    if plan.process.iter().any(|step| !non_empty(&step.stage)) {
        return Err("教案教学过程存在空环节名称".into());
    }
    let _total_minutes: u32 = plan.process.iter().map(|step| step.minutes).sum();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_exam_without_answer() {
        let value = json!({
            "meta": {"title":"测试","subject":"数学","grade":3,"totalScore":100,"durationMin":60},
            "sections": [{"title":"一、计算","score":100,"items":[{"id":"1","stem":"1+1","answer":""}]}]
        });
        assert!(validate_exam_paper(&value)
            .unwrap_err()
            .contains("缺少答案"));
    }

    #[test]
    fn accepts_minimal_valid_exam() {
        let value = json!({
            "meta": {"title":"测试","subject":"数学","grade":3,"totalScore":100,"durationMin":60},
            "sections": [{"title":"一、计算","score":100,"items":[{"id":"1","stem":"1+1","answer":"2","score":100}]}]
        });
        assert!(validate_exam_paper(&value).is_ok());
    }

    #[test]
    fn rejects_lesson_without_process() {
        let value = json!({
            "meta": {"title":"测试教案","subject":"数学","grade":3,"durationMin":40},
            "objectives": {"knowledge":["会计算"]},
            "process": []
        });
        assert!(validate_lesson_plan(&value)
            .unwrap_err()
            .contains("教学过程"));
    }
}
