//! 从电子课本网抓取公开目录，写入本地课标包。
//! 优先：https://www.dzkbw.org/book/xxxx.html 页内 <pre> 目录（含 2026 秋版分篇古诗）
//! 回退：http://www.dzkbw.com/ books 页 bookmulu 链接目录

use crate::knowledge::{KnowledgePack, SourceInfo, UnitInfo};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Manager};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";
const ORG_SITE: &str = "https://www.dzkbw.org";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCurriculumResult {
    pub ok: bool,
    pub message: String,
    pub updated: Vec<String>,
    pub failed: Vec<String>,
    pub data_dir: String,
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| e.to_string())
}

fn fetch_html(url: &str) -> Result<String, String> {
    let c = client()?;
    let resp = c
        .get(url)
        .header("User-Agent", UA)
        .send()
        .map_err(|e| format!("请求失败 {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} : {url}", resp.status()));
    }
    let bytes = resp.bytes().map_err(|e| e.to_string())?;
    // dzkbw.org 多为 UTF-8；.com 多为 GBK
    if url.contains("dzkbw.org") {
        if let Ok(s) = std::str::from_utf8(&bytes) {
            return Ok(s.to_string());
        }
        let (cow, _, _) = encoding_rs::UTF_8.decode(&bytes);
        return Ok(cow.into_owned());
    }
    // 先试 UTF-8，再 GBK
    if let Ok(s) = std::str::from_utf8(&bytes) {
        if s.contains("单元") || s.contains("年级") || s.contains("<pre") {
            return Ok(s.to_string());
        }
    }
    let (cow, _, _) = encoding_rs::GBK.decode(&bytes);
    Ok(cow.into_owned())
}

fn strip_tags(s: &str) -> String {
    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    let re_style = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    let re_tag = Regex::new(r"(?is)<[^>]+>").unwrap();
    let s = re_script.replace_all(s, " ");
    let s = re_style.replace_all(&s, " ");
    let s = re_tag.replace_all(&s, "");
    html_escape_decode(&s)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn html_escape_decode(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn extract_links(html: &str) -> Vec<(String, String)> {
    let re = Regex::new(r#"(?is)<a[^>]+href=["']([^"']+)["'][^>]*>(.*?)</a>"#).unwrap();
    let mut out = Vec::new();
    for cap in re.captures_iter(html) {
        let href = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("").to_string();
        let text = strip_tags(cap.get(2).map(|m| m.as_str()).unwrap_or(""));
        if !text.is_empty() {
            out.push((href, text));
        }
    }
    out
}

fn join_url(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    if href.starts_with("//") {
        return format!("http:{}", href);
    }
    let base = base.trim_end_matches('/');
    if href.starts_with('/') {
        // origin
        if let Some(pos) = base.find("://") {
            let rest = &base[pos + 3..];
            let host = rest.split('/').next().unwrap_or("");
            return format!("{}://{}{}", &base[..pos], host, href);
        }
    }
    // relative
    let mut base_path = base.to_string();
    if !base_path.ends_with('/') {
        if let Some(i) = base_path.rfind('/') {
            // if last segment looks like file, strip it
            let last = &base_path[i + 1..];
            if last.contains('.') && !last.contains("dzkbw") {
                base_path = base_path[..=i].to_string();
            } else {
                base_path.push('/');
            }
        }
    }
    format!("{}{}", base_path.trim_end_matches('/'), format!("/{}", href.trim_start_matches('/')))
}

fn parse_grade_sem(url: &str) -> Option<(u8, String)> {
    let re = Regex::new(r"/(?:xs|ws)?([1-6])([sx])(?:[_/]|$)").unwrap();
    if let Some(c) = re.captures(url) {
        let g: u8 = c.get(1)?.as_str().parse().ok()?;
        let s = if c.get(2)?.as_str() == "s" {
            "shang"
        } else {
            "xia"
        };
        return Some((g, s.into()));
    }
    let re2 = Regex::new(r"/([1-6])([sx])(?:[_/]|$)").unwrap();
    if let Some(c) = re2.captures(url) {
        let g: u8 = c.get(1)?.as_str().parse().ok()?;
        let s = if c.get(2)?.as_str() == "s" {
            "shang"
        } else {
            "xia"
        };
        return Some((g, s.into()));
    }
    None
}

fn find_book_links(index_url: &str, subject_path_key: &str) -> Result<Vec<(String, String, u8, String, i32)>, String> {
    let html = fetch_html(index_url)?;
    let mut results = Vec::new();
    for (href, title) in extract_links(&html) {
        let full = join_url(index_url, &href);
        if !full.contains(subject_path_key) {
            continue;
        }
        if !full.contains("/books/") {
            continue;
        }
        if Regex::new(r"/\d{3}\.htm").unwrap().is_match(&full) {
            continue;
        }
        let Some((grade, sem)) = parse_grade_sem(&full) else {
            continue;
        };
        let mut score = 0;
        if full.contains("/xs") {
            score += 100;
        }
        if full.contains("/ws") {
            score -= 50;
        }
        if Regex::new(r"20\d{2}").unwrap().is_match(&full)
            || Regex::new(r"20\d{2}").unwrap().is_match(&title)
        {
            score += 30;
        }
        if title.contains("上册") || title.contains("下册") || title.contains("年级") {
            score += 10;
        }
        let url = format!("{}/", full.trim_end_matches('/'));
        results.push((url, title, grade, sem, score));
    }
    Ok(results)
}

fn pick_best(links: Vec<(String, String, u8, String, i32)>) -> HashMap<(u8, String), (String, String)> {
    let mut buckets: HashMap<(u8, String), Vec<(String, String, i32)>> = HashMap::new();
    for (url, title, g, s, score) in links {
        buckets
            .entry((g, s))
            .or_default()
            .push((url, title, score));
    }
    let mut chosen = HashMap::new();
    for (key, mut items) in buckets {
        items.sort_by(|a, b| b.2.cmp(&a.2));
        if let Some((url, title, _)) = items.into_iter().next() {
            chosen.insert(key, (url, title));
        }
    }
    chosen
}

fn is_section_as_lesson(t: &str) -> bool {
    matches!(
        t,
        "口语交际" | "习作" | "语文园地" | "阅读" | "写话" | "日积月累"
    )
}

fn is_unit_title(t: &str) -> bool {
    if is_section_as_lesson(t) {
        return false;
    }
    let hard = Regex::new(
        r"^(?:第[一二三四五六七八九十百零\d]+单元|[一二三四五六七八九十]+[、.\s．][^\s]{1,20}|整理与复习|数学好玩|总复习|快乐读书吧|我上学了|我上学啦|汉语拼音|综合实践|综合性学习)",
    )
    .unwrap();
    if hard.is_match(t) {
        return true;
    }
    if t.contains("单元") && t.chars().count() <= 24 {
        return true;
    }
    false
}

fn group_catalog(entries: &[String]) -> Vec<UnitInfo> {
    let mut units: Vec<UnitInfo> = Vec::new();
    let mut current: Option<usize> = None;

    for t in entries {
        let t = t.trim();
        if t.is_empty() {
            continue;
        }
        if is_unit_title(t) {
            units.push(UnitInfo {
                id: format!("u{}", units.len() + 1),
                name: t.to_string(),
                lessons: vec![],
                points: vec![],
            });
            current = Some(units.len() - 1);
            continue;
        }
        if current.is_none() {
            units.push(UnitInfo {
                id: "u1".into(),
                name: t.to_string(),
                lessons: vec![],
                points: vec![],
            });
            current = Some(0);
            continue;
        }
        let idx = current.unwrap();
        if units[idx].name == t {
            continue;
        }
        if !units[idx].lessons.iter().any(|x| x == t) {
            units[idx].lessons.push(t.to_string());
        }
    }

    for u in &mut units {
        let mut pts = vec![u.name.clone()];
        for x in u.lessons.iter().take(16) {
            if !pts.contains(x) {
                pts.push(x.clone());
            }
        }
        u.points = pts.into_iter().take(20).collect();
    }
    units
}

/// 从 dzkbw.org book 页的 <pre> 目录提取行（可含分篇古诗）
fn extract_pre_catalog(html: &str) -> Vec<String> {
    let re = Regex::new(r"(?is)<pre[^>]*>(.*?)</pre>").unwrap();
    let mut best: Vec<String> = Vec::new();
    let mut best_score = 0i32;
    for cap in re.captures_iter(html) {
        let body = html_escape_decode(cap.get(1).map(|m| m.as_str()).unwrap_or(""));
        let lines: Vec<String> = body
            .lines()
            .map(|l| {
                l.replace('\u{00a0}', " ")
                    .replace("&ldquo;", "“")
                    .replace("&rdquo;", "”")
                    .trim()
                    .to_string()
            })
            .filter(|l| !l.is_empty() && l.chars().count() < 80)
            .collect();
        let unit_n = lines
            .iter()
            .filter(|l| l.contains("单元") || is_unit_title(l))
            .count() as i32;
        let score = unit_n * 20 + lines.len() as i32;
        if score > best_score && lines.len() >= 6 {
            best_score = score;
            best = lines;
        }
    }
    best
}

fn extract_book_catalog(html: &str, book_url: &str) -> Vec<String> {
    let pre = extract_pre_catalog(html);
    if pre.len() >= 8 {
        return pre;
    }
    let mulu = extract_book_mulu(html, book_url);
    if mulu.len() >= pre.len() {
        mulu
    } else {
        pre
    }
}

fn extract_book_mulu(html: &str, book_url: &str) -> Vec<String> {
    let book_path = {
        let p = book_url
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        let path = p.find('/').map(|i| &p[i..]).unwrap_or("/");
        path.trim_end_matches('/').to_string()
    };

    let re_chunk =
        Regex::new(r#"(?is)<div[^>]+class=["'][^"']*bookmulu[^"']*["'][^>]*>(.*?)</div>"#)
            .unwrap();
    let mut region = String::new();
    for cap in re_chunk.captures_iter(html) {
        region.push_str(cap.get(1).map(|m| m.as_str()).unwrap_or(""));
        region.push('\n');
    }
    if region.trim().is_empty() {
        region = html.to_string();
    }

    let page_re = Regex::new(r"(?i)/\d{2,4}\.htm?$").unwrap();
    let mut titles = Vec::new();
    for (href, text) in extract_links(&region) {
        let full = join_url(book_url, &href);
        let path = {
            let p = full
                .trim_start_matches("http://")
                .trim_start_matches("https://");
            p.find('/').map(|i| &p[i..]).unwrap_or("/").to_string()
        };
        if !page_re.is_match(&path) {
            continue;
        }
        if !path.contains(&book_path) && !href.rsplit('/').next().unwrap_or("").ends_with(".htm")
        {
            // relative 001.htm under same book still ok if region is bookmulu
            if !region.contains("bookmulu") && !path.contains(book_path.trim_start_matches('/')) {
                continue;
            }
        }
        // if full path doesn't include book folder name, still accept numbered htm from mulu region
        let t = text.trim().to_string();
        if t.is_empty() {
            continue;
        }
        if titles.last().map(|x| x == &t).unwrap_or(false) {
            continue;
        }
        titles.push(t);
    }

    if titles.len() < 5 {
        titles.clear();
        for (href, text) in extract_links(html) {
            let full = join_url(book_url, &href);
            if !page_re.is_match(&full) {
                continue;
            }
            if !full.contains(&book_path) {
                continue;
            }
            let t = text.trim().to_string();
            if t.is_empty() {
                continue;
            }
            if titles.last().map(|x| x == &t).unwrap_or(false) {
                continue;
            }
            titles.push(t);
        }
    }
    titles
}

fn build_pack(
    subject: &str,
    edition: &str,
    grade: u8,
    semester: &str,
    title: &str,
    units: Vec<UnitInfo>,
    catalog_url: &str,
    edition_label: &str,
    subject_label: &str,
    entry_count: usize,
) -> KnowledgePack {
    let sem_label = if semester == "shang" { "上册" } else { "下册" };
    KnowledgePack {
        subject: subject.into(),
        edition: edition.into(),
        grade,
        semester: semester.into(),
        title: title.into(),
        source: SourceInfo {
            platform: "国家中小学智慧教育平台 + 电子课本网目录".into(),
            platform_url: "https://basic.smartedu.cn/".into(),
            classroom_url: "https://basic.smartedu.cn/syncClassroom".into(),
            material_url: "https://basic.smartedu.cn/tchMaterial".into(),
            elec_edu_url: "https://basic.smartedu.cn/elecEdu".into(),
            note: "单元/课时目录优先来自 dzkbw.org book 页 pre 目录（含分篇古诗）；失败时回退 dzkbw.com。不含教材正文。".into(),
            edition_label: edition_label.into(),
            subject_label: subject_label.into(),
            smartedu_path_hint: format!(
                "小学 → {grade}年级 → {subject_label} → {edition_label} → {sem_label}"
            ),
            catalog_site: ORG_SITE.into(),
            catalog_ref: catalog_url.into(),
            entry_count,
            unit_count: units.len(),
        },
        units,
        exam_hints: vec![
            "命题必须落在本册单元/课时范围内，禁止超纲".into(),
            "单元测试只考所选单元课时".into(),
            "期中覆盖前半册单元，期末覆盖全册".into(),
            format!("目录来源（电子课本网）：{catalog_url}"),
            "风格可参考智慧教育平台同步课堂 https://basic.smartedu.cn/syncClassroom".into(),
        ],
    }
}

fn parse_grade_sem_from_title(title: &str) -> Option<(u8, String)> {
    let grade_map = [
        ("一年级", 1u8),
        ("二年级", 2),
        ("三年级", 3),
        ("四年级", 4),
        ("五年级", 5),
        ("六年级", 6),
    ];
    let mut grade = None;
    for (k, g) in grade_map {
        if title.contains(k) {
            grade = Some(g);
            break;
        }
    }
    let grade = grade?;
    let sem = if title.contains("下册") {
        "xia"
    } else if title.contains("上册") || title.contains("全一册") {
        "shang"
    } else {
        return None;
    };
    Some((grade, sem.into()))
}

/// 书名打分：学科 + 版本匹配，优先新版年份
fn score_org_book_title(
    title: &str,
    subject_keys: &[&str],
    edition_keys: &[&str],
    exclude_keys: &[&str],
) -> i32 {
    if title.contains("高一")
        || title.contains("高二")
        || title.contains("高三")
        || title.contains("七年级")
        || title.contains("八年级")
        || title.contains("九年级")
        || title.contains("必修")
        || title.contains("选择性")
        || title.contains("教师")
        || title.contains("用书")
    {
        return -1000;
    }
    if exclude_keys.iter().any(|k| !k.is_empty() && title.contains(k)) {
        return -1000;
    }
    if !subject_keys.iter().any(|k| title.contains(k)) {
        return -1000;
    }
    // 版本关键字：非空时至少命中一个
    if !edition_keys.is_empty() && !edition_keys.iter().any(|k| title.contains(k)) {
        return -1000;
    }
    let mut score = 10;
    if title.contains("2026") {
        score += 120;
    } else if title.contains("2025") {
        score += 90;
    } else if title.contains("2024") {
        score += 70;
    }
    if title.contains("秋") {
        score += 8;
    }
    if title.contains("春") {
        score += 4;
    }
    if title.contains("部编") || title.contains("统编") {
        score += 25;
    }
    // 版本命中加分
    for k in edition_keys {
        if title.contains(k) {
            score += 20;
        }
    }
    score
}

/// 从 dzkbw.org 索引页收集 book/xxxx.html
fn find_org_book_links(
    index_urls: &[&str],
    subject_keys: &[&str],
    edition_keys: &[&str],
    exclude_keys: &[&str],
) -> Result<Vec<(String, String, u8, String, i32)>, String> {
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for index_url in index_urls {
        let html = match fetch_html(index_url) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("索引失败 {index_url}: {e}");
                continue;
            }
        };
        for (href, title) in extract_links(&html) {
            if !href.contains("/book/") || !href.contains(".html") {
                continue;
            }
            let full = join_url(index_url, &href);
            // 只要 book/数字.html
            if !Regex::new(r"/book/\d+\.html").unwrap().is_match(&full) {
                continue;
            }
            let title = title.trim().to_string();
            if title.is_empty() {
                continue;
            }
            let score = score_org_book_title(&title, subject_keys, edition_keys, exclude_keys);
            if score < 0 {
                continue;
            }
            let Some((grade, sem)) = parse_grade_sem_from_title(&title) else {
                continue;
            };
            if !seen.insert(full.clone()) {
                continue;
            }
            results.push((full, title, grade, sem, score));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    if results.is_empty() {
        return Err("未在 dzkbw.org 找到匹配的小学教材 book 页".into());
    }
    Ok(results)
}

struct OrgScrapeSpec {
    subject: &'static str,
    edition: &'static str,
    edition_label: &'static str,
    subject_label: &'static str,
    subject_keys: &'static [&'static str],
    edition_keys: &'static [&'static str],
    exclude_keys: &'static [&'static str],
    /// 仅同步这些年级；空表示 1–6
    grades: &'static [u8],
    index_urls: &'static [&'static str],
    /// .com 回退索引（可空）
    fallback_com: Option<(&'static str, &'static str)>,
}

fn scrape_org_subject(
    spec: &OrgScrapeSpec,
    out_dir: &Path,
) -> (Vec<String>, Vec<String>) {
    let mut updated = Vec::new();
    let mut failed = Vec::new();

    let links = match find_org_book_links(
        spec.index_urls,
        spec.subject_keys,
        spec.edition_keys,
        spec.exclude_keys,
    ) {
        Ok(v) => v,
        Err(e) => {
            // org 失败则尝试 .com 回退
            if let Some((com_url, path_key)) = spec.fallback_com {
                return scrape_subject(
                    com_url,
                    spec.subject,
                    spec.edition,
                    path_key,
                    spec.edition_label,
                    spec.subject_label,
                    out_dir,
                );
            }
            failed.push(format!("{}/{} org 索引失败: {e}", spec.subject, spec.edition));
            return (updated, failed);
        }
    };

    // 年级过滤
    let links: Vec<_> = if spec.grades.is_empty() {
        links
    } else {
        links
            .into_iter()
            .filter(|(_, _, g, _, _)| spec.grades.contains(g))
            .collect()
    };
    if links.is_empty() {
        failed.push(format!(
            "{}/{}: 过滤后无可用书目",
            spec.subject, spec.edition
        ));
        if let Some((com_url, path_key)) = spec.fallback_com {
            return scrape_subject(
                com_url,
                spec.subject,
                spec.edition,
                path_key,
                spec.edition_label,
                spec.subject_label,
                out_dir,
            );
        }
        return (updated, failed);
    }

    let chosen = pick_best(links);
    if let Err(e) = fs::create_dir_all(out_dir) {
        failed.push(format!("创建目录失败 {out_dir:?}: {e}"));
        return (updated, failed);
    }

    let mut keys: Vec<_> = chosen.keys().cloned().collect();
    keys.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    for (g, s) in keys {
        if !spec.grades.is_empty() && !spec.grades.contains(&g) {
            continue;
        }
        let (url, title_hint) = chosen.get(&(g, s.clone())).unwrap().clone();
        let label = format!("{}/{}/grade-{}-{}", spec.subject, spec.edition, g, s);
        match fetch_html(&url) {
            Ok(html) => {
                let titles = extract_book_catalog(&html, &url);
                if titles.len() < 5 {
                    failed.push(format!(
                        "{label}: 目录条目过少（{}），跳过 {url}",
                        titles.len()
                    ));
                    continue;
                }
                let units = group_catalog(&titles);
                if units.is_empty() {
                    failed.push(format!("{label}: 未能识别单元结构"));
                    continue;
                }
                let book_title = guess_title(&title_hint, &html, g, &s);
                let pack = build_pack(
                    spec.subject,
                    spec.edition,
                    g,
                    &s,
                    &book_title,
                    units,
                    &url,
                    spec.edition_label,
                    spec.subject_label,
                    titles.len(),
                );
                let path = out_dir.join(format!("grade-{g}-{s}.json"));
                match serde_json::to_string_pretty(&pack) {
                    Ok(json) => {
                        if let Err(e) = fs::write(&path, json) {
                            failed.push(format!("{label} 写入失败: {e}"));
                        } else {
                            updated.push(format!(
                                "{label} ({}单元/{}条目) ← {url}",
                                pack.units.len(),
                                titles.len()
                            ));
                        }
                    }
                    Err(e) => failed.push(format!("{label} 序列化失败: {e}")),
                }
                std::thread::sleep(Duration::from_millis(350));
            }
            Err(e) => failed.push(format!("{label}: {e}")),
        }
    }

    // 一册都没成功且有 .com 回退
    if updated.is_empty() {
        if let Some((com_url, path_key)) = spec.fallback_com {
            let (u2, f2) = scrape_subject(
                com_url,
                spec.subject,
                spec.edition,
                path_key,
                spec.edition_label,
                spec.subject_label,
                out_dir,
            );
            updated.extend(u2);
            failed.extend(f2);
        }
    }
    (updated, failed)
}

fn guess_title(page_title_hint: &str, html: &str, grade: u8, semester: &str) -> String {
    let re = Regex::new(r"([一二三四五六]年级).{0,8}(上册|下册)").unwrap();
    if let Some(c) = re.captures(page_title_hint) {
        return c.get(0).unwrap().as_str().to_string();
    }
    if let Some(cap) = Regex::new(r"(?is)<title>(.*?)</title>")
        .unwrap()
        .captures(html)
    {
        let t = strip_tags(cap.get(1).map(|m| m.as_str()).unwrap_or(""));
        if let Some(c) = re.captures(&t) {
            return c.get(0).unwrap().as_str().to_string();
        }
    }
    format!(
        "{}年级{}册",
        grade,
        if semester == "shang" { "上" } else { "下" }
    )
}

fn scrape_subject(
    index_url: &str,
    subject: &str,
    edition: &str,
    subject_path_key: &str,
    edition_label: &str,
    subject_label: &str,
    out_dir: &Path,
) -> (Vec<String>, Vec<String>) {
    let mut updated = Vec::new();
    let mut failed = Vec::new();

    let links = match find_book_links(index_url, subject_path_key) {
        Ok(v) => v,
        Err(e) => {
            failed.push(format!("{subject}/{edition} 索引失败: {e}"));
            return (updated, failed);
        }
    };
    let chosen = pick_best(links);
    if let Err(e) = fs::create_dir_all(out_dir) {
        failed.push(format!("创建目录失败 {out_dir:?}: {e}"));
        return (updated, failed);
    }

    let mut keys: Vec<_> = chosen.keys().cloned().collect();
    keys.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    for (g, s) in keys {
        let (url, title_hint) = chosen.get(&(g, s.clone())).unwrap().clone();
        let label = format!("{subject}/{edition}/grade-{g}-{s}");
        match fetch_html(&url) {
            Ok(html) => {
                let titles = extract_book_catalog(&html, &url);
                let units = group_catalog(&titles);
                let book_title = guess_title(&title_hint, &html, g, &s);
                let pack = build_pack(
                    subject,
                    edition,
                    g,
                    &s,
                    &book_title,
                    units,
                    &url,
                    edition_label,
                    subject_label,
                    titles.len(),
                );
                let path = out_dir.join(format!("grade-{g}-{s}.json"));
                match serde_json::to_string_pretty(&pack) {
                    Ok(json) => {
                        if let Err(e) = fs::write(&path, json) {
                            failed.push(format!("{label} 写入失败: {e}"));
                        } else {
                            updated.push(format!(
                                "{label} ({}单元/{}条目)",
                                pack.units.len(),
                                titles.len()
                            ));
                        }
                    }
                    Err(e) => failed.push(format!("{label} 序列化失败: {e}")),
                }
                std::thread::sleep(Duration::from_millis(350));
            }
            Err(e) => failed.push(format!("{label}: {e}")),
        }
    }
    (updated, failed)
}

/// 可写课标目录：%APPDATA%/shijuan/shenqi/curriculum
pub fn curriculum_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("curriculum");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn update_curriculum_from_dzkbw(app: &AppHandle) -> Result<UpdateCurriculumResult, String> {
    let root = curriculum_data_dir(app)?;
    let mut updated = Vec::new();
    let mut failed = Vec::new();

    // 全部学科/版本：统一走 dzkbw.org book 页 pre 目录（优先新版年份）
    let specs: &[OrgScrapeSpec] = &[
        OrgScrapeSpec {
            subject: "chinese",
            edition: "renjiao",
            edition_label: "人教统编版",
            subject_label: "语文",
            subject_keys: &["语文"],
            edition_keys: &["人教", "部编", "统编"],
            exclude_keys: &["苏教", "北师大", "S版", "A版"],
            grades: &[],
            index_urls: &[
                "https://www.dzkbw.org/subject/rjb/yuwen.html",
                "https://www.dzkbw.org/textbooks/bbb.html",
                "https://www.dzkbw.org/textbooks/rjb.html",
            ],
            fallback_com: Some((
                "http://www.dzkbw.com/books/rjb/yuwen/",
                "/rjb/yuwen/",
            )),
        },
        OrgScrapeSpec {
            subject: "math",
            edition: "beishida",
            edition_label: "北师大版",
            subject_label: "数学",
            subject_keys: &["数学"],
            edition_keys: &["北师大"],
            exclude_keys: &["苏教", "人教版数学", "人教统编"],
            grades: &[],
            index_urls: &[
                "https://www.dzkbw.org/subject/bsd/shuxue.html",
                "https://www.dzkbw.org/textbooks/bsd.html",
                "https://www.dzkbw.org/textbooks/rjb.html",
            ],
            fallback_com: Some((
                "http://www.dzkbw.com/books/bsd/shuxue/",
                "/bsd/shuxue/",
            )),
        },
        OrgScrapeSpec {
            subject: "math",
            edition: "renjiao",
            edition_label: "人教版",
            subject_label: "数学",
            subject_keys: &["数学"],
            edition_keys: &["人教"],
            exclude_keys: &["北师大", "苏教", "语文", "英语"],
            grades: &[],
            index_urls: &[
                "https://www.dzkbw.org/subject/rjb/shuxue.html",
                "https://www.dzkbw.org/textbooks/rjb.html",
            ],
            fallback_com: Some((
                "http://www.dzkbw.com/books/rjb/shuxue/",
                "/rjb/shuxue/",
            )),
        },
        OrgScrapeSpec {
            subject: "math",
            edition: "sujiao",
            edition_label: "苏教版",
            subject_label: "数学",
            subject_keys: &["数学"],
            edition_keys: &["苏教"],
            exclude_keys: &["北师大", "人教"],
            grades: &[],
            index_urls: &[
                "https://www.dzkbw.org/subject/sjb/shuxue.html",
                "https://www.dzkbw.org/textbooks/sjb.html",
                "https://www.dzkbw.org/textbooks/rjb.html",
            ],
            fallback_com: Some((
                "http://www.dzkbw.com/books/sjb/shuxue/",
                "/sjb/shuxue/",
            )),
        },
        OrgScrapeSpec {
            subject: "english",
            edition: "renjiao",
            edition_label: "人教版",
            subject_label: "英语",
            subject_keys: &["英语"],
            edition_keys: &["人教", "PEP", "精通", "新起点"],
            exclude_keys: &["牛津", "外研", "苏教", "北师大"],
            grades: &[3, 4, 5, 6],
            index_urls: &[
                "https://www.dzkbw.org/subject/rjb/yingyu.html",
                "https://www.dzkbw.org/textbooks/rjb.html",
            ],
            fallback_com: Some((
                "http://www.dzkbw.com/books/rjb/yingyu/",
                "/rjb/yingyu/",
            )),
        },
    ];

    for spec in specs {
        let out_dir = root.join(spec.subject).join(spec.edition);
        let (u, f) = scrape_org_subject(spec, &out_dir);
        updated.extend(u);
        failed.extend(f);
    }

    // 同步后写 index，列出本机已有科目
    let mut subjects = Vec::new();
    for spec in specs {
        subjects.push(serde_json::json!({
            "subject": spec.subject,
            "edition": spec.edition,
            "label": format!("{}·{}", spec.subject_label, spec.edition_label),
        }));
    }
    let index = serde_json::json!({
        "catalogSite": ORG_SITE,
        "platform": "https://basic.smartedu.cn/",
        "updatedAt": chrono_like_now(),
        "primarySource": "https://www.dzkbw.org/book/*.html 页内 <pre> 目录",
        "fallbackSource": "http://www.dzkbw.com/ books bookmulu",
        "rebuild": "应用内「同步课标」",
        "example": "https://www.dzkbw.org/book/4833.html",
        "subjects": subjects,
        "note": "全部学科/版本统一从 dzkbw.org 择优新版目录同步到本机；失败册次回退旧站。",
    });
    let _ = fs::write(
        root.join("index.json"),
        serde_json::to_string_pretty(&index).unwrap_or_default(),
    );

    let ok = !updated.is_empty();
    let message = if ok && failed.is_empty() {
        format!(
            "课标全量同步成功（dzkbw.org），共 {} 册：语文人教、数学北师大/人教/苏教、英语人教",
            updated.len()
        )
    } else if ok {
        format!(
            "课标全量同步部分成功：{} 册完成，{} 项失败/跳过。全部按 dzkbw.org 优先。",
            updated.len(),
            failed.len()
        )
    } else {
        format!("课标同步失败：{}", failed.join("；"))
    };

    Ok(UpdateCurriculumResult {
        ok,
        message,
        updated,
        failed,
        data_dir: root.display().to_string(),
    })
}

fn chrono_like_now() -> String {
    // 避免额外依赖：简单本地时间戳
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}
