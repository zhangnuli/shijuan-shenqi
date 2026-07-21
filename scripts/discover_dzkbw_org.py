# -*- coding: utf-8 -*-
"""发现 dzkbw.org 上的教材 book 页，并解析 pre 目录。"""
from __future__ import annotations

import re
import urllib.request
from pathlib import Path

UA = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/122.0.0.0 Safari/537.36"
}
OUT = Path(__file__).resolve().parent / "_org_discover.txt"


def fetch(url: str) -> str:
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=30) as resp:
        raw = resp.read()
    for enc in ("utf-8", "gbk", "gb2312"):
        try:
            return raw.decode(enc)
        except Exception:
            continue
    return raw.decode("utf-8", "ignore")


def main() -> None:
    lines: list[str] = []
    pages = [
        "https://www.dzkbw.org/subject/rjb/yuwen.html",
        "https://www.dzkbw.org/grade/rjb/sannianji.html",
        "https://www.dzkbw.org/textbooks/bbb.html",
        "https://www.dzkbw.org/textbooks/rjb.html",
    ]
    all_links: list[tuple[str, str]] = []
    for url in pages:
        try:
            html = fetch(url)
        except Exception as e:
            lines.append(f"FAIL {url}: {e}")
            continue
        lines.append(f"OK {url} len={len(html)}")
        for href, title in re.findall(
            r'href="(/book/\d+\.html)"[^>]*?(?:title="([^"]*)")?', html
        ):
            t = title or ""
            all_links.append((href, t))
        # also from link text
        for href, text in re.findall(
            r'href="(/book/\d+\.html)"[^>]*>([^<]{2,80})</a>', html
        ):
            all_links.append((href, re.sub(r"<[^>]+>", "", text).strip()))

    # unique by href, prefer longer title
    best: dict[str, str] = {}
    for href, title in all_links:
        if href not in best or len(title) > len(best[href]):
            best[href] = title

    cn = []
    for href, title in sorted(best.items(), key=lambda x: x[1]):
        if "语文" not in title and "yuwen" not in href:
            # still keep if grade keywords
            if not any(k in title for k in ("年级", "册")):
                continue
        if "语文" in title or re.search(r"[一二三四五六]年级", title):
            if "语文" in title:
                cn.append((href, title))
                lines.append(f"{href}\t{title}")

    lines.append(f"\nChinese books: {len(cn)}")
    # probe grade3 shang catalog
    for href, title in cn:
        if "三年级" in title and "上" in title and "语文" in title:
            lines.append(f"CAND {href} {title}")

    # parse 4833 pre
    html = fetch("https://www.dzkbw.org/book/4833.html")
    m = re.search(r"<pre>([\s\S]*?)</pre>", html, re.I)
    if m:
        pre = m.group(1)
        pre = pre.replace("&ldquo;", "“").replace("&rdquo;", "”").replace("&nbsp;", " ")
        lines.append("\n=== PRE 4833 ===\n" + pre[:1500])

    OUT.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {OUT} lines={len(lines)}")


if __name__ == "__main__":
    main()
