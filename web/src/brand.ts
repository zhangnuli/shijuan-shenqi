/** 校名 / 学期卷头 */
export interface BrandHeader {
  schoolName?: string
  academicYear?: string
  schoolTerm?: string
  className?: string
}

export function brandLines(b?: BrandHeader | null): string[] {
  if (!b) return []
  const lines: string[] = []
  if (b.schoolName?.trim()) lines.push(b.schoolName.trim())
  const yearTerm = [b.academicYear?.trim(), b.schoolTerm?.trim()].filter(Boolean).join(' ')
  if (yearTerm) lines.push(yearTerm)
  return lines
}

export function brandSubtitle(b?: BrandHeader | null): string {
  return brandLines(b).join(' · ')
}
