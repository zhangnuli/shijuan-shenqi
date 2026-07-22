mod ai;
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
        Ok(build_template_paper(&req, &pack))
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

fn build_template_paper(req: &GenerateRequest, pack: &knowledge::KnowledgePack) -> Value {
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
                    "type": "fill",
                    "title": "一、填空题（每空2分，共20分）",
                    "score": 20,
                    "items": (0..5).map(|i| {
                        let kp = sample_points.get(i).cloned().unwrap_or_else(|| "本单元知识点".into());
                        serde_json::json!({
                            "id": format!("1-{}", i+1),
                            "stem": format!("{}. 与「{}」相关：请填写正确答案。（　　）", i+1, kp),
                            "options": [],
                            "answer": "（见教师评阅）",
                            "analysis": "模板题，建议使用 AI 组卷获得完整题目",
                            "score": 4,
                            "knowledgePoints": [kp]
                        })
                    }).collect::<Vec<_>>()
                },
                {
                    "type": "judge",
                    "title": "二、判断题（每题2分，共10分）",
                    "score": 10,
                    "items": (0..5).map(|i| {
                        serde_json::json!({
                            "id": format!("2-{}", i+1),
                            "stem": format!("{}. 下列说法正确。（　　）", i+1),
                            "options": [],
                            "answer": "√",
                            "analysis": "模板占位",
                            "score": 2,
                            "knowledgePoints": []
                        })
                    }).collect::<Vec<_>>()
                },
                {
                    "type": "choice",
                    "title": "三、选择题（每题2分，共10分）",
                    "score": 10,
                    "items": (0..5).map(|i| {
                        serde_json::json!({
                            "id": format!("3-{}", i+1),
                            "stem": format!("{}. 请选择正确答案。（　　）", i+1),
                            "options": ["A. 选项一", "B. 选项二", "C. 选项三", "D. 选项四"],
                            "answer": "A",
                            "analysis": "模板占位",
                            "score": 2,
                            "knowledgePoints": []
                        })
                    }).collect::<Vec<_>>()
                },
                {
                    "type": "calc",
                    "title": "四、计算题（共30分）",
                    "score": 30,
                    "items": [
                        {"id":"4-1","stem":"1. 直接写出得数（每题2分，共12分）\n6×7＝　　  48÷6＝　　  25+36＝　　  90-47＝　　  0×9＝　　  1×8＝","options":[],"answer":"42；8；61；43；0；8","analysis":"","score":12,"knowledgePoints":["口算"]},
                        {"id":"4-2","stem":"2. 脱式计算（每题6分，共18分）\n（1）36+18÷6\n（2）(45-9)÷6\n（3）100-6×9","options":[],"answer":"39；6；46","analysis":"先乘除后加减","score":18,"knowledgePoints":["混合运算"]}
                    ]
                },
                {
                    "type": "problem",
                    "title": "五、解决问题（共30分）",
                    "score": 30,
                    "items": (0..3).map(|i| {
                        let kp = sample_points.get(i).cloned().unwrap_or_else(|| "应用".into());
                        serde_json::json!({
                            "id": format!("5-{}", i+1),
                            "stem": format!("{}.（10分）结合「{}」编一道应用题并解答。\n（模板卷：请改用 AI 组卷生成完整应用题）", i+1, kp),
                            "options": [],
                            "answer": "略",
                            "analysis": "",
                            "score": 10,
                            "knowledgePoints": [kp]
                        })
                    }).collect::<Vec<_>>()
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
