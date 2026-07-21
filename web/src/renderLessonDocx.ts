import {
  AlignmentType,
  BorderStyle,
  Document,
  Footer,
  Header,
  Packer,
  PageNumber,
  Paragraph,
  Table,
  TableCell,
  TableRow,
  TextRun,
  WidthType,
  ShadingType,
  VerticalAlign,
} from 'docx'
import type { LessonPlan } from './types'
import { brandLines, type BrandHeader } from './brand'

const PAGE_W = 11906
const MARGIN = 900
const CONTENT_W = PAGE_W - MARGIN * 2

const thin = { style: BorderStyle.SINGLE as const, size: 8, color: '000000' }
const thinBorders = { top: thin, bottom: thin, left: thin, right: thin }

function run(text: string, opts: { bold?: boolean; size?: number; font?: string } = {}) {
  return new TextRun({
    text,
    bold: opts.bold,
    size: opts.size ?? 21,
    font: opts.font ?? '宋体',
  })
}

function p(
  text: string,
  opts: { bold?: boolean; center?: boolean; before?: number; after?: number; size?: number; font?: string } = {},
) {
  return new Paragraph({
    alignment: opts.center ? AlignmentType.CENTER : AlignmentType.LEFT,
    spacing: { before: opts.before ?? 0, after: opts.after ?? 80, line: 360 },
    children: [run(text, { bold: opts.bold, size: opts.size, font: opts.font })],
  })
}

function h2(text: string) {
  return p(text, { bold: true, size: 22, font: '黑体', before: 160, after: 80 })
}

function bullets(items: string[]) {
  if (!items?.length) return [p('（略）')]
  return items.map((t, i) => p(`${i + 1}. ${t}`, { after: 40 }))
}

function processTable(plan: LessonPlan) {
  const rows = plan.process || []
  const colW = [1400, 1000, 2800, 2600, CONTENT_W - 7800]
  const widths = colW.map((w, i) => (i === colW.length - 1 ? CONTENT_W - colW.slice(0, -1).reduce((a, b) => a + b, 0) : w))
  const headers = ['环节', '时间', '教学内容', '教师活动', '学生活动']

  const cell = (text: string, i: number, head = false) =>
    new TableCell({
      borders: thinBorders,
      width: { size: widths[i], type: WidthType.DXA },
      shading: head ? { fill: 'F2F2F2', type: ShadingType.CLEAR } : undefined,
      verticalAlign: VerticalAlign.TOP,
      margins: { top: 60, bottom: 60, left: 60, right: 60 },
      children: [
        new Paragraph({
          spacing: { after: 0, line: 300 },
          children: [run(text || '　', { bold: head, size: 18 })],
        }),
      ],
    })

  return new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({
        children: headers.map((h, i) => cell(h, i, true)),
      }),
      ...rows.map(
        (r) =>
          new TableRow({
            children: [
              cell(r.stage || '', 0),
              cell(r.minutes != null ? `${r.minutes}′` : '', 1),
              cell([r.content, r.intent ? `（意图：${r.intent}）` : ''].filter(Boolean).join('\n'), 2),
              cell(r.teacherActivity || '', 3),
              cell(r.studentActivity || '', 4),
            ],
          }),
      ),
    ],
  })
}

export async function renderLessonDocx(
  plan: LessonPlan,
  brand?: BrandHeader | null,
  opts?: { audience?: 'teacher' | 'parent' },
): Promise<Blob> {
  const m = plan.meta || ({} as LessonPlan['meta'])
  const audience = opts?.audience || 'teacher'

  if (audience === 'parent') {
    return renderParentGuideDocx(plan, brand)
  }

  const children: Paragraph[] = []

  for (const line of brandLines(brand)) {
    children.push(p(line, { bold: true, center: true, size: 22, font: '黑体', after: 20 }))
  }

  children.push(
    p('教　案（教师版）', { bold: true, center: true, size: 36, font: '黑体', before: 80, after: 60 }),
  )
  children.push(
    p(
      `${m.edition || ''} · ${m.subject || ''} · ${m.grade || ''}年级${m.semester || ''} · ${m.unitName || ''}`,
      { center: true, after: 40, size: 18 },
    ),
  )
  children.push(p(`课题：${m.title || m.lessonName || ''}`, { bold: true, size: 24, after: 40 }))
  const school = brand?.schoolName?.trim() || m.school || ''
  const teacher = m.teacher || '________'
  children.push(
    p(
      `${school ? `学校：${school}　　` : ''}课时：第 ${m.periods || 1} 课时　　约 ${m.durationMin || 40} 分钟　　教师：${teacher}`,
      { after: 120, size: 19 },
    ),
  )

  children.push(h2('一、教学目标'))
  children.push(p('（一）知识与技能', { bold: true, after: 40, size: 20 }))
  children.push(...bullets(plan.objectives?.knowledge || []))
  children.push(p('（二）过程与方法', { bold: true, after: 40, size: 20, before: 60 }))
  children.push(...bullets(plan.objectives?.ability || []))
  children.push(p('（三）情感态度与价值观', { bold: true, after: 40, size: 20, before: 60 }))
  children.push(...bullets(plan.objectives?.emotion || []))

  children.push(h2('二、教学重难点'))
  children.push(p('重点：', { bold: true, after: 20 }))
  children.push(...bullets(plan.keyPoints || []))
  children.push(p('难点：', { bold: true, after: 20, before: 40 }))
  children.push(...bullets(plan.difficultPoints || []))

  children.push(h2('三、教学准备'))
  children.push(p(`教师：${(plan.preparation?.teacher || []).join('、') || '课件、板书'}`, { after: 40 }))
  children.push(p(`学生：${(plan.preparation?.student || []).join('、') || '课本、练习本'}`, { after: 40 }))

  children.push(h2('四、教学过程'))
  // table as any into children - Document accepts (Paragraph|Table)[]
  const docChildren: (Paragraph | Table)[] = [...children, processTable(plan)]

  docChildren.push(h2('五、板书设计'))
  for (const line of (plan.boardDesign || '（见黑板）').split('\n')) {
    docChildren.push(p(line, { after: 40 }))
  }

  docChildren.push(h2('六、作业布置'))
  docChildren.push(...bullets(plan.homework || []))

  docChildren.push(h2('七、教学反思'))
  docChildren.push(p(plan.reflection || '（课后填写）', { after: 80 }))

  const doc = new Document({
    styles: {
      default: { document: { run: { font: '宋体', size: 21 } } },
    },
    sections: [
      {
        properties: {
          page: {
            size: { width: PAGE_W, height: 16838 },
            margin: { top: MARGIN, right: MARGIN, bottom: MARGIN, left: MARGIN },
          },
        },
        headers: {
          default: new Header({
            children: [
              new Paragraph({
                alignment: AlignmentType.RIGHT,
                children: [run('教案', { size: 14 })],
              }),
            ],
          }),
        },
        footers: {
          default: new Footer({
            children: [
              new Paragraph({
                alignment: AlignmentType.CENTER,
                children: [
                  run('第 ', { size: 14 }),
                  new TextRun({ children: [PageNumber.CURRENT], size: 14, font: '宋体' }),
                  run(' 页', { size: 14 }),
                ],
              }),
            ],
          }),
        },
        children: docChildren,
      },
    ],
  })

  return Packer.toBlob(doc)
}

async function renderParentGuideDocx(
  plan: LessonPlan,
  brand?: BrandHeader | null,
): Promise<Blob> {
  const m = plan.meta || ({} as LessonPlan['meta'])
  const g = plan.parentGuide
  const children: (Paragraph | Table)[] = []

  for (const line of brandLines(brand)) {
    children.push(p(line, { bold: true, center: true, size: 22, font: '黑体', after: 20 }))
  }
  children.push(
    p('家长辅导手册', { bold: true, center: true, size: 34, font: '黑体', before: 80, after: 40 }),
  )
  children.push(
    p(
      `${m.edition || ''} · ${m.subject || ''} · ${m.grade || ''}年级${m.semester || ''} · ${m.unitName || ''}`,
      { center: true, after: 40, size: 18 },
    ),
  )
  children.push(p(`课题：${m.title || m.lessonName || ''}`, { bold: true, size: 24, after: 60 }))
  children.push(p(g?.summary || g?.title || '配合本课学习的家庭辅导建议。', { after: 100 }))

  children.push(h2('一、孩子这节课要会什么（大白话）'))
  children.push(...bullets(g?.goalsInPlain || []))

  children.push(h2('二、课前预习建议'))
  children.push(...bullets(g?.previewTips || []))

  children.push(h2('三、怎么陪（步骤）'))
  for (const step of g?.accompanySteps || []) {
    children.push(
      p(
        `${step.step || '步骤'}${step.minutes != null ? `（约 ${step.minutes} 分钟）` : ''}`,
        { bold: true, before: 80, after: 40 },
      ),
    )
    if (step.how) children.push(p(`怎么做：${step.how}`, { after: 30 }))
    if (step.say) children.push(p(`可以说：${step.say}`, { after: 40 }))
  }
  if (!g?.accompanySteps?.length) children.push(p('（略）'))

  children.push(h2('四、可以这样问'))
  children.push(...bullets(g?.keyQuestions || []))

  children.push(h2('五、常见卡点与纠正'))
  children.push(...bullets(g?.commonMistakes || []))

  children.push(h2('六、家庭小练习'))
  children.push(...bullets(g?.homePractice || plan.homework || []))

  children.push(h2('七、鼓励孩子'))
  children.push(p(g?.encourage || '今天你已经很努力了，我们一起慢慢进步。', { after: 80 }))

  const doc = new Document({
    styles: {
      default: { document: { run: { font: '宋体', size: 21 } } },
    },
    sections: [
      {
        properties: {
          page: {
            size: { width: PAGE_W, height: 16838 },
            margin: { top: MARGIN, right: MARGIN, bottom: MARGIN, left: MARGIN },
          },
        },
        headers: {
          default: new Header({
            children: [
              new Paragraph({
                alignment: AlignmentType.RIGHT,
                children: [run('家长辅导手册', { size: 14 })],
              }),
            ],
          }),
        },
        footers: {
          default: new Footer({
            children: [
              new Paragraph({
                alignment: AlignmentType.CENTER,
                children: [
                  run('第 ', { size: 14 }),
                  new TextRun({ children: [PageNumber.CURRENT], size: 14, font: '宋体' }),
                  run(' 页', { size: 14 }),
                ],
              }),
            ],
          }),
        },
        children,
      },
    ],
  })
  return Packer.toBlob(doc)
}

export function buildLessonPrintHtml(
  plan: LessonPlan,
  brand?: BrandHeader | null,
  opts?: { audience?: 'teacher' | 'parent' },
): string {
  const m = plan.meta || ({} as LessonPlan['meta'])
  const audience = opts?.audience || 'teacher'
  const esc = (s: string) =>
    String(s || '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
  const lis = (arr?: string[]) =>
    (arr || []).map((t) => `<li>${esc(t)}</li>`).join('') || '<li>（略）</li>'
  const brands = brandLines(brand)
    .map((l) => `<div class="school">${esc(l)}</div>`)
    .join('')

  if (audience === 'parent') {
    const g = plan.parentGuide
    const steps = (g?.accompanySteps || [])
      .map(
        (s) =>
          `<div class="step"><b>${esc(s.step || '')}${s.minutes != null ? `（约${s.minutes}分钟）` : ''}</b>
          <div>怎么做：${esc(s.how || '')}</div>
          <div class="muted">可以说：${esc(s.say || '')}</div></div>`,
      )
      .join('')
    return `<!DOCTYPE html><html lang="zh-CN"><head><meta charset="UTF-8"/><title>${esc(g?.title || '家长辅导手册')}</title>
<style>
@page{size:A4;margin:14mm}
body{font-family:"宋体",SimSun,serif;font-size:11pt;line-height:1.65;color:#000;margin:0}
h1{text-align:center;font-family:"黑体",SimHei,sans-serif;font-size:18pt;margin:0 0 8px}
h2{font-family:"黑体",SimHei,sans-serif;font-size:12pt;margin:14px 0 6px;border-bottom:1px solid #000;padding-bottom:2px}
.school{text-align:center;font-family:"黑体",SimHei,sans-serif;font-size:12pt;font-weight:bold;margin:0 0 4px}
.sub{text-align:center;font-size:10.5pt;margin-bottom:6px}
ul{margin:4px 0 8px 1.2em;padding:0}
.step{margin:8px 0;padding:6px 8px;border:1px solid #ddd;border-radius:4px}
.muted{color:#444;font-size:10pt;margin-top:4px}
.encourage{margin-top:12px;padding:10px;background:#fafafa;border-left:3px solid #333}
</style></head><body>
${brands}
<h1>家长辅导手册</h1>
<div class="sub">${esc(m.edition || '')} · ${esc(String(m.subject || ''))} · ${esc(String(m.grade || ''))}年级${esc(m.semester || '')}</div>
<div class="sub">课题：${esc(m.title || m.lessonName || '')}</div>
<p>${esc(g?.summary || '')}</p>
<h2>一、孩子这节课要会什么</h2><ul>${lis(g?.goalsInPlain)}</ul>
<h2>二、课前预习建议</h2><ul>${lis(g?.previewTips)}</ul>
<h2>三、怎么陪</h2>${steps || '<p>（略）</p>'}
<h2>四、可以这样问</h2><ul>${lis(g?.keyQuestions)}</ul>
<h2>五、常见卡点与纠正</h2><ul>${lis(g?.commonMistakes)}</ul>
<h2>六、家庭小练习</h2><ul>${lis(g?.homePractice || plan.homework)}</ul>
<div class="encourage"><b>鼓励：</b>${esc(g?.encourage || '今天你已经很努力了。')}</div>
</body></html>`
  }

  const processRows = (plan.process || [])
    .map(
      (r) => `<tr>
      <td>${esc(r.stage || '')}</td>
      <td>${r.minutes != null ? r.minutes + '′' : ''}</td>
      <td>${esc(r.content || '')}${r.intent ? `<br/><span class="muted">意图：${esc(r.intent)}</span>` : ''}</td>
      <td>${esc(r.teacherActivity || '')}</td>
      <td>${esc(r.studentActivity || '')}</td>
    </tr>`,
    )
    .join('')

  return `<!DOCTYPE html><html lang="zh-CN"><head><meta charset="UTF-8"/><title>${esc(m.title || '教案')}</title>
<style>
@page{size:A4;margin:14mm}
body{font-family:"宋体",SimSun,serif;font-size:11pt;line-height:1.6;color:#000;margin:0}
h1{text-align:center;font-family:"黑体",SimHei,sans-serif;font-size:18pt;margin:0 0 8px}
h2{font-family:"黑体",SimHei,sans-serif;font-size:12pt;margin:14px 0 6px;border-bottom:1px solid #000;padding-bottom:2px}
.school{text-align:center;font-family:"黑体",SimHei,sans-serif;font-size:12pt;font-weight:bold;margin:0 0 4px}
.sub,.meta{text-align:center;font-size:10.5pt;margin-bottom:6px}
table{width:100%;border-collapse:collapse;margin:8px 0;font-size:10pt}
th,td{border:1px solid #000;padding:5px 6px;vertical-align:top}
th{background:#f2f2f2}
ul{margin:4px 0 8px 1.2em;padding:0}
.muted{color:#444;font-size:9.5pt}
pre.board{white-space:pre-wrap;font-family:"宋体",SimSun,serif;border:1px solid #999;padding:8px;margin:6px 0}
</style></head><body>
${brands}
<h1>教　案（教师版）</h1>
<div class="sub">${esc(m.edition || '')} · ${esc(m.subject || '')} · ${m.grade || ''}年级${esc(m.semester || '')} · ${esc(m.unitName || '')}</div>
<div class="meta"><b>课题：</b>${esc(m.title || m.lessonName || '')}　　第 ${m.periods || 1} 课时　　约 ${m.durationMin || 40} 分钟</div>
<h2>一、教学目标</h2>
<p><b>（一）知识与技能</b></p><ul>${lis(plan.objectives?.knowledge)}</ul>
<p><b>（二）过程与方法</b></p><ul>${lis(plan.objectives?.ability)}</ul>
<p><b>（三）情感态度与价值观</b></p><ul>${lis(plan.objectives?.emotion)}</ul>
<h2>二、教学重难点</h2>
<p><b>重点：</b></p><ul>${lis(plan.keyPoints)}</ul>
<p><b>难点：</b></p><ul>${lis(plan.difficultPoints)}</ul>
<h2>三、教学准备</h2>
<p>教师：${esc((plan.preparation?.teacher || []).join('、'))}</p>
<p>学生：${esc((plan.preparation?.student || []).join('、'))}</p>
<h2>四、教学过程</h2>
<table><thead><tr><th>环节</th><th>时间</th><th>教学内容</th><th>教师活动</th><th>学生活动</th></tr></thead>
<tbody>${processRows}</tbody></table>
<h2>五、板书设计</h2>
<pre class="board">${esc(plan.boardDesign || '')}</pre>
<h2>六、作业布置</h2><ul>${lis(plan.homework)}</ul>
<h2>七、教学反思</h2>
<p>${esc(plan.reflection || '（课后填写）')}</p>
</body></html>`
}
