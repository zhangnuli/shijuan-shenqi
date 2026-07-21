# -*- coding: utf-8 -*-
"""
从电子课本网 http://www.dzkbw.com/ 抓取公开目录，生成知识点包。

规则：
- 只取「本册目录」链接：href 指向当前册目录下的 *.htm（如 001.htm）
- 优先解析 class=bookmulu 区域
- 不下载教材正文/图片
"""
from __future__ import annotations

import json
import re
import time
import urllib.error
import urllib.request
from html import unescape
from pathlib import Path
from urllib.parse import urljoin, urlparse

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "web" / "src-tauri" / "resources" / "data"
LOG = ROOT / "scripts" / "_scrape_log.txt"

UA = {
    "User-Agent": (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
        "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"
    )
}

PLATFORM = {
    "platform": "国家中小学智慧教育平台 + 电子课本网目录",
    "platformUrl": "https://basic.smartedu.cn/",
    "classroomUrl": "https://basic.smartedu.cn/syncClassroom",
    "materialUrl": "https://basic.smartedu.cn/tchMaterial",
    "elecEduUrl": "https://basic.smartedu.cn/elecEdu",
    "catalogSite": "http://www.dzkbw.com/",
    "note": "单元/课时目录来自电子课本网公开页 bookmulu；命题对齐智慧教育平台同步课程。不含教材正文。",
}


def log(msg: str) -> None:
    print(msg, flush=True)
    with LOG.open("a", encoding="utf-8") as f:
        f.write(msg + "\n")


def fetch(url: str) -> str:
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=30) as resp:
        raw = resp.read()
    for enc in ("gb2312", "gbk", "utf-8"):
        try:
            return raw.decode(enc)
        except Exception:
            continue
    return raw.decode("utf-8", "ignore")


def strip_tags(s: str) -> str:
    s = re.sub(r"<script[\s\S]*?</script>", " ", s, flags=re.I)
    s = re.sub(r"<style[\s\S]*?</style>", " ", s, flags=re.I)
    s = re.sub(r"<[^>]+>", "", s)
    s = unescape(s)
    return re.sub(r"\s+", " ", s).strip()


def extract_links(html: str) -> list[tuple[str, str]]:
    pairs = re.findall(r'<a[^>]+href=["\']([^"\']+)["\'][^>]*>(.*?)</a>', html, re.I | re.S)
    out: list[tuple[str, str]] = []
    for href, text in pairs:
        t = strip_tags(text)
        if t:
            out.append((href.strip(), t))
    return out


def parse_grade_sem_from_url(url: str) -> tuple[int, str] | None:
    # xs1s_2024 / xs2x_2026 / 4s / 6x / ws6s_2024
    m = re.search(r"/(?:xs|ws)?([1-6])([sx])(?:[_/]|$)", url)
    if m:
        return int(m.group(1)), ("shang" if m.group(2) == "s" else "xia")
    m = re.search(r"/([1-6])([sx])(?:[_/]|$)", url)
    if m:
        return int(m.group(1)), ("shang" if m.group(2) == "s" else "xia")
    return None


def find_book_links(index_url: str, subject_path_key: str) -> list[tuple[str, str, int, str, int]]:
    """
    返回 (url, title, grade, semester, score)
    score 用于择优：六三学制 xs 优先于五四 ws；带年份优先。
    """
    html = fetch(index_url)
    results: list[tuple[str, str, int, str, int]] = []
    for href, title in extract_links(html):
        full = urljoin(index_url, href)
        if subject_path_key not in full:
            continue
        if "/books/" not in full:
            continue
        # 只要册首页，不要 001.htm
        if re.search(r"/\d{3}\.htm", full, re.I):
            continue
        gs = parse_grade_sem_from_url(full)
        if not gs:
            continue
        grade, sem = gs
        score = 0
        if "/xs" in full or re.search(r"/xs[1-6][sx]", full):
            score += 100  # 六三学制
        if "/ws" in full:
            score -= 50  # 五四学制靠后
        if re.search(r"20\d{2}", full) or re.search(r"20\d{2}", title):
            score += 30
        if "上册" in title or "下册" in title or "年级" in title:
            score += 10
        results.append((full.rstrip("/") + "/", title, grade, sem, score))
    return results


def pick_best(links: list[tuple[str, str, int, str, int]]) -> dict[tuple[int, str], tuple[str, str]]:
    buckets: dict[tuple[int, str], list[tuple[str, str, int]]] = {}
    for url, title, g, s, score in links:
        buckets.setdefault((g, s), []).append((url, title, score))
    chosen: dict[tuple[int, str], tuple[str, str]] = {}
    for key, items in buckets.items():
        items.sort(key=lambda x: x[2], reverse=True)
        chosen[key] = (items[0][0], items[0][1])
    return chosen


# 明确单元标题；口语交际/习作/语文园地作为“板块”挂在当前单元下，不单独拆成大单元
HARD_UNIT_RE = re.compile(
    r"^(?:"
    r"第[一二三四五六七八九十百零\d]+单元"
    r"|[一二三四五六七八九十]+[、.\s．][^\s]{1,20}"  # 一 认识更大的数 / 二、线与角
    r"|整理与复习"
    r"|数学好玩"
    r"|总复习"
    r"|快乐读书吧"
    r"|我上学了|我上学啦"
    r"|汉语拼音"
    r"|综合实践"
    r"|综合性学习"
    r")"
)

SECTION_AS_LESSON = re.compile(r"^(口语交际|习作|语文园地|阅读|写话|日积月累)$")


def is_unit_title(t: str) -> bool:
    if SECTION_AS_LESSON.match(t):
        return False
    if HARD_UNIT_RE.search(t):
        return True
    if re.match(r"^第[一二三四五六七八九十\d]+单元", t):
        return True
    if "单元" in t and len(t) <= 24:
        return True
    return False


def group_catalog(entries: list[str]) -> list[dict]:
    units: list[dict] = []
    current: dict | None = None

    for t in entries:
        t = t.strip()
        if not t:
            continue

        if is_unit_title(t):
            current = {"id": f"u{len(units) + 1}", "name": t, "lessons": [], "points": []}
            units.append(current)
            continue

        if current is None:
            # 开篇非单元条目：先建占位单元
            current = {"id": "u1", "name": t, "lessons": [], "points": []}
            units.append(current)
            continue

        if t == current["name"]:
            continue

        # 板块标题保留在 lessons 里，便于命题覆盖
        current["lessons"].append(t)

    cleaned = []
    for u in units:
        lessons: list[str] = []
        for x in u["lessons"]:
            if x not in lessons:
                lessons.append(x)
        u["lessons"] = lessons
        pts: list[str] = [u["name"]]
        for x in lessons[:16]:
            if x not in pts:
                pts.append(x)
        u["points"] = pts[:20]
        cleaned.append(u)
    return cleaned


def extract_book_mulu(html: str, book_url: str) -> list[str]:
    """提取本册真实目录标题。"""
    book_path = urlparse(book_url).path.rstrip("/")
    # 1) 优先 bookmulu 区块
    chunks: list[str] = []
    for m in re.finditer(
        r'<div[^>]+class=["\'][^"\']*bookmulu[^"\']*["\'][^>]*>([\s\S]*?)</div>',
        html,
        re.I,
    ):
        chunks.append(m.group(1))
    region = "\n".join(chunks) if chunks else html

    titles: list[str] = []
    for href, text in extract_links(region):
        full = urljoin(book_url, href)
        path = urlparse(full).path
        # 只要本册目录下的页码 htm
        if not re.search(r"/\d{2,4}\.htm?$", path, re.I):
            continue
        # 路径前缀匹配当前册
        if book_path not in path and not path.startswith(book_path):
            # 有的相对路径只有 001.htm
            if not re.match(r"^\d{2,4}\.htm?$", href.split("/")[-1], re.I):
                if book_path.split("/")[-1] not in path:
                    continue
        t = text.strip()
        if not t:
            continue
        if titles and titles[-1] == t:
            continue
        titles.append(t)

    # 2) 若区域解析失败，全页扫描 *.htm
    if len(titles) < 5:
        titles = []
        for href, text in extract_links(html):
            full = urljoin(book_url, href)
            path = urlparse(full).path
            if not re.search(r"/\d{2,4}\.htm?$", path, re.I):
                continue
            if book_path not in path:
                continue
            t = text.strip()
            if not t:
                continue
            if titles and titles[-1] == t:
                continue
            titles.append(t)

    return titles


def build_pack(
    subject: str,
    edition: str,
    grade: int,
    semester: str,
    title: str,
    units: list[dict],
    catalog_url: str,
    edition_label: str,
    subject_label: str,
    raw_titles: list[str],
) -> dict:
    sem_label = "上册" if semester == "shang" else "下册"
    return {
        "subject": subject,
        "edition": edition,
        "grade": grade,
        "semester": semester,
        "title": title,
        "source": {
            **PLATFORM,
            "editionLabel": edition_label,
            "subjectLabel": subject_label,
            "catalogRef": catalog_url,
            "smarteduPathHint": f"小学 → {grade}年级 → {subject_label} → {edition_label} → {sem_label}",
            "entryCount": len(raw_titles),
            "unitCount": len(units),
        },
        "units": units,
        "examHints": [
            "命题必须落在本册单元/课时范围内，禁止超纲",
            "单元测试只考所选单元课时",
            "期中覆盖前半册单元，期末覆盖全册",
            f"目录来源（电子课本网）：{catalog_url}",
            "风格可参考智慧教育平台同步课堂与基础性作业 https://basic.smartedu.cn/syncClassroom",
        ],
    }


def scrape_subject(
    index_url: str,
    subject: str,
    edition: str,
    subject_path_key: str,
    edition_label: str,
    subject_label: str,
    out_dir: Path,
) -> None:
    log(f"\n=== INDEX {index_url} ===")
    links = find_book_links(index_url, subject_path_key)
    log(f"candidates: {len(links)}")
    chosen = pick_best(links)
    for (g, s), (url, title) in sorted(chosen.items()):
        log(f"pick {g}-{s}: {title} -> {url}")

    out_dir.mkdir(parents=True, exist_ok=True)
    # 清理旧 debug
    for p in out_dir.glob("_debug_*.json"):
        p.unlink()

    for (g, s), (url, title) in sorted(chosen.items()):
        log(f"fetch {g}-{s}: {url}")
        try:
            html = fetch(url)
            titles = extract_book_mulu(html, url)
            units = group_catalog(titles)
            book_title = f"{g}年级{'上' if s == 'shang' else '下'}册"
            m = re.search(
                r"([一二三四五六]年级).{0,8}(上册|下册)",
                title,
            )
            if m:
                book_title = m.group(0)
            else:
                # 从页面 <title>
                tm = re.search(r"<title>(.*?)</title>", html, re.I | re.S)
                if tm:
                    tt = strip_tags(tm.group(1))
                    m2 = re.search(r"([一二三四五六]年级).{0,8}(上册|下册)", tt)
                    if m2:
                        book_title = m2.group(0)

            pack = build_pack(
                subject,
                edition,
                g,
                s,
                book_title,
                units,
                url,
                edition_label,
                subject_label,
                titles,
            )
            path = out_dir / f"grade-{g}-{s}.json"
            path.write_text(json.dumps(pack, ensure_ascii=False, indent=2), encoding="utf-8")
            log(
                f"  wrote {path.name} units={len(units)} "
                f"lessons={sum(len(u['lessons']) for u in units)} entries={len(titles)}"
            )
            if len(units) < 2 or len(titles) < 5:
                (out_dir / f"_debug_grade-{g}-{s}.json").write_text(
                    json.dumps({"url": url, "titles": titles}, ensure_ascii=False, indent=2),
                    encoding="utf-8",
                )
                log("  WARN: too few catalog entries")
            time.sleep(0.35)
        except urllib.error.HTTPError as e:
            log(f"  HTTP {e.code}")
        except Exception as e:
            log(f"  ERR {e}")


def main() -> None:
    if LOG.exists():
        LOG.unlink()
    log("start scrape http://www.dzkbw.com/")

    scrape_subject(
        "http://www.dzkbw.com/books/bsd/shuxue/",
        subject="math",
        edition="beishida",
        subject_path_key="/bsd/shuxue/",
        edition_label="北师大版",
        subject_label="数学",
        out_dir=OUT / "math" / "beishida",
    )
    scrape_subject(
        "http://www.dzkbw.com/books/rjb/yuwen/",
        subject="chinese",
        edition="renjiao",
        subject_path_key="/rjb/yuwen/",
        edition_label="人教统编版",
        subject_label="语文",
        out_dir=OUT / "chinese" / "renjiao",
    )

    index = {
        "catalogSite": "http://www.dzkbw.com/",
        "platform": "https://basic.smartedu.cn/",
        "rebuild": "python scripts/scrape_dzkbw.py",
        "subjects": [
            {
                "subject": "math",
                "edition": "beishida",
                "label": "数学·北师大版",
                "index": "http://www.dzkbw.com/books/bsd/shuxue/",
            },
            {
                "subject": "chinese",
                "edition": "renjiao",
                "label": "语文·人教统编版",
                "index": "http://www.dzkbw.com/books/rjb/yuwen/",
            },
        ],
    }
    (OUT / "index.json").write_text(json.dumps(index, ensure_ascii=False, indent=2), encoding="utf-8")
    log("done")


if __name__ == "__main__":
    main()
