//! 数学答案简易验算：口算/四则表达式

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// 从题干提取简单算式，如 6×7＝  36+18÷6
fn extract_expr(stem: &str) -> Option<String> {
    let s = stem
        .replace('×', "*")
        .replace('÷', "/")
        .replace('＋', "+")
        .replace('－', "-")
        .replace('（', "(")
        .replace('）', ")")
        .replace('＝', "=")
        .replace(' ', "");
    // 取第一个含运算符的片段
    let re = Regex::new(r"([0-9\.\+\-\*/\(\)]+)=?").ok()?;
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
    if let Some(cap) = re.captures(&s) {
        let e = cap.get(1)?.as_str().to_string();
        if e.chars().any(|c| "+-*/".contains(c)) {
            return Some(e);
        }
    }
    None
}

fn parse_answer_number(ans: &str) -> Option<f64> {
    let cleaned = ans.trim().replace('，', "").replace(',', "");
    let s = cleaned
        .split(|c: char| c == '；' || c == ';' || c == '、' || c == ' ')
        .next()
        .unwrap_or("")
        .trim();
    // 分数 a/b
    if let Some((a, b)) = s.split_once('/') {
        let na: f64 = a.trim().parse().ok()?;
        let nb: f64 = b.trim().parse().ok()?;
        if nb.abs() < 1e-12 {
            return None;
        }
        return Some(na / nb);
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
    (a - b).abs() < 1e-6 || ((a - b).abs() / a.abs().max(1.0) < 1e-6)
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
        let sec_type = sec.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let sec_title = sec.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let is_calc = {
            let t = format!("{sec_type}{sec_title}");
            t.contains("计算") || t.contains("口算") || t.contains("calc") || t.contains("竖式") || t.contains("脱式")
        };
        let items = match sec.get("items").and_then(|v| v.as_array()) {
            Some(a) => a,
            None => continue,
        };
        for (ii, item) in items.iter().enumerate() {
            let stem = item.get("stem").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let answer = item.get("answer").and_then(|v| v.as_str()).unwrap_or("").to_string();
            // 仅尝试可解析的计算题
            let try_calc = is_calc || extract_expr(&stem).is_some();
            if !try_calc {
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
                        items_out.push(VerifyItemResult {
                            section_index: si,
                            item_index: ii,
                            stem,
                            answer,
                            status: "mismatch".into(),
                            computed: Some(format_num(v)),
                            message: format!("算式 {expr} 结果应为 {}", format_num(v)),
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

fn format_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{:.4}", v).trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
