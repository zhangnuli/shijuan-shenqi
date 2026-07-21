# -*- coding: utf-8 -*-
import re
import urllib.request
from html import unescape
from pathlib import Path

URL = "http://www.dzkbw.com/books/bsd/shuxue/xs1s_2024/"
OUT = Path(__file__).resolve().parent / "_debug_page.txt"

req = urllib.request.Request(URL, headers={"User-Agent": "Mozilla/5.0"})
raw = urllib.request.urlopen(req, timeout=30).read()
html = raw.decode("gb2312", "ignore")

# save raw sample length
lines = [f"len={len(html)}", f"url={URL}", ""]

# ids/classes
ids = sorted(set(re.findall(r'id=["\']([^"\']+)["\']', html)))
classes = sorted(set(re.findall(r'class=["\']([^"\']+)["\']', html)))
lines.append("## IDS")
lines.extend(ids)
lines.append("\n## CLASSES")
lines.extend(classes)

# all link texts
def strip_tags(s):
    s = re.sub(r"<[^>]+>", "", s)
    return unescape(re.sub(r"\s+", " ", s)).strip()

pairs = re.findall(r'<a[^>]+href=["\']([^"\']+)["\'][^>]*>(.*?)</a>', html, re.I | re.S)
lines.append("\n## LINKS")
for href, t in pairs:
    t = strip_tags(t)
    if t:
        lines.append(f"{t}\t{href}")

# img alt/title
imgs = re.findall(r'<img[^>]+>', html, re.I)
lines.append("\n## IMGS sample")
for tag in imgs[:40]:
    alts = re.findall(r'alt=["\']([^"\']*)["\']', tag, re.I)
    titles = re.findall(r'title=["\']([^"\']*)["\']', tag, re.I)
    srcs = re.findall(r'(?:data-original|src)=["\']([^"\']+)["\']', tag, re.I)
    lines.append(f"alt={alts} title={titles} src0={srcs[:1]}")

# look for 第一单元 or 生活
for key in ["第一单元", "生活中的数", "走进美丽", "mulu", "listmain", "booklist", "zml"]:
    lines.append(f"find {key}: {html.find(key)}")

OUT.write_text("\n".join(lines), encoding="utf-8")
print("wrote", OUT)
