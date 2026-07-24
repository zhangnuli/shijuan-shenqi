//! 同步小学试卷网公开 HTML 页面，作为本地命题素材库。
//! 只读取站点公开页面；网盘地址仅保留来源，不自动下载。

use crate::storage::{read_json, write_json};
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const BASE_URL: &str = "https://appsj.szxuexiao.com";
const SCHEMA_VERSION: u32 = 1;
const MAX_CACHE_ITEMS: usize = 500;
const MAX_CONTENT_CHARS: usize = 20_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSjResource {
    pub id: String,
    pub title: String,
    pub url: String,
    pub subject: String,
    pub grade: u8,
    pub semester: String,
    pub edition: String,
    pub exam_type: String,
    #[serde(default)]
    pub unit_name: String,
    pub content: String,
    #[serde(default)]
    pub download_url: String,
    #[serde(default)]
    pub download_code: String,
    pub source_name: String,
    pub source_home: String,
    pub synced_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSjSyncRequest {
    pub subject: String,
    pub grade: u8,
    pub semester: String,
    pub exam_type: String,
    #[serde(default)]
    pub unit_name: String,
    #[serde(default)]
    pub max_pages: Option<u8>,
    #[serde(default)]
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSjSyncReport {
    pub ok: bool,
    pub message: String,
    pub discovered: usize,
    pub fetched: usize,
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: Vec<String>,
    pub total: usize,
    pub data_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSjStore {
    #[serde(default = "schema_version")]
    schema_version: u32,
    #[serde(default)]
    last_synced_at: u64,
    #[serde(default)]
    resources: Vec<AppSjResource>,
}

impl Default for AppSjStore {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            last_synced_at: 0,
            resources: Vec::new(),
        }
    }
}

fn schema_version() -> u32 {
    SCHEMA_VERSION
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("question-bank");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir.join("appsj_resources.json"))
}

fn load_store(app: &AppHandle) -> Result<AppSjStore, String> {
    Ok(read_json(&store_path(app)?)?.unwrap_or_default())
}

fn save_store(app: &AppHandle, store: &AppSjStore) -> Result<(), String> {
    write_json(&store_path(app)?, store)
}

fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(35))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("ShiJuanShenQi/0.1 (public-resource-index; contact via project page)")
        .build()
        .map_err(|error| format!("HTTP 客户端创建失败: {error}"))
}

fn fetch_html(client: &Client, url: &str) -> Result<String, String> {
    let response = client
        .get(url)
        .send()
        .map_err(|error| format!("请求失败 {url}: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}: {url}", response.status()));
    }
    let bytes = response.bytes().map_err(|error| error.to_string())?;
    if let Ok(text) = std::str::from_utf8(&bytes) {
        return Ok(text.to_string());
    }
    let (text, _, _) = encoding_rs::GBK.decode(&bytes);
    Ok(text.into_owned())
}

fn grade_semester_slug(grade: u8, semester: &str) -> Result<&'static str, String> {
    match (grade, semester) {
        (1, "shang") => Ok("yinianji_s"),
        (1, "xia") => Ok("yinianji_x"),
        (2, "shang") => Ok("ernianji_s"),
        (2, "xia") => Ok("ernianji_x"),
        (3, "shang") => Ok("shannianji_s"),
        (3, "xia") => Ok("shannianji_x"),
        (4, "shang") => Ok("shinianji_s"),
        (4, "xia") => Ok("shinianji_x"),
        (5, "shang") => Ok("wunianji_s"),
        (5, "xia") => Ok("wunianji_x"),
        (6, "shang") => Ok("liunianji_s"),
        (6, "xia") => Ok("liunianji_x"),
        _ => Err("仅支持 1-6 年级及上/下册".into()),
    }
}

fn decode_entities(input: &str) -> String {
    let mut text = input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    let numeric = Regex::new(r"&#(\d+);").unwrap();
    text = numeric
        .replace_all(&text, |caps: &regex::Captures<'_>| {
            caps.get(1)
                .and_then(|value| value.as_str().parse::<u32>().ok())
                .and_then(char::from_u32)
                .map(|value| value.to_string())
                .unwrap_or_default()
        })
        .into_owned();
    text
}

fn html_to_text(html: &str) -> String {
    let re_noise =
        Regex::new(r"(?is)<(?:script|style|svg)[^>]*>.*?</(?:script|style|svg)>").unwrap();
    let re_image = Regex::new(r"(?is)<img[^>]*>").unwrap();
    let re_block =
        Regex::new(r"(?is)</?(?:p|div|section|article|h[1-6]|li|tr|table|br|hr)[^>]*>").unwrap();
    let re_cell = Regex::new(r"(?is)</?(?:td|th)[^>]*>").unwrap();
    let re_tag = Regex::new(r"(?is)<[^>]+>").unwrap();
    let text = re_noise.replace_all(html, " ");
    let text = re_image.replace_all(&text, " ");
    let text = re_cell.replace_all(&text, "\t");
    let text = re_block.replace_all(&text, "\n");
    let text = re_tag.replace_all(&text, " ");
    let text = decode_entities(&text);
    let mut lines = Vec::new();
    for raw in text.lines() {
        let line = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        if !line.is_empty() && lines.last() != Some(&line) {
            lines.push(line);
        }
    }
    lines.join("\n")
}

fn extract_listing_links(html: &str) -> Vec<(String, String)> {
    let re = Regex::new(
        r#"(?is)<a[^>]+class=[\"'][^\"']*list-group-item[^\"']*[\"'][^>]+href=[\"'](/html/(\d+)\.html)[\"'][^>]*>(.*?)</a>"#,
    )
    .unwrap();
    let mut seen = HashSet::new();
    let mut links = Vec::new();
    for captures in re.captures_iter(html) {
        let id = captures.get(2).map(|value| value.as_str()).unwrap_or("");
        if id.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        let path = captures.get(1).map(|value| value.as_str()).unwrap_or("");
        let title = html_to_text(captures.get(3).map(|value| value.as_str()).unwrap_or(""));
        if !title.is_empty() {
            links.push((format!("{BASE_URL}{path}"), title));
        }
    }
    links
}

fn subject_of(title: &str) -> &'static str {
    if title.contains("数学") {
        "math"
    } else if title.contains("语文") {
        "chinese"
    } else if title.contains("英语") {
        "english"
    } else {
        ""
    }
}

fn edition_of(title: &str) -> &'static str {
    if title.contains("北师大") {
        "beishida"
    } else if title.contains("苏教") {
        "sujiao"
    } else if title.contains("统编") || title.contains("部编") || title.contains("人教") {
        "renjiao"
    } else {
        ""
    }
}

fn exam_type_of(title: &str) -> &'static str {
    if title.contains("期中") {
        "midterm"
    } else if title.contains("期末") || title.contains("升学") || title.contains("毕业") {
        "final"
    } else if title.contains("单元") || title.contains("阶段") {
        "unit"
    } else if title.contains("口算") || title.contains("专项") {
        "oral"
    } else if title.contains("课时") || title.contains("同步练习") {
        "lesson"
    } else if title.contains("作业") {
        "homework"
    } else {
        ""
    }
}

fn title_matches(title: &str, request: &AppSjSyncRequest) -> bool {
    if subject_of(title) != request.subject {
        return false;
    }
    let found_exam = exam_type_of(title);
    if !request.exam_type.is_empty() && !found_exam.is_empty() && found_exam != request.exam_type {
        return false;
    }
    if request.exam_type == "unit" && !request.unit_name.trim().is_empty() {
        let unit = request.unit_name.replace(' ', "");
        let compact = title.replace(' ', "");
        if unit.contains("第") && unit.contains("单元") && !compact.contains(&unit) {
            let number = unit
                .trim_start_matches('第')
                .split('单')
                .next()
                .unwrap_or("");
            if !number.is_empty() && !compact.contains(&format!("第{number}单元")) {
                return false;
            }
        }
    }
    true
}

fn capture_first(html: &str, pattern: &str) -> String {
    Regex::new(pattern)
        .ok()
        .and_then(|re| re.captures(html))
        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
        .unwrap_or_default()
}

fn parse_resource(
    html: &str,
    fallback_title: &str,
    url: &str,
    request: &AppSjSyncRequest,
) -> Result<AppSjResource, String> {
    let title_html = capture_first(
        html,
        r#"(?is)<h2[^>]*class=[\"'][^\"']*post-title[^\"']*[\"'][^>]*>(.*?)</h2>"#,
    );
    let title = {
        let parsed = html_to_text(&title_html);
        if parsed.is_empty() {
            fallback_title.to_string()
        } else {
            parsed
        }
    };
    let entry = capture_first(
        html,
        r#"(?is)<div[^>]*class=[\"'][^\"']*entry-content[^\"']*[\"'][^>]*>(.*?)<div[^>]*class=[\"'][^\"']*share-block"#,
    );
    if entry.is_empty() {
        return Err("详情页缺少正文区域".into());
    }
    let download_url = capture_first(
        &entry,
        r#"(?is)href=[\"'](https?://pan\.baidu\.com/[^\"']+)[\"']"#,
    );
    let download_code = capture_first(&entry, r#"(?is)提取码[^<]{0,20}<[^>]*>([^<]+)</"#)
        .trim()
        .to_string();
    let without_download = Regex::new(
        r#"(?is)<div[^>]*class=[\"'][^\"']*download-section[^\"']*[\"'][^>]*>.*?</div>"#,
    )
    .unwrap()
    .replace_all(&entry, " ")
    .into_owned();
    let content: String = html_to_text(&without_download)
        .chars()
        .take(MAX_CONTENT_CHARS)
        .collect();
    if content.chars().count() < 20 {
        return Err("详情页没有足够的公开文字素材".into());
    }
    let id = Regex::new(r"/html/(\d+)\.html")
        .unwrap()
        .captures(url)
        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
        .unwrap_or_else(|| url.to_string());
    Ok(AppSjResource {
        id,
        title: title.clone(),
        url: url.to_string(),
        subject: subject_of(&title).to_string(),
        grade: request.grade,
        semester: request.semester.clone(),
        edition: edition_of(&title).to_string(),
        exam_type: exam_type_of(&title).to_string(),
        unit_name: request.unit_name.clone(),
        content,
        download_url,
        download_code,
        source_name: "小学试卷网".into(),
        source_home: BASE_URL.into(),
        synced_at: now_secs(),
    })
}

pub fn list_resources(app: &AppHandle) -> Result<Vec<AppSjResource>, String> {
    let mut resources = load_store(app)?.resources;
    resources.sort_by(|a, b| b.synced_at.cmp(&a.synced_at));
    Ok(resources)
}

fn resource_matches(
    resource: &AppSjResource,
    subject: &str,
    edition: &str,
    grade: u8,
    semester: &str,
    exam_type: &str,
    unit_name: &str,
) -> bool {
    if resource.subject != subject || resource.grade != grade || resource.semester != semester {
        return false;
    }
    if !edition.is_empty() && !resource.edition.is_empty() && resource.edition != edition {
        return false;
    }
    if !exam_type.is_empty() && !resource.exam_type.is_empty() && resource.exam_type != exam_type {
        return false;
    }
    if exam_type == "unit" && !unit_name.trim().is_empty() {
        let unit = unit_name.replace(' ', "");
        let haystack = format!("{}{}", resource.title, resource.unit_name).replace(' ', "");
        if unit.contains("单元") && !haystack.contains(&unit) {
            return false;
        }
    }
    true
}

pub fn pick_resource_snippets(
    app: &AppHandle,
    subject: &str,
    edition: &str,
    grade: u8,
    semester: &str,
    exam_type: &str,
    unit_name: &str,
    limit: usize,
) -> Result<Vec<String>, String> {
    let resources = list_resources(app)?;
    Ok(resources
        .into_iter()
        .filter(|resource| {
            resource_matches(
                resource, subject, edition, grade, semester, exam_type, unit_name,
            )
        })
        .take(limit)
        .map(|resource| {
            let excerpt: String = resource.content.chars().take(1_500).collect();
            format!("来源《{}》\n{}", resource.title, excerpt)
        })
        .collect())
}

/// 仅返回能由内置表达式求值器得到明确答案的算式。
pub fn pick_verified_math_expressions(
    app: &AppHandle,
    edition: &str,
    grade: u8,
    semester: &str,
    exam_type: &str,
    unit_name: &str,
    limit: usize,
) -> Result<Vec<(String, String, String)>, String> {
    let resources = list_resources(app)?;
    let expression = Regex::new(r"\d[\d\.\+\-\*/\(\)]{2,30}").unwrap();
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for resource in resources.into_iter().filter(|resource| {
        resource_matches(
            resource, "math", edition, grade, semester, exam_type, unit_name,
        )
    }) {
        let normalized = resource
            .content
            .replace('×', "*")
            .replace('÷', "/")
            .replace('＋', "+")
            .replace('−', "-")
            .replace('－', "-")
            .replace('（', "(")
            .replace('）', ")");
        for found in expression.find_iter(&normalized) {
            let raw = found.as_str().trim_matches(|c| c == '(' || c == ')');
            if raw.len() < 3 || !seen.insert(raw.to_string()) {
                continue;
            }
            if let Ok(answer) = crate::verify::evaluate_simple_expression(raw) {
                let display = raw.replace('*', "×").replace('/', "÷");
                items.push((display, answer, resource.title.clone()));
                if items.len() >= limit {
                    return Ok(items);
                }
            }
        }
    }
    Ok(items)
}

pub fn sync_resources(
    app: &AppHandle,
    request: AppSjSyncRequest,
) -> Result<AppSjSyncReport, String> {
    if !(1..=6).contains(&request.grade) {
        return Err("题库同步仅支持小学 1-6 年级".into());
    }
    if !matches!(request.subject.as_str(), "math" | "chinese" | "english") {
        return Err("题库同步仅支持数学、语文和英语".into());
    }
    let slug = grade_semester_slug(request.grade, &request.semester)?;
    let max_pages = request.max_pages.unwrap_or(3).clamp(1, 5);
    let max_items = request.max_items.unwrap_or(20).clamp(1, 50);
    let client = client()?;
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    let mut failed = Vec::new();
    for page in 1..=max_pages {
        let file = if page == 1 {
            "index.html".to_string()
        } else {
            format!("index_{page}.html")
        };
        let url = format!("{BASE_URL}/{slug}/{file}");
        match fetch_html(&client, &url) {
            Ok(html) => {
                for (detail_url, title) in extract_listing_links(&html) {
                    if seen.insert(detail_url.clone()) && title_matches(&title, &request) {
                        links.push((detail_url, title));
                    }
                }
            }
            Err(error) => failed.push(error),
        }
        if links.len() >= max_items {
            break;
        }
        thread::sleep(Duration::from_millis(250));
    }
    links.truncate(max_items);
    let discovered = links.len();
    let mut parsed = Vec::new();
    let mut skipped = 0;
    for (url, title) in links {
        match fetch_html(&client, &url)
            .and_then(|html| parse_resource(&html, &title, &url, &request))
        {
            Ok(resource) => parsed.push(resource),
            Err(error) => {
                skipped += 1;
                failed.push(format!("{title}: {error}"));
            }
        }
        thread::sleep(Duration::from_millis(350));
    }
    let fetched = parsed.len();
    let mut store = load_store(app)?;
    let mut positions: HashMap<String, usize> = store
        .resources
        .iter()
        .enumerate()
        .map(|(index, resource)| (resource.id.clone(), index))
        .collect();
    let mut added = 0;
    let mut updated = 0;
    for resource in parsed {
        if let Some(index) = positions.get(&resource.id).copied() {
            store.resources[index] = resource;
            updated += 1;
        } else {
            positions.insert(resource.id.clone(), store.resources.len());
            store.resources.push(resource);
            added += 1;
        }
    }
    store
        .resources
        .sort_by(|a, b| b.synced_at.cmp(&a.synced_at));
    store.resources.truncate(MAX_CACHE_ITEMS);
    store.schema_version = SCHEMA_VERSION;
    store.last_synced_at = now_secs();
    save_store(app, &store)?;
    let data_path = store_path(app)?.display().to_string();
    let ok = fetched > 0;
    let message = if ok {
        format!("已同步 {fetched} 份公开网页素材，新增 {added}，更新 {updated}")
    } else {
        "没有找到符合当前条件的公开网页素材，可放宽卷型或稍后重试".into()
    };
    Ok(AppSjSyncReport {
        ok,
        message,
        discovered,
        fetched,
        added,
        updated,
        skipped,
        failed,
        total: store.resources.len(),
        data_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_parser_deduplicates_detail_links() {
        let html = r#"
            <a class="list-group-item" href="/html/7092.html">三年级数学下册期末测试卷</a>
            <a class="list-group-item other" href="/html/7092.html">重复</a>
            <a class="list-group-item" href="/html/7041.html"><b>统编版三年级语文下册第四单元测试卷</b></a>
        "#;
        let links = extract_listing_links(html);
        assert_eq!(links.len(), 2);
        assert!(links[1].1.contains("第四单元"));
    }

    #[test]
    fn detail_parser_extracts_public_text_and_source_link() {
        let html = r#"
            <h2 class="post-title">北师大版三年级数学下册期末测试卷</h2>
            <div class="entry-content">
              <p><b>试卷结构</b></p><p>一、计算题 20 分</p>
              <p>432÷4、208×5、846÷6</p>
              <div class="download-section"><a href="https://pan.baidu.com/s/demo?pwd=abcd">下载</a><span>提取码：</span><b>abcd</b></div>
            </div><div class="share-block"></div>
        "#;
        let request = AppSjSyncRequest {
            subject: "math".into(),
            grade: 3,
            semester: "xia".into(),
            exam_type: "final".into(),
            ..Default::default()
        };
        let resource = parse_resource(
            html,
            "fallback",
            "https://appsj.szxuexiao.com/html/1.html",
            &request,
        )
        .unwrap();
        assert!(resource.content.contains("432÷4"));
        assert!(!resource.content.contains("下载"));
        assert_eq!(
            resource.download_url,
            "https://pan.baidu.com/s/demo?pwd=abcd"
        );
        assert_eq!(resource.exam_type, "final");
    }

    #[test]
    fn title_filter_respects_subject_and_exam_type() {
        let request = AppSjSyncRequest {
            subject: "chinese".into(),
            grade: 3,
            semester: "shang".into(),
            exam_type: "unit".into(),
            unit_name: "第四单元".into(),
            ..Default::default()
        };
        assert!(title_matches(
            "统编版三年级语文上册第四单元测试卷",
            &request
        ));
        assert!(!title_matches("统编版三年级语文上册期末测试卷", &request));
        assert!(!title_matches("三年级数学上册第四单元测试卷", &request));
    }
}
