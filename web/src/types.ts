export interface ProviderPreset {
  id: string
  name: string
  baseUrl: string
  defaultModel: string
  models: string[]
  apiStyle: string
}

export interface AppConfig {
  providerId: string
  apiBase: string
  apiKey: string
  apiKeyConfigured?: boolean
  model: string
  temperature: number
  exportDir: string
  defaultSubject?: string
  defaultEdition?: string
  defaultGrade?: number
  defaultSemester?: string
  defaultExamType?: string
  defaultDifficulty?: string
  /** 导出学生卷是否附参考答案页 */
  exportAttachAnswers?: boolean
  /** student | with_answers | both */
  exportMode?: string
  /** 文件名模板：{school}{grade}{subject}{title}{date}{variant}{type} */
  exportFilenamePattern?: string
  historyMax?: number
  schoolName?: string
  academicYear?: string
  schoolTerm?: string
  defaultClassName?: string
}

export interface HistoryEntry {
  id: string
  createdAt: number
  title: string
  summary: string
  /** exam | lessonPlan */
  kind?: string
  paper: ExamPaper
  formSnapshot?: Record<string, unknown>
}

export interface UnitInfo {
  id: string
  name: string
  lessons?: string[]
  points: string[]
}

export interface SourceInfo {
  platform?: string
  platformUrl?: string
  classroomUrl?: string
  materialUrl?: string
  elecEduUrl?: string
  catalogSite?: string
  catalogRef?: string
  note?: string
  editionLabel?: string
  subjectLabel?: string
  smarteduPathHint?: string
  entryCount?: number
  unitCount?: number
}

export interface CatalogItem {
  subject: string
  subjectLabel: string
  edition: string
  editionLabel: string
  grade: number
  semester: string
  semesterLabel: string
  title: string
  path: string
  units: UnitInfo[]
  source?: SourceInfo
  /** bundled | user */
  origin?: string
}

export interface DifficultyRatio {
  basic: number
  medium: number
  hard: number
}

export interface GenerateRequest {
  subject: string
  edition: string
  grade: number
  semester: string
  examType: string
  unitId?: string | null
  difficulty: string
  totalScore: number
  durationMin: number
  knowledgePath: string
  /** 单元测勾选的课时 */
  selectedLessons?: string[]
  /** 难度配比（百分比，约 100） */
  difficultyRatio?: DifficultyRatio
  /** 题库混组：本地骨架 + AI 换情境 */
  mixBank?: boolean
  /** 掺入校本收藏题 */
  useSchoolBank?: boolean
  schoolBankSnippets?: string[]
  /** 模板市集 id */
  templateId?: string | null
  structureOverride?: string | null
  templateHints?: string[]
}

export interface TemplateSectionSpec {
  type: string
  title: string
  score: number
  itemCount: number
  hint?: string
}

export interface PaperTemplate {
  kind: string
  id: string
  name: string
  description?: string
  subject?: string
  grades?: number[]
  examType?: string
  tags?: string[]
  durationMin?: number
  totalScore?: number
  sections?: TemplateSectionSpec[]
  processStages?: string[]
  promptHints?: string[]
  origin?: string
  createdAt?: number
}

export interface QualityIssue {
  level: 'error' | 'warn' | 'info' | string
  code: string
  message: string
  sectionIndex?: number | null
  itemIndex?: number | null
  itemId?: string | null
}

export interface QualityReport {
  score: number
  summary: string
  errorCount: number
  warnCount: number
  infoCount: number
  issues: QualityIssue[]
  specSummary: string
  knowledgeCount: number
  unmarkedCount: number
  mathMismatch: number
  mathChecked: number
}

export interface VerifyItemResult {
  sectionIndex: number
  itemIndex: number
  stem: string
  answer: string
  status: 'ok' | 'mismatch' | 'skip' | 'error' | string
  computed?: string | null
  message: string
}

export interface VerifyReport {
  total: number
  checked: number
  ok: number
  mismatch: number
  skipped: number
  items: VerifyItemResult[]
}

/** 双向细目表 */
export interface SpecKpRow {
  knowledgePoint: string
  itemCount: number
  totalScore: number
  scoreRatio: number
  itemIds: string[]
  sectionTitles: string[]
}

export interface SpecSectionRow {
  title: string
  sectionType: string
  itemCount: number
  score: number
  scoreRatio: number
}

export interface SpecTable {
  title: string
  subject: string
  grade: number
  totalScore: number
  totalItems: number
  scoredItems: number
  knowledgeRows: SpecKpRow[]
  sectionRows: SpecSectionRow[]
  uncoveredNote: string
  summary: string
}

export interface FavoriteItem {
  id: string
  createdAt: number
  stem: string
  options?: string[]
  answer?: string
  analysis?: string
  score?: number
  knowledgePoints?: string[]
  sectionType?: string
  subject?: string
  grade?: number
  edition?: string
  sourceTitle?: string
  tags?: string[]
}

export interface BankPaper {
  id: string
  createdAt: number
  title: string
  summary?: string
  subject?: string
  grade?: number
  paper: ExamPaper
}

export interface ParallelSet {
  kind?: string
  meta?: {
    title?: string
    count?: number
    variants?: string[]
  }
  papers: ExamPaper[]
}

export interface ReviewOutline {
  kind?: string
  meta?: {
    title?: string
    subject?: string
    grade?: number
    durationMin?: number
    source?: string
  }
  overview?: string
  knowledgeFocus?: string[]
  wrongSamples?: string[]
  process?: Array<{
    stage: string
    minutes?: number
    content?: string
    teacherActivity?: string
    studentActivity?: string
  }>
  points?: Array<{
    knowledgePoint: string
    errorPattern?: string
    keyExplain?: string
    boardNote?: string
    practice?: string[]
    minutes?: number
  }>
  homework?: string[]
  reflection?: string
}

export interface ExamItem {
  id: string
  stem: string
  options?: string[]
  answer?: string
  analysis?: string
  score?: number
  knowledgePoints?: string[]
}

export interface ExamSection {
  type: string
  title: string
  score: number
  items: ExamItem[]
}

export interface ExamPaper {
  meta: {
    edition: string
    subject: string
    grade: number
    semester: string
    examType: string
    title: string
    totalScore: number
    durationMin: number
    source?: string
    curriculumSource?: string
    /** A / B / C 平行卷标记 */
    variant?: string
  }
  sections: ExamSection[]
  kind?: string
}

export interface LessonProcessStep {
  stage: string
  minutes?: number
  content?: string
  teacherActivity?: string
  studentActivity?: string
  intent?: string
}

/** 家长辅导手册（家校共育） */
export interface ParentGuide {
  title?: string
  summary?: string
  goalsInPlain?: string[]
  previewTips?: string[]
  accompanySteps?: Array<{
    step: string
    minutes?: number
    how?: string
    say?: string
  }>
  keyQuestions?: string[]
  commonMistakes?: string[]
  homePractice?: string[]
  encourage?: string
}

export interface LessonPlan {
  kind?: string
  meta: {
    title: string
    edition?: string
    subject?: string
    grade?: number
    semester?: string
    unitName?: string
    lessonName?: string
    /** 新授课/练习课/复习课/讲评课 */
    lessonType?: string
    periods?: number
    durationMin?: number
    /** 教师版 / 教师版+家长版 */
    audience?: string
    school?: string
    teacher?: string
    error?: string
  }
  objectives?: {
    knowledge?: string[]
    ability?: string[]
    emotion?: string[]
  }
  keyPoints?: string[]
  difficultPoints?: string[]
  preparation?: {
    teacher?: string[]
    student?: string[]
  }
  process?: LessonProcessStep[]
  boardDesign?: string
  homework?: string[]
  reflection?: string
  /** 家长版辅导内容 */
  parentGuide?: ParentGuide
  error?: string
}

/** 单元全课时教案包 */
export interface LessonPlanBundle {
  kind?: string
  meta: {
    title: string
    unitName?: string
    edition?: string
    subject?: string
    grade?: number
    semester?: string
    count?: number
    lessonType?: string
  }
  plans: LessonPlan[]
}
