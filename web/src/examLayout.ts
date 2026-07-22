import type { ExamItem } from './types'

/** Return the calculation-related section label used by screen, print, and DOCX layouts. */
export function isCalculationSection(type = '', title = ''): boolean {
  return /计算|口算|竖式|脱式|解方程|calc|直接写出/i.test(`${type}${title}`)
}

export function stripItemNumber(stem: string): string {
  return stem.trim().replace(/^[\d０-９]+[\.、．)）]\s]*/, '').trim()
}

/**
 * Short one-line calculations need only a small answer space. Higher-scored
 * calculations are kept as worked problems even when their stem is an expression.
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
  if (item.score != null && item.score > 2) return false
  if (/[A-Za-z\u4e00-\u9fff]/.test(stem)) return false
  if (!/[0-9０-９]/.test(stem) || !/[+\-×÷*/＋－]/.test(stem)) return false
  return true
}
