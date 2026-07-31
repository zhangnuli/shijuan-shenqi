import type { ExamItem } from './types'

/** Return the calculation-related section label used by screen, print, and DOCX layouts. */
export function isCalculationSection(type = '', title = ''): boolean {
  return /计算|口算|竖式|脱式|解方程|calc|直接写出/i.test(`${type}${title}`)
}

export function stripItemNumber(stem: string): string {
  return stem.trim().replace(/^[\d０-９]+[\.、．)）]\s]*/, '').trim()
}

/**
 * Short one-line arithmetic expressions can share a row. The score does not
 * decide the layout: a 3- or 5-point expression still should not waste a
 * whole line when it does not require a written solution.
 */
export function isCompactCalculationItem(
  item: Pick<ExamItem, 'stem' | 'score' | 'options'>,
  sectionType = '',
  sectionTitle = '',
): boolean {
  if (!isCalculationSection(sectionType, sectionTitle) || item.options?.length) return false
  const raw = item.stem || ''
  const stem = stripItemNumber(raw).replace(/[\s　]+/g, '')
  if (!stem || raw.includes('\n') || stem.length > 36) return false
  if (/[A-Za-z\u4e00-\u9fff]/.test(stem)) return false
  if (!/[0-9０-９]/.test(stem) || !/[+\-×÷*/＋－]/.test(stem)) return false
  return true
}
