import type { ExamPaper } from './types'
import type { BrandHeader } from './brand'

type AnswerSheetItem = ExamPaper['sections'][number]['items'][number]

function escapeHtml(value: unknown): string {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function answerNumber(item: AnswerSheetItem, index: number): string {
  const raw = String(item.id || '').trim().replace(/[.、．)]$/, '')
  return /^(?:\d+|[一二三四五六七八九十百千]+)$/.test(raw) ? raw : String(index + 1)
}

function answerHtml(answer: unknown): string {
  const value = String(answer ?? '').trim() || '略'
  return escapeHtml(value).replace(/\r?\n/g, '<br />')
}

export function buildAnswerSheetPrintHtml(p: ExamPaper, brand: BrandHeader = {}): string {
  const meta = p.meta || ({} as ExamPaper['meta'])
  const brandLines = [
    brand.schoolName,
    [brand.academicYear, brand.schoolTerm].filter((value) => value?.trim()).join(' '),
  ]
    .filter((value) => value?.trim())
    .map((value) => `<div class="school">${escapeHtml(value)}</div>`)
    .join('')
  const title = escapeHtml(meta.title || '试卷')
  const sub = [
    meta.edition,
    meta.subject,
    meta.grade ? `${meta.grade}年级` : '',
    meta.semester,
    meta.examType,
  ]
    .filter(Boolean)
    .map(escapeHtml)
    .join(' · ')
  const sections = (p.sections || [])
    .map((section, sectionIndex) => {
      const sectionTitle = escapeHtml(section.title || `第${sectionIndex + 1}大题`)
      const sectionKind = `${section.type || ''}${section.title || ''}`
      const isDenseSection = /choice|选择|judge|判断|fill|填空|拼音|积累|字词|默写/i.test(sectionKind)
      const rows = (section.items || [])
        .map(
          (item, itemIndex) => {
            const value = String(item.answer ?? '').trim()
            const rowClass = value.includes('\n') || value.length > 16 ? 'answer-row wide' : 'answer-row'
            return `
        <div class="${rowClass}">
          <span class="answer-no">${escapeHtml(answerNumber(item, itemIndex))}.</span>
          <span class="answer-value">${answerHtml(item.answer)}</span>
        </div>`
          },
        )
        .join('')
      const columnsClass = isDenseSection ? 'cols-4' : 'cols-3'
      return `<section class="answer-section"><h2>${sectionTitle}</h2><div class="answer-rows ${columnsClass}">${rows || '<div class="empty">本大题暂无答案</div>'}</div></section>`
    })
    .join('')

  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8" />
<title>${title} · 大字答案</title>
<style>
  @page {
    size: A4;
    margin: 14mm 16mm 17mm;
    @bottom-center {
      content: "大字答案 · 第 " counter(page) " 页";
      font-family: "宋体", SimSun, serif;
      font-size: 9pt;
      color: #444;
    }
  }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    color: #000;
    font-family: "宋体", SimSun, "Microsoft YaHei", serif;
    font-size: 16pt;
    line-height: 1.45;
  }
  .school {
    text-align: center;
    font-family: "黑体", SimHei, sans-serif;
    font-size: 13pt;
    font-weight: bold;
    line-height: 1.35;
  }
  h1 {
    margin: 5px 0 2px;
    text-align: center;
    font-family: "黑体", SimHei, sans-serif;
    font-size: 22pt;
    line-height: 1.35;
  }
  .sub {
    margin: 0 0 12px;
    text-align: center;
    font-size: 11.5pt;
    line-height: 1.4;
  }
  .answer-section {
    margin: 0 0 6px;
    break-inside: avoid;
    page-break-inside: avoid;
  }
  .answer-section h2 {
    margin: 6px 0 2px;
    padding-bottom: 2px;
    border-bottom: 1px solid #000;
    font-family: "黑体", SimHei, sans-serif;
    font-size: 16pt;
    line-height: 1.3;
    break-after: avoid;
  }
  .answer-rows {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    column-gap: 18px;
    break-inside: avoid;
  }
  .answer-rows.cols-4 {
    grid-template-columns: repeat(4, minmax(0, 1fr));
    column-gap: 12px;
  }
  .answer-row {
    display: grid;
    grid-template-columns: 3em minmax(0, 1fr);
    gap: 0.25em;
    min-height: 1.45em;
    align-items: start;
    break-inside: avoid;
  }
  .answer-row.wide {
    grid-column: 1 / -1;
  }
  .answer-no {
    font-family: "Arial", sans-serif;
    font-weight: bold;
    text-align: right;
    white-space: nowrap;
  }
  .answer-value {
    white-space: normal;
    overflow-wrap: anywhere;
  }
  .empty {
    color: #555;
    font-size: 13pt;
  }
  @media print {
    body { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
  }
</style>
</head>
<body>
  ${brandLines}
  <h1>${title} · 大字答案</h1>
  <div class="sub">${sub || '参考答案'}　　共 ${(p.sections || []).reduce((total, section) => total + (section.items || []).length, 0)} 题</div>
  ${sections}
</body>
</html>`
}
