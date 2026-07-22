//! 数学答案简易验算：口算/四则表达式

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyItemResult {
    pub section_index: usize,
    pub item_index: usize,
    pub stem: String,
    pub answer: String,
    pub status: String, // ok | mismatch | skip | error
    pub computed: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyReport {
    pub total: usize,
    pub checked: usize,
    pub ok: usize,
    pub mismatch: usize,
    pub skipped: usize,
    pub items: Vec<VerifyItemResult>,
}

fn normalize_math_text(stem: &str) -> String {
    stem.replace('×', "*")
        .replace('÷', "/")
        .replace('＋', "+")
        .replace('－', "-")
        .replace('—', "-")
        .replace('–', "-")
        .replace('（', "(")
        .replace('）', ")")
        .replace('＝', "=")
        .replace('．', ".")
        .replace(' ', "")
        .replace('\u{3000}', "") // 全角空格
}

/// 从题干提取简单算式，如 6×7＝  36+18÷6
fn extract_expr(stem: &str) -> Option<String> {
    let s = normalize_math_text(stem);
    // 优先匹配 数字 运算符 数字
    let re2 = Regex::new(r"[0-9\.]+[\+\-\*/][0-9\.\+\-\*/\(\)]*").ok()?;
    if let Some(m) = re2.find(&s) {
        let mut e = m.as_str().trim_end_matches('=').to_string();
        // 去掉题号残留
        if let Some(pos) = e.find(|c: char| c.is_ascii_digit()) {
            e = e[pos..].to_string();
        }
        if e.contains('+') || e.contains('-') || e.contains('*') || e.contains('/') {
            return Some(e);
        }
    }
    let re = Regex::new(r"([0-9\.\+\-\*/\(\)]+)=?").ok()?;
    if let Some(cap) = re.captures(&s) {
        let e = cap.get(1)?.as_str().to_string();
        if e.chars().any(|c| "+-*/".contains(c)) {
            return Some(e);
        }
    }
    None
}

/// 题干是否基本是「纯算式」（口算/脱式），而不是夹了分数的应用/填空叙述
fn is_pure_calc_stem(stem: &str) -> bool {
    let raw = stem.trim();
    if raw.is_empty() {
        return false;
    }
    // 去掉常见填空括号后再统计
    let cleaned = raw
        .replace('（', "(")
        .replace('）', ")")
        .replace('　', " ")
        .replace('＿', "_")
        .replace('_', " ")
        .replace('—', " ")
        .replace('–', " ");
    let mut math = 0usize;
    let mut other = 0usize;
    for c in cleaned.chars() {
        if c.is_whitespace() {
            continue;
        }
        let is_math = c.is_ascii_digit()
            || matches!(
                c,
                '+' | '-'
                    | '*'
                    | '/'
                    | '×'
                    | '÷'
                    | '＋'
                    | '－'
                    | '='
                    | '＝'
                    | '.'
                    | '．'
                    | '('
                    | ')'
                    | '（'
                    | '）'
            );
        if is_math {
            math += 1;
        } else {
            other += 1;
        }
    }
    let total = math + other;
    if total == 0 || math < 3 {
        return false;
    }
    // 允许少量中文如「计算」「得」；叙述题通常 other 很多
    other <= 4 && (math as f64) / (total as f64) >= 0.75
}

/// 是否应对该题做自动验算（题干须以算式为主，避免叙述里夹带 3/8 被误抽）
fn should_try_verify(stem: &str) -> bool {
    extract_expr(stem).is_some() && is_pure_calc_stem(stem)
}

fn parse_answer_number(ans: &str) -> Option<f64> {
    let mut s = ans.trim().to_string();
    if s.is_empty() {
        return None;
    }
    // 去掉中文/英文括号注释：0.375（小数）、3/8(最简)
    if let Ok(re) = Regex::new(r"[（(][^）)]*[）)]") {
        s = re.replace_all(&s, "").to_string();
    }
    s = s
        .replace('，', "")
        .replace(',', "")
        .replace('．', ".")
        .replace('÷', "/")
        .replace('／', "/")
        .replace('％', "%")
        .replace('%', "%")
        .replace(' ', "")
        .replace('\u{3000}', "");

    // 多答案时取第一段
    let s = s
        .split(|c: char| c == '；' || c == ';' || c == '、' || c == ',' || c == '，')
        .next()
        .unwrap_or("")
        .trim();
    if s.is_empty() {
        return None;
    }

    // 百分数 37.5% → 0.375
    if let Some(body) = s.strip_suffix('%') {
        let n: f64 = body.trim().parse().ok()?;
        return Some(n / 100.0);
    }

    // 带分数 a b/c（极少见，空格已被去掉则跳过）
    // 假分数/真分数 a/b
    if let Some((a, b)) = s.split_once('/') {
        // 避免把日期或比号误解析：要求两侧都是纯数字
        let na: f64 = a.trim().parse().ok()?;
        let nb: f64 = b.trim().parse().ok()?;
        if nb.abs() < 1e-12 {
            return None;
        }
        return Some(na / nb);
    }

    // 抽出前导数值（容忍「0.375元」「12米」）
    if let Ok(re) = Regex::new(r"^[+-]?\d+(\.\d+)?") {
        if let Some(m) = re.find(s) {
            return m.as_str().parse().ok();
        }
    }
    s.parse().ok()
}

/// 极简表达式求值（支持 + - * / 与括号，左结合优先级）
fn eval_expr(expr: &str) -> Result<f64, String> {
    let chars: Vec<char> = expr.chars().filter(|c| !c.is_whitespace()).collect();
    let mut i = 0usize;

    fn parse_expr(chars: &[char], i: &mut usize) -> Result<f64, String> {
        let mut val = parse_term(chars, i)?;
        while *i < chars.len() {
            match chars[*i] {
                '+' => {
                    *i += 1;
                    val += parse_term(chars, i)?;
                }
                '-' => {
                    *i += 1;
                    val -= parse_term(chars, i)?;
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn parse_term(chars: &[char], i: &mut usize) -> Result<f64, String> {
        let mut val = parse_factor(chars, i)?;
        while *i < chars.len() {
            match chars[*i] {
                '*' => {
                    *i += 1;
                    val *= parse_factor(chars, i)?;
                }
                '/' => {
                    *i += 1;
                    let r = parse_factor(chars, i)?;
                    if r.abs() < 1e-12 {
                        return Err("除数为 0".into());
                    }
                    val /= r;
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn parse_factor(chars: &[char], i: &mut usize) -> Result<f64, String> {
        if *i >= chars.len() {
            return Err("表达式不完整".into());
        }
        if chars[*i] == '(' {
            *i += 1;
            let v = parse_expr(chars, i)?;
            if *i >= chars.len() || chars[*i] != ')' {
                return Err("括号不匹配".into());
            }
            *i += 1;
            return Ok(v);
        }
        if chars[*i] == '-' {
            *i += 1;
            return Ok(-parse_factor(chars, i)?);
        }
        let start = *i;
        while *i < chars.len() && (chars[*i].is_ascii_digit() || chars[*i] == '.') {
            *i += 1;
        }
        if start == *i {
            return Err(format!("无法解析: {}", chars[*i]));
        }
        let num: String = chars[start..*i].iter().collect();
        num.parse().map_err(|_| format!("数字无效: {num}"))
    }

    let v = parse_expr(&chars, &mut i)?;
    if i != chars.len() {
        return Err("表达式含无法识别字符".into());
    }
    Ok(v)
}

fn nearly_eq(a: f64, b: f64) -> bool {
    if a.is_nan() || b.is_nan() {
        return false;
    }
    let diff = (a - b).abs();
    if diff < 1e-6 {
        return true;
    }
    let scale = a.abs().max(b.abs()).max(1.0);
    diff / scale < 1e-6
}

pub fn verify_paper_math(paper: &Value) -> VerifyReport {
    let mut items_out = Vec::new();
    let mut ok = 0usize;
    let mut mismatch = 0usize;
    let mut skipped = 0usize;
    let mut checked = 0usize;

    let sections = match paper.get("sections").and_then(|v| v.as_array()) {
        Some(s) => s,
        None => {
            return VerifyReport {
                total: 0,
                checked: 0,
                ok: 0,
                mismatch: 0,
                skipped: 0,
                items: vec![],
            }
        }
    };

    for (si, sec) in sections.iter().enumerate() {
        let items = match sec.get("items").and_then(|v| v.as_array()) {
            Some(a) => a,
            None => continue,
        };
        for (ii, item) in items.iter().enumerate() {
            let stem = item
                .get("stem")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let answer = answer_to_string(item.get("answer"));

            if !should_try_verify(&stem) {
                skipped += 1;
                items_out.push(VerifyItemResult {
                    section_index: si,
                    item_index: ii,
                    stem: stem.clone(),
                    answer: answer.clone(),
                    status: "skip".into(),
                    computed: None,
                    message: "非可自动验算题型".into(),
                });
                continue;
            }

            let Some(expr) = extract_expr(&stem) else {
                skipped += 1;
                items_out.push(VerifyItemResult {
                    section_index: si,
                    item_index: ii,
                    stem,
                    answer,
                    status: "skip".into(),
                    computed: None,
                    message: "题干中未识别到算式".into(),
                });
                continue;
            };

            // 答案若含多个，只验第一个能解析的数
            let Some(ans_n) = parse_answer_number(&answer) else {
                skipped += 1;
                items_out.push(VerifyItemResult {
                    section_index: si,
                    item_index: ii,
                    stem,
                    answer,
                    status: "skip".into(),
                    computed: None,
                    message: "答案无法解析为数值".into(),
                });
                continue;
            };
            checked += 1;
            match eval_expr(&expr) {
                Ok(v) => {
                    if nearly_eq(v, ans_n) {
                        ok += 1;
                        items_out.push(VerifyItemResult {
                            section_index: si,
                            item_index: ii,
                            stem,
                            answer,
                            status: "ok".into(),
                            computed: Some(format_num(v)),
                            message: "一致".into(),
                        });
                    } else {
                        mismatch += 1;
                        let computed = format_num(v);
                        items_out.push(VerifyItemResult {
                            section_index: si,
                            item_index: ii,
                            stem,
                            answer: answer.clone(),
                            status: "mismatch".into(),
                            computed: Some(computed.clone()),
                            message: format!(
                                "算式 {expr} 计算得 {computed}，卷面答案为 {answer}"
                            ),
                        });
                    }
                }
                Err(e) => {
                    skipped += 1;
                    checked = checked.saturating_sub(1);
                    items_out.push(VerifyItemResult {
                        section_index: si,
                        item_index: ii,
                        stem,
                        answer,
                        status: "error".into(),
                        computed: None,
                        message: e,
                    });
                }
            }
        }
    }

    VerifyReport {
        total: items_out.len(),
        checked,
        ok,
        mismatch,
        skipped,
        items: items_out,
    }
}

fn answer_to_string(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => other
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| other.to_string()),
        None => String::new(),
    }
}

fn format_num(v: f64) -> String {
    if !v.is_finite() {
        return v.to_string();
    }
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        // 最多 6 位小数，去掉尾随 0
        let s = format!("{:.6}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// 将可自动验算且不一致的计算题答案改为程序计算结果（仅纯算式）。
/// 返回（修正后的试卷, 修正题数）。
pub fn repair_math_answers(paper: &Value) -> (Value, usize) {
    let mut out = paper.clone();
    let report = verify_paper_math(paper);
    if report.mismatch == 0 {
        return (out, 0);
    }
    let Some(sections) = out.get_mut("sections").and_then(|v| v.as_array_mut()) else {
        return (out, 0);
    };
    let mut fixed = 0usize;
    for it in report.items.iter().filter(|x| x.status == "mismatch") {
        let Some(computed) = it.computed.as_ref() else {
            continue;
        };
        let Some(sec) = sections.get_mut(it.section_index) else {
            continue;
        };
        let Some(items) = sec.get_mut("items").and_then(|v| v.as_array_mut()) else {
            continue;
        };
        let Some(item) = items.get_mut(it.item_index) else {
            continue;
        };
        // 仅修正「纯算式」题，避免误改正文应用题
        if !is_pure_calc_stem(&it.stem) && it.stem.chars().count() > 24 {
            continue;
        }
        if let Some(obj) = item.as_object_mut() {
            obj.insert("answer".into(), json!(computed));
            // 若有解析字段，附注自动修正，便于老师察觉
            if let Some(analysis) = obj.get("analysis").and_then(|v| v.as_str()) {
                if !analysis.contains("验算自动修正") {
                    obj.insert(
                        "analysis".into(),
                        json!(format!("{analysis}（验算自动修正答案）")),
                    );
                }
            }
            fixed += 1;
        }
    }
    (out, fixed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fraction_decimal_matches() {
        let paper = json!({
            "sections": [{
                "type": "calc",
                "title": "一、口算",
                "items": [
                    {"stem": "3÷8＝", "answer": "0.375"},
                    {"stem": "3/8＝", "answer": "0.375"},
                    {"stem": "3÷8＝", "answer": "3/8"},
                ]
            }]
        });
        let r = verify_paper_math(&paper);
        assert_eq!(r.checked, 3, "{r:?}");
        assert_eq!(r.mismatch, 0, "{r:?}");
        assert_eq!(r.ok, 3, "{r:?}");
    }

    #[test]
    fn word_problem_with_embedded_fraction_is_skipped() {
        let paper = json!({
            "meta": {"subject": "数学"},
            "sections": [{
                "type": "fill",
                "title": "一、填空题",
                "items": [
                    {"stem": "把3/8化成小数是（　　）", "answer": "0.375"},
                    {"stem": "一块饼的3/8分给小朋友，还剩这张饼的（　　）", "answer": "5/8"},
                ]
            }]
        });
        let r = verify_paper_math(&paper);
        // 叙述题不应因抽到 3/8 而误判不一致
        assert_eq!(r.mismatch, 0, "{r:?}");
        assert!(r.skipped >= 1, "{r:?}");
    }

    #[test]
    fn pure_calc_mismatch_reports_answer() {
        let paper = json!({
            "sections": [{
                "type": "calc",
                "title": "一、计算",
                "items": [
                    {"stem": "3÷8＝", "answer": "0.3"},
                ]
            }]
        });
        let r = verify_paper_math(&paper);
        assert_eq!(r.mismatch, 1);
        let msg = &r.items[0].message;
        assert!(msg.contains("0.375"), "{msg}");
        assert!(msg.contains("0.3"), "{msg}");
        assert!(msg.contains("卷面答案"), "{msg}");
    }

    #[test]
    fn parse_percent_and_unit() {
        assert!(nearly_eq(parse_answer_number("37.5%").unwrap(), 0.375));
        assert!(nearly_eq(parse_answer_number("0.375元").unwrap(), 0.375));
        assert!(nearly_eq(parse_answer_number("3/8").unwrap(), 0.375));
        assert!(nearly_eq(
            parse_answer_number("0.375（小数）").unwrap(),
            0.375
        ));
    }

    #[test]
    fn repair_overwrites_wrong_calc_answer() {
        let paper = json!({
            "sections": [{
                "type": "calc",
                "title": "一、口算",
                "items": [
                    {"stem": "3÷8＝", "answer": "0.3", "analysis": "除法"}
                ]
            }]
        });
        let (fixed, n) = repair_math_answers(&paper);
        assert_eq!(n, 1);
        let ans = fixed["sections"][0]["items"][0]["answer"].as_str().unwrap();
        assert_eq!(ans, "0.375");
        let r = verify_paper_math(&fixed);
        assert_eq!(r.mismatch, 0);
        assert_eq!(r.ok, 1);
    }

    #[test]
    fn extract_simple_exprs() {
        assert_eq!(extract_expr("6×7＝").as_deref(), Some("6*7"));
        assert_eq!(extract_expr("3÷8＝").as_deref(), Some("3/8"));
        assert_eq!(extract_expr("36+18÷6").as_deref(), Some("36+18/6"));
    }
}
