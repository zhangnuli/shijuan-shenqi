# -*- coding: utf-8 -*-
"""抓取电子课本网公开目录，用于对齐北师大数学 / 人教语文单元结构。"""
from __future__ import annotations

import json
import re
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "web" / "src-tauri" / "resources" / "data"

UA = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"}


def fetch(url: str) -> str:
    req = urllib.request.Request(url, headers=UA)
    raw = urllib.request.urlopen(req, timeout=30).read()
    for enc in ("gb2312", "gbk", "utf-8"):
        try:
            return raw.decode(enc)
        except Exception:
            pass
    return raw.decode("utf-8", "ignore")


def extract_links(html: str) -> list[str]:
    titles = re.findall(r">([^<>]{2,80})</a>", html)
    noise = {
        "电子课本",
        "数学",
        "英语",
        "语文",
        "一年级",
        "二年级",
        "三年级",
        "四年级",
        "五年级",
        "六年级",
        "北师大版",
        "人教版",
        "首页",
        "根据城市选择课本版本",
        "快速搜索课本",
        "古诗大全",
        "诗句大全",
        "加入收藏",
        "电子课本网",
    }
    out: list[str] = []
    for t in titles:
        t = t.strip().replace("\u3000", " ")
        if t in noise:
            continue
        if not re.search(r"[\u4e00-\u9fff]", t):
            continue
        if any(
            x in t
            for x in (
                "找北师",
                "找人教",
                "按年级",
                "按科目",
                "按阶段",
                "收藏",
                "古诗",
                "伤感",
                "备案",
                "闽公",
                "电子课本",
            )
        ):
            continue
        if out and out[-1] == t:
            continue
        out.append(t)
    return out


def group_into_units(links: list[str]) -> list[dict]:
    """
    将目录链接粗分为单元：
    - 含「第x单元」「单元」或独立大主题作为单元名
    - 其后课时归入 lessons
    """
    units: list[dict] = []
    current: dict | None = None

    unit_pat = re.compile(r"^第[一二三四五六七八九十\d]+单元")
    skip_tail = ("总复习", "数与代数", "图形与几何", "统计与概率", "综合与实践")

    for t in links:
        if t in skip_tail and current is None:
            continue
        is_unit = bool(unit_pat.search(t)) or t.startswith("整理与复习") or t.startswith(
            "数学好玩"
        )
        # 某些站点没有“第x单元”，用较短主题行当单元
        if is_unit or (
            current is None
            and len(t) <= 12
            and not t.startswith("综合实践")
            and "课时" not in t
        ):
            # 若是课时风格则不当单元
            if len(t) >= 2:
                current = {
                    "id": f"u{len(units) + 1}",
                    "name": t,
                    "lessons": [],
                    "points": [],
                }
                units.append(current)
                continue
        if current is None:
            current = {
                "id": "u1",
                "name": "本册内容",
                "lessons": [],
                "points": [],
            }
            units.append(current)
        # 过滤导航残留
        if t in ("小学数学", "小学语文") or t.endswith("电子课本"):
            continue
        current["lessons"].append(t)
        # 知识点默认用课时名
        if t not in current["points"]:
            current["points"].append(t)

    # 清理空单元、过碎单元
    cleaned = []
    for u in units:
        if not u["lessons"] and not u["points"]:
            continue
        # 若 points 过多，保留前 12
        u["points"] = u["points"][:12]
        cleaned.append(u)
    return cleaned


def build_pack(
    subject: str,
    edition: str,
    grade: int,
    semester: str,
    title: str,
    units: list[dict],
    source_url: str,
) -> dict:
    return {
        "subject": subject,
        "edition": edition,
        "grade": grade,
        "semester": semester,
        "title": title,
        "source": {
            "platform": "国家中小学智慧教育平台（对齐参考）",
            "platformUrl": "https://basic.smartedu.cn/",
            "classroomUrl": "https://basic.smartedu.cn/syncClassroom",
            "materialUrl": "https://basic.smartedu.cn/tchMaterial",
            "catalogRef": source_url,
            "note": "单元/课时目录对齐公开教材目录与智慧教育平台课程结构；命题知识点据此约束，不存储教材正文。",
        },
        "units": units,
        "examHints": [
            "严格依据本册单元与课时命题，避免超纲",
            "单元测试聚焦所选单元课时",
            "期中覆盖前半册单元，期末覆盖全册",
            "可参考智慧教育平台同步课程与基础性作业风格",
        ],
    }


BSD_MATH = {
    (1, "shang"): ("xs1s_2024/", "一年级上册"),
    (1, "xia"): ("xs1x/", "一年级下册"),
    (2, "shang"): ("2s/", "二年级上册"),
    (2, "xia"): ("2x/", "二年级下册"),
    (3, "shang"): ("3s/", "三年级上册"),
    (3, "xia"): ("3x/", "三年级下册"),
    (4, "shang"): ("4s/", "四年级上册"),
    (4, "xia"): ("4x/", "四年级下册"),
    (5, "shang"): ("5s/", "五年级上册"),
    (5, "xia"): ("5x/", "五年级下册"),
    (6, "shang"): ("6s/", "六年级上册"),
    (6, "xia"): ("6x/", "六年级下册"),
}

# 人教语文公开目录路径（电子课本网）
RENJIAO_CHINESE = {
    (1, "shang"): ("/books/rjb/yuwen/xs1s/", "一年级上册"),
    (1, "xia"): ("/books/rjb/yuwen/xs1x/", "一年级下册"),
    (2, "shang"): ("/books/rjb/yuwen/2s/", "二年级上册"),
    (2, "xia"): ("/books/rjb/yuwen/2x/", "二年级下册"),
    (3, "shang"): ("/books/rjb/yuwen/3s/", "三年级上册"),
    (3, "xia"): ("/books/rjb/yuwen/3x/", "三年级下册"),
    (4, "shang"): ("/books/rjb/yuwen/4s/", "四年级上册"),
    (4, "xia"): ("/books/rjb/yuwen/4x/", "四年级下册"),
    (5, "shang"): ("/books/rjb/yuwen/5s/", "五年级上册"),
    (5, "xia"): ("/books/rjb/yuwen/5x/", "五年级下册"),
    (6, "shang"): ("/books/rjb/yuwen/6s/", "六年级上册"),
    (6, "xia"): ("/books/rjb/yuwen/6x/", "六年级下册"),
}


def refine_grade1_shang_2024(links: list[str]) -> list[dict]:
    """2024 秋一年级上册目录结构更清晰，手工归并更稳。"""
    # 基于抓取结果的标准结构
    return [
        {
            "id": "u1",
            "name": "第一单元 生活中的数",
            "lessons": [
                "走进美丽乡村",
                "玩具",
                "小猫钓鱼",
                "文具",
                "数鸡蛋",
                "快乐的午餐",
                "动物乐园",
            ],
            "points": [
                "1-10 的认识",
                "数数、读数、写数",
                "基数与序数",
                "数的大小比较",
                "数的组成",
            ],
        },
        {
            "id": "u2",
            "name": "第二单元 5以内数的加与减",
            "lessons": ["一共有多少", "还剩下多少", "可爱的小猫"],
            "points": ["加法意义", "减法意义", "5 以内加减口算", "简单实际问题"],
        },
        {
            "id": "u3",
            "name": "综合实践 介绍我的教室",
            "lessons": ["观察教室", "介绍教室", "校园开放日"],
            "points": ["观察与描述", "综合实践表达"],
        },
        {
            "id": "u4",
            "name": "第三单元 分类",
            "lessons": ["整理房间", "一起来分类", "猜数游戏"],
            "points": ["按标准分类", "分类思考", "简单整理"],
        },
        {
            "id": "u5",
            "name": "第四单元 6-10 的加减",
            "lessons": ["背土豆", "课间", "小鸡吃食", "乘车", "挖红薯", "可爱的企鹅"],
            "points": ["6-10 的认识", "6-10 加减", "连加连减初步", "解决问题"],
        },
        {
            "id": "u6",
            "name": "整理与复习",
            "lessons": ["能解决吗", "做个加法表", "做个减法表"],
            "points": ["知识梳理", "计算熟练", "综合应用"],
        },
        {
            "id": "u7",
            "name": "数学好玩 一起做游戏",
            "lessons": ["一起做游戏"],
            "points": ["数学游戏", "策略意识"],
        },
        {
            "id": "u8",
            "name": "第五单元 认识图形",
            "lessons": ["认识图形", "我说你做", "怎样搭得高"],
            "points": ["立体图形初步", "平面图形初步", "动手操作"],
        },
        {
            "id": "u9",
            "name": "综合实践 记录我的一天",
            "lessons": ["淘气的一天", "记录我的一天", "分享我的一天"],
            "points": ["时间顺序", "记录与表达"],
        },
        {
            "id": "u10",
            "name": "总复习",
            "lessons": ["数与代数", "图形与几何", "统计与概率", "综合与实践"],
            "points": ["全册知识综合", "易错点回顾"],
        },
    ]


def main() -> None:
    # --- 数学北师大 ---
    math_dir = OUT_DIR / "math" / "beishida"
    math_dir.mkdir(parents=True, exist_ok=True)
    for (grade, sem), (path, title) in BSD_MATH.items():
        url = "http://www.dzkbw.com/books/bsd/shuxue/" + path
        print("fetch", url)
        try:
            html = fetch(url)
            links = extract_links(html)
            if grade == 1 and sem == "shang":
                units = refine_grade1_shang_2024(links)
            else:
                units = group_into_units(links)
            # 若抓取太少则保留旧文件提示
            if len(units) < 2:
                print("  warn: few units", len(units), "links", len(links))
            pack = build_pack(
                "math",
                "beishida",
                grade,
                sem,
                title,
                units,
                url,
            )
            # 智慧教育直达（按年级进入同步课堂，具体教材需用户在平台选择版本）
            pack["source"]["smarteduSearchHint"] = (
                f"在 https://basic.smartedu.cn/syncClassroom 选择：小学 / {grade}年级 / 数学 / 北师大版 / {'上册' if sem=='shang' else '下册'}"
            )
            out = math_dir / f"grade-{grade}-{sem}.json"
            out.write_text(json.dumps(pack, ensure_ascii=False, indent=2), encoding="utf-8")
            print("  wrote", out.name, "units", len(units))
        except Exception as e:
            print("  ERR", e)

    # --- 语文人教 ---
    cn_dir = OUT_DIR / "chinese" / "renjiao"
    cn_dir.mkdir(parents=True, exist_ok=True)
    for (grade, sem), (path, title) in RENJIAO_CHINESE.items():
        url = "http://www.dzkbw.com" + path
        print("fetch", url)
        try:
            html = fetch(url)
            links = extract_links(html)
            units = group_into_units(links)
            pack = build_pack(
                "chinese",
                "renjiao",
                grade,
                sem,
                title,
                units,
                url,
            )
            pack["source"]["smarteduSearchHint"] = (
                f"在 https://basic.smartedu.cn/syncClassroom 选择：小学 / {grade}年级 / 语文 / 统编（人教） / {'上册' if sem=='shang' else '下册'}"
            )
            pack["examHints"].extend(
                [
                    "课内字词与课文填空优先覆盖本单元篇目",
                    "阅读材料难度对齐该年级同步课堂",
                ]
            )
            out = cn_dir / f"grade-{grade}-{sem}.json"
            out.write_text(json.dumps(pack, ensure_ascii=False, indent=2), encoding="utf-8")
            print("  wrote", out.name, "units", len(units), "links", len(links))
        except Exception as e:
            print("  ERR", e)

    print("done")


if __name__ == "__main__":
    main()
