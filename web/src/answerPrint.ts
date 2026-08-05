import type { BrandHeader } from './brand'
import type { ExamPaper } from './types'

type AnswerItem = ExamPaper['sections'][number]['items'][number]

function escapeHtml(value: unknown): string {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function itemNumber(item: AnswerItem, index: number): string {
  const raw = String(item.id || '').trim().replace(/[.、．)）]$/, '')
  return /^(?:\d+|[一二三四五六七八九十百千]+)$/.test(raw) ? raw : String(index + 1)
}

function formatAnswer(answer: unknown): string {
  const value = String(answer ?? '').trim() || '略'
  return escapeHtml(value).replace(/\r?\n/g, '<br />')
}

function sectionLayout(type = '', title = ''): { columns: number; wideAt: number } {
  const kind = `${type}${title}`
  if (/choice|选择|judge|判断/i.test(kind)) return { columns: 6, wideAt: 6 }
  if (/fill|填空|拼音|积累|字词|默写/i.test(kind)) return { columns: 4, wideAt: 14 }
  if (/calc|计算|口算|竖式|脱式|方程/i.test(kind)) return { columns: 4, wideAt: 16 }
  return { columns: 2, wideAt: 28 }
}

export function buildAnswerPrintHtml(p: ExamPaper, brand: BrandHeader = {}): string {
  const meta = p.meta
  const title = escapeHtml(meta.title || '试卷')
  const schoolBlock = [
    brand.schoolName,
    [brand.academicYear, brand.schoolTerm].filter((value) => value?.trim()).join(' '),
  ]
    .filter((value) => value?.trim())
    .map((value) => `<div class="school">${escapeHtml(value)}</div>`)
    .join('')
  const subtitle = [
    meta.edition,
    meta.subject,
    meta.grade ? `${meta.grade}年级` : '',
    meta.semester,
    meta.examType,
  ]
    .filter(Boolean)
    .map(escapeHtml)
    .join(' · ')
  const itemCount = (p.sections || []).reduce(
    (total, section) => total + (section.items || []).length,
    0,
  )
  const sections = (p.sections || [])
    .map((section, sectionIndex) => {
      const layout = sectionLayout(section.type, section.title)
      const rows = (section.items || [])
        .map((item, itemIndex) => {
          const value = String(item.answer ?? '').trim()
          const isWide = value.includes('\n') || value.length > layout.wideAt
          return `<div class="answer-item${isWide ? ' wide' : ''}"><span class="no">${escapeHtml(itemNumber(item, itemIndex))}.</span><span class="value">${formatAnswer(item.answer)}</span></div>`
        })
        .join('')
      const sectionTitle = escapeHtml(section.title || `第${sectionIndex + 1}大题`)
      return `<section class="answer-section"><h2>${sectionTitle}</h2><div class="answer-grid cols-${layout.columns}">${rows || '<div class="empty">暂无答案</div>'}</div></section>`
    })
    .join('')

  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8" />
<title>${title} · 参考答案</title>
<style>
  @page {
    size: A4;
    margin: 10mm 11mm 14mm;
    @bottom-center {
      content: "参考答案 · 第 " counter(page) " 页";
      font-family: "宋体", SimSun, serif;
      font-size: 8.5pt;
      color: #444;
    }
  }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    color: #000;
    font-family: "宋体", SimSun, "Microsoft YaHei", serif;
    font-size: 11pt;
    line-height: 1.32;
  }
  .school {
    text-align: center;
    font-family: "黑体", SimHei, sans-serif;
    font-size: 10pt;
    font-weight: bold;
    line-height: 1.25;
  }
  h1 {
    margin: 3px 0 1px;
    text-align: center;
    font-family: "黑体", SimHei, sans-serif;
    font-size: 16pt;
    line-height: 1.25;
  }
  .sub {
    margin: 0 0 6px;
    text-align: center;
    font-size: 9pt;
  }
  .answer-section {
    margin: 0 0 4px;
    break-inside: auto;
  }
  .answer-section h2 {
    margin: 4px 0 2px;
    padding-bottom: 1px;
    border-bottom: 1px solid #000;
    font-family: "黑体", SimHei, sans-serif;
    font-size: 11pt;
    line-height: 1.25;
    break-after: avoid;
    page-break-after: avoid;
  }
  .answer-grid {
    display: grid;
    gap: 0 10px;
  }
  .answer-grid.cols-2 { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .answer-grid.cols-4 { grid-template-columns: repeat(4, minmax(0, 1fr)); }
  .answer-grid.cols-6 { grid-template-columns: repeat(6, minmax(0, 1fr)); }
  .answer-item {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 3px;
    min-height: 1.45em;
    align-items: baseline;
    break-inside: avoid;
  }
  .answer-item.wide { grid-column: 1 / -1; }
  .no {
    font-family: Arial, sans-serif;
    font-weight: bold;
    white-space: nowrap;
  }
  .value {
    overflow-wrap: anywhere;
  }
  .empty { color: #555; }
  @media print {
    body { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
  }
</style>
</head>
<body>
  ${schoolBlock}
  <h1>${title} · 参考答案</h1>
  <div class="sub">${subtitle || '参考答案'}　　共 ${itemCount} 题</div>
  ${sections}
</body>
</html>`
}
