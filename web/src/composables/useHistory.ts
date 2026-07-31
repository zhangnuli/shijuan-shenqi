import { computed, type Ref } from 'vue'
import type { ExamPaper, HistoryEntry, HistoryWork } from '../types'

export type HistoryFilter = 'all' | 'exam' | 'lesson' | 'other'
export type HistoryKind = 'exam' | 'lesson' | 'other'

type HistoryShape = Partial<ExamPaper> & {
  kind?: string
  knowledgeFocus?: string[]
  process?: unknown[]
  objectives?: unknown
  plans?: unknown[]
}

function shapeOf(work: HistoryWork): HistoryShape {
  return work as HistoryShape
}

export function useHistory(
  historyList: Ref<HistoryEntry[]>,
  historyFilter: Ref<HistoryFilter>,
  historyKeyword: Ref<string>,
) {
  function historyKindOf(entry: HistoryEntry): HistoryKind {
    const work = shapeOf(entry.paper)
    const raw = (entry.kind || work.kind || '').toString()
    if (raw === 'reviewOutline' || work.knowledgeFocus) return 'other'
    if (raw === 'parallelSet') return 'other'
    if (
      raw === 'lessonPlan' ||
      raw === 'lessonPlanBundle' ||
      (Array.isArray(work.plans) && work.plans.length) ||
      (work.process && work.objectives)
    ) {
      return 'lesson'
    }
    return 'exam'
  }

  function historyKindLabel(entry: HistoryEntry): string {
    const work = shapeOf(entry.paper)
    const raw = (entry.kind || work.kind || '').toString()
    if (raw === 'lessonPlanBundle' || (Array.isArray(work.plans) && work.plans.length)) return '全课时'
    if (raw === 'reviewOutline' || work.knowledgeFocus) return '讲评'
    if (raw === 'parallelSet') return '平行卷'
    if (historyKindOf(entry) === 'lesson') return '教案'
    return '试卷'
  }

  function historyKindTagType(entry: HistoryEntry): 'primary' | 'warning' | 'success' | 'info' {
    const label = historyKindLabel(entry)
    if (label === '教案' || label === '全课时') return 'warning'
    if (label === '讲评') return 'success'
    if (label === '平行卷') return 'info'
    return 'primary'
  }

  const historyCounts = computed(() => {
    const counts = { all: historyList.value.length, exam: 0, lesson: 0, other: 0 }
    for (const entry of historyList.value) counts[historyKindOf(entry)] += 1
    return counts
  })

  const filteredHistory = computed(() => {
    let list = historyList.value
    if (historyFilter.value !== 'all') {
      list = list.filter((entry) => historyKindOf(entry) === historyFilter.value)
    }
    const keyword = historyKeyword.value.trim().toLowerCase()
    if (!keyword) return list
    return list.filter((entry) =>
      [entry.title, entry.summary, historyKindLabel(entry)]
        .filter(Boolean)
        .some((value) => value.toLowerCase().includes(keyword)),
    )
  })

  return {
    historyKindOf,
    historyKindLabel,
    historyKindTagType,
    historyCounts,
    filteredHistory,
  }
}
