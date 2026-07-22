//! 对接自有资源站（如 100875）电子书接口：
//! - POST {base}/ebook/catalogList  resId
//! - POST {base}/ebook/bookPictureList  bookId, contributeId
//! - POST {base}/urlRel/generateUrl  inputUrl（相对路径页图）
//!
//! 正文以分页 JPG 形式提供，按「单元 bookId → 起始页 → 下一单元起始页」截取页图。

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EbookLinkParts {
    pub base_url: String,
    pub res_id: String,
    pub book_id: String,
    pub contribute_id: String,
    pub first_num: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EbookCatalogItem {
    pub book_id: String,
    pub cata_name: String,
    pub deep: String,
    pub parent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EbookCatalog {
    pub book_name: String,
    pub subject_name: String,
    pub subject_id: String,
    pub section_id: String,
    pub version_id: String,
    pub cover: String,
    pub items: Vec<EbookCatalogItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EbookUnitPages {
    pub book_name: String,
    pub subject_name: String,
    pub unit_name: String,
    pub book_id: String,
    pub start_page: u32,
    pub end_page: u32,
    pub total_book_pages: u32,
    pub pages: Vec<EbookPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EbookPage {
    pub index: u32,
    pub url: String,
}

fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(45))
        .user_agent("ShiJuanShenQi/0.1 (ebook-print; self-site)")
        .build()
        .map_err(|e| format!("HTTP 客户端失败: {e}"))
}

fn normalize_base(base: &str) -> String {
    let b = base.trim().trim_end_matches('/');
    if b.is_empty() {
        "https://www.100875.com.cn".into()
    } else {
        b.into()
    }
}

/// 解析 eBookAndTeacher.html?resId=&bookId=&firstNum=&contributeId=
pub fn parse_ebook_url(url: &str) -> Result<EbookLinkParts, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("链接为空".into());
    }
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("链接无法解析: {e}"))?;
    let base_url = format!(
        "{}://{}",
        parsed.scheme(),
        parsed.host_str().unwrap_or("www.100875.com.cn")
    );
    let mut res_id = String::new();
    let mut book_id = String::new();
    let mut contribute_id = String::new();
    let mut first_num = String::new();
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "resId" => res_id = v.into_owned(),
            "bookId" => book_id = v.into_owned(),
            "contributeId" => contribute_id = v.into_owned(),
            "firstNum" => first_num = v.into_owned(),
            _ => {}
        }
    }
    if res_id.is_empty() {
        return Err("链接中缺少 resId".into());
    }
    Ok(EbookLinkParts {
        base_url,
        res_id,
        book_id,
        contribute_id,
        first_num,
    })
}

fn post_form(base: &str, path: &str, form: &[(&str, &str)]) -> Result<Value, String> {
    let c = client()?;
    let url = format!("{}{}", normalize_base(base), path);
    let resp = c
        .post(&url)
        .form(form)
        .send()
        .map_err(|e| format!("请求失败 {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("接口 HTTP {}: {url}", resp.status()));
    }
    let v: Value = resp
        .json()
        .map_err(|e| format!("JSON 解析失败 {url}: {e}"))?;
    let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("");
    if status != "success" {
        let msg = v
            .get("message")
            .and_then(|x| x.as_str())
            .unwrap_or("未知错误");
        return Err(format!("接口返回失败: {msg}"));
    }
    Ok(v)
}

pub fn fetch_catalog(base: &str, res_id: &str) -> Result<EbookCatalog, String> {
    let v = post_form(base, "/ebook/catalogList", &[("resId", res_id)])?;
    let data0 = v
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| "目录 data 为空".to_string())?;

    let items = data0
        .get("catalogList")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .map(|it| EbookCatalogItem {
                    book_id: it
                        .get("bookId")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    cata_name: it
                        .get("cataName")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    deep: it
                        .get("deep")
                        .and_then(|x| x.as_str().map(|s| s.to_string()).or_else(|| {
                            x.as_i64().map(|n| n.to_string())
                        }))
                        .unwrap_or_default(),
                    parent_id: it
                        .get("parentId")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
                .filter(|it| !it.book_id.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(EbookCatalog {
        book_name: data0
            .get("bookName")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
        subject_name: data0
            .get("subjectName")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
        subject_id: data0
            .get("subjectId")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
        section_id: data0
            .get("sectionId")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
        version_id: data0
            .get("versionId")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
        cover: data0
            .get("picture")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
        items,
    })
}

struct PageAnchor {
    index: u32,
    total: u32,
    picture: String,
    folder_rel: String,
}

fn parse_page_num(s: &str) -> Option<u32> {
    s.trim().parse().ok()
}

fn folder_from_picture_url(picture: &str) -> Result<String, String> {
    // https://cdn.../dzkb/xxsx/3s/016.jpg?auth_Key=... → dzkb/xxsx/3s/
    let no_q = picture.split('?').next().unwrap_or(picture);
    let after_host = if let Some(i) = no_q.find("://") {
        let rest = &no_q[i + 3..];
        rest.find('/')
            .map(|j| rest[j + 1..].to_string())
            .unwrap_or_default()
    } else {
        no_q.trim_start_matches('/').to_string()
    };
    // strip NNN.jpg
    if after_host.len() >= 7 {
        let stem = &after_host[..after_host.len() - 7]; // remove "016.jpg"
        Ok(stem.to_string())
    } else {
        Err(format!("无法从图片 URL 解析目录: {picture}"))
    }
}

fn page_file_name(n: u32) -> String {
    format!("{n:03}.jpg")
}

fn fetch_page_anchor(base: &str, book_id: &str, contribute_id: &str) -> Result<PageAnchor, String> {
    let v = post_form(
        base,
        "/ebook/bookPictureList",
        &[("bookId", book_id), ("contributeId", contribute_id)],
    )?;
    let row = v
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| "bookPictureList data 为空".to_string())?;
    let picture = row
        .get("picture")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if picture.is_empty() || picture == "wu" {
        return Err("该单元暂无页图资源".into());
    }
    let index = row
        .get("indexNum")
        .and_then(|x| x.as_str())
        .and_then(parse_page_num)
        .or_else(|| row.get("indexNum").and_then(|x| x.as_u64()).map(|n| n as u32))
        .ok_or_else(|| "无法解析 indexNum".to_string())?;
    let total = row
        .get("totalNum")
        .and_then(|x| x.as_str())
        .and_then(parse_page_num)
        .or_else(|| row.get("totalNum").and_then(|x| x.as_u64()).map(|n| n as u32))
        .unwrap_or(index);
    let folder_rel = folder_from_picture_url(&picture)?;
    Ok(PageAnchor {
        index,
        total,
        picture,
        folder_rel,
    })
}

fn generate_page_url(base: &str, folder_rel: &str, page: u32) -> Result<String, String> {
    let input = format!("{}{}", folder_rel, page_file_name(page));
    let v = post_form(base, "/urlRel/generateUrl", &[("inputUrl", &input)])?;
    v.get("data")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "generateUrl 未返回 URL".to_string())
}

/// 拉取某单元页图：从该 bookId 起始页到下一单元起始页（不含）
pub fn fetch_unit_pages(
    base: &str,
    res_id: &str,
    book_id: &str,
    contribute_id: &str,
    max_pages: u32,
) -> Result<EbookUnitPages, String> {
    if book_id.is_empty() {
        return Err("bookId 为空".into());
    }
    if contribute_id.is_empty() {
        return Err("contributeId 为空（链接里 contributeId 参数）".into());
    }
    let max_pages = max_pages.clamp(1, 80);

    let catalog = fetch_catalog(base, res_id)?;
    // 主目录一般 deep=5 为单元
    let units: Vec<&EbookCatalogItem> = catalog
        .items
        .iter()
        .filter(|it| it.deep == "5" || it.deep.is_empty())
        .collect();
    let units = if units.is_empty() {
        catalog.items.iter().collect::<Vec<_>>()
    } else {
        units
    };

    let pos = units
        .iter()
        .position(|u| u.book_id == book_id)
        .ok_or_else(|| "目录中未找到该 bookId，请确认链接与 resId 是否同一本书".to_string())?;

    let unit_name = units[pos].cata_name.clone();
    let start = fetch_page_anchor(base, book_id, contribute_id)?;

    let end_exclusive = if pos + 1 < units.len() {
        match fetch_page_anchor(base, &units[pos + 1].book_id, contribute_id) {
            Ok(next) if next.index > start.index => next.index,
            _ => (start.total + 1).max(start.index + 1),
        }
    } else {
        (start.total + 1).max(start.index + 1)
    };

    let mut end = end_exclusive.saturating_sub(1).max(start.index);
    // 上限保护
    if end + 1 - start.index > max_pages {
        end = start.index + max_pages - 1;
    }
    if end > start.total {
        end = start.total;
    }

    let mut pages = Vec::new();
    for i in start.index..=end {
        let url = if i == start.index {
            start.picture.clone()
        } else {
            generate_page_url(base, &start.folder_rel, i)?
        };
        pages.push(EbookPage { index: i, url });
        // 轻微节流，避免打爆签名接口
        if i > start.index {
            std::thread::sleep(Duration::from_millis(40));
        }
    }

    Ok(EbookUnitPages {
        book_name: catalog.book_name,
        subject_name: catalog.subject_name,
        unit_name,
        book_id: book_id.to_string(),
        start_page: start.index,
        end_page: end,
        total_book_pages: start.total,
        pages,
    })
}
