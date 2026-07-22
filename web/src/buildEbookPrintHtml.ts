/** 电子书单元页图 → 可打印 HTML */

export interface EbookPage {
  index: number
  url: string
}

export interface EbookUnitPages {
  bookName: string
  subjectName: string
  unitName: string
  bookId: string
  startPage: number
  endPage: number
  totalBookPages: number
  pages: EbookPage[]
}

function esc(s: string) {
  return String(s || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

export function buildEbookPrintHtml(data: EbookUnitPages): string {
  const title = `${data.subjectName || ''} · ${data.bookName || ''} · ${data.unitName || '单元'}`.trim()
  const pages = (data.pages || [])
    .map(
      (p) => `
    <section class="page">
      <div class="page-meta">第 ${p.index} 页</div>
      <img src="${esc(p.url)}" alt="p${p.index}" />
    </section>`,
    )
    .join('')

  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8" />
<title>${esc(title)}</title>
<style>
  @page {
    size: A4;
    margin: 8mm 9mm 12mm 9mm;
    @bottom-center {
      content: "${esc(data.unitName || '电子书')} · 第 " counter(page) " 页";
      font-family: "宋体", SimSun, serif;
      font-size: 9pt;
      color: #444;
    }
  }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    padding: 0;
    font-family: "宋体", SimSun, "Microsoft YaHei", serif;
    color: #000;
    background: #fff;
  }
  .cover {
    text-align: center;
    padding: 12mm 8mm 6mm;
    page-break-after: always;
  }
  .cover h1 {
    font-family: "黑体", SimHei, sans-serif;
    font-size: 16pt;
    margin: 0 0 8px;
  }
  .cover .sub { font-size: 11pt; margin: 4px 0; }
  .cover .note {
    margin-top: 16px;
    font-size: 9pt;
    color: #444;
    line-height: 1.5;
  }
  .page {
    page-break-after: always;
    page-break-inside: avoid;
    text-align: center;
    padding: 0;
  }
  .page:last-child { page-break-after: auto; }
  .page-meta {
    font-size: 9pt;
    color: #555;
    margin: 0 0 4px;
  }
  .page img {
    max-width: 100%;
    max-height: 250mm;
    height: auto;
    width: auto;
    display: inline-block;
  }
  @media print {
    body { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
  }
</style>
</head>
<body>
  <div class="cover">
    <h1>${esc(title)}</h1>
    <div class="sub">页码范围：${data.startPage} – ${data.endPage}
      （全书约 ${data.totalBookPages} 页）</div>
    <div class="sub">共 ${data.pages?.length || 0} 页图</div>
    <div class="note">
      来源：自有资源站电子书页图打印稿。<br/>
      仅供本校/本站授权范围内教学使用。
    </div>
  </div>
  ${pages || '<p style="padding:24px;text-align:center">无页图</p>'}
</body>
</html>`
}
