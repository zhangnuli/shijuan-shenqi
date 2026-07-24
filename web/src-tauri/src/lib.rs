mod ai;
mod appsj;
mod bank;
mod config;
mod contracts;
mod curriculum_diff;
mod ebook_site;
mod generate;
mod history;
mod knowledge;
mod lesson_plan;
mod print_util;
mod quality;
mod review;
mod scrape_dzkbw;
mod secret;
mod spec_table;
mod storage;
mod templates;
mod verify;

use bank::{
    add_favorite, clear_favorites, delete_bank_paper, delete_favorite, get_bank_paper,
    import_paper_and_items, list_bank_papers, list_favorites, pick_favorite_snippets,
    AddFavoriteRequest, BankPaper, FavoriteItem,
};
use appsj::{AppSjResource, AppSjSyncReport, AppSjSyncRequest};
use config::{
    clear_api_key as remove_config_api_key, load_config, load_config_for_frontend,
    provider_presets, save_config, AppConfig, ProviderPreset,
};
use generate::{
    generate_parallel_set, generate_variant_b, generate_with_ai, regenerate_item, GenerateRequest,
    RegenItemRequest,
};
use history::{
    add_history, clear_history, delete_history, get_history, list_history, HistoryEntry,
};
use curriculum_diff::{diff_curriculum_pack, CurriculumDiffReport};
use knowledge::{list_catalog, load_pack, CatalogItem};
use lesson_plan::{
    generate_lesson_plan, generate_unit_all_lessons, template_lesson_plan,
    template_unit_all_lessons, LessonPlanRequest,
};
use quality::{inspect_paper, QualityReport};
use review::{generate_redrill_paper, generate_review_outline, RedrillRequest, ReviewRequest};
use scrape_dzkbw::{update_curriculum_from_dzkbw, UpdateCurriculumResult};
use serde_json::Value;
use spec_table::{build_spec_table, SpecTable};
use templates::{
    apply_paper_template, delete_user_template, export_template_json, get_template,
    import_template, list_all_templates, save_template_from_paper, template_structure_line,
    PaperTemplate,
};
use tauri::AppHandle;
use verify::{verify_paper_math, VerifyReport};

#[tauri::command]
fn get_provider_presets() -> Vec<ProviderPreset> {
    provider_presets()
}

#[tauri::command]
fn get_config() -> AppConfig {
    load_config_for_frontend()
}

#[tauri::command]
fn set_config(cfg: AppConfig) -> Result<(), String> {
    save_config(&cfg)
}

#[tauri::command]
fn clear_api_key() -> Result<(), String> {
    remove_config_api_key()
}

#[tauri::command]
fn get_catalog(app: AppHandle) -> Result<Vec<CatalogItem>, String> {
    list_catalog(&app)
}

/// 从电子课本网一键更新课标（写入用户目录，优先于内置包）
#[tauri::command]
async fn update_curriculum(app: AppHandle) -> Result<UpdateCurriculumResult, String> {
    tauri::async_runtime::spawn_blocking(move || update_curriculum_from_dzkbw(&app))
        .await
        .map_err(|e| format!("任务失败: {e}"))?
}

/// 同步小学试卷网公开 HTML 命题素材，不自动下载第三方网盘文件。
#[tauri::command]
async fn sync_appsj_resources(
    app: AppHandle,
    request: AppSjSyncRequest,
) -> Result<AppSjSyncReport, String> {
    tauri::async_runtime::spawn_blocking(move || appsj::sync_resources(&app, request))
        .await
        .map_err(|error| format!("任务失败: {error}"))?
}

#[tauri::command]
fn list_appsj_resources(app: AppHandle) -> Result<Vec<AppSjResource>, String> {
    appsj::list_resources(&app)
}

#[tauri::command]
fn get_curriculum_dir(app: AppHandle) -> Result<String, String> {
    scrape_dzkbw::curriculum_data_dir(&app).map(|p| p.display().to_string())
}

#[tauri::command]
fn cancel_generation() {
    ai::request_cancel();
}

#[tauri::command]
fn diff_curriculum(app: AppHandle, path: String) -> Result<CurriculumDiffReport, String> {
    diff_curriculum_pack(&app, &path)
}

/// 导出运行日志文本到指定路径（前端传入保存路径）
#[tauri::command]
fn export_runtime_log(app: AppHandle, target_path: String) -> Result<String, String> {
    use std::fs;
    use std::io::Write;
    use tauri::Manager;

    let mut chunks: Vec<String> = Vec::new();
    chunks.push(format!("# 试卷神器运行日志导出\n时间戳: {}\n", {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }));

    // 应用数据目录
    if let Ok(dir) = app.path().app_data_dir() {
        chunks.push(format!("app_data_dir: {}\n", dir.display()));
        // 常见 log 文件
        for name in ["logs", "log"] {
            let p = dir.join(name);
            if p.is_dir() {
                if let Ok(rd) = fs::read_dir(&p) {
                    for e in rd.flatten() {
                        let fp = e.path();
                        if fp.is_file() {
                            if let Ok(s) = fs::read_to_string(&fp) {
                                chunks.push(format!(
                                    "\n--- {} ---\n{}\n",
                                    fp.display(),
                                    if s.len() > 200_000 {
                                        format!("{}…(截断)", &s[..200_000])
                                    } else {
                                        s
                                    }
                                ));
                            }
                        }
                    }
                }
            }
        }
        // 课标 index
        let idx = dir.join("curriculum").join("index.json");
        if idx.exists() {
            if let Ok(s) = fs::read_to_string(&idx) {
                chunks.push(format!("\n--- curriculum/index.json ---\n{s}\n"));
            }
        }
    }

    // 配置摘要（不含 key）
    let cfg = load_config_for_frontend();
    chunks.push(format!(
        "\n--- config.summary ---\nprovider={}\napiBase={}\nmodel={}\napiKeyConfigured={}\n",
        cfg.provider_id, cfg.api_base, cfg.model, cfg.api_key_configured
    ));

    let body = chunks.join("");
    let mut f = fs::File::create(&target_path).map_err(|e| format!("无法写入日志: {e}"))?;
    f.write_all(body.as_bytes())
        .map_err(|e| format!("写入日志失败: {e}"))?;
    Ok(target_path)
}

#[tauri::command]
fn get_app_info(app: AppHandle) -> Result<Value, String> {
    use tauri::Manager;
    let data = app
        .path()
        .app_data_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    Ok(serde_json::json!({
        "name": "试卷神器",
        "version": env!("CARGO_PKG_VERSION"),
        "appDataDir": data,
        "updateNote": "自动更新源：https://github.com/zhangnuli/shijuan-shenqi/releases 。打 tag 由 CI 上传 latest.json 与签名包；私钥用 .tauri/shijuan.key（仅本机/Actions Secret，勿提交）。",
        "offlineNote": "可在接口设置中选择「本地 Ollama」，API Base 填 http://127.0.0.1:11434/v1，模型填本地已拉取名称。"
    }))
}

#[tauri::command]
async fn generate_paper(app: AppHandle, mut req: GenerateRequest) -> Result<Value, String> {
    ai::clear_cancel_flag();
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = load_config();
        let pack = load_pack(&app, &req.knowledge_path)?;
        // 校本收藏掺入
        if req.use_school_bank && req.school_bank_snippets.is_empty() {
            if let Ok(snips) = pick_favorite_snippets(&app, &req.subject, req.grade, 8) {
                req.school_bank_snippets = snips;
            }
        }
        if req.mix_bank && req.public_bank_snippets.is_empty() {
            let unit_name = req
                .unit_id
                .as_ref()
                .and_then(|id| {
                    pack.units
                        .iter()
                        .find(|unit| &unit.id == id || unit.name.contains(id))
                })
                .map(|unit| unit.name.as_str())
                .unwrap_or("");
            if let Ok(snippets) = appsj::pick_resource_snippets(
                &app,
                &req.subject,
                &req.edition,
                req.grade,
                &req.semester,
                &req.exam_type,
                unit_name,
                6,
            ) {
                req.public_bank_snippets = snippets;
            }
        }
        // 模板市集：注入结构
        if let Some(tid) = req.template_id.clone().filter(|s| !s.is_empty()) {
            if let Ok(tpl) = get_template(&app, &tid) {
                if req.structure_override.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                    req.structure_override = Some(template_structure_line(&tpl));
                }
                if req.template_hints.is_empty() {
                    req.template_hints = tpl.prompt_hints.clone();
                }
                if req.total_score == 0 && tpl.total_score > 0 {
                    req.total_score = tpl.total_score;
                }
                if !tpl.exam_type.is_empty()
                    && tpl.kind == "paperTemplate"
                    && matches!(
                        tpl.exam_type.as_str(),
                        "unit" | "midterm" | "final" | "oral" | "lesson" | "homework" | "redrill"
                    )
                {
                    // 不强制覆盖用户已选手动卷型，仅当与模板一致时带时长建议
                }
            }
        }
        generate_with_ai(&cfg, &req, &pack)
    })
    .await
    .map_err(|e| format!("任务失败: {e}"))?
}

#[tauri::command]
fn list_templates(app: AppHandle) -> Result<Vec<PaperTemplate>, String> {
    list_all_templates(&app)
}

#[tauri::command]
fn get_market_template(app: AppHandle, id: String) -> Result<PaperTemplate, String> {
    get_template(&app, &id)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyTemplateReq {
    template_id: String,
    subject: String,
    edition: String,
    grade: u8,
    semester: String,
    #[serde(default)]
    unit_name: String,
    #[serde(default)]
    knowledge_path: String,
}

#[tauri::command]
fn apply_market_template(app: AppHandle, req: ApplyTemplateReq) -> Result<Value, String> {
    let tpl = get_template(&app, &req.template_id)?;
    let pack = if req.knowledge_path.is_empty() {
        None
    } else {
        Some(load_pack(&app, &req.knowledge_path)?)
    };
    apply_paper_template(
        &tpl,
        &req.subject,
        &req.edition,
        req.grade,
        &req.semester,
        &req.unit_name,
        pack.as_ref(),
    )
}

#[tauri::command]
fn save_paper_as_template(
    app: AppHandle,
    paper: Value,
    name: Option<String>,
) -> Result<PaperTemplate, String> {
    save_template_from_paper(&app, &paper, name)
}

#[tauri::command]
fn import_market_template(app: AppHandle, template: Value) -> Result<PaperTemplate, String> {
    import_template(&app, template)
}

#[tauri::command]
fn delete_market_template(app: AppHandle, id: String) -> Result<(), String> {
    delete_user_template(&app, &id)
}

#[tauri::command]
fn export_market_template(app: AppHandle, id: String) -> Result<Value, String> {
    export_template_json(&app, &id)
}

#[tauri::command]
async fn generate_template_paper(app: AppHandle, req: GenerateRequest) -> Result<Value, String> {
    ai::clear_cancel_flag();
    tauri::async_runtime::spawn_blocking(move || {
        let pack = load_pack(&app, &req.knowledge_path)?;
        let unit_name = req
            .unit_id
            .as_ref()
            .and_then(|id| {
                pack.units
                    .iter()
                    .find(|unit| &unit.id == id || unit.name.contains(id))
            })
            .map(|unit| unit.name.as_str())
            .unwrap_or("");
        let verified = if req.mix_bank && req.subject == "math" {
            appsj::pick_verified_math_expressions(
                &app,
                &req.edition,
                req.grade,
                &req.semester,
                &req.exam_type,
                unit_name,
                24,
            )
            .unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(build_template_paper(&req, &pack, &verified))
    })
    .await
    .map_err(|e| format!("任务失败: {e}"))?
}

#[tauri::command]
async fn regenerate_one_item(req: RegenItemRequest) -> Result<Value, String> {
    ai::clear_cancel_flag();
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = load_config();
        regenerate_item(&cfg, &req)
    })
    .await
    .map_err(|e| format!("任务失败: {e}"))?
}

#[tauri::command]
async fn generate_paper_b(paper: Value) -> Result<Value, String> {
    ai::clear_cancel_flag();
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = load_config();
        generate_variant_b(&cfg, &paper)
    })
    .await
    .map_err(|e| format!("任务失败: {e}"))?
}

/// 一键平行卷 A/B/C
#[tauri::command]
async fn generate_parallel_abc(paper: Value) -> Result<Value, String> {
    ai::clear_cancel_flag();
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = load_config();
        generate_parallel_set(&cfg, &paper)
    })
    .await
    .map_err(|e| format!("任务失败: {e}"))?
}

#[tauri::command]
fn build_paper_spec_table(paper: Value) -> SpecTable {
    build_spec_table(&paper)
}

#[tauri::command]
fn bank_list_favorites(app: AppHandle) -> Result<Vec<FavoriteItem>, String> {
    list_favorites(&app)
}

#[tauri::command]
fn bank_list_papers(app: AppHandle) -> Result<Vec<BankPaper>, String> {
    list_bank_papers(&app)
}

#[tauri::command]
fn bank_add_favorite(app: AppHandle, req: AddFavoriteRequest) -> Result<FavoriteItem, String> {
    add_favorite(&app, req)
}

#[tauri::command]
fn bank_delete_favorite(app: AppHandle, id: String) -> Result<(), String> {
    delete_favorite(&app, &id)
}

#[tauri::command]
fn bank_clear_favorites(app: AppHandle) -> Result<(), String> {
    clear_favorites(&app)
}

#[tauri::command]
fn bank_import_paper(
    app: AppHandle,
    paper: Value,
    also_items: bool,
) -> Result<Value, String> {
    let (entry, added) = import_paper_and_items(&app, paper, also_items)?;
    Ok(serde_json::json!({
        "paper": entry,
        "itemsAdded": added
    }))
}

#[tauri::command]
fn bank_delete_paper(app: AppHandle, id: String) -> Result<(), String> {
    delete_bank_paper(&app, &id)
}

#[tauri::command]
fn bank_get_paper(app: AppHandle, id: String) -> Result<BankPaper, String> {
    get_bank_paper(&app, &id)
}

#[tauri::command]
async fn generate_review(req: ReviewRequest) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = load_config();
        generate_review_outline(&cfg, &req)
    })
    .await
    .map_err(|e| format!("任务失败: {e}"))?
}

#[tauri::command]
async fn generate_redrill(req: RedrillRequest) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = load_config();
        generate_redrill_paper(&cfg, &req)
    })
    .await
    .map_err(|e| format!("任务失败: {e}"))?
}

#[tauri::command]
fn inspect_exam_paper(paper: Value) -> QualityReport {
    inspect_paper(&paper)
}

#[tauri::command]
fn bank_pick_snippets(
    app: AppHandle,
    subject: String,
    grade: u8,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    pick_favorite_snippets(&app, &subject, grade, limit.unwrap_or(8))
}

#[tauri::command]
fn history_list(app: AppHandle) -> Result<Vec<HistoryEntry>, String> {
    list_history(&app)
}

#[tauri::command]
fn history_add(
    app: AppHandle,
    paper: Value,
    form_snapshot: Option<Value>,
) -> Result<HistoryEntry, String> {
    let max = load_config().history_max as usize;
    add_history(&app, paper, form_snapshot, max)
}

#[tauri::command]
fn history_get(app: AppHandle, id: String) -> Result<HistoryEntry, String> {
    get_history(&app, &id)
}

#[tauri::command]
fn history_delete(app: AppHandle, id: String) -> Result<(), String> {
    delete_history(&app, &id)
}

#[tauri::command]
fn history_clear(app: AppHandle) -> Result<(), String> {
    clear_history(&app)
}

#[tauri::command]
async fn generate_lesson(
    app: AppHandle,
    req: LessonPlanRequest,
) -> Result<Value, String> {
    ai::clear_cancel_flag();
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = load_config();
        let pack = load_pack(&app, &req.knowledge_path)?;
        generate_lesson_plan(&cfg, &req, &pack)
    })
    .await
    .map_err(|e| format!("任务失败: {e}"))?
}

#[tauri::command]
fn generate_lesson_template(
    app: AppHandle,
    req: LessonPlanRequest,
) -> Result<Value, String> {
    let pack = load_pack(&app, &req.knowledge_path)?;
    Ok(template_lesson_plan(&req, &pack))
}

#[tauri::command]
async fn generate_unit_lessons(
    app: AppHandle,
    req: LessonPlanRequest,
) -> Result<Value, String> {
    ai::clear_cancel_flag();
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = load_config();
        let pack = load_pack(&app, &req.knowledge_path)?;
        generate_unit_all_lessons(&cfg, &req, &pack)
    })
    .await
    .map_err(|e| format!("任务失败: {e}"))?
}

#[tauri::command]
fn generate_unit_lessons_template(
    app: AppHandle,
    req: LessonPlanRequest,
) -> Result<Value, String> {
    let pack = load_pack(&app, &req.knowledge_path)?;
    Ok(template_unit_all_lessons(&req, &pack))
}

#[tauri::command]
fn verify_math_paper(paper: Value) -> VerifyReport {
    verify_paper_math(&paper)
}

/// 将 HTML 转为无系统页眉页脚的 PDF（避免页脚出现 tauri.localhost），返回 PDF 绝对路径。
#[tauri::command]
async fn print_html_document(html: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = print_util::html_to_pdf(&html)?;
        Ok(path.display().to_string())
    })
    .await
    .map_err(|e| format!("打印任务失败: {e}"))?
}

/// 解析自有电子书阅读页链接（resId/bookId/contributeId）
#[tauri::command]
fn ebook_parse_url(url: String) -> Result<ebook_site::EbookLinkParts, String> {
    ebook_site::parse_ebook_url(&url)
}

/// 拉取电子书目录
#[tauri::command]
async fn ebook_catalog(base_url: String, res_id: String) -> Result<ebook_site::EbookCatalog, String> {
    tauri::async_runtime::spawn_blocking(move || ebook_site::fetch_catalog(&base_url, &res_id))
        .await
        .map_err(|e| format!("任务失败: {e}"))?
}

/// 拉取某单元页图列表（用于打印正文页）
/// 参数与前端 camelCase 对齐：baseUrl / resId / bookId / contributeId / maxPages
#[tauri::command]
async fn ebook_unit_pages(
    base_url: String,
    res_id: String,
    book_id: String,
    contribute_id: String,
    max_pages: Option<u32>,
) -> Result<ebook_site::EbookUnitPages, String> {
    let max_pages = max_pages.unwrap_or(30);
    tauri::async_runtime::spawn_blocking(move || {
        ebook_site::fetch_unit_pages(&base_url, &res_id, &book_id, &contribute_id, max_pages)
    })
    .await
    .map_err(|e| format!("任务失败: {e}"))?
}

fn offline_math_expressions(
    grade: u8,
    verified: &[(String, String, String)],
) -> Vec<(String, String, String)> {
    use std::collections::HashSet;

    let fallback: &[&str] = match grade {
        1..=2 => &[
            "8+7", "16-9", "6+13", "20-8", "5×4", "18÷3", "7×3", "24÷6",
            "12+9-5", "30-8+6", "4×6+3", "36÷6+8", "45-17", "28+35",
            "9×5", "42÷7", "63-26", "37+18", "3×8", "32÷4", "50-23+7",
            "18+12÷3", "6×7-9", "48÷8+15", "25+16-8", "54-19", "8×6",
            "49÷7", "70-28", "34+29",
        ],
        3..=4 => &[
            "432÷4", "208×5", "846÷6", "720÷3÷2", "450-120×3", "8×(156-89)",
            "315÷5×4", "36+18÷6", "(45-9)÷6", "100-6×9", "125×8", "960÷6",
            "304×3", "728÷7", "240+360÷6", "18×5+120", "900-425", "376+589",
            "45×12", "864÷8", "600-24×9", "(320+160)÷6", "25×16", "1000-368",
            "144÷9+36", "72×5", "630÷7", "48×6-90", "250×4", "936÷9",
        ],
        _ => &[
            "125+375", "960÷24", "48×25", "7.5+2.8", "12.6-4.75", "3.2×5",
            "24÷0.6", "1.2×(3.5+1.5)", "36÷0.4-15", "(24+36)÷5", "2.5×16",
            "18.4÷4", "125×32", "4500÷18", "6.25+3.75", "14.8-6.9", "0.75×8",
            "7.2÷0.9", "45×12-180", "(720-240)÷8", "3.6×25", "81÷0.3",
            "1250-486", "768+945", "144÷12+38", "32×15", "630÷21", "4.8×6-9.8",
            "250×16", "936÷18",
        ],
    };
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for (expression, answer, source) in verified.iter().cloned() {
        if seen.insert(expression.clone()) {
            items.push((expression, answer, source));
        }
    }
    for expression in fallback {
        if items.len() >= 30 || !seen.insert((*expression).to_string()) {
            continue;
        }
        if let Ok(answer) = verify::evaluate_simple_expression(expression) {
            items.push(((*expression).to_string(), answer, "内置可验算题库".into()));
        }
    }
    items
}

fn shifted_math_answer(answer: &str, delta: f64) -> String {
    let value = answer.parse::<f64>().unwrap_or(0.0) + delta;
    if (value - value.round()).abs() < 1e-9 {
        format!("{:.0}", value)
    } else {
        let mut text = format!("{value:.6}");
        while text.ends_with('0') {
            text.pop();
        }
        text.trim_end_matches('.').to_string()
    }
}

fn offline_math_problems(grade: u8) -> Vec<Value> {
    if grade <= 2 {
        vec![
            serde_json::json!({"id":"5-1","stem":"1. 文具盒里原有18支铅笔，用去7支，又放入5支，现在有多少支？","options":[],"answer":"16支","analysis":"18-7+5=16（支）","score":10,"knowledgePoints":["加减混合"]}),
            serde_json::json!({"id":"5-2","stem":"2. 4个小组，每组有6名同学，一共有多少名同学？","options":[],"answer":"24名","analysis":"4×6=24（名）","score":10,"knowledgePoints":["乘法应用"]}),
            serde_json::json!({"id":"5-3","stem":"3. 把20本练习本平均分给5名同学，每名同学分到多少本？","options":[],"answer":"4本","analysis":"20÷5=4（本）","score":10,"knowledgePoints":["平均分"]}),
        ]
    } else if grade <= 4 {
        vec![
            serde_json::json!({"id":"5-1","stem":"1. 三年级有78名同学参加活动，平均分成6组，每组有多少名同学？","options":[],"answer":"13名","analysis":"78÷6=13（名）","score":10,"knowledgePoints":["除法应用"]}),
            serde_json::json!({"id":"5-2","stem":"2. 同学们3天折了306只纸鹤。照这样计算，7天可以折多少只？","options":[],"answer":"714只","analysis":"306÷3×7=714（只）","score":10,"knowledgePoints":["归一问题"]}),
            serde_json::json!({"id":"5-3","stem":"3. 用3个边长4厘米的正方形拼成一个长方形，拼成长方形的周长是多少厘米？","options":[],"answer":"32厘米","analysis":"长为12厘米、宽为4厘米，周长=(12+4)×2=32（厘米）","score":10,"knowledgePoints":["长方形周长"]}),
        ]
    } else {
        vec![
            serde_json::json!({"id":"5-1","stem":"1. 一批图书有480本，第一天借出总数的25%，第二天借出90本，还剩多少本？","options":[],"answer":"270本","analysis":"480-480×25%-90=270（本）","score":10,"knowledgePoints":["百分数应用"]}),
            serde_json::json!({"id":"5-2","stem":"2. 一个长方体长8厘米、宽5厘米、高4厘米，它的体积是多少立方厘米？","options":[],"answer":"160立方厘米","analysis":"8×5×4=160（立方厘米）","score":10,"knowledgePoints":["长方体体积"]}),
            serde_json::json!({"id":"5-3","stem":"3. 一辆汽车3小时行驶225千米。按这个速度，行驶7小时可行多少千米？","options":[],"answer":"525千米","analysis":"225÷3×7=525（千米）","score":10,"knowledgePoints":["归一问题"]}),
        ]
    }
}

fn build_template_paper(
    req: &GenerateRequest,
    pack: &knowledge::KnowledgePack,
    verified_math: &[(String, String, String)],
) -> Value {
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
    let exam = generate::exam_type_label(&req.exam_type);
    let unit_name = req
        .unit_id
        .as_ref()
        .and_then(|id| pack.units.iter().find(|u| &u.id == id || u.name.contains(id)))
        .map(|u| u.name.clone())
        .unwrap_or_default();

    let title = if req.exam_type == "unit" && !unit_name.is_empty() {
        format!(
            "小学{}{}{}年级{}{}·{}",
            edition_cn, subject_cn, req.grade, sem, exam, unit_name
        )
    } else {
        format!(
            "小学{}{}{}年级{}{}",
            edition_cn, subject_cn, req.grade, sem, exam
        )
    };

    let points: Vec<String> = match req.exam_type.as_str() {
        "unit" => pack
            .units
            .iter()
            .find(|u| req.unit_id.as_ref().map(|id| &u.id == id || u.name.contains(id)).unwrap_or(false))
            .or_else(|| pack.units.first())
            .map(|u| u.points.clone())
            .unwrap_or_default(),
        "midterm" => pack
            .units
            .iter()
            .take((pack.units.len() + 1) / 2)
            .flat_map(|u| u.points.clone())
            .collect(),
        _ => pack.units.iter().flat_map(|u| u.points.clone()).collect(),
    };

    let sample_points: Vec<String> = points.into_iter().take(8).collect();

    if req.subject == "math" {
        let expressions = offline_math_expressions(req.grade, verified_math);
        let fill_items: Vec<Value> = (0..5)
            .map(|index| {
                let (expression, answer, source) = &expressions[index];
                serde_json::json!({
                    "id": format!("1-{}", index + 1),
                    "stem": format!("{}. {}＝（　　）", index + 1, expression),
                    "options": [],
                    "answer": answer,
                    "analysis": format!("程序验算：{}={}; 来源：{}", expression, answer, source),
                    "score": 4,
                    "knowledgePoints": ["四则运算"]
                })
            })
            .collect();
        let judge_items: Vec<Value> = (0..5)
            .map(|index| {
                let (expression, answer, source) = &expressions[index + 5];
                let is_correct = index % 2 == 0;
                let shown = if is_correct {
                    answer.clone()
                } else {
                    shifted_math_answer(answer, 1.0)
                };
                serde_json::json!({
                    "id": format!("2-{}", index + 1),
                    "stem": format!("{}. {}＝{}。（　　）", index + 1, expression, shown),
                    "options": [],
                    "answer": if is_correct { "√" } else { "×" },
                    "analysis": format!("程序验算结果为 {}; 来源：{}", answer, source),
                    "score": 2,
                    "knowledgePoints": ["计算判断"]
                })
            })
            .collect();
        let choice_items: Vec<Value> = (0..5)
            .map(|index| {
                let (expression, answer, source) = &expressions[index + 10];
                let correct_index = index % 4;
                let distractors = [
                    shifted_math_answer(answer, -1.0),
                    shifted_math_answer(answer, 1.0),
                    shifted_math_answer(answer, 10.0),
                ];
                let mut values = Vec::new();
                let mut distractor_index = 0;
                for option_index in 0..4 {
                    if option_index == correct_index {
                        values.push(answer.clone());
                    } else {
                        values.push(distractors[distractor_index].clone());
                        distractor_index += 1;
                    }
                }
                let labels = ["A", "B", "C", "D"];
                let options = values
                    .iter()
                    .enumerate()
                    .map(|(option_index, value)| format!("{}. {}", labels[option_index], value))
                    .collect::<Vec<_>>();
                serde_json::json!({
                    "id": format!("3-{}", index + 1),
                    "stem": format!("{}. {} 的得数是（　　）。", index + 1, expression),
                    "options": options,
                    "answer": labels[correct_index],
                    "analysis": format!("{}={}; 来源：{}", expression, answer, source),
                    "score": 2,
                    "knowledgePoints": ["四则运算"]
                })
            })
            .collect();
        let calc_slice = &expressions[15..25];
        let calc_stem = calc_slice
            .chunks(4)
            .map(|row| {
                row.iter()
                    .map(|(expression, _, _)| format!("{expression}＝　　　"))
                    .collect::<Vec<_>>()
                    .join("    ")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let calc_answers = calc_slice
            .iter()
            .map(|(_, answer, _)| answer.clone())
            .collect::<Vec<_>>()
            .join("；");
        let public_source_count = verified_math.len().min(24);
        serde_json::json!({
            "meta": {
                "edition": edition_cn,
                "subject": subject_cn,
                "grade": req.grade,
                "semester": sem,
                "examType": exam,
                "title": title,
                "totalScore": req.total_score,
                "durationMin": req.duration_min,
                "source": if public_source_count > 0 {
                    format!("本地可验算题库 + 公开网页素材（{} 道算式）", public_source_count)
                } else {
                    "本地可验算题库".to_string()
                }
            },
            "sections": [
                {
                    "type": "fill",
                    "title": "一、填空题（每空2分，共20分）",
                    "score": 20,
                    "items": fill_items
                },
                {
                    "type": "judge",
                    "title": "二、判断题（每题2分，共10分）",
                    "score": 10,
                    "items": judge_items
                },
                {
                    "type": "choice",
                    "title": "三、选择题（每题2分，共10分）",
                    "score": 10,
                    "items": choice_items
                },
                {
                    "type": "calc",
                    "title": "四、计算题（共30分）",
                    "score": 30,
                    "items": [{
                        "id":"4-1",
                        "stem": format!("1. 计算下面各题。\n{}", calc_stem),
                        "options":[],
                        "answer": calc_answers,
                        "analysis":"所有算式均经本地表达式求值器验算",
                        "score":30,
                        "knowledgePoints":["口算", "混合运算"]
                    }]
                },
                {
                    "type": "problem",
                    "title": "五、解决问题（共30分）",
                    "score": 30,
                    "items": offline_math_problems(req.grade)
                }
            ]
        })
    } else {
        serde_json::json!({
            "meta": {
                "edition": edition_cn,
                "subject": subject_cn,
                "grade": req.grade,
                "semester": sem,
                "examType": exam,
                "title": title,
                "totalScore": req.total_score,
                "durationMin": req.duration_min,
                "source": "template"
            },
            "sections": [
                {
                    "type": "pinyin",
                    "title": "一、积累与运用（30分）",
                    "score": 30,
                    "items": [
                        {"id":"1-1","stem":"1. 看拼音写词语（10分）\n（根据本单元字词默写，模板占位）","options":[],"answer":"略","analysis":"","score":10,"knowledgePoints": sample_points.get(0).cloned().into_iter().collect::<Vec<_>>()},
                        {"id":"1-2","stem":"2. 选字填空 / 近反义词（10分）","options":[],"answer":"略","analysis":"","score":10,"knowledgePoints":[]},
                        {"id":"1-3","stem":"3. 按课文内容填空（10分）","options":[],"answer":"略","analysis":"","score":10,"knowledgePoints":[]}
                    ]
                },
                {
                    "type": "reading",
                    "title": "二、阅读理解（45分）",
                    "score": 45,
                    "items": [
                        {"id":"2-1","stem":"1. 课内阅读（20分）\n阅读本单元课文选段，完成练习。（模板：请用 AI 生成完整选段与题目）","options":[],"answer":"略","analysis":"","score":20,"knowledgePoints":[]},
                        {"id":"2-2","stem":"2. 课外阅读（25分）\n阅读短文，完成练习。","options":[],"answer":"略","analysis":"","score":25,"knowledgePoints":[]}
                    ]
                },
                {
                    "type": "writing",
                    "title": "三、习作（25分）",
                    "score": 25,
                    "items": [
                        {"id":"3-1","stem": format!("请以本册学习主题写一篇习作。要求：中心明确，语句通顺，不少于{}字。\n（模板题目，建议 AI 生成贴合单元的题目）", if req.grade <= 2 { 200 } else if req.grade <= 4 { 350 } else { 400 }),"options":[],"answer":"略","analysis":"","score":25,"knowledgePoints":["习作"]}
                    ]
                }
            ]
        })
    }
}

#[cfg(test)]
mod offline_bank_tests {
    use super::*;

    #[test]
    fn offline_math_bank_has_enough_verified_expressions() {
        for grade in [1, 3, 5] {
            let items = offline_math_expressions(grade, &[]);
            assert!(items.len() >= 25, "grade {grade} only has {}", items.len());
            for (expression, answer, _) in items.iter().take(25) {
                assert_eq!(
                    verify::evaluate_simple_expression(expression).unwrap(),
                    *answer
                );
            }
        }
    }

    #[test]
    fn crawled_verified_expression_is_preferred() {
        let crawled = vec![("432÷4".into(), "108".into(), "公开来源".into())];
        let items = offline_math_expressions(3, &crawled);
        assert_eq!(items[0], crawled[0]);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_provider_presets,
            get_config,
            set_config,
            clear_api_key,
            get_catalog,
            update_curriculum,
            sync_appsj_resources,
            list_appsj_resources,
            get_curriculum_dir,
            cancel_generation,
            diff_curriculum,
            export_runtime_log,
            get_app_info,
            generate_paper,
            generate_template_paper,
            list_templates,
            get_market_template,
            apply_market_template,
            save_paper_as_template,
            import_market_template,
            delete_market_template,
            export_market_template,
            regenerate_one_item,
            generate_paper_b,
            generate_parallel_abc,
            build_paper_spec_table,
            bank_list_favorites,
            bank_list_papers,
            bank_add_favorite,
            bank_delete_favorite,
            bank_clear_favorites,
            bank_import_paper,
            bank_delete_paper,
            bank_get_paper,
            generate_review,
            generate_redrill,
            inspect_exam_paper,
            bank_pick_snippets,
            history_list,
            history_add,
            history_get,
            history_delete,
            history_clear,
            generate_lesson,
            generate_lesson_template,
            generate_unit_lessons,
            generate_unit_lessons_template,
            verify_math_paper,
            print_html_document,
            ebook_parse_url,
            ebook_catalog,
            ebook_unit_pages,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
