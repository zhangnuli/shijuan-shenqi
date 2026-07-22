/**
 * 小学试卷 Word 排版（真题卷风格）
 * - 大标题 / 信息栏 / 得分栏
 * - 按题型：选择横排选项、口算分栏、应用题「解/答」留白
 * - 学生卷正文不含答案；答案单独分页
 */
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
  PageBreak,
} from 'docx'
import type { ExamItem, ExamPaper, ExamSection } from './types'
import { brandLines, type BrandHeader } from './brand'

const PAGE_W = 11906 // A4
const PAGE_H = 16838
const MARGIN = 720 // ~1.27cm，略收边距，多装内容
const CONTENT_W = PAGE_W - MARGIN * 2

const thin = { style: BorderStyle.SINGLE as const, size: 8, color: '000000' }
const thinBorders = { top: thin, bottom: thin, left: thin, right: thin }
const no = { style: BorderStyle.NONE as const, size: 0, color: 'FFFFFF' }
const noBorders = { top: no, bottom: no, left: no, right: no }
const bottomOnly = {
  top: no,
  left: no,
  right: no,
  bottom: { style: BorderStyle.SINGLE as const, size: 12, color: '000000' },
}

type DocChild = Paragraph | Table

function run(
  text: string,
  opts: { bold?: boolean; size?: number; font?: string; color?: string } = {},
) {
  return new TextRun({
    text,
    bold: opts.bold,
    size: opts.size ?? 21, // 10.5pt
    font: opts.font ?? '宋体',
    color: opts.color ?? '000000',
  })
}

function para(
  children: TextRun[] | string,
  opts: {
    center?: boolean
    before?: number
    after?: number
    line?: number
    indent?: number
  } = {},
) {
  const kids =
    typeof children === 'string' ? [run(children)] : children.length ? children : [run('')]
  return new Paragraph({
    alignment: opts.center ? AlignmentType.CENTER : AlignmentType.LEFT,
    spacing: {
      before: opts.before ?? 0,
      after: opts.after ?? 40,
      line: opts.line ?? 300,
    },
    indent: opts.indent ? { firstLine: opts.indent } : undefined,
    children: kids,
  })
}

/** 空白行（适中行距，避免整页太空） */
function emptyLines(n: number, after = 60, line = 360): Paragraph[] {
  return Array.from({ length: n }, () =>
    new Paragraph({
      spacing: { after, line },
      children: [run('　')],
    }),
  )
}

/** 带下划线的答题行 */
function answerLines(n: number): Paragraph[] {
  return Array.from({ length: n }, () =>
    new Paragraph({
      spacing: { after: 40, line: 360 },
      border: bottomOnly,
      children: [run('　')],
    }),
  )
}

/** 大题标题：一、填空题（…） */
function sectionHead(title: string) {
  return new Paragraph({
    spacing: { before: 120, after: 60, line: 300 },
    children: [run(title, { bold: true, size: 22, font: '黑体' })],
  })
}

/** 班级姓名得分栏 */
function infoBar() {
  const labels = ['班级', '姓名', '学号', '得分']
  const w = Math.floor(CONTENT_W / 4)
  const widths = [w, w, w, CONTENT_W - w * 3]
  return new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({
        children: labels.map((lab, i) =>
          new TableCell({
            borders: noBorders,
            width: { size: widths[i], type: WidthType.DXA },
            margins: { top: 40, bottom: 40, left: 40, right: 40 },
            children: [
              para([run(`${lab}：`, { size: 20 }), run('____________', { size: 20 })], {
                after: 20,
              }),
            ],
          }),
        ),
      }),
    ],
  })
}

/** 得分登记表 */
function scoreTable(sections: ExamSection[], total: number) {
  const cn = ['一', '二', '三', '四', '五', '六', '七', '八', '九', '十']
  const heads = ['题号', ...sections.map((_, i) => cn[i] || String(i + 1)), '总分']
  const n = heads.length
  const col = Math.floor(CONTENT_W / n)
  const widths = heads.map((_, i) => (i === n - 1 ? CONTENT_W - col * (n - 1) : col))

  const mk = (text: string, i: number, head = false) =>
    new TableCell({
      borders: thinBorders,
      width: { size: widths[i], type: WidthType.DXA },
      shading: head ? { fill: 'F2F2F2', type: ShadingType.CLEAR } : undefined,
      verticalAlign: VerticalAlign.CENTER,
      margins: { top: 50, bottom: 50, left: 40, right: 40 },
      children: [
        new Paragraph({
          alignment: AlignmentType.CENTER,
          children: [run(text, { bold: head, size: 18 })],
        }),
      ],
    })

  return new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({ children: heads.map((h, i) => mk(h, i, true)) }),
      new TableRow({
        children: heads.map((_, i) =>
          mk(i === 0 ? '得分' : i === n - 1 ? String(total || '') : '', i),
        ),
      }),
      new TableRow({
        children: heads.map((_, i) => mk(i === 0 ? '阅卷' : '', i)),
      }),
    ],
  })
}

function ensureNumbered(stem: string, index: number): string {
  const t = (stem || '').trim()
  if (!t) return `${index}.`
  // 已有 1. / 1、 / （1） 等编号
  if (/^(\d+[\.、．)]|（\d+）|\(\d+\))/.test(t)) return t
  return `${index}. ${t}`
}

/** 选择题选项：两列或一行 */
function optionsBlock(options: string[]): DocChild[] {
  const opts = options.map((o) => o.trim()).filter(Boolean)
  if (!opts.length) return []

  // 4 个选项：两行两列
  if (opts.length === 4) {
    const half = Math.floor(CONTENT_W / 2)
    const widths = [half, CONTENT_W - half]
    const row = (a: string, b: string) =>
      new TableRow({
        children: [a, b].map(
          (t, i) =>
            new TableCell({
              borders: noBorders,
              width: { size: widths[i], type: WidthType.DXA },
              margins: { top: 20, bottom: 20, left: 200, right: 40 },
              children: [para(t, { after: 20 })],
            }),
        ),
      })
    return [
      new Table({
        width: { size: CONTENT_W, type: WidthType.DXA },
        columnWidths: widths,
        rows: [row(opts[0], opts[1]), row(opts[2], opts[3])],
      }),
    ]
  }

  // 其他：同一行用全角空格分隔
  return [para(opts.join('　　　'), { after: 80, indent: 200 })]
}

/** 口算/直接写得数：一行多题 */
function calcGrid(items: string[], cols = 4): Table {
  const colW = Math.floor(CONTENT_W / cols)
  const widths = Array.from({ length: cols }, (_, i) =>
    i === cols - 1 ? CONTENT_W - colW * (cols - 1) : colW,
  )
  const rows: TableRow[] = []
  for (let i = 0; i < items.length; i += cols) {
    const slice = items.slice(i, i + cols)
    while (slice.length < cols) slice.push('')
    rows.push(
      new TableRow({
        children: slice.map(
          (expr, j) =>
            new TableCell({
              borders: noBorders,
              width: { size: widths[j], type: WidthType.DXA },
              margins: { top: 40, bottom: 60, left: 40, right: 40 },
              children: [
                para(expr ? `${expr.replace(/＝\s*$/, '').replace(/=\s*$/, '')}＝` : '　', {
                  after: 20,
                }),
                // 口算得数书写区（一行即可）
                para('　　', { after: 40, line: 320 }),
              ],
            }),
        ),
      }),
    )
  }
  return new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows,
  })
}

/** 竖式/脱式计算框 */
function calcBoxes(exprs: string[], cols = 2): Table {
  const colW = Math.floor(CONTENT_W / cols)
  const widths = Array.from({ length: cols }, (_, i) =>
    i === cols - 1 ? CONTENT_W - colW * (cols - 1) : colW,
  )
  const rows: TableRow[] = []
  for (let i = 0; i < exprs.length; i += cols) {
    const slice = exprs.slice(i, i + cols)
    while (slice.length < cols) slice.push('')
    rows.push(
      new TableRow({
        children: slice.map(
          (expr, j) =>
            new TableCell({
              borders: thinBorders,
              width: { size: widths[j], type: WidthType.DXA },
              margins: { top: 60, bottom: 80, left: 80, right: 80 },
              children: expr
                ? [
                    para(expr, { center: true, after: 40 }),
                    para('＝', { center: true, after: 30 }),
                    // 竖式/脱式步骤区：适中留白
                    ...emptyLines(4, 50, 360),
                  ]
                : [para('　')],
            }),
        ),
      }),
    )
  }
  return new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows,
  })
}

function isCalcSection(type: string, title: string) {
  const t = `${type}${title}`
  return /calc|计算|口算|竖式|脱式|直接写出/.test(t)
}

function isChoiceSection(type: string, title: string) {
  return /choice|选择/.test(`${type}${title}`)
}

function isJudgeSection(type: string, title: string) {
  return /judge|判断/.test(`${type}${title}`)
}

function isProblemSection(type: string, title: string) {
  return /problem|解决|应用|操作|实践|画图/.test(`${type}${title}`)
}

function isWritingSection(type: string, title: string) {
  return /writing|习作|作文|小练笔/.test(`${type}${title}`)
}

function isReadingSection(type: string, title: string) {
  return /reading|阅读/.test(`${type}${title}`)
}

/** 从计算大题的 stem 里拆出口算小题 */
function splitInlineCalcs(stem: string): string[] {
  // 匹配 6×7＝  48÷6＝  或 6×7=  等
  const parts = stem
    .replace(/^[\d]+[\.、．)\s]*/, '')
    .split(/[\n\r]+|　　+|\s{2,}/)
    .map((s) => s.trim())
    .filter(Boolean)

  const calcs: string[] = []
  for (const p of parts) {
    // 一行多个：用全角/半角空格再拆
    const bits = p.split(/(?<=＝|=)\s+|(?<=[＝=])　+|\s{2,}|(?<=[0-9])\s+(?=[0-9])/)
    for (const b of bits) {
      const t = b.trim()
      if (/[0-9０-９].*[+\-×÷\*\/＋－]/.test(t) || /[＝=]/.test(t)) {
        calcs.push(t.endsWith('＝') || t.endsWith('=') ? t : `${t}＝`)
      }
    }
  }
  // 再用更松的正则扫整段
  if (calcs.length < 2) {
    const re = /[0-9０-９.]+[\s]*[+\-×÷\*\u00d7\u00f7＋－][\s]*[0-9０-９.]+/g
    const found = stem.match(re) || []
    if (found.length >= 2) {
      return found.map((x) => (x.includes('＝') || x.includes('=') ? x : `${x}＝`))
    }
  }
  return calcs
}

function renderItem(
  item: ExamItem,
  index: number,
  sec: ExamSection,
  withAnswers: boolean,
): DocChild[] {
  const out: DocChild[] = []
  const type = sec.type || ''
  const title = sec.title || ''
  const stem = ensureNumbered(item.stem || '', index)
  const lines = stem.split(/\n/)

  // —— 计算题：尽量分栏 ——
  if (isCalcSection(type, title) && !withAnswers) {
    const inline = splitInlineCalcs(item.stem || '')
    if (
      inline.length >= 3 &&
      /直接|口算|写出得数/.test(`${item.stem || ''}${title || ''}`)
    ) {
      out.push(para(lines[0] || `${index}. 直接写出得数`, { after: 60 }))
      out.push(calcGrid(inline, Math.min(4, inline.length)))
      return out
    }
    if (
      inline.length >= 1 &&
      (/竖式|脱式|简便/.test(item.stem || '') || /竖式|脱式/.test(title))
    ) {
      out.push(para(lines[0] || `${index}.`, { after: 60 }))
      // 多道算式
      const exprs =
        inline.length >= 2
          ? inline
          : lines
              .slice(1)
              .map((l) => l.replace(/^[（(]?\d+[)）.\、．]\s*/, '').trim())
              .filter((l) => l && /[0-9]/.test(l))
      if (exprs.length) {
        out.push(calcBoxes(exprs, exprs.length >= 3 ? 2 : Math.min(2, exprs.length)))
        return out
      }
    }
  }

  // 常规题干
  lines.forEach((line, i) => {
    out.push(
      para(line, {
        after: i === lines.length - 1 ? 40 : 20,
        line: 400,
      }),
    )
  })

  // 选择题选项
  if (item.options?.length || isChoiceSection(type, title)) {
    out.push(...optionsBlock(item.options || []))
  }

  // 判断题补括号
  if (isJudgeSection(type, title) && !/[（(]\s*[）)]|（　　）|\( {0,4}\)/.test(stem)) {
    out.push(para('（　　）', { after: 40 }))
  }

  if (withAnswers) {
    out.push(
      para([run('【答案】', { bold: true, size: 19 }), run(item.answer ?? '略', { size: 19 })], {
        after: 20,
      }),
    )
    if (item.analysis) {
      out.push(
        para([run('【解析】', { bold: true, size: 18 }), run(item.analysis, { size: 18 })], {
          after: 80,
        }),
      )
    }
    return out
  }

  // 学生卷答题区：按题型适度留白；数学应用/计算不画下划线
  if (isProblemSection(type, title)) {
    out.push(...emptyLines(5, 40, 360))
  } else if (isWritingSection(type, title)) {
    out.push(...answerLines(12))
  } else if (isReadingSection(type, title)) {
    out.push(...answerLines(3))
  } else if (isCalcSection(type, title)) {
    out.push(...emptyLines(2, 40, 320))
  } else if (isChoiceSection(type, title) || isJudgeSection(type, title)) {
    // 选择/判断：题干+选项即可，仅留极小间距
    out.push(para('', { after: 40, line: 240 }))
  } else if (/fill|填空|拼音|积累|字词|默写/.test(`${type}${title}`)) {
    // 填空一般已在题干中留空，不再叠答题线
    out.push(para('', { after: 40, line: 240 }))
  } else {
    out.push(...emptyLines(1, 40, 320))
  }

  return out
}

function renderSection(sec: ExamSection, withAnswers: boolean): DocChild[] {
  const out: DocChild[] = [sectionHead(sec.title || '大题')]
  const items = sec.items || []

  // 整大题都是短口算时，合并成一张分栏表
  if (
    !withAnswers &&
    isCalcSection(sec.type || '', sec.title || '') &&
    items.length >= 4 &&
    items.every((it) => {
      const s = it.stem || ''
      return s.length < 40 && /[0-9].*[+\-×÷]/.test(s)
    })
  ) {
    const exprs = items.map((it, i) => {
      const raw = (it.stem || '').replace(/^[\d]+[\.、．)\s]*/, '').trim()
      return raw.includes('＝') || raw.includes('=') ? raw : `${raw}＝`
    })
    out.push(calcGrid(exprs, 4))
    return out
  }

  items.forEach((item, idx) => {
    out.push(...renderItem(item, idx + 1, sec, withAnswers))
  })
  return out
}

/**
 * @param withAnswers 是否整卷为答案版（题干+答案）
 * @param attachAnswerPage 学生卷末尾是否附参考答案页（仅 withAnswers=false 时有效）
 * @param brand 校名/学期卷头
 */
export async function renderExamDocx(
  paper: ExamPaper,
  withAnswers: boolean,
  attachAnswerPage = true,
  brand?: BrandHeader | null,
): Promise<Blob> {
  const { meta, sections } = paper
  const children: DocChild[] = []

  // —— 卷头（学校 / 学期）——
  for (const line of brandLines(brand)) {
    children.push(
      new Paragraph({
        alignment: AlignmentType.CENTER,
        spacing: { before: 40, after: 20, line: 320 },
        children: [run(line, { bold: true, size: 22, font: '黑体' })],
      }),
    )
  }

  children.push(
    new Paragraph({
      alignment: AlignmentType.CENTER,
      spacing: { before: 40, after: 20, line: 320 },
      children: [
        run(withAnswers ? `${meta.title || '试卷'}（参考答案）` : meta.title || '试卷', {
          bold: true,
          size: 32,
          font: '黑体',
        }),
      ],
    }),
  )

  const sub = [
    meta.edition,
    meta.subject,
    meta.grade ? `${meta.grade}年级` : '',
    meta.semester,
    meta.examType,
  ]
    .filter(Boolean)
    .join('·')

  children.push(
    para(
      `${sub}　　满分${meta.totalScore ?? 100}分　　时间${meta.durationMin ?? 60}分钟`,
      { center: true, after: 60 },
    ),
  )

  if (!withAnswers) {
    // 若有默认班级，写在信息栏提示
    if (brand?.className?.trim()) {
      children.push(
        para(`班级：${brand.className.trim()}　　姓名：________　　学号：________　　得分：________`, {
          center: true,
          after: 40,
        }),
      )
    } else {
      children.push(infoBar())
    }
    children.push(scoreTable(sections || [], meta.totalScore ?? 100))
    children.push(
      para(
        [
          run('注意事项：', { bold: true, size: 17 }),
          run('1.认真审题，书写工整；2.计算题注意验算；3.应用题写清解题过程与答句。', {
            size: 17,
          }),
        ],
        { after: 80, before: 40 },
      ),
    )
  } else {
    children.push(
      para('（教师用）请对照学生卷批改。', { center: true, after: 80 }),
    )
  }

  // —— 正文 ——
  for (const sec of sections || []) {
    children.push(...renderSection(sec, withAnswers))
  }

  if (!withAnswers) {
    children.push(
      para('—— 试卷结束，请仔细检查 ——', {
        center: true,
        before: 120,
        after: 40,
      }),
    )
    // 可选：学生卷末附参考答案页
    if (attachAnswerPage) {
      children.push(new Paragraph({ children: [new PageBreak()] }))
      children.push(
        new Paragraph({
          alignment: AlignmentType.CENTER,
          spacing: { before: 80, after: 160 },
          children: [
            run(`${meta.title || '试卷'} · 参考答案`, { bold: true, size: 32, font: '黑体' }),
          ],
        }),
      )
      for (const sec of sections || []) {
        children.push(sectionHead(sec.title || ''))
        ;(sec.items || []).forEach((item, idx) => {
          const no = item.id || `${idx + 1}`
          children.push(
            para(
              [run(`${no}　`, { bold: true, size: 20 }), run(item.answer ?? '略', { size: 20 })],
              { after: 40 },
            ),
          )
          if (item.analysis) {
            children.push(para(`　　解析：${item.analysis}`, { after: 60 }))
          }
        })
      }
    }
  }

  const doc = new Document({
    styles: {
      default: {
        document: {
          run: { font: '宋体', size: 21 },
        },
      },
    },
    sections: [
      {
        properties: {
          page: {
            size: { width: PAGE_W, height: PAGE_H },
            margin: {
              top: MARGIN,
              right: MARGIN,
              bottom: MARGIN,
              left: MARGIN,
            },
          },
        },
        // 页眉去掉，避免与卷头重复、挤占正文
        headers: {
          default: new Header({
            children: [new Paragraph({ children: [] })],
          }),
        },
        footers: {
          default: new Footer({
            children: [
              new Paragraph({
                alignment: AlignmentType.CENTER,
                border: {
                  top: { style: BorderStyle.SINGLE, size: 4, color: '999999', space: 6 },
                },
                spacing: { before: 60 },
                children: [
                  // 页码字段用 Arial，宋体下部分 Word 会显示异常
                  new TextRun({
                    text: withAnswers ? '参考答案 · 第 ' : '第 ',
                    size: 16,
                    font: '宋体',
                    color: '444444',
                  }),
                  new TextRun({
                    children: [PageNumber.CURRENT],
                    size: 16,
                    font: 'Arial',
                    color: '444444',
                  }),
                  new TextRun({
                    text: ' / ',
                    size: 16,
                    font: 'Arial',
                    color: '444444',
                  }),
                  new TextRun({
                    children: [PageNumber.TOTAL_PAGES],
                    size: 16,
                    font: 'Arial',
                    color: '444444',
                  }),
                  new TextRun({
                    text: ' 页',
                    size: 16,
                    font: '宋体',
                    color: '444444',
                  }),
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

export function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}
