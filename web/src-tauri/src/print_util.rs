//! 打印工具：用本机 Edge/Chrome 无头导出 PDF（关闭系统页眉页脚），
//! 避免 WebView `window.print()` 在页脚留下 `tauri.localhost`。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn file_url(path: &Path) -> Result<String, String> {
    let abs = path
        .canonicalize()
        .map_err(|e| format!("无法解析路径 {}: {e}", path.display()))?;
    let s = abs.to_string_lossy();
    // Windows: \\?\C:\... → C:/...
    let trimmed = s.strip_prefix(r"\\?\").unwrap_or(&s);
    let unified = trimmed.replace('\\', "/");
    if unified.starts_with('/') {
        Ok(format!("file://{unified}"))
    } else {
        Ok(format!("file:///{unified}"))
    }
}

fn find_chromium() -> Result<PathBuf, String> {
    let candidates = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files\Chromium\Application\chrome.exe",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.is_file() {
            return Ok(p);
        }
    }
    // PATH 中的 msedge / chrome
    for name in ["msedge", "chrome", "chromium"] {
        if let Ok(out) = Command::new("where").arg(name).output() {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                if let Some(line) = text.lines().next() {
                    let p = PathBuf::from(line.trim());
                    if p.is_file() {
                        return Ok(p);
                    }
                }
            }
        }
    }
    Err(
        "未找到 Microsoft Edge 或 Google Chrome。请安装后重试，或使用「导出 Word」再打印。"
            .into(),
    )
}

/// 将完整 HTML 转为临时 PDF（无 Chromium 默认页眉/页脚 URL），返回 PDF 路径。
pub fn html_to_pdf(html: &str) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("shijuan-shenqi-print");
    fs::create_dir_all(&dir).map_err(|e| format!("创建临时目录失败: {e}"))?;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let html_path = dir.join(format!("print-{ts}.html"));
    let pdf_path = dir.join(format!("print-{ts}.pdf"));

    // 确保 charset，便于中文
    let body = if html.contains("<meta charset") || html.contains("charset=") {
        html.to_string()
    } else {
        html.replacen(
            "<head>",
            "<head><meta charset=\"UTF-8\" />",
            1,
        )
    };
    fs::write(&html_path, body.as_bytes()).map_err(|e| format!("写入临时 HTML 失败: {e}"))?;

    let browser = find_chromium()?;
    let url = file_url(&html_path)?;
    let pdf_arg = format!("--print-to-pdf={}", pdf_path.display());

    let output = Command::new(&browser)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--disable-extensions",
            "--disable-popup-blocking",
            "--no-first-run",
            "--no-default-browser-check",
            // 关键默认页眉页脚（否则会出现 tauri.localhost / 文件路径）
            "--no-pdf-header-footer",
            &pdf_arg,
            &url,
        ])
        .output()
        .map_err(|e| format!("启动浏览器导出 PDF 失败: {e}"))?;

    // 等文件系统落盘
    for _ in 0..50 {
        if pdf_path.is_file() {
            let len = fs::metadata(&pdf_path).map(|m| m.len()).unwrap_or(0);
            if len > 100 {
                // 清理 html 即可，pdf 留给用户/系统
                let _ = fs::remove_file(&html_path);
                return Ok(pdf_path);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(format!(
        "PDF 生成超时或失败（exit={:?}）。\nstderr: {}\nstdout: {}",
        output.status.code(),
        stderr.chars().take(500).collect::<String>(),
        stdout.chars().take(300).collect::<String>(),
    ))
}
