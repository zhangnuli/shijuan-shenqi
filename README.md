# 试卷神器

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GitHub](https://img.shields.io/badge/GitHub-zhangnuli%2Fshijuan--shenqi-181717?logo=github)](https://github.com/zhangnuli/shijuan-shenqi)

基于 **Tauri 2 · Vue 3 · Rust** 的小学组卷与教案**桌面工具**。  
课标、历史、校本题库与模板均在本机管理；通过 OpenAI 兼容接口生成结构化 JSON，再在本地完成 Word / 打印排版。

> **开源范围**：应用源码 + 内置教材**公开目录大纲**（单元/课时/要点提示）。  
> **不包含**：教材正文、统考原题版权内容、任何个人 API Key。

## 功能概览

| 类别 | 能力 |
|------|------|
| 教材 | 数学（北师大 / 人教 / 苏教）1–6 年级；语文人教统编 1–6；英语人教 3–6 |
| 试卷 | 单元 / 期中 / 期末 / 口算·字词 / 课时练 / 作业练 / 错题再练；模板市集；A/B/C 平行卷 |
| 教案 | 新授·练习·复习·讲评；全课时；**教师版 + 家长辅导手册**；卷案一键联动 |
| 教研 | 双向细目表、数学验算、质检、作业讲评稿、校本收藏 |
| 课标 | 查看课标、内置 vs 同步 diff；从 [dzkbw.org](https://www.dzkbw.org) 同步公开目录 |
| 输出 | 学生卷 / 答案 / 教案 DOCX、打印预览 |

无 API Key 时仍可使用**结构模板**（占位卷 / 模板教案）。

## 截图与界面

启动后左右分栏：左侧组卷/教案参数，右侧预览；顶栏提供历史、查看课标、导出日志、接口设置等。

## 环境要求

- Node.js **18+**
- Rust **1.77.2+**（建议较新稳定版）
- Windows：MSVC 工具链（Visual Studio Build Tools）
- （可选）[Ollama](https://ollama.com/) 作离线本地模型

## 快速开始

```bash
# 根目录
npm install
npm --prefix web install

# 开发（桌面）
npm run dev
```

仅前端页面：

```bash
npm run web:dev
```

检查与测试：

```bash
npm run check
```

打包 Windows 安装包：

```bash
npm run build
```

产物目录：`web/src-tauri/target/release/bundle/`（NSIS / MSI）。  

**自动更新**：已接入 `tauri-plugin-updater`，更新源为  
[GitHub Releases](https://github.com/zhangnuli/shijuan-shenqi/releases)  
（`.../releases/latest/download/latest.json`）。

**发版（推荐）**：在仓库 Settings → Secrets 配置  
`TAURI_SIGNING_PRIVATE_KEY`（本机 `.tauri/shijuan.key` 全文，**勿提交私钥**），然后：

```bash
git tag v0.1.1
git push origin v0.1.1
```

CI（`.github/workflows/release.yml`）会构建 Windows 安装包并上传 Release（含 `latest.json`）。  
本地打包：`cd web` 后设置 `TAURI_SIGNING_PRIVATE_KEY_PATH` 再 `npm run tauri:build`。

## 配置 AI

1. 打开应用 → **接口设置**
2. 选择厂商预设，或自定义 **API Base**（一般为 `https://xxx/v1`）
3. 填写 **API Key** 与 **模型名**

| 预设 | 说明 |
|------|------|
| xAI / OpenAI / DeepSeek 等 | 云端 OpenAI 兼容接口 |
| **本地 Ollama** | Base：`http://127.0.0.1:11434/v1`，Key 可空 |

密钥在 Windows 下使用 **DPAPI** 按当前用户加密存储，**不会**写入普通配置 JSON，也**不应**提交到 Git。

## 项目结构

```text
web/src/                       Vue UI、打印与 DOCX
web/src/composables/           前端可复用逻辑
web/src/services/              Tauri IPC 边界
web/src-tauri/src/             Rust：AI、组卷、教案、课标、历史、模板、质检…
web/src-tauri/resources/data/  内置课标包（仅大纲）
scripts/                       课标采集 / 构建脚本
.github/workflows/             打 tag 自动发版
```

## 课标与合规

- 内置与同步数据**只含公开目录、课时与知识点提示**，不下载、不存储教材正文。
- 应用内「同步课标」优先从 **dzkbw.org** 的 book 页解析 `<pre>` 目录；失败时回退旧站逻辑。
- 也可本地执行：

```bash
python scripts/scrape_dzkbw.py
python scripts/build_official_curriculum.py
```

请合理控制抓取频率，并遵守目标网站服务条款。生成的试卷/教案为 **AI 模拟内容**，使用时请自行审校，勿冒充正式统考卷。

## 安全与本地数据

| 数据 | 位置（示意） |
|------|----------------|
| 配置 / 加密密钥 | 本机应用配置目录 |
| 历史、校本库、模板 | 本机应用数据目录（带版本 JSON + 备份） |
| 同步课标 | 应用数据目录 `curriculum/`（优先于内置包） |

- 启用 CSP；另存为路径经系统文件对话框授权。
- 顶栏可 **导出运行日志**（不含密钥），便于排查 API 问题。

## 贡献

欢迎 Issue / PR：修 bug、补课标大纲、改进 UI 均可。

1. Fork 本仓库  
2. 新建分支：`git checkout -b feature/your-change`  
3. 本地执行 `npm run check`  
4. 提交 PR 并说明动机与自测方式  

## 许可证

本项目以 [MIT License](LICENSE) 开源。

第三方依赖遵循各自许可证。内置课标大纲来自公开教材目录信息，**不等于**教材内容授权；商用或再分发请自行评估合规要求。

## 免责声明

本软件按「现状」提供，作者不对 AI 生成内容的准确性、适用性作保证。请教师在教学使用前认真审阅试题与教案。
