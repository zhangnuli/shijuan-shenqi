<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Setting,
  Document,
  MagicStick,
  Download,
  Refresh,
  Link,
  Upload,
  Printer,
  ArrowDown,
  ZoomIn,
  ZoomOut,
  Delete,
} from '@element-plus/icons-vue'
import type {
  AppConfig,
  AppSjResource,
  AppSjSyncReport,
  CatalogItem,
  ExamPaper,
  ExamItem,
  FavoriteItem,
  BankPaper,
  GenerateRequest,
  HistoryEntry,
  LessonPlan,
  LessonPlanBundle,
  ParallelSet,
  ProviderPreset,
  ReviewOutline,
  SpecTable,
  QualityReport,
  PaperTemplate,
  VerifyReport,
} from './types'
import { renderExamDocx } from './renderDocx'
import { renderLessonDocx, buildLessonPrintHtml } from './renderLessonDocx'
import { saveDocxFile } from './saveFile'
import { printHtml } from './printExam'
import { buildEbookPrintHtml, type EbookUnitPages } from './buildEbookPrintHtml'
import { isCalculationSection, isCompactCalculationItem } from './examLayout'
import type { BrandHeader } from './brand'
import { aiSteps, aiTips, formatFriendlyError, useAiProgress } from './composables/useAiProgress'
import { invokeCommand as invoke } from './services/tauriClient'
import AiProgressDialog from './components/AiProgressDialog.vue'

const loading = ref(false)
const updatingCurriculum = ref(false)
const syncingQuestionBank = ref(false)
const publicBankResources = ref<AppSjResource[]>([])
const publicBankLastReport = ref<AppSjSyncReport | null>(null)
const catalog = ref<CatalogItem[]>([])
const presets = ref<ProviderPreset[]>([])
const paper = ref<ExamPaper | null>(null)
const lessonPlan = ref<LessonPlan | null>(null)
/** exam | lesson */
const workMode = ref<'exam' | 'lesson'>('exam')
const lessonName = ref('')
const lessonPeriods = ref(1)
/** new | practice | review | feedback */
const lessonType = ref('new')
/** 写教案时是否附带家长版 */
const includeParentGuide = ref(true)
/** 预览：教师版 | 家长版 */
const lessonAudienceView = ref<'teacher' | 'parent'>('teacher')
const settingsVisible = ref(false)
const historyVisible = ref(false)
const printPreviewVisible = ref(false)
const printPreviewHtml = ref('')
const printPreviewIsAnswer = ref(false)
const curriculumDir = ref('')
/** 课标浏览 */
const curriculumBrowserVisible = ref(false)
const browserSubject = ref('chinese')
const browserEdition = ref('renjiao')
const browserGrade = ref(3)
const browserSemester = ref('shang')
const browserUnitId = ref('')
const browserKeyword = ref('')
const curriculumDiff = ref<CurriculumDiffReport | null>(null)
const diffLoading = ref(false)
const onboardingVisible = ref(false)
const onboardingStep = ref(0)
const appInfo = ref<{ version?: string; appDataDir?: string; updateNote?: string; offlineNote?: string } | null>(null)

/** 自有站电子书单元页图打印 */
const ebookDialogVisible = ref(false)
const ebookLoading = ref(false)
const ebookFetchingPages = ref(false)
const ebookForm = reactive({
  url: 'https://www.100875.com.cn/show/eBookAndTeacher.html?resId=b613783f20cf42c689844433cce53c81&bookId=120170718181613001800&firstNum=a9f3f20e2e5749a3a94140ae57f883e0&contributeId=9220',
  baseUrl: 'https://www.100875.com.cn',
  resId: '',
  bookId: '',
  contributeId: '',
  maxPages: 30,
})
const ebookCatalog = ref<{
  bookName: string
  subjectName: string
  items: Array<{ bookId: string; cataName: string; deep: string }>
} | null>(null)
const ebookUnitPages = ref<EbookUnitPages | null>(null)

const ebookUnitOptions = computed(() => {
  const items = ebookCatalog.value?.items || []
  const units = items.filter((x) => String(x.deep) === '5')
  return units.length ? units : items
})

const ONBOARD_KEY = 'shijuan_onboarded_v1'

interface CurriculumDiffReport {
  path: string
  subject: string
  edition: string
  grade: number
  semester: string
  hasBundled: boolean
  hasUser: boolean
  summary: string
  units: Array<{
    unitId: string
    unitName: string
    status: string
    bundledLessons: string[]
    userLessons: string[]
    added: string[]
    removed: string[]
  }>
  addedUnits: string[]
  removedUnits: string[]
}
const historyList = ref<HistoryEntry[]>([])
const regeneratingKey = ref('') // sectionIndex-itemIndex
const makingB = ref(false)
const lessonLoading = ref(false)
/** 右侧同时有卷和案时的预览页签 */
const previewTab = ref<'exam' | 'lesson'>('exam')
const examViewMode = ref<'student' | 'teacher' | 'answers'>('teacher')
const previewZoom = ref(1)
const linking = ref(false)
const linkProgress = ref('')
/** 单元全课时教案包 */
const lessonBundle = ref<LessonPlanBundle | null>(null)
const bundleIndex = ref(0)
const outputCenterVisible = ref(false)
const headerPreviewVisible = ref(false)
const verifying = ref(false)
const verifyReport = ref<VerifyReport | null>(null)
const verifyDialogVisible = ref(false)

/** P2 · 教研 */
const specTable = ref<SpecTable | null>(null)
const specDialogVisible = ref(false)
const makingParallel = ref(false)
const parallelSet = ref<ParallelSet | null>(null)
const parallelIndex = ref(0)
const bankVisible = ref(false)
const bankTab = ref<'items' | 'papers'>('items')
const favorites = ref<FavoriteItem[]>([])
const bankPapers = ref<BankPaper[]>([])
const reviewOutline = ref<ReviewOutline | null>(null)
const reviewDialogVisible = ref(false)
const reviewLoading = ref(false)
const wrongKeys = ref<string[]>([])
const reviewKpText = ref('')
const reviewUseAi = ref(true)
const qualityReport = ref<QualityReport | null>(null)
const qualityDialogVisible = ref(false)
const qualityLoading = ref(false)
const redrillLoading = ref(false)
const templateMarketVisible = ref(false)
const templateList = ref<PaperTemplate[]>([])
const templateFilter = ref<'all' | 'paper' | 'lesson' | 'mine'>('all')
const templateKeyword = ref('')
const selectedTemplateId = ref('')
const applyingTemplate = ref(false)
/** 侧栏高级参数默认收起，避免表单过长 */
const advancedOpen = ref<string[]>([])

function setPreviewZoom(next: number) {
  previewZoom.value = Math.min(1.3, Math.max(0.75, Number(next.toFixed(2))))
}

function onExamToolCommand(cmd: string) {
  if (cmd === 'lessonTpl') {
    onLessonTemplate()
    return
  }
  if (cmd === 'unitTpl') {
    onGenerateUnitAllLessons(false)
    return
  }
  if (cmd === 'browseKb') {
    openCurriculumBrowser()
    return
  }
  if (cmd === 'ebookPrint') {
    openEbookPrintDialog()
    return
  }
  const map: Record<string, () => void> = {
    b: () => makePaperB(),
    verify: () => onVerifyMath(),
    spec: () => onBuildSpecTable(),
    quality: () => onQualityCheck(),
    parallel: () => makeParallelABC(),
    review: () => openReviewPanel(),
    redrill: () => onGenerateRedrill(),
    bank: () => openBank(),
    saveBank: () => saveCurrentToBank(),
    saveTpl: () => saveCurrentAsTemplate(),
    shell: () => onTemplateGenerate(),
    sync: () => onUpdateCurriculum(),
    prefs: () => savePrefsOnly(),
  }
  map[cmd]?.()
}

const {
  panelVisible: aiPanelVisible,
  status: aiStatus,
  stepIndex: aiStepIndex,
  tipIndex: aiTipIndex,
  elapsed: aiElapsed,
  errorText: aiErrorText,
  successSummary: aiSuccessSummary,
  progress: aiProgress,
  start: startAiPanel,
  succeed: stopAiPanelSuccess,
  fail: stopAiPanelError,
} = useAiProgress()

const examTypeLabel = computed(() => {
  const m: Record<string, string> = {
    unit: '单元测试',
    midterm: '期中模拟',
    final: '期末模拟',
    oral: '口算专项',
    lesson: '课时练习',
    homework: '课后作业',
    redrill: '错题再练',
  }
  return m[form.examType] || '模拟卷'
})

/** 是否需要选单元（专项/单元测） */
const needsUnit = computed(() =>
  ['unit', 'oral', 'lesson', 'homework', 'redrill'].includes(form.examType),
)

const aiTargetLabel = computed(
  () =>
    `${form.grade}年级${form.semester === 'shang' ? '上册' : '下册'} · ${subjectLabel.value} · ${examTypeLabel.value} · ${form.difficulty} · 配比 ${form.ratioBasic}/${form.ratioMedium}/${form.ratioHard}`,
)

const form = reactive({
  subject: 'math',
  edition: 'beishida',
  grade: 3,
  semester: 'shang',
  examType: 'unit',
  unitId: '' as string,
  difficulty: '标准',
  totalScore: 100,
  durationMin: 40,
  /** 单元测勾选课时 */
  selectedLessons: [] as string[],
  /** 难度配比 % */
  ratioBasic: 40,
  ratioMedium: 40,
  ratioHard: 20,
  /** 题库混组 */
  mixBank: true,
  /** 校本收藏参与组卷 */
  useSchoolBank: false,
  /** 当前选用的模板市集 id */
  templateId: '' as string,
  /** 结构约束：默认允许 AI 适度调整模板 */
  structureMode: 'adaptive' as 'strict' | 'adaptive' | 'free',
})

const config = reactive<AppConfig>({
  providerId: 'xai',
  apiBase: 'https://api.x.ai/v1',
  apiKey: '',
  apiKeyConfigured: false,
  model: 'grok-4.5',
  temperature: 0.4,
  exportDir: '',
  defaultSubject: 'math',
  defaultEdition: 'beishida',
  defaultGrade: 3,
  defaultSemester: 'shang',
  defaultExamType: 'unit',
  defaultDifficulty: '标准',
  exportAttachAnswers: true,
  exportMode: 'with_answers',
  exportFilenamePattern: '{school}{grade}年级-{subject}-{title}-{date}',
  historyMax: 30,
  schoolName: '',
  academicYear: '',
  schoolTerm: '',
  defaultClassName: '',
})

watch(
  () => config.exportMode,
  (mode) => {
    config.exportAttachAnswers = mode !== 'student'
  },
)

const historyFilter = ref<'all' | 'exam' | 'lesson' | 'other'>('all')
const historyKeyword = ref('')
const hasApiKey = computed(() => Boolean(config.apiKey.trim() || config.apiKeyConfigured))
const isLocalAi = computed(() => {
  const base = config.apiBase.trim().toLowerCase()
  return base.includes('127.0.0.1') || base.includes('localhost')
})
const aiReady = computed(() => hasApiKey.value || isLocalAi.value)
const aiReadyHint = computed(() =>
  isLocalAi.value && !hasApiKey.value ? '本地模型已就绪' : 'AI 接口已配置',
)
const structureModeLabel = computed(() => {
  if (!aiReady.value) return form.subject === 'math' ? '本地题库组卷' : '本地结构卷'
  if (form.structureMode === 'strict') return '按严格模板组卷'
  if (form.structureMode === 'free') return '自由智能组卷'
  return form.templateId ? '按模板智能组卷' : '智能组卷'
})

/** 统一历史类型：exam | lesson | other */
function historyKindOf(h: HistoryEntry): 'exam' | 'lesson' | 'other' {
  const paper = h.paper as ExamPaper &
    LessonPlan &
    LessonPlanBundle &
    ReviewOutline & { kind?: string }
  const raw = (h.kind || paper?.kind || '').toString()
  if (raw === 'reviewOutline' || paper?.knowledgeFocus) return 'other'
  if (raw === 'parallelSet') return 'other'
  if (raw === 'lessonPlan' || raw === 'lessonPlanBundle') return 'lesson'
  if (Array.isArray(paper?.plans) && paper.plans.length) return 'lesson'
  if (paper?.process && paper?.objectives) return 'lesson'
  if (paper?.sections) return 'exam'
  return 'exam'
}

function historyKindLabel(h: HistoryEntry): string {
  const paper = h.paper as ExamPaper &
    LessonPlan &
    LessonPlanBundle &
    ReviewOutline & { kind?: string }
  const raw = (h.kind || paper?.kind || '').toString()
  if (raw === 'lessonPlanBundle' || (Array.isArray(paper?.plans) && paper.plans.length)) return '全课时'
  if (raw === 'reviewOutline' || paper?.knowledgeFocus) return '讲评'
  if (raw === 'parallelSet') return '平行卷'
  if (historyKindOf(h) === 'lesson') return '教案'
  return '试卷'
}

function historyKindTagType(h: HistoryEntry): 'primary' | 'warning' | 'success' | 'info' {
  const lab = historyKindLabel(h)
  if (lab === '教案' || lab === '全课时') return 'warning'
  if (lab === '讲评') return 'success'
  if (lab === '平行卷') return 'info'
  return 'primary'
}

const historyCounts = computed(() => {
  const list = historyList.value || []
  let exam = 0
  let lesson = 0
  let other = 0
  for (const h of list) {
    const k = historyKindOf(h)
    if (k === 'exam') exam++
    else if (k === 'lesson') lesson++
    else other++
  }
  return { all: list.length, exam, lesson, other }
})

const filteredHistory = computed(() => {
  let list = historyList.value || []
  if (historyFilter.value !== 'all') {
    list = list.filter((h) => historyKindOf(h) === historyFilter.value)
  }
  const q = historyKeyword.value.trim().toLowerCase()
  if (q) {
    list = list.filter(
      (h) =>
        (h.title || '').toLowerCase().includes(q) ||
        (h.summary || '').toLowerCase().includes(q) ||
        historyKindLabel(h).includes(q),
    )
  }
  return list
})

function currentBrand(): BrandHeader {
  return {
    schoolName: config.schoolName || '',
    academicYear: config.academicYear || '',
    schoolTerm: config.schoolTerm || '',
    className: config.defaultClassName || '',
  }
}

const headerPreviewText = computed(() => {
  const b = currentBrand()
  const lines = [
    b.schoolName?.trim(),
    [b.academicYear?.trim(), b.schoolTerm?.trim()].filter(Boolean).join(' '),
    b.className?.trim() ? `班级：${b.className.trim()}` : '',
  ].filter(Boolean)
  return lines.length ? lines.join('\n') : '（未设置校名卷头，可在接口设置中填写）'
})

const activeBundlePlan = computed(() => {
  if (!lessonBundle.value?.plans?.length) return null
  const i = Math.min(bundleIndex.value, lessonBundle.value.plans.length - 1)
  return lessonBundle.value.plans[i] || null
})

/** 当前预览的教案（单份或全课时包中的一份） */
const displayLesson = computed(() => {
  if (previewTab.value === 'lesson' || workMode.value === 'lesson') {
    return activeBundlePlan.value || lessonPlan.value
  }
  return lessonPlan.value
})

const editionOptions = computed(() => {
  if (form.subject === 'math') {
    return [
      { value: 'beishida', label: '北师大版' },
      { value: 'renjiao', label: '人教版' },
      { value: 'sujiao', label: '苏教版' },
    ]
  }
  if (form.subject === 'english') {
    return [{ value: 'renjiao', label: '人教版' }]
  }
  return [{ value: 'renjiao', label: '人教统编版' }]
})

const gradeOptions = computed(() => {
  if (form.subject === 'english') {
    return [3, 4, 5, 6]
  }
  return [1, 2, 3, 4, 5, 6]
})

const subjectLabel = computed(() => {
  const m: Record<string, string> = { math: '数学', chinese: '语文', english: '英语' }
  return m[form.subject] || form.subject
})

const ratioSum = computed(
  () => form.ratioBasic + form.ratioMedium + form.ratioHard,
)

/** 难度滑条：0 基础 / 1 标准 / 2 拔高 */
const difficultyLevel = computed({
  get() {
    if (form.difficulty === '基础') return 0
    if (form.difficulty === '拔高') return 2
    return 1
  },
  set(v: number) {
    form.difficulty = v <= 0 ? '基础' : v >= 2 ? '拔高' : '标准'
  },
})

const difficultyMarks = {
  0: '基础',
  1: '标准',
  2: '拔高',
}

const currentPack = computed(() => {
  return (
    catalog.value.find(
      (c) =>
        c.subject === form.subject &&
        c.edition === form.edition &&
        c.grade === form.grade &&
        c.semester === form.semester,
    ) || null
  )
})

const unitOptions = computed(() => currentPack.value?.units || [])

const selectedUnit = computed(() =>
  unitOptions.value.find((u) => u.id === form.unitId) || null,
)

async function openExternal(url: string) {
  try {
    await openUrl(url)
  } catch {
    window.open(url, '_blank')
  }
}

async function openSmartedu(kind: 'home' | 'classroom' | 'material' | 'elec') {
  const s = currentPack.value?.source
  const map: Record<string, string> = {
    home: s?.platformUrl || 'https://basic.smartedu.cn/',
    classroom: s?.classroomUrl || 'https://basic.smartedu.cn/syncClassroom',
    material: s?.materialUrl || 'https://basic.smartedu.cn/tchMaterial',
    elec: s?.elecEduUrl || 'https://basic.smartedu.cn/elecEdu',
  }
  await openExternal(map[kind])
}

async function openCatalogRef() {
  const url = currentPack.value?.source?.catalogRef || 'https://www.dzkbw.org/'
  await openExternal(url)
}

const browserEditionOptions = computed(() => {
  if (browserSubject.value === 'math') {
    return [
      { value: 'beishida', label: '北师大版' },
      { value: 'renjiao', label: '人教版' },
      { value: 'sujiao', label: '苏教版' },
    ]
  }
  return [{ value: 'renjiao', label: browserSubject.value === 'english' ? '人教版' : '人教统编版' }]
})

const browserGradeOptions = computed(() =>
  browserSubject.value === 'english' ? [3, 4, 5, 6] : [1, 2, 3, 4, 5, 6],
)

const browserPack = computed(() => {
  return (
    catalog.value.find(
      (c) =>
        c.subject === browserSubject.value &&
        c.edition === browserEdition.value &&
        c.grade === browserGrade.value &&
        c.semester === browserSemester.value,
    ) || null
  )
})

const browserUnits = computed(() => {
  const units = browserPack.value?.units || []
  const q = browserKeyword.value.trim().toLowerCase()
  if (!q) return units
  return units.filter((u) => {
    const hay = [u.name, ...(u.lessons || []), ...(u.points || [])].join(' ').toLowerCase()
    return hay.includes(q)
  })
})

const browserActiveUnit = computed(() => {
  const units = browserUnits.value
  if (!units.length) return null
  return units.find((u) => u.id === browserUnitId.value) || units[0]
})

const catalogPackList = computed(() => {
  // 按学科版本分组摘要
  return [...catalog.value].sort((a, b) => {
    if (a.subject !== b.subject) return a.subject.localeCompare(b.subject)
    if (a.edition !== b.edition) return a.edition.localeCompare(b.edition)
    if (a.grade !== b.grade) return a.grade - b.grade
    return a.semester.localeCompare(b.semester)
  })
})

function openCurriculumBrowser() {
  browserSubject.value = form.subject
  browserEdition.value = form.edition
  browserGrade.value = form.grade
  browserSemester.value = form.semester
  browserUnitId.value = form.unitId || selectedUnit.value?.id || ''
  browserKeyword.value = ''
  curriculumBrowserVisible.value = true
}

watch(browserSubject, (s) => {
  browserEdition.value = s === 'math' ? 'beishida' : 'renjiao'
  if (s === 'english' && browserGrade.value < 3) browserGrade.value = 3
})

watch(browserPack, (p) => {
  if (p?.units?.length) {
    if (!p.units.some((u) => u.id === browserUnitId.value)) {
      browserUnitId.value = p.units[0].id
    }
  } else {
    browserUnitId.value = ''
  }
})

function applyBrowserToForm() {
  form.subject = browserSubject.value
  form.edition = browserEdition.value
  form.grade = browserGrade.value
  form.semester = browserSemester.value
  if (browserUnitId.value) form.unitId = browserUnitId.value
  nextTick(() => syncLessonsFromUnit())
  curriculumBrowserVisible.value = false
  ElMessage.success('已套用到组卷参数')
}

function copyBrowserUnitText() {
  const u = browserActiveUnit.value
  const p = browserPack.value
  if (!u || !p) {
    ElMessage.warning('无单元可复制')
    return
  }
  const lines = [
    `${p.editionLabel || ''} · ${p.subjectLabel || ''} · ${p.title}`,
    `来源：${p.origin === 'user' ? '已同步' : '内置'} · ${p.source?.catalogRef || ''}`,
    '',
    `【${u.name}】`,
    `课时（${u.lessons?.length || 0}）：`,
    ...(u.lessons?.length ? u.lessons.map((x, i) => `${i + 1}. ${x}`) : ['（无课时明细）']),
    '',
    `要点（${u.points?.length || 0}）：`,
    ...(u.points || []).map((x) => `- ${x}`),
  ]
  navigator.clipboard?.writeText(lines.join('\n')).then(
    () => ElMessage.success('单元课标已复制，可粘贴对比'),
    () => ElMessage.warning('复制失败'),
  )
}

function copyBrowserAllText() {
  const p = browserPack.value
  if (!p) {
    ElMessage.warning('无课标包')
    return
  }
  const lines = [
    `${p.editionLabel} · ${p.subjectLabel} · ${p.title}`,
    `origin=${p.origin || 'bundled'} path=${p.path}`,
    `catalog=${p.source?.catalogRef || ''}`,
    `单元数 ${p.units.length}`,
    '',
  ]
  for (const u of p.units) {
    lines.push(`## ${u.name}`)
    for (const les of u.lessons || []) lines.push(`  - ${les}`)
    lines.push('')
  }
  navigator.clipboard?.writeText(lines.join('\n')).then(
    () => ElMessage.success('全册课标目录已复制'),
    () => ElMessage.warning('复制失败'),
  )
}

async function openBrowserCatalogSource() {
  const url =
    browserPack.value?.source?.catalogRef ||
    browserPack.value?.source?.catalogSite ||
    'https://www.dzkbw.org/'
  await openExternal(url)
}

function selectBrowserPack(c: CatalogItem) {
  browserSubject.value = c.subject
  browserEdition.value = c.edition
  browserGrade.value = c.grade
  browserSemester.value = c.semester
  browserUnitId.value = c.units[0]?.id || ''
  curriculumDiff.value = null
}

async function runCurriculumDiff() {
  const p = browserPack.value
  if (!p?.path) {
    ElMessage.warning('请先选择课标包')
    return
  }
  diffLoading.value = true
  try {
    curriculumDiff.value = await invoke<CurriculumDiffReport>('diff_curriculum', { path: p.path })
    ElMessage.success(curriculumDiff.value.summary)
  } catch (e) {
    ElMessage.error(`对照失败：${e}`)
  } finally {
    diffLoading.value = false
  }
}

function diffStatusLabel(s: string) {
  const m: Record<string, string> = {
    same: '一致',
    changed: '有差异',
    onlyBundled: '仅内置',
    onlyUser: '仅同步',
  }
  return m[s] || s
}

function diffStatusType(s: string) {
  if (s === 'same') return 'success'
  if (s === 'changed') return 'warning'
  if (s === 'onlyUser') return 'primary'
  return 'info'
}

async function cancelAiGeneration() {
  try {
    await invoke('cancel_generation')
  } catch {
    /* ignore */
  }
  loading.value = false
  lessonLoading.value = false
  linking.value = false
  makingParallel.value = false
  stopAiPanelError('已取消生成（若网络请求已发出，可能仍会在后台结束）')
}

function finishOnboarding() {
  try {
    localStorage.setItem(ONBOARD_KEY, '1')
  } catch {
    /* ignore */
  }
  onboardingVisible.value = false
}

async function exportRuntimeLog() {
  try {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const path = await save({
      title: '导出运行日志',
      defaultPath: `试卷神器-日志-${new Date().toISOString().slice(0, 10)}.txt`,
      filters: [{ name: '文本', extensions: ['txt'] }],
    })
    if (!path) return
    const out = await invoke<string>('export_runtime_log', { targetPath: path })
    ElMessage.success(`日志已导出：${out}`)
  } catch (e) {
    ElMessage.error(`导出日志失败：${e}`)
  }
}

async function showAppInfo() {
  try {
    appInfo.value = await invoke('get_app_info')
    await ElMessageBox.confirm(
      `版本：${appInfo.value?.version || '—'}\n数据目录：${appInfo.value?.appDataDir || '—'}\n\n${appInfo.value?.updateNote || ''}\n\n${appInfo.value?.offlineNote || ''}`,
      '关于试卷神器',
      {
        confirmButtonText: '检查更新',
        cancelButtonText: '关闭',
        distinguishCancelAndClose: true,
      },
    )
    await checkAppUpdate(true)
  } catch (e) {
    if (e === 'cancel' || e === 'close') return
    // 用户点了检查更新但失败
    if (String(e).includes('更新') || String(e).includes('endpoint') || String(e).includes('连接')) {
      ElMessage.error(String(e))
    }
  }
}

const updateChecking = ref(false)
const updateProgress = ref(0)

async function checkAppUpdate(fromAbout = false) {
  updateChecking.value = true
  updateProgress.value = 0
  try {
    const { checkForAppUpdate } = await import('./services/updater')
    const result = await checkForAppUpdate()
    if (!result.available) {
      if (fromAbout) ElMessage.success('当前已是最新版本（或更新服务未配置）')
      return
    }
    await ElMessageBox.confirm(
      `发现新版本 v${result.version}\n\n${result.body || '建议更新以获得修复与新功能。'}\n\n更新将下载安装包并重启应用。`,
      '发现更新',
      {
        confirmButtonText: '立即更新',
        cancelButtonText: '稍后',
        type: 'info',
      },
    )
    ElMessage.info('正在下载更新…')
    await result.install((pct) => {
      updateProgress.value = pct
    })
  } catch (e) {
    if (e === 'cancel' || e === 'close') return
    ElMessage.error(`检查/安装更新失败：${e}`)
  } finally {
    updateChecking.value = false
  }
}

watch(
  () => form.subject,
  (s) => {
    form.edition = s === 'math' ? 'beishida' : 'renjiao'
    if (s === 'english' && form.grade < 3) form.grade = 3
    form.durationMin = s === 'math' ? 60 : s === 'english' ? 60 : 90
  },
)

watch(
  () => form.examType,
  (t) => {
    if (t === 'oral') form.durationMin = 15
    else if (t === 'lesson' || t === 'homework' || t === 'redrill') form.durationMin = 20
    else if (t === 'unit') form.durationMin = form.subject === 'math' ? 40 : form.subject === 'english' ? 40 : 45
    else if (t === 'midterm') form.durationMin = form.subject === 'math' ? 60 : 80
    else form.durationMin = form.subject === 'math' ? 90 : 100
    if (t === 'oral' && form.subject === 'math') form.totalScore = 100
    else if (t === 'lesson' || t === 'homework' || t === 'redrill') form.totalScore = form.totalScore > 60 ? 50 : form.totalScore
  },
)

/** 当前单元下可选课时（无 lessons 时用单元名兜底） */
const lessonOptions = computed(() => {
  const u = selectedUnit.value
  if (!u) return [] as string[]
  if (u.lessons?.length) return [...u.lessons]
  return u.name ? [u.name] : []
})

/** 随单元/课标切换，刷新课时范围默认勾选 */
function syncLessonsFromUnit() {
  const u = selectedUnit.value
  if (!u) {
    form.selectedLessons = []
    lessonName.value = ''
    return
  }
  const opts = u.lessons?.length ? [...u.lessons] : u.name ? [u.name] : []
  // 默认全选当前单元课时；并清掉不属于本单元的旧勾选
  form.selectedLessons = opts
  lessonName.value = opts[0] || u.name || ''
}

watch(currentPack, (p) => {
  if (p?.units?.length) {
    const stillValid = p.units.some((u) => u.id === form.unitId)
    // 换年级/版本时各册常用相同 id（u1/u2），即使 unitId 没变也要刷新课时
    if (!stillValid) {
      form.unitId = p.units[0].id
    }
    nextTick(() => syncLessonsFromUnit())
  } else {
    form.unitId = ''
    form.selectedLessons = []
    lessonName.value = ''
  }
})

watch(
  () => form.unitId,
  () => {
    syncLessonsFromUnit()
  },
)

// 课标异步加载完成后：若已有 unitId 但课时仍空，补一次
watch(
  () => lessonOptions.value.join('\0'),
  (key, prev) => {
    if (!key || key === prev) return
    const opts = lessonOptions.value
    if (!opts.length) return
    // 当前勾选与选项完全无关时（换单元/换册），重置为全选
    const stillOk =
      form.selectedLessons.length > 0 &&
      form.selectedLessons.every((n) => opts.includes(n))
    if (!stillOk) {
      form.selectedLessons = [...opts]
    }
    if (!lessonName.value || !opts.includes(lessonName.value)) {
      lessonName.value = opts[0] || ''
    }
  },
)

function applyPreset(id: string) {
  const p = presets.value.find((x) => x.id === id)
  if (!p) return
  config.providerId = p.id
  config.apiBase = p.baseUrl
  config.model = p.defaultModel
}

const modelOptions = computed(() => {
  const p = presets.value.find((x) => x.id === config.providerId)
  const list = p?.models?.length ? [...p.models] : []
  if (config.model && !list.includes(config.model)) list.unshift(config.model)
  return list
})

function applyFormDefaultsFromConfig() {
  if (config.defaultSubject) form.subject = config.defaultSubject
  if (config.defaultEdition) form.edition = config.defaultEdition
  if (config.defaultGrade) form.grade = config.defaultGrade
  if (config.defaultSemester) form.semester = config.defaultSemester
  if (config.defaultExamType) form.examType = config.defaultExamType
  if (config.defaultDifficulty) form.difficulty = config.defaultDifficulty
  // 按时长惯例
  if (form.examType === 'unit') form.durationMin = form.subject === 'math' ? 40 : form.subject === 'english' ? 40 : 45
  else if (form.examType === 'midterm') form.durationMin = form.subject === 'math' ? 60 : 80
  else form.durationMin = form.subject === 'math' ? 90 : 100
}

async function refreshHistory() {
  try {
    historyList.value = await invoke<HistoryEntry[]>('history_list')
  } catch {
    historyList.value = []
  }
}

async function saveToHistory(p: ExamPaper) {
  try {
    await invoke('history_add', {
      paper: p,
      formSnapshot: { ...form },
    })
    await refreshHistory()
  } catch (e) {
    console.warn('保存历史失败', e)
  }
}

async function loadAll() {
  try {
    const [c, p, cfg, dir, info, bankResources] = await Promise.all([
      invoke<CatalogItem[]>('get_catalog'),
      invoke<ProviderPreset[]>('get_provider_presets'),
      invoke<AppConfig>('get_config'),
      invoke<string>('get_curriculum_dir').catch(() => ''),
      invoke<{ version?: string; appDataDir?: string }>('get_app_info').catch(() => null),
      invoke<AppSjResource[]>('list_appsj_resources').catch(() => []),
    ])
    catalog.value = c
    presets.value = p
    curriculumDir.value = dir
    if (info) appInfo.value = info
    publicBankResources.value = bankResources
    Object.assign(config, {
      exportAttachAnswers: true,
      exportMode: 'with_answers',
      exportFilenamePattern: '{school}{grade}年级-{subject}-{title}-{date}',
      historyMax: 30,
      defaultGrade: 3,
      defaultSemester: 'shang',
      defaultExamType: 'unit',
      ...cfg,
    })
    // 旧配置无 exportMode 时，从开关推导
    if (!cfg.exportMode) {
      config.exportMode = cfg.exportAttachAnswers === false ? 'student' : 'with_answers'
    }
    applyFormDefaultsFromConfig()
    await nextTick()
    if (!form.unitId && currentPack.value?.units?.[0]) {
      form.unitId = currentPack.value.units[0].id
    }
    syncLessonsFromUnit()
    await refreshHistory()
    try {
      if (!localStorage.getItem(ONBOARD_KEY)) {
        onboardingVisible.value = true
        onboardingStep.value = 0
      }
    } catch {
      /* ignore */
    }
  } catch (e) {
    ElMessage.error(`初始化失败：${e}`)
  }
}

async function onSyncQuestionBank() {
  const scope = `${form.grade}年级${form.semester === 'shang' ? '上册' : '下册'}${subjectLabel.value}${examTypeLabel.value}`
  try {
    await ElMessageBox.confirm(
      `将从 appsj.szxuexiao.com 读取“${scope}”的公开网页内容，缓存试卷结构、考点和典型题素材。不会自动下载百度网盘文件；无明确答案的内容只作为命题素材。`,
      '同步公开题库',
      { type: 'info', confirmButtonText: '开始同步', cancelButtonText: '取消' },
    )
  } catch {
    return
  }
  syncingQuestionBank.value = true
  try {
    const report = await invoke<AppSjSyncReport>('sync_appsj_resources', {
      request: {
        subject: form.subject,
        grade: form.grade,
        semester: form.semester,
        examType: form.examType,
        unitName: needsUnit.value ? selectedUnit.value?.name || '' : '',
        maxPages: 3,
        maxItems: 20,
      },
    })
    publicBankLastReport.value = report
    publicBankResources.value = await invoke<AppSjResource[]>('list_appsj_resources')
    if (report.ok) {
      ElMessage.success(report.message)
    } else {
      ElMessage.warning(report.message)
    }
    if (report.failed?.length) console.warn('公开题库同步部分失败', report.failed)
  } catch (e) {
    ElMessage.error(`公开题库同步失败：${e}`)
  } finally {
    syncingQuestionBank.value = false
  }
}

async function onUpdateCurriculum() {
  try {
    await ElMessageBox.confirm(
      '将从 dzkbw.org 全量同步小学课标目录到本机（语文人教、数学北师大/人教/苏教、英语人教 3–6）。优先 2026/2024 新版，含古诗分篇；失败册次回退旧站。约 2–4 分钟，请保持网络畅通。',
      '同步课标',
      { type: 'info', confirmButtonText: '开始', cancelButtonText: '取消' },
    )
  } catch {
    return
  }

  updatingCurriculum.value = true
  try {
    const res = await invoke<{
      ok: boolean
      message: string
      updated: string[]
      failed: string[]
      dataDir: string
    }>('update_curriculum')
    curriculumDir.value = res.dataDir || curriculumDir.value
    if (res.ok) {
      ElMessage.success(res.message || '课标已更新')
      if (res.failed?.length) {
        console.warn('课标更新部分失败', res.failed)
      }
      catalog.value = await invoke<CatalogItem[]>('get_catalog')
    } else {
      ElMessage.error(res.message || '课标更新失败')
    }
  } catch (e) {
    ElMessage.error(`课标更新失败：${e}`)
  } finally {
    updatingCurriculum.value = false
  }
}

function buildReq(): GenerateRequest {
  const pack = currentPack.value
  if (!pack) throw new Error('未找到对应教材知识点包')
  // 配比归一：和为 0 时回退默认
  let b = form.ratioBasic
  let m = form.ratioMedium
  let h = form.ratioHard
  const sum = b + m + h
  if (sum <= 0) {
    b = 40
    m = 40
    h = 20
  }
  return {
    subject: form.subject,
    edition: form.edition,
    grade: form.grade,
    semester: form.semester,
    examType: form.examType,
    unitId: needsUnit.value ? form.unitId : null,
    difficulty: form.difficulty,
    totalScore: form.totalScore,
    durationMin: form.durationMin,
    knowledgePath: pack.path,
    selectedLessons:
      needsUnit.value && form.selectedLessons.length
        ? [...form.selectedLessons]
        : [],
    difficultyRatio: { basic: b, medium: m, hard: h },
    mixBank: form.mixBank,
    useSchoolBank: form.useSchoolBank,
    schoolBankSnippets: [],
    publicBankSnippets: [],
    templateId: form.templateId || null,
    structureOverride: null,
    structureMode: form.structureMode,
    templateHints: [],
  }
}

const filteredTemplates = computed(() => {
  let list = templateList.value
  if (templateFilter.value === 'paper') {
    list = list.filter((t) => t.kind !== 'lessonTemplate')
  } else if (templateFilter.value === 'lesson') {
    list = list.filter((t) => t.kind === 'lessonTemplate')
  } else if (templateFilter.value === 'mine') {
    list = list.filter((t) => t.origin === 'user')
  }
  // 优先当前学科
  const sub = form.subject
  list = [...list].sort((a, b) => {
    const as = !a.subject || a.subject === sub || a.kind === 'lessonTemplate' ? 0 : 1
    const bs = !b.subject || b.subject === sub || b.kind === 'lessonTemplate' ? 0 : 1
    return as - bs
  })
  const q = templateKeyword.value.trim().toLowerCase()
  if (q) {
    list = list.filter(
      (t) =>
        t.name.toLowerCase().includes(q) ||
        (t.description || '').toLowerCase().includes(q) ||
        (t.tags || []).some((x) => x.toLowerCase().includes(q)),
    )
  }
  return list
})

const activeTemplate = computed(() =>
  templateList.value.find((t) => t.id === (form.templateId || selectedTemplateId.value)) || null,
)

async function refreshTemplates() {
  templateList.value = await invoke<PaperTemplate[]>('list_templates')
}

async function openTemplateMarket() {
  try {
    await refreshTemplates()
    selectedTemplateId.value = form.templateId || ''
    templateMarketVisible.value = true
  } catch (e) {
    ElMessage.error(`打开模板市集失败：${e}`)
  }
}

function selectTemplateCard(t: PaperTemplate) {
  selectedTemplateId.value = t.id
}

function applyTemplateParams(t: PaperTemplate) {
  if (t.kind === 'lessonTemplate') {
    workMode.value = 'lesson'
    if (t.examType === 'practice' || t.examType === 'review' || t.examType === 'feedback' || t.examType === 'new') {
      lessonType.value = t.examType
    }
    if (t.durationMin) form.durationMin = t.durationMin
    form.templateId = t.id
    return
  }
  workMode.value = 'exam'
  if (t.subject && ['math', 'chinese', 'english'].includes(t.subject)) {
    form.subject = t.subject
  }
  if (t.examType) form.examType = t.examType
  if (t.durationMin) form.durationMin = t.durationMin
  if (t.totalScore) form.totalScore = t.totalScore
  form.templateId = t.id
}

async function applyTemplateShell(t?: PaperTemplate) {
  const tpl = t || templateList.value.find((x) => x.id === selectedTemplateId.value)
  if (!tpl) {
    ElMessage.warning('请先选择模板')
    return
  }
  if (tpl.kind === 'lessonTemplate') {
    applyTemplateParams(tpl)
    // 教案：用现有单课模板生成
    try {
      applyingTemplate.value = true
      const req = buildLessonReq()
      const result = await invoke<LessonPlan>('generate_lesson_template', { req })
      lessonPlan.value = result
      lessonBundle.value = null
      previewTab.value = 'lesson'
      templateMarketVisible.value = false
      ElMessage.success(`已套用教案结构：${tpl.name}`)
    } catch (e) {
      ElMessage.error(String(e))
    } finally {
      applyingTemplate.value = false
    }
    return
  }
  applyingTemplate.value = true
  try {
    applyTemplateParams(tpl)
    const pack = currentPack.value
    const unitName = selectedUnit.value?.name || ''
    const paperResult = await invoke<ExamPaper>('apply_market_template', {
      req: {
        templateId: tpl.id,
        subject: form.subject,
        edition: form.edition,
        grade: form.grade,
        semester: form.semester,
        unitName,
        knowledgePath: pack?.path || '',
      },
    })
    paper.value = paperResult
    verifyReport.value = null
    parallelSet.value = null
    workMode.value = 'exam'
    previewTab.value = 'exam'
    templateMarketVisible.value = false
    ElMessage.success(`已套用空壳：${tpl.name}（占位题，可用「按模板组卷」填充）`)
  } catch (e) {
    ElMessage.error(`套用失败：${e}`)
  } finally {
    applyingTemplate.value = false
  }
}

async function generateWithSelectedTemplate() {
  const tpl = templateList.value.find((x) => x.id === selectedTemplateId.value)
    || (form.templateId ? templateList.value.find((x) => x.id === form.templateId) : null)
  if (!tpl) {
    ElMessage.warning('请先选择试卷模板')
    return
  }
  if (tpl.kind === 'lessonTemplate') {
    applyTemplateParams(tpl)
    templateMarketVisible.value = false
    await onGenerateLesson()
    return
  }
  applyTemplateParams(tpl)
  templateMarketVisible.value = false
  await onAiGenerate()
}

function clearSelectedTemplate() {
  form.templateId = ''
  selectedTemplateId.value = ''
  ElMessage.info('已取消模板约束')
}

async function saveCurrentAsTemplate() {
  if (!paper.value) {
    ElMessage.warning('请先有一份试卷')
    return
  }
  try {
    const { value } = await ElMessageBox.prompt('模板名称', '另存为模板', {
      inputValue: `${paper.value.meta?.title || '我的卷'}·结构`,
      confirmButtonText: '保存',
      cancelButtonText: '取消',
    })
    const t = await invoke<PaperTemplate>('save_paper_as_template', {
      paper: paper.value,
      name: value,
    })
    await refreshTemplates()
    form.templateId = t.id
    ElMessage.success(`已保存到「我的模板」：${t.name}`)
  } catch (e) {
    if (e !== 'cancel') ElMessage.error(String(e))
  }
}

async function importTemplateFile() {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.json,application/json'
  input.onchange = async () => {
    const file = input.files?.[0]
    if (!file) return
    try {
      const data = JSON.parse(await file.text())
      const t = await invoke<PaperTemplate>('import_market_template', { template: data })
      await refreshTemplates()
      templateFilter.value = 'mine'
      selectedTemplateId.value = t.id
      ElMessage.success(`已导入：${t.name}`)
    } catch (e) {
      ElMessage.error(`导入失败：${e}`)
    }
  }
  input.click()
}

async function exportTemplateFile(id: string) {
  try {
    const data = await invoke<PaperTemplate>('export_market_template', { id })
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
    const name = `${data.name || id}.json`.replace(/[\\/:*?"<>|]/g, '_')
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = name
    a.click()
    URL.revokeObjectURL(url)
    ElMessage.success('模板 JSON 已下载')
  } catch (e) {
    ElMessage.error(String(e))
  }
}

async function removeUserTemplate(id: string) {
  try {
    await invoke('delete_market_template', { id })
    if (form.templateId === id) form.templateId = ''
    if (selectedTemplateId.value === id) selectedTemplateId.value = ''
    await refreshTemplates()
    ElMessage.success('已删除')
  } catch (e) {
    ElMessage.error(String(e))
  }
}

function subjectTag(s?: string) {
  if (!s) return '通用'
  const m: Record<string, string> = { math: '数学', chinese: '语文', english: '英语' }
  return m[s] || s
}

function pad2(n: number) {
  return n < 10 ? `0${n}` : String(n)
}

function buildExportFilename(opts?: { answers?: boolean; suffix?: string }) {
  const p = paper.value
  const meta = p?.meta
  const d = new Date()
  const date = `${d.getFullYear()}${pad2(d.getMonth() + 1)}${pad2(d.getDate())}`
  const school = (config.schoolName || '').trim()
  const pattern =
    config.exportFilenamePattern ||
    '{school}{grade}年级-{subject}-{title}-{date}'
  let name = pattern
    .replace(/\{school\}/g, school ? `${school}-` : '')
    .replace(/\{grade\}/g, String(meta?.grade || form.grade || ''))
    .replace(/\{subject\}/g, meta?.subject || subjectLabel.value)
    .replace(/\{title\}/g, meta?.title || '试卷')
    .replace(/\{date\}/g, date)
    .replace(/\{variant\}/g, meta?.variant || '')
    .replace(/\{type\}/g, meta?.examType || examTypeLabel.value)
  if (opts?.answers) name += '-答案'
  if (opts?.suffix) name += opts.suffix
  return name.replace(/[\\/:*?"<>|]+/g, '_').replace(/-+/g, '-').replace(/^-|-$/g, '')
}

async function onQualityCheck() {
  if (!paper.value) {
    ElMessage.warning('请先生成试卷')
    return
  }
  qualityLoading.value = true
  try {
    const report = await invoke<QualityReport>('inspect_exam_paper', { paper: paper.value })
    qualityReport.value = report
    qualityDialogVisible.value = true
    if (report.errorCount > 0) {
      ElMessage.warning(`质检：${report.errorCount} 个错误，建议修订`)
    } else if (report.warnCount > 0) {
      ElMessage.info(`质检：${report.warnCount} 条警告`)
    } else {
      ElMessage.success(`质检通过（${report.score} 分）`)
    }
  } catch (e) {
    ElMessage.error(`质检失败：${e}`)
  } finally {
    qualityLoading.value = false
  }
}

async function onGenerateRedrill() {
  if (!paper.value) {
    ElMessage.warning('请先打开试卷')
    return
  }
  const wrongItems = wrongKeys.value.map((k) => {
    const [si, ii] = k.split('-').map(Number)
    return { sectionIndex: si, itemIndex: ii }
  })
  const knowledgePoints = reviewKpText.value
    .split(/[,，、\s]+/)
    .map((s) => s.trim())
    .filter(Boolean)
  if (!wrongItems.length && !knowledgePoints.length) {
    ElMessage.warning('请勾选错题或填写知识点')
    return
  }
  redrillLoading.value = true
  try {
    if (aiReady.value) await invoke('set_config', { cfg: { ...config } })
    const exam = await invoke<ExamPaper>('generate_redrill', {
      req: {
        paper: paper.value,
        wrongItems,
        knowledgePoints,
        subject: paper.value.meta?.subject || subjectLabel.value,
        grade: paper.value.meta?.grade || form.grade,
        totalScore: 50,
        durationMin: 20,
      },
    })
    paper.value = exam
    verifyReport.value = null
    parallelSet.value = null
    form.examType = 'redrill'
    workMode.value = 'exam'
    previewTab.value = 'exam'
    await saveToHistory(exam)
    reviewDialogVisible.value = false
    ElMessage.success('已生成错题再练卷')
    // 自动质检
    try {
      qualityReport.value = await invoke<QualityReport>('inspect_exam_paper', { paper: exam })
    } catch {
      /* ignore */
    }
  } catch (e) {
    ElMessage.error(`再练卷失败：${formatFriendlyError(e)}`)
  } finally {
    redrillLoading.value = false
  }
}

async function onVerifyMath() {
  if (!paper.value) {
    ElMessage.warning('请先生成试卷')
    return
  }
  if (form.subject !== 'math' && paper.value.meta?.subject !== '数学') {
    ElMessage.info('验算目前主要针对数学计算题')
  }
  verifying.value = true
  try {
    const report = await invoke<VerifyReport>('verify_math_paper', { paper: paper.value })
    verifyReport.value = report
    verifyDialogVisible.value = true
    if (report.mismatch > 0) {
      ElMessage.warning(`发现 ${report.mismatch} 道答案可能有误`)
    } else if (report.checked > 0) {
      ElMessage.success(`已校验 ${report.checked} 道，均一致`)
    } else {
      ElMessage.info('未识别到可自动验算的计算题')
    }
  } catch (e) {
    ElMessage.error(`验算失败：${e}`)
  } finally {
    verifying.value = false
  }
}

async function onBuildSpecTable() {
  if (!paper.value) {
    ElMessage.warning('请先生成试卷')
    return
  }
  try {
    const table = await invoke<SpecTable>('build_paper_spec_table', { paper: paper.value })
    specTable.value = table
    specDialogVisible.value = true
  } catch (e) {
    ElMessage.error(`细目表生成失败：${e}`)
  }
}

function copySpecTableText() {
  if (!specTable.value) return
  const t = specTable.value
  const lines = [
    `双向细目表 · ${t.title}`,
    t.summary,
    '',
    '【按知识点】',
    ...t.knowledgeRows.map(
      (r) =>
        `${r.knowledgePoint}\t${r.itemCount}题\t${r.totalScore}分\t${r.scoreRatio}%\t${r.itemIds.join(',')}`,
    ),
    '',
    '【按大题】',
    ...t.sectionRows.map(
      (r) => `${r.title}\t${r.itemCount}题\t${r.score}分\t${r.scoreRatio}%`,
    ),
    '',
    t.uncoveredNote,
  ]
  navigator.clipboard?.writeText(lines.join('\n')).then(
    () => ElMessage.success('细目表已复制'),
    () => ElMessage.warning('复制失败，请手动选择文本'),
  )
}

async function makeParallelABC() {
  if (!paper.value) {
    ElMessage.warning('请先完成组卷')
    return
  }
  if (!aiReady.value) {
    ElMessage.warning('请先配置 AI 接口（云端 API Key 或本地模型）')
    settingsVisible.value = true
    return
  }
  makingParallel.value = true
  startAiPanel()
  try {
    await invoke('set_config', { cfg: { ...config } })
    const set = await invoke<ParallelSet>('generate_parallel_abc', { paper: paper.value })
    parallelSet.value = set
    parallelIndex.value = 0
    if (set.papers?.[0]) {
      paper.value = set.papers[0]
      verifyReport.value = null
      await saveToHistory(set.papers[0])
    }
    // 另存 B/C 到历史
    for (let i = 1; i < (set.papers?.length || 0); i++) {
      await saveToHistory(set.papers[i])
    }
    stopAiPanelSuccess(set.papers?.[0] || paper.value!)
    ElMessage.success('已生成平行卷 A/B/C（已写入历史）')
  } catch (e) {
    stopAiPanelError(e)
    ElMessage.error(`平行卷失败：${formatFriendlyError(e)}`)
  } finally {
    makingParallel.value = false
    loading.value = false
  }
}

function selectParallel(i: number) {
  if (!parallelSet.value?.papers?.[i]) return
  parallelIndex.value = i
  paper.value = parallelSet.value.papers[i]
  verifyReport.value = null
  wrongKeys.value = []
  previewTab.value = 'exam'
  workMode.value = 'exam'
}

function itemKey(si: number, ii: number) {
  return `${si}-${ii}`
}

function toggleWrong(si: number, ii: number) {
  const k = itemKey(si, ii)
  const i = wrongKeys.value.indexOf(k)
  if (i >= 0) wrongKeys.value.splice(i, 1)
  else wrongKeys.value.push(k)
}

function isWrong(si: number, ii: number) {
  return wrongKeys.value.includes(itemKey(si, ii))
}

async function favoriteItem(si: number, ii: number) {
  if (!paper.value) return
  const sec = paper.value.sections[si]
  const item = sec?.items?.[ii]
  if (!item) return
  try {
    await invoke('bank_add_favorite', {
      req: {
        item,
        sectionType: sec.type || '',
        subject: paper.value.meta?.subject || form.subject,
        grade: paper.value.meta?.grade || form.grade,
        edition: paper.value.meta?.edition || form.edition,
        sourceTitle: paper.value.meta?.title || '',
        tags: [],
      },
    })
    ElMessage.success('已加入校本收藏')
  } catch (e) {
    ElMessage.warning(String(e))
  }
}

async function openBank() {
  try {
    favorites.value = await invoke<FavoriteItem[]>('bank_list_favorites')
    bankPapers.value = await invoke<BankPaper[]>('bank_list_papers')
    bankVisible.value = true
  } catch (e) {
    ElMessage.error(`打开校本库失败：${e}`)
  }
}

async function refreshBank() {
  favorites.value = await invoke<FavoriteItem[]>('bank_list_favorites')
  bankPapers.value = await invoke<BankPaper[]>('bank_list_papers')
}

async function removeFavorite(id: string) {
  await invoke('bank_delete_favorite', { id })
  await refreshBank()
  ElMessage.success('已删除')
}

async function clearFavoritesAll() {
  await ElMessageBox.confirm('确定清空全部收藏题？', '确认', { type: 'warning' })
  await invoke('bank_clear_favorites')
  await refreshBank()
}

async function openBankPaper(id: string) {
  const entry = await invoke<BankPaper>('bank_get_paper', { id })
  paper.value = entry.paper
  parallelSet.value = null
  verifyReport.value = null
  wrongKeys.value = []
  workMode.value = 'exam'
  previewTab.value = 'exam'
  bankVisible.value = false
  ElMessage.success('已打开校本卷')
}

async function deleteBankPaperEntry(id: string) {
  await invoke('bank_delete_paper', { id })
  await refreshBank()
}

async function importSchoolPaper() {
  try {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.json,application/json'
    input.onchange = async () => {
      const file = input.files?.[0]
      if (!file) return
      try {
        const text = await file.text()
        const data = JSON.parse(text)
        // 支持直接试卷 或 { paper: ... }
        const paperJson = data.sections ? data : data.paper
        if (!paperJson?.sections) {
          ElMessage.error('JSON 需包含 meta 与 sections（试卷格式）')
          return
        }
        const also = await ElMessageBox.confirm(
          '是否同时把卷内小题加入校本收藏？',
          '导入本校卷',
          {
            distinguishCancelAndClose: true,
            confirmButtonText: '卷+题目',
            cancelButtonText: '仅试卷',
            type: 'info',
          },
        )
          .then(() => true)
          .catch((action) => (action === 'cancel' ? false : null))
        if (also === null) return
        const res = await invoke<{ paper: BankPaper; itemsAdded: number }>('bank_import_paper', {
          paper: paperJson,
          alsoItems: also,
        })
        await refreshBank()
        bankTab.value = 'papers'
        bankVisible.value = true
        ElMessage.success(
          also
            ? `已导入校本卷，并收藏 ${res.itemsAdded} 题`
            : '已导入校本卷',
        )
      } catch (e) {
        ElMessage.error(`导入失败：${e}`)
      }
    }
    input.click()
  } catch (e) {
    ElMessage.error(String(e))
  }
}

async function saveCurrentToBank() {
  if (!paper.value) {
    ElMessage.warning('当前无试卷')
    return
  }
  try {
    await invoke('bank_import_paper', { paper: paper.value, alsoItems: false })
    ElMessage.success('当前卷已存入校本库')
  } catch (e) {
    ElMessage.error(String(e))
  }
}

async function onGenerateReview() {
  if (!paper.value) {
    ElMessage.warning('请先打开试卷')
    return
  }
  const wrongItems = wrongKeys.value.map((k) => {
    const [si, ii] = k.split('-').map(Number)
    return { sectionIndex: si, itemIndex: ii }
  })
  const knowledgePoints = reviewKpText.value
    .split(/[,，、\s]+/)
    .map((s) => s.trim())
    .filter(Boolean)
  if (!wrongItems.length && !knowledgePoints.length) {
    ElMessage.warning('请勾选错题，或填写知识点后再生成讲评稿')
    return
  }
  reviewLoading.value = true
  try {
    if (reviewUseAi.value && aiReady.value) {
      await invoke('set_config', { cfg: { ...config } })
    }
    const outline = await invoke<ReviewOutline>('generate_review', {
      req: {
        paper: paper.value,
        wrongItems,
        knowledgePoints,
        subject: paper.value.meta?.subject || subjectLabel.value,
        grade: paper.value.meta?.grade || form.grade,
        useAi: reviewUseAi.value && aiReady.value,
      },
    })
    reviewOutline.value = outline
    reviewDialogVisible.value = true
    await saveToHistory(outline as unknown as ExamPaper)
    ElMessage.success('讲评提纲已生成')
  } catch (e) {
    ElMessage.error(`讲评稿失败：${formatFriendlyError(e)}`)
  } finally {
    reviewLoading.value = false
  }
}

function openReviewPanel() {
  if (!paper.value) {
    ElMessage.warning('请先生成或打开试卷')
    return
  }
  // 预填知识点：从细目或全卷收集
  if (!reviewKpText.value && paper.value.sections) {
    const set = new Set<string>()
    for (const sec of paper.value.sections) {
      for (const it of sec.items || []) {
        for (const kp of it.knowledgePoints || []) {
          if (kp && kp !== '（未标注知识点）') set.add(kp)
        }
      }
    }
    reviewKpText.value = [...set].slice(0, 8).join('、')
  }
  reviewDialogVisible.value = true
}

function copyReviewText() {
  const o = reviewOutline.value
  if (!o) return
  const lines = [
    o.meta?.title || '作业讲评提纲',
    o.overview || '',
    '',
    '【聚焦】' + (o.knowledgeFocus || []).join('、'),
    '',
    '【过程】',
    ...(o.process || []).map(
      (p) =>
        `${p.stage}（${p.minutes || '?'}分）\n  ${p.content || ''}\n  师：${p.teacherActivity || ''}\n  生：${p.studentActivity || ''}`,
    ),
    '',
    '【知识点讲评】',
    ...(o.points || []).map(
      (p) =>
        `· ${p.knowledgePoint}\n  错因：${p.errorPattern || ''}\n  讲法：${p.keyExplain || ''}\n  板书：${p.boardNote || ''}\n  练习：${(p.practice || []).join('；')}`,
    ),
    '',
    '【作业】' + (o.homework || []).join('；'),
    '【反思】' + (o.reflection || ''),
  ]
  navigator.clipboard?.writeText(lines.join('\n')).then(
    () => ElMessage.success('讲评稿已复制'),
    () => ElMessage.warning('复制失败'),
  )
}

function verifyStatusType(status: string) {
  if (status === 'ok') return 'success'
  if (status === 'mismatch') return 'danger'
  if (status === 'error') return 'warning'
  return 'info'
}

function verifyStatusLabel(status: string) {
  const m: Record<string, string> = {
    ok: '一致',
    mismatch: '不一致',
    skip: '跳过',
    error: '解析失败',
  }
  return m[status] || status
}

function verifyItemOf(si: number, ii: number) {
  return verifyReport.value?.items.find((x) => x.sectionIndex === si && x.itemIndex === ii)
}

async function onAiGenerate() {
  if (!aiReady.value) {
    if (form.subject !== 'math') {
      ElMessage.warning('当前无 AI 模式仅能保证数学题答案可验算；语文和英语将生成结构卷')
    }
    await onTemplateGenerate()
    return
  }
  if (!currentPack.value) {
    ElMessage.warning('当前年级、册次暂无课标数据，请先更新课标或更换册次')
    return
  }

  loading.value = true
  startAiPanel()
  try {
    await invoke('set_config', { cfg: { ...config } })
    const req = buildReq()
    const result = await invoke<ExamPaper>('generate_paper', { req })
    paper.value = result
    verifyReport.value = null
    workMode.value = 'exam'
    previewTab.value = 'exam'
    await saveToHistory(result)
    stopAiPanelSuccess(result)
  } catch (e) {
    stopAiPanelError(e)
  } finally {
    loading.value = false
  }
}

async function onTemplateGenerate() {
  loading.value = true
  try {
    const req = buildReq()
    const result = await invoke<ExamPaper>('generate_template_paper', { req })
    paper.value = result
    await saveToHistory(result)
    if (form.subject === 'math') {
      ElMessage.success('已用本地可验算题库生成完整数学卷')
    } else {
      ElMessage.success('已生成结构卷；语文和英语完整命题仍需 AI 或带答案的结构化题库')
    }
  } catch (e) {
    ElMessage.error(`模板生成失败：${e}`)
  } finally {
    loading.value = false
  }
}

async function saveSettings() {
  try {
    const updatedApiKey = config.apiKey.trim()
    // 同步当前组卷参数为默认偏好（可选：仅保存设置面板字段）
    config.defaultSubject = form.subject
    config.defaultEdition = form.edition
    config.defaultGrade = form.grade
    config.defaultSemester = form.semester
    config.defaultExamType = form.examType
    config.defaultDifficulty = form.difficulty
    await invoke('set_config', { cfg: { ...config } })
    if (updatedApiKey) {
      config.apiKeyConfigured = true
      config.apiKey = ''
    }
    ElMessage.success('设置已保存')
    settingsVisible.value = false
  } catch (e) {
    ElMessage.error(`保存失败：${e}`)
  }
}

async function clearSavedApiKey() {
  try {
    await ElMessageBox.confirm('确定清除本机已保存的 API 密钥？', '清除密钥', {
      type: 'warning',
      confirmButtonText: '清除',
      cancelButtonText: '取消',
    })
    await invoke('clear_api_key')
    config.apiKey = ''
    config.apiKeyConfigured = false
    ElMessage.success('API 密钥已清除')
  } catch {
    // 用户取消时无需提示。
  }
}

async function savePrefsOnly() {
  try {
    config.defaultSubject = form.subject
    config.defaultEdition = form.edition
    config.defaultGrade = form.grade
    config.defaultSemester = form.semester
    config.defaultExamType = form.examType
    config.defaultDifficulty = form.difficulty
    await invoke('set_config', { cfg: { ...config } })
    ElMessage.success('已将当前参数设为默认')
  } catch (e) {
    ElMessage.error(`保存失败：${e}`)
  }
}

async function exportDocx() {
  const exportLesson =
    (displayLesson.value || lessonPlan.value) &&
    (previewTab.value === 'lesson' || (!paper.value && workMode.value === 'lesson'))
  if (exportLesson && (displayLesson.value || lessonPlan.value)) {
    try {
      const plan = displayLesson.value || lessonPlan.value!
      const isParent = lessonAudienceView.value === 'parent' && !!plan.parentGuide
      const blob = await renderLessonDocx(plan, currentBrand(), {
        audience: isParent ? 'parent' : 'teacher',
      })
      const name = `${plan.meta?.title || '教案'}${isParent ? '-家长版' : '-教师版'}.docx`
      const path = await saveDocxFile(blob, name)
      if (path === null) {
        ElMessage.info('已取消导出')
        return
      }
      ElMessage.success(path.includes('\\') || path.includes('/') ? `已保存：${path}` : '教案已导出')
    } catch (e) {
      ElMessage.error(`导出失败：${e}`)
    }
    return
  }
  if (!paper.value) {
    ElMessage.warning('请先完成组卷')
    return
  }
  try {
    const mode = config.exportMode || (config.exportAttachAnswers !== false ? 'with_answers' : 'student')
    if (mode === 'both') {
      const blobS = await renderExamDocx(paper.value, false, false, currentBrand())
      const pathS = await saveDocxFile(blobS, buildExportFilename({ suffix: '-学生卷' }))
      if (pathS === null) {
        ElMessage.info('已取消导出')
        return
      }
      const blobA = await renderExamDocx(paper.value, true, true, currentBrand())
      const pathA = await saveDocxFile(blobA, buildExportFilename({ answers: true }))
      if (pathA === null) {
        ElMessage.info('学生卷已保存，答案导出已取消')
        return
      }
      ElMessage.success('已分别导出学生卷与答案')
      return
    }
    const attach = mode === 'with_answers' || (mode !== 'student' && config.exportAttachAnswers !== false)
    const blob = await renderExamDocx(paper.value, false, attach, currentBrand())
    const path = await saveDocxFile(blob, buildExportFilename())
    if (path === null) {
      ElMessage.info('已取消导出')
      return
    }
    ElMessage.success(
      path.includes('\\') || path.includes('/')
        ? `已保存：${path}`
        : attach
          ? 'Word 已导出（含参考答案页）'
          : 'Word 已导出（仅学生卷）',
    )
  } catch (e) {
    ElMessage.error(`导出失败：${e}`)
  }
}

async function exportAnswersOnly() {
  if (workMode.value === 'lesson' && previewTab.value === 'lesson' && !paper.value) {
    ElMessage.info('教案请使用「导出教案」')
    return
  }
  if (!paper.value) {
    ElMessage.warning('请先完成组卷')
    return
  }
  try {
    const blob = await renderExamDocx(paper.value, true, true, currentBrand())
    const path = await saveDocxFile(blob, buildExportFilename({ answers: true }))
    if (path === null) {
      ElMessage.info('已取消导出')
      return
    }
    ElMessage.success(path.includes('\\') || path.includes('/') ? `已保存：${path}` : '参考答案已导出')
  } catch (e) {
    ElMessage.error(`导出失败：${e}`)
  }
}

function buildLessonReq() {
  const pack = currentPack.value
  if (!pack) throw new Error('未找到课标')
  // 教案默认绑定单元
  if (!form.unitId && pack.units[0]) form.unitId = pack.units[0].id
  return {
    subject: form.subject,
    edition: form.edition,
    grade: form.grade,
    semester: form.semester,
    knowledgePath: pack.path,
    unitId: form.unitId || null,
    lessonName: lessonName.value || null,
    periods: lessonPeriods.value || 1,
    durationMin: form.durationMin || 40,
    lessonType: lessonType.value || 'new',
    includeParent: includeParentGuide.value !== false,
  }
}

async function onGenerateLesson() {
  if (!aiReady.value) {
    ElMessage.warning('请先配置 AI 接口（云端 API Key 或本地模型）')
    settingsVisible.value = true
    return
  }
  if (!currentPack.value) {
    ElMessage.warning('当前年级、册次暂无课标数据')
    return
  }
  lessonLoading.value = true
  startAiPanel()
  try {
    await invoke('set_config', { cfg: { ...config } })
    const req = buildLessonReq()
    const result = await invoke<LessonPlan>('generate_lesson', { req })
    lessonPlan.value = result
    workMode.value = 'lesson'
    previewTab.value = 'lesson'
    await saveToHistory(result as unknown as ExamPaper)
    stopAiPanelSuccess({
      meta: {
        title: result.meta?.title || '教案',
        totalScore: 0,
        durationMin: result.meta?.durationMin || 40,
        edition: result.meta?.edition || '',
        subject: result.meta?.subject || '',
        grade: result.meta?.grade || form.grade,
        semester: result.meta?.semester || '',
        examType: '教案',
      },
      sections: [{ type: 'lesson', title: '教案', score: 0, items: [{ id: '1', stem: '教案已生成' }] }],
    })
  } catch (e) {
    stopAiPanelError(e)
  } finally {
    lessonLoading.value = false
  }
}

async function onLessonTemplate() {
  if (!currentPack.value) {
    ElMessage.warning('当前年级、册次暂无课标数据')
    return
  }
  lessonLoading.value = true
  try {
    const req = buildLessonReq()
    const result = await invoke<LessonPlan>('generate_lesson_template', { req })
    lessonPlan.value = result
    paper.value = null
    workMode.value = 'lesson'
    previewTab.value = 'lesson'
    await saveToHistory(result as unknown as ExamPaper)
    ElMessage.success('已生成教案结构模板')
  } catch (e) {
    ElMessage.error(`模板失败：${e}`)
  } finally {
    lessonLoading.value = false
  }
}

/**
 * 同一单元：先写教案，再出配套单元练习卷
 */
async function onGenerateLinked() {
  if (!aiReady.value) {
    ElMessage.warning('请先配置 AI 接口（云端 API Key 或本地模型）')
    settingsVisible.value = true
    return
  }
  if (!currentPack.value) {
    ElMessage.warning('当前年级、册次暂无课标数据')
    return
  }
  // 联动默认按单元
  if (!form.unitId && currentPack.value.units[0]) {
    form.unitId = currentPack.value.units[0].id
  }
  form.examType = 'unit'

  linking.value = true
  loading.value = true
  lessonLoading.value = true
  startAiPanel()
  linkProgress.value = '教案'

  try {
    await invoke('set_config', { cfg: { ...config } })

    // 1) 教案
    aiStepIndex.value = 0
    const lessonReq = buildLessonReq()
    const plan = await invoke<LessonPlan>('generate_lesson', { req: lessonReq })
    lessonPlan.value = plan
    await saveToHistory(plan as unknown as ExamPaper)

    // 2) 配套单元卷
    linkProgress.value = '练习卷'
    aiStepIndex.value = Math.min(aiSteps.length - 1, 2)
    const examReq = buildReq()
    examReq.examType = 'unit'
    examReq.unitId = form.unitId
    // 单元练习卷时长略短于期末
    if (examReq.durationMin > 50) examReq.durationMin = 40
    const exam = await invoke<ExamPaper>('generate_paper', { req: examReq })
    // 标题上注明配套
    if (exam.meta) {
      const unitLabel =
        selectedUnit.value?.name || plan.meta?.unitName || '本单元'
      if (!String(exam.meta.title || '').includes('配套')) {
        exam.meta.title = `${exam.meta.title || unitLabel}（配套练习）`
      }
      exam.meta.examType = exam.meta.examType || '单元测试'
    }
    paper.value = exam
    await saveToHistory(exam)

    workMode.value = 'lesson'
    previewTab.value = 'lesson'
    stopAiPanelSuccess({
      meta: {
        title: '教案 + 配套练习',
        totalScore: exam.meta?.totalScore || 100,
        durationMin: exam.meta?.durationMin || 40,
        edition: exam.meta?.edition || '',
        subject: exam.meta?.subject || '',
        grade: exam.meta?.grade || form.grade,
        semester: exam.meta?.semester || '',
        examType: '卷案联动',
      },
      sections: [
        {
          type: 'linked',
          title: '联动结果',
          score: 0,
          items: [
            { id: '1', stem: `教案：${plan.meta?.title || '已生成'}` },
            { id: '2', stem: `练习卷：${exam.meta?.title || '已生成'}` },
          ],
        },
      ],
    })
    ElMessage.success('已生成教案与配套练习卷，可在预览中切换查看')
  } catch (e) {
    stopAiPanelError(e)
  } finally {
    linking.value = false
    loading.value = false
    lessonLoading.value = false
    linkProgress.value = ''
  }
}

async function exportLinkedBoth() {
  if (!lessonPlan.value && !paper.value && !lessonBundle.value) {
    ElMessage.warning('请先完成卷案联动或分别生成')
    return
  }
  try {
    if (lessonBundle.value?.plans?.length) {
      for (const plan of lessonBundle.value.plans) {
        if (plan.error) continue
        const blob = await renderLessonDocx(plan, currentBrand())
        const name = `${plan.meta?.title || plan.meta?.lessonName || '教案'}.docx`
        const path = await saveDocxFile(blob, name)
        if (path === null) {
          ElMessage.info('已取消后续导出')
          return
        }
      }
    } else if (lessonPlan.value) {
      const blob = await renderLessonDocx(lessonPlan.value, currentBrand())
      const name = `${lessonPlan.value.meta?.title || '教案'}.docx`
      const path = await saveDocxFile(blob, name)
      if (path === null) {
        ElMessage.info('已取消导出')
        return
      }
    }
    if (paper.value) {
      const attach = config.exportAttachAnswers !== false
      const blob = await renderExamDocx(paper.value, false, attach, currentBrand())
      const name = `${paper.value.meta.title || '配套练习'}.docx`
      const path = await saveDocxFile(blob, name)
      if (path === null) {
        ElMessage.info('练习卷已取消')
        return
      }
    }
    ElMessage.success('已按顺序保存完成')
  } catch (e) {
    ElMessage.error(`导出失败：${e}`)
  }
}

/** 单元全课时教案（AI） */
async function onGenerateUnitAllLessons(useAi: boolean) {
  if (useAi && !aiReady.value) {
    ElMessage.warning('请先配置 AI 接口（云端 API Key 或本地模型）')
    settingsVisible.value = true
    return
  }
  if (!currentPack.value) {
    ElMessage.warning('当前年级、册次暂无课标数据')
    return
  }
  if (!form.unitId && currentPack.value.units[0]) {
    form.unitId = currentPack.value.units[0].id
  }
  const n = selectedUnit.value?.lessons?.length || 1
  if (useAi && n > 1) {
    try {
      await ElMessageBox.confirm(
        `将为「${selectedUnit.value?.name || '本单元'}」共 ${n} 个课时依次生成教案，约需较长时间，是否继续？`,
        '全课时教案',
        { type: 'info', confirmButtonText: '开始', cancelButtonText: '取消' },
      )
    } catch {
      return
    }
  }

  lessonLoading.value = true
  startAiPanel()
  try {
    await invoke('set_config', { cfg: { ...config } })
    const req = buildLessonReq()
    const bundle = useAi
      ? await invoke<LessonPlanBundle>('generate_unit_lessons', { req })
      : await invoke<LessonPlanBundle>('generate_unit_lessons_template', { req })
    lessonBundle.value = bundle
    bundleIndex.value = 0
    const first = bundle.plans?.find((p) => !p.error) || bundle.plans?.[0] || null
    lessonPlan.value = first
    paper.value = null
    workMode.value = 'lesson'
    previewTab.value = 'lesson'
    // 历史：存整个 bundle
    await saveToHistory(bundle as unknown as ExamPaper)
    stopAiPanelSuccess({
      meta: {
        title: bundle.meta?.title || '全课时教案',
        totalScore: 0,
        durationMin: form.durationMin,
        edition: bundle.meta?.edition || '',
        subject: bundle.meta?.subject || '',
        grade: bundle.meta?.grade || form.grade,
        semester: bundle.meta?.semester || '',
        examType: '全课时教案',
      },
      sections: [
        {
          type: 'bundle',
          title: '全课时',
          score: 0,
          items: [{ id: '1', stem: `共 ${bundle.plans?.length || 0} 份教案` }],
        },
      ],
    })
    ElMessage.success(`已生成 ${bundle.plans?.length || 0} 份课时教案`)
  } catch (e) {
    stopAiPanelError(e)
  } finally {
    lessonLoading.value = false
  }
}

function openOutputCenter() {
  if (!paper.value && !lessonPlan.value && !lessonBundle.value) {
    ElMessage.warning('请先生成试卷或教案')
    return
  }
  outputCenterVisible.value = true
}

async function outputAction(cmd: string) {
  try {
    switch (cmd) {
      case 'exam':
        if (!paper.value) {
          ElMessage.warning('当前无试卷')
          return
        }
        previewTab.value = 'exam'
        await exportDocx()
        break
      case 'answers':
        if (!paper.value) {
          ElMessage.warning('当前无试卷')
          return
        }
        previewTab.value = 'exam'
        await exportAnswersOnly()
        break
      case 'lesson':
        if (!displayLesson.value && !lessonPlan.value) {
          ElMessage.warning('当前无教案')
          return
        }
        previewTab.value = 'lesson'
        // 确保 export 用当前 display
        if (displayLesson.value) lessonPlan.value = displayLesson.value
        await exportDocx()
        break
      case 'bundle':
        if (!lessonBundle.value?.plans?.length) {
          ElMessage.warning('当前无全课时教案包')
          return
        }
        await exportLinkedBoth()
        break
      case 'all':
        await exportLinkedBoth()
        break
      case 'print_exam':
        if (!paper.value) {
          ElMessage.warning('当前无试卷')
          return
        }
        previewTab.value = 'exam'
        openPrintPreview(false)
        break
      case 'print_answers':
        if (!paper.value) {
          ElMessage.warning('当前无试卷')
          return
        }
        previewTab.value = 'exam'
        openPrintPreview(true)
        break
      case 'print_lesson':
        if (!displayLesson.value && !lessonPlan.value) {
          ElMessage.warning('当前无教案')
          return
        }
        previewTab.value = 'lesson'
        if (displayLesson.value) lessonPlan.value = displayLesson.value
        openPrintPreview(false)
        break
      default:
        break
    }
    if (cmd.startsWith('export') || ['exam', 'answers', 'lesson', 'bundle', 'all'].includes(cmd)) {
      // keep center open for multi export
    }
  } catch (e) {
    ElMessage.error(String(e))
  }
}

async function historyExport(id: string, asAnswers = false) {
  try {
    const entry = await invoke<HistoryEntry>('history_get', { id })
    const raw = entry.paper as ExamPaper & LessonPlan & LessonPlanBundle & { kind?: string }
    if (raw?.kind === 'lessonPlanBundle' && (raw as LessonPlanBundle).plans) {
      const bundle = raw as LessonPlanBundle
      for (const plan of bundle.plans || []) {
        if (plan.error) continue
        const blob = await renderLessonDocx(plan, currentBrand())
        const name = `${plan.meta?.title || '教案'}.docx`
        const path = await saveDocxFile(blob, name)
        if (path === null) return
      }
      ElMessage.success('全课时教案已导出')
      return
    }
    if (raw?.kind === 'lessonPlan' || (raw as LessonPlan).process) {
      const blob = await renderLessonDocx(raw as LessonPlan, currentBrand())
      const path = await saveDocxFile(blob, `${entry.title || '教案'}.docx`)
      if (path === null) return
      ElMessage.success('教案已导出')
      return
    }
    const exam = raw as ExamPaper
    const blob = await renderExamDocx(
      exam,
      asAnswers,
      asAnswers ? true : config.exportAttachAnswers !== false,
      currentBrand(),
    )
    const path = await saveDocxFile(
      blob,
      asAnswers ? `${entry.title || '试卷'}-答案.docx` : `${entry.title || '试卷'}.docx`,
    )
    if (path === null) return
    ElMessage.success(asAnswers ? '答案已导出' : '试卷已导出')
  } catch (e) {
    ElMessage.error(`导出失败：${e}`)
  }
}

async function historyPrint(id: string, asAnswers = false) {
  try {
    const entry = await invoke<HistoryEntry>('history_get', { id })
    const raw = entry.paper as ExamPaper & LessonPlan & LessonPlanBundle & { kind?: string }
    if (raw?.kind === 'lessonPlanBundle' && (raw as LessonPlanBundle).plans?.length) {
      const plan = (raw as LessonPlanBundle).plans.find((p) => !p.error) || (raw as LessonPlanBundle).plans[0]
      printPreviewHtml.value = buildLessonPrintHtml(plan, currentBrand())
      printPreviewIsAnswer.value = false
      printPreviewVisible.value = true
      historyVisible.value = false
      return
    }
    if (raw?.kind === 'lessonPlan' || (raw as LessonPlan).process) {
      printPreviewHtml.value = buildLessonPrintHtml(raw as LessonPlan, currentBrand())
      printPreviewIsAnswer.value = false
      printPreviewVisible.value = true
      historyVisible.value = false
      return
    }
    printPreviewHtml.value = buildPrintHtml(raw as ExamPaper, asAnswers)
    printPreviewIsAnswer.value = asAnswers
    printPreviewVisible.value = true
    historyVisible.value = false
  } catch (e) {
    ElMessage.error(`打印准备失败：${e}`)
  }
}

/** 生成打印用 HTML（学生卷，不含答案） */
function buildPrintHtml(p: ExamPaper, withAnswers: boolean): string {
  const meta = p.meta
  const brand = currentBrand()
  const schoolBlock = [brand.schoolName, [brand.academicYear, brand.schoolTerm].filter(Boolean).join(' ')]
    .filter((s) => s && String(s).trim())
    .map((s) => `<div class="school">${String(s).replace(/</g, '&lt;')}</div>`)
    .join('')
  const classLine = brand.className?.trim()
    ? `班级：${brand.className.trim()}　姓名：________　学号：________　得分：________`
    : '班级：________　姓名：________　学号：________　得分：________'
  const sub = [meta.edition, meta.subject, meta.grade ? `${meta.grade}年级` : '', meta.semester, meta.examType]
    .filter(Boolean)
    .join(' · ')

  const sectionKind = (sec: { type?: string; title?: string }) =>
    `${sec.type || ''}${sec.title || ''}`

  const sectionsHtml = (p.sections || [])
    .map((sec) => {
      const kind = sectionKind(sec)
      const isProblem = /解决|应用|problem|操作|实践/i.test(kind)
      const isWriting = /习作|作文|writing|小练笔/i.test(kind)
      const isReading = /阅读|reading/i.test(kind)
      const isCalc = isCalculationSection(sec.type, sec.title)
      const isChoice = /choice|选择/i.test(kind)
      const isJudge = /judge|判断/i.test(kind)
      const isFill = /fill|填空|拼音|积累|字词|默写/i.test(kind)

      const renderedItems = (sec.items || []).map((item, idx) => {
          const stem = (item.stem || `${idx + 1}.`).replace(/</g, '&lt;').replace(/>/g, '&gt;')
          const opts = (item.options || [])
            .map((o) => `<span class="opt">${String(o).replace(/</g, '&lt;')}</span>`)
            .join('')
          const optsBlock = opts ? `<div class="opts">${opts}</div>` : ''
          const compactCalc = isCompactCalculationItem(item, sec.type, sec.title)
          const ans = withAnswers
            ? `<div class="ans">答案：${String(item.answer ?? '略').replace(/</g, '&lt;')}</div>`
            : ''
          // 按题型控制留白：数学应用题/计算只留白，不画答题下划线
          let blank = ''
          if (!withAnswers) {
            if (isProblem) {
              blank = `<div class="problem-space"></div>`
            } else if (isWriting) {
              blank = `<div class="write-lines">${'<div class="wline"></div>'.repeat(12)}</div>`
            } else if (isReading) {
              blank = `<div class="write-lines short">${'<div class="wline"></div>'.repeat(3)}</div>`
            } else if (isCalc && !compactCalc) {
              blank = `<div class="calc-space"></div>`
            } else if (isChoice || isJudge || isFill) {
              blank = ''
            } else {
              blank = `<div class="item-gap"></div>`
            }
          }
          return {
            compact: compactCalc,
            html: `<div class="item${compactCalc ? ' calc-item' : ''}"><div class="stem">${stem.replace(/\n/g, '<br/>')}</div>${optsBlock}${blank}${ans}</div>`,
          }
        })
      const items: string[] = []
      let compactRun: string[] = []
      const flushCompactRun = () => {
        if (compactRun.length) {
          items.push(`<div class="calc-grid">${compactRun.join('')}</div>`)
          compactRun = []
        }
      }
      for (const rendered of renderedItems) {
        if (rendered.compact) compactRun.push(rendered.html)
        else {
          flushCompactRun()
          items.push(rendered.html)
        }
      }
      flushCompactRun()
      return `<div class="sec"><h2>${(sec.title || '').replace(/</g, '&lt;')}</h2>${items.join('')}</div>`
    })
    .join('')

  const title = (meta.title || '试卷').replace(/</g, '&lt;')
  const footerLabel = withAnswers ? '参考答案' : '试卷'
  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8" />
<title>${title}</title>
<style>
  /* 页脚页码：Chromium / WebView2 打印支持 @page 边距框 */
  @page {
    size: A4;
    margin: 12mm 11mm 16mm 11mm;
    @bottom-center {
      content: "${footerLabel} · 第 " counter(page) " 页";
      font-family: "宋体", SimSun, serif;
      font-size: 9pt;
      color: #333;
    }
  }
  * { box-sizing: border-box; }
  body {
    font-family: "宋体", SimSun, "Microsoft YaHei", serif;
    font-size: 10.5pt;
    color: #000;
    line-height: 1.45;
    margin: 0;
    padding: 0;
  }
  h1 {
    text-align: center;
    font-family: "黑体", SimHei, sans-serif;
    font-size: 16pt;
    font-weight: bold;
    margin: 0 0 4px;
  }
  .sub, .info {
    text-align: center;
    font-size: 10pt;
    margin: 0 0 4px;
  }
  .score {
    width: 100%;
    border-collapse: collapse;
    margin: 6px 0 8px;
    font-size: 9.5pt;
  }
  .score th, .score td {
    border: 1px solid #000;
    padding: 3px 4px;
    text-align: center;
  }
  .note {
    font-size: 9pt;
    margin: 0 0 8px;
    line-height: 1.4;
  }
  /* 大题允许跨页，避免半页空白 */
  .sec { margin-bottom: 8px; page-break-inside: auto; }
  .sec h2 {
    font-family: "黑体", SimHei, sans-serif;
    font-size: 11pt;
    margin: 8px 0 4px;
    font-weight: bold;
  }
  .item { margin-bottom: 5px; page-break-inside: avoid; }
  .stem { white-space: pre-wrap; }
  .calc-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px 18px;
    margin: 4px 0 8px;
    page-break-inside: avoid;
  }
  .calc-grid .calc-item { margin: 0; min-height: 24px; }
  .calc-grid .calc-item .stem { white-space: nowrap; }
  .opts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2px 12px;
    padding-left: 1.2em;
    margin-top: 2px;
  }
  .item-gap { height: 4px; }
  .problem-space { height: 96px; margin: 2px 0 6px; }
  .write-lines { margin: 3px 0 6px; }
  .write-lines.short .wline { height: 20px; }
  .wline { height: 22px; border-bottom: 1px solid #333; margin-bottom: 1px; }
  .calc-space { height: 64px; margin: 2px 0 6px; }
  .ans { color: #000; margin-top: 2px; font-size: 10pt; }
  .school { text-align:center; font-family:"黑体",SimHei,sans-serif; font-size:11pt; font-weight:bold; margin:0 0 2px; }
  .end {
    text-align: center;
    margin-top: 10px;
    font-size: 9pt;
    color: #333;
  }
  @media print {
    body { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
  }
</style>
</head>
<body>
  ${schoolBlock}
  <h1>${title}${withAnswers ? '（参考答案）' : ''}</h1>
  <div class="sub">${sub}　　满分${meta.totalScore ?? 100}分　　时间${meta.durationMin ?? 60}分钟</div>
  ${
    withAnswers
      ? ''
      : `<div class="info">${classLine}</div>
  <table class="score">
    <tr><th>题号</th>${(p.sections || []).map((_, i) => `<th>${'一二三四五六七八九十'[i] || i + 1}</th>`).join('')}<th>总分</th></tr>
    <tr><td>得分</td>${(p.sections || []).map(() => '<td></td>').join('')}<td></td></tr>
    <tr><td>阅卷</td>${(p.sections || []).map(() => '<td></td>').join('')}<td></td></tr>
  </table>
  <div class="note">注意事项：1.认真审题，书写工整；2.计算题注意验算；3.应用题写清解题过程与答句。</div>`
  }
  ${sectionsHtml}
  ${withAnswers ? '' : '<div class="end">—— 试卷结束，请仔细检查 ——</div>'}
</body>
</html>`
}

function openPrintPreview(withAnswers = false) {
  const plan = displayLesson.value || lessonPlan.value
  const showLesson =
    plan && (previewTab.value === 'lesson' || (!paper.value && workMode.value === 'lesson'))
  if (showLesson && plan) {
    printPreviewIsAnswer.value = false
    const isParent = lessonAudienceView.value === 'parent' && !!plan.parentGuide
    printPreviewHtml.value = buildLessonPrintHtml(plan, currentBrand(), {
      audience: isParent ? 'parent' : 'teacher',
    })
    printPreviewVisible.value = true
    return
  }
  if (!paper.value) {
    ElMessage.warning(workMode.value === 'lesson' ? '请先生成教案或练习卷' : '请先完成组卷')
    return
  }
  printPreviewIsAnswer.value = withAnswers
  printPreviewHtml.value = buildPrintHtml(paper.value, withAnswers)
  printPreviewVisible.value = true
}

async function confirmPrint() {
  try {
    const result = await printHtml(printPreviewHtml.value)
    printPreviewVisible.value = false
    if (result.mode === 'pdf') {
      ElMessage.success(
        printPreviewIsAnswer.value
          ? '已生成参考答案 PDF（无 tauri.localhost 页脚），请在打开的 PDF 中打印'
          : '已生成试卷 PDF（无 tauri.localhost 页脚），请在打开的 PDF 中打印',
      )
    } else {
      ElMessage.warning(
        '已打开系统打印。若页脚出现 tauri.localhost，请在打印对话框取消勾选「页眉和页脚」',
      )
    }
  } catch (e) {
    ElMessage.error(`打印失败：${e}`)
  }
}

function openEbookPrintDialog() {
  ebookDialogVisible.value = true
  ebookUnitPages.value = null
}

async function parseEbookUrl() {
  try {
    const parts = await invoke<{
      baseUrl: string
      resId: string
      bookId: string
      contributeId: string
      firstNum: string
    }>('ebook_parse_url', { url: ebookForm.url })
    ebookForm.baseUrl = parts.baseUrl || ebookForm.baseUrl
    ebookForm.resId = parts.resId
    ebookForm.bookId = parts.bookId
    ebookForm.contributeId = parts.contributeId
    ElMessage.success('链接已解析')
    await loadEbookCatalog()
  } catch (e) {
    ElMessage.error(`解析链接失败：${e}`)
  }
}

async function loadEbookCatalog() {
  if (!ebookForm.resId.trim()) {
    ElMessage.warning('请先填写 resId 或粘贴完整阅读页链接并解析')
    return
  }
  ebookLoading.value = true
  ebookUnitPages.value = null
  try {
    const cat = await invoke<{
      bookName: string
      subjectName: string
      items: Array<{ bookId: string; cataName: string; deep: string }>
    }>('ebook_catalog', {
      baseUrl: ebookForm.baseUrl || 'https://www.100875.com.cn',
      resId: ebookForm.resId.trim(),
    })
    ebookCatalog.value = cat
    if (!ebookForm.bookId && cat.items?.length) {
      const units = cat.items.filter((x) => String(x.deep) === '5')
      ebookForm.bookId = (units[0] || cat.items[0]).bookId
    }
    ElMessage.success(`已加载《${cat.bookName || '电子书'}》目录（${cat.items?.length || 0} 项）`)
  } catch (e) {
    ElMessage.error(`加载目录失败：${e}`)
  } finally {
    ebookLoading.value = false
  }
}

async function fetchEbookUnitAndPreview() {
  if (!ebookForm.resId || !ebookForm.bookId || !ebookForm.contributeId) {
    ElMessage.warning('需要 resId、bookId、contributeId（可从阅读页链接解析）')
    return
  }
  ebookFetchingPages.value = true
  try {
    const pages = await invoke<EbookUnitPages>('ebook_unit_pages', {
      baseUrl: ebookForm.baseUrl || 'https://www.100875.com.cn',
      resId: ebookForm.resId.trim(),
      bookId: ebookForm.bookId.trim(),
      contributeId: ebookForm.contributeId.trim(),
      maxPages: ebookForm.maxPages || 30,
    })
    ebookUnitPages.value = pages
    printPreviewIsAnswer.value = false
    printPreviewHtml.value = buildEbookPrintHtml(pages)
    printPreviewVisible.value = true
    ElMessage.success(
      `已取「${pages.unitName}」第 ${pages.startPage}–${pages.endPage} 页，共 ${pages.pages?.length || 0} 张，可确认打印`,
    )
  } catch (e) {
    ElMessage.error(`拉取单元页图失败：${e}`)
  } finally {
    ebookFetchingPages.value = false
  }
}

async function printExam(withAnswers = false) {
  openPrintPreview(withAnswers)
}

async function regenerateItem(sectionIndex: number, itemIndex: number) {
  if (!paper.value || !aiReady.value) {
    ElMessage.warning(!aiReady.value ? '请先配置 AI 接口（云端 API Key 或本地模型）' : '请先完成组卷')
    if (!aiReady.value) settingsVisible.value = true
    return
  }
  const key = `${sectionIndex}-${itemIndex}`
  regeneratingKey.value = key
  try {
    const item = await invoke<ExamItem>('regenerate_one_item', {
      req: {
        paper: paper.value,
        sectionIndex,
        itemIndex,
        knowledgePath: currentPack.value?.path || '',
        subject: form.subject,
        grade: form.grade,
      },
    })
    const next = JSON.parse(JSON.stringify(paper.value)) as ExamPaper
    if (!next.sections[sectionIndex]?.items[itemIndex]) {
      throw new Error('题目位置无效')
    }
    next.sections[sectionIndex].items[itemIndex] = {
      ...next.sections[sectionIndex].items[itemIndex],
      ...item,
    }
    paper.value = next
    await saveToHistory(next)
    ElMessage.success('已更换该题')
  } catch (e) {
    ElMessage.error(`换题失败：${formatFriendlyError(e)}`)
  } finally {
    regeneratingKey.value = ''
  }
}

async function makePaperB() {
  if (!paper.value) {
    ElMessage.warning('请先完成组卷')
    return
  }
  if (!aiReady.value) {
    ElMessage.warning('请先配置 AI 接口（云端 API Key 或本地模型）')
    settingsVisible.value = true
    return
  }
  makingB.value = true
  try {
    const b = await invoke<ExamPaper>('generate_paper_b', { paper: paper.value })
    paper.value = b
    verifyReport.value = null
    await saveToHistory(b)
    ElMessage.success('已生成 B 卷')
  } catch (e) {
    ElMessage.error(`B 卷生成失败：${formatFriendlyError(e)}`)
  } finally {
    makingB.value = false
  }
}

async function openHistory() {
  await refreshHistory()
  historyVisible.value = true
}

async function loadHistoryEntry(id: string) {
  try {
    const entry = await invoke<HistoryEntry>('history_get', { id })
    const raw = entry.paper as ExamPaper & LessonPlan & LessonPlanBundle & { kind?: string }
    if (raw?.kind === 'lessonPlanBundle' && (raw as LessonPlanBundle).plans) {
      const bundle = raw as LessonPlanBundle
      lessonBundle.value = bundle
      bundleIndex.value = 0
      lessonPlan.value = bundle.plans?.find((p) => !p.error) || bundle.plans?.[0] || null
      paper.value = null
      workMode.value = 'lesson'
      previewTab.value = 'lesson'
    } else if (raw?.kind === 'lessonPlan' || (raw as LessonPlan)?.process) {
      lessonPlan.value = raw as LessonPlan
      lessonBundle.value = null
      paper.value = null
      workMode.value = 'lesson'
      previewTab.value = 'lesson'
    } else {
      paper.value = entry.paper
      lessonPlan.value = null
      lessonBundle.value = null
      workMode.value = 'exam'
      previewTab.value = 'exam'
    }
    if (entry.formSnapshot) {
      Object.assign(form, entry.formSnapshot)
    }
    historyVisible.value = false
    ElMessage.success('已打开历史记录')
  } catch (e) {
    ElMessage.error(`打开失败：${e}`)
  }
}

async function clearPaper() {
  if (workMode.value === 'lesson' || lessonBundle.value || lessonPlan.value) {
    if (!lessonPlan.value && !lessonBundle.value && !paper.value) return
    await ElMessageBox.confirm('确定清空当前预览内容？', '清空确认', {
      type: 'warning',
      confirmButtonText: '清空',
      cancelButtonText: '取消',
    })
    lessonPlan.value = null
    lessonBundle.value = null
    if (workMode.value === 'lesson') return
  }
  if (!paper.value) return
  await ElMessageBox.confirm('确定清空当前试卷预览？', '清空确认', {
    type: 'warning',
    confirmButtonText: '清空',
    cancelButtonText: '取消',
  })
  paper.value = null
  verifyReport.value = null
  parallelSet.value = null
  wrongKeys.value = []
  reviewOutline.value = null
  specTable.value = null
}

function selectBundlePlan(i: number) {
  if (!lessonBundle.value?.plans?.length) return
  bundleIndex.value = i
  const plan = lessonBundle.value.plans[i]
  if (plan && !plan.error) {
    lessonPlan.value = plan
  }
}

async function removeHistory(id: string) {
  try {
    await invoke('history_delete', { id })
    await refreshHistory()
    ElMessage.success('已删除')
  } catch (e) {
    ElMessage.error(`删除失败：${e}`)
  }
}

async function clearAllHistory() {
  try {
    await ElMessageBox.confirm('确定清空全部历史记录？', '清空历史', {
      type: 'warning',
      confirmButtonText: '清空',
      cancelButtonText: '取消',
    })
    await invoke('history_clear')
    await refreshHistory()
    ElMessage.success('历史已清空')
  } catch {
    /* cancel */
  }
}

function formatHistoryTime(ts: number) {
  if (!ts) return ''
  const d = new Date(ts * 1000)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

onMounted(loadAll)
</script>

<template>
  <div class="layout">
    <header class="topbar">
      <div class="brand">
        <div class="brand-mark">试</div>
        <div>
          <div class="brand-name">试卷神器</div>
          <div class="brand-desc">小学组卷 · 多版本数学 / 语文 / 英语</div>
        </div>
      </div>
      <div class="topbar-actions">
        <el-button text @click="openHistory">历史记录</el-button>
        <el-button text type="primary" @click="openCurriculumBrowser">查看课标</el-button>
        <el-button text type="primary" :icon="Printer" @click="openEbookPrintDialog">打印电子书</el-button>
        <el-button text :icon="Link" @click="openSmartedu('classroom')">同步课堂</el-button>
        <el-button text :icon="Link" @click="openCatalogRef">教材目录</el-button>
        <el-button text @click="exportRuntimeLog">导出日志</el-button>
        <el-button text :loading="updateChecking" @click="checkAppUpdate(true)">
          {{ updateChecking && updateProgress > 0 ? `更新 ${updateProgress}%` : '检查更新' }}
        </el-button>
        <el-button text @click="showAppInfo">关于</el-button>
        <el-button :icon="Setting" @click="settingsVisible = true">接口设置</el-button>
      </div>
    </header>

    <div class="workspace">
      <!-- 左侧参数 -->
      <aside class="sidebar">
        <div class="card sidebar-card">
          <div class="card-head">
            <span class="card-title">{{ workMode === 'exam' ? '组卷参数' : '教案参数' }}</span>
            <el-radio-group v-model="workMode" size="small" class="mode-switch">
              <el-radio-button value="exam">试卷</el-radio-button>
              <el-radio-button value="lesson">教案</el-radio-button>
            </el-radio-group>
          </div>
          <div class="card-body">
            <el-form label-position="top" class="param-form" size="default">
              <div class="form-row-2">
                <el-form-item label="学科">
                  <el-select v-model="form.subject" class="w-full">
                    <el-option label="数学" value="math" />
                    <el-option label="语文" value="chinese" />
                    <el-option label="英语" value="english" />
                  </el-select>
                </el-form-item>
                <el-form-item label="版本">
                  <el-select v-model="form.edition" class="w-full">
                    <el-option
                      v-for="e in editionOptions"
                      :key="e.value"
                      :label="e.label"
                      :value="e.value"
                    />
                  </el-select>
                </el-form-item>
              </div>
              <div class="form-row-2">
                <el-form-item label="年级">
                  <el-select v-model="form.grade" class="w-full">
                    <el-option
                      v-for="g in gradeOptions"
                      :key="g"
                      :label="`${g} 年级`"
                      :value="g"
                    />
                  </el-select>
                </el-form-item>
                <el-form-item label="册别">
                  <el-select v-model="form.semester" class="w-full">
                    <el-option label="上册" value="shang" />
                    <el-option label="下册" value="xia" />
                  </el-select>
                </el-form-item>
              </div>

              <template v-if="workMode === 'exam'">
                <el-form-item label="卷型">
                  <el-select v-model="form.examType" class="w-full">
                    <el-option-group label="常规检测">
                      <el-option label="单元测试" value="unit" />
                      <el-option label="期中模拟" value="midterm" />
                      <el-option label="期末模拟" value="final" />
                    </el-option-group>
                    <el-option-group label="日常专项">
                      <el-option label="口算/字词" value="oral" />
                      <el-option label="课时练习" value="lesson" />
                      <el-option label="课后作业" value="homework" />
                    </el-option-group>
                  </el-select>
                </el-form-item>
                <el-form-item v-if="needsUnit" label="单元">
                  <el-select
                    v-model="form.unitId"
                    class="w-full"
                    placeholder="选择单元"
                    filterable
                    :title="selectedUnit?.name || ''"
                  >
                    <el-option
                      v-for="u in unitOptions"
                      :key="u.id"
                      :label="u.name"
                      :value="u.id"
                    />
                  </el-select>
                </el-form-item>
                <el-form-item v-else label="范围">
                  <el-input :model-value="examTypeLabel" disabled class="w-full" />
                </el-form-item>
                <el-form-item
                  v-if="needsUnit && lessonOptions.length"
                  label="课时范围"
                >
                  <el-select
                    v-model="form.selectedLessons"
                    class="w-full"
                    multiple
                    collapse-tags
                    collapse-tags-tooltip
                    filterable
                    placeholder="可多选；默认全选"
                  >
                    <el-option v-for="n in lessonOptions" :key="n" :label="n" :value="n" />
                  </el-select>
                </el-form-item>
                <div class="form-row-2 difficulty-row">
                  <el-form-item label="难度" class="difficulty-item">
                    <el-slider
                      v-model="difficultyLevel"
                      :min="0"
                      :max="2"
                      :step="1"
                      :marks="difficultyMarks"
                      :show-tooltip="false"
                      class="diff-slider"
                    />
                  </el-form-item>
                  <el-form-item label="时长(分)">
                    <el-input-number
                      v-model="form.durationMin"
                      :min="10"
                      :max="150"
                      class="w-full"
                      controls-position="right"
                    />
                  </el-form-item>
                </div>

                <el-collapse v-model="advancedOpen" class="advanced-collapse">
                  <el-collapse-item title="高级选项" name="adv">
                    <el-form-item label="结构模式">
                      <el-select v-model="form.structureMode" class="w-full">
                        <el-option label="智能结构（推荐）" value="adaptive" />
                        <el-option label="严格模板" value="strict" />
                        <el-option label="自由组卷" value="free" />
                      </el-select>
                      <div class="field-hint">
                        <template v-if="form.structureMode === 'strict'">严格遵循模板的大题顺序、题量与分值。</template>
                        <template v-else-if="form.structureMode === 'free'">只锁定范围、总分和时长，允许 AI 自行设计卷面结构。</template>
                        <template v-else>模板作为参考，AI 可按知识点适度调整题型和分值。</template>
                      </div>
                    </el-form-item>
                    <el-form-item label="满分">
                      <el-input-number v-model="form.totalScore" :min="30" :max="150" class="w-full" controls-position="right" />
                    </el-form-item>
                    <el-form-item label="难度配比（%）">
                      <div class="ratio-row">
                        <div class="ratio-item">
                          <span>基础</span>
                          <el-input-number v-model="form.ratioBasic" :min="0" :max="100" :step="5" controls-position="right" size="small" />
                        </div>
                        <div class="ratio-item">
                          <span>中等</span>
                          <el-input-number v-model="form.ratioMedium" :min="0" :max="100" :step="5" controls-position="right" size="small" />
                        </div>
                        <div class="ratio-item">
                          <span>拔高</span>
                          <el-input-number v-model="form.ratioHard" :min="0" :max="100" :step="5" controls-position="right" size="small" />
                        </div>
                      </div>
                      <div class="field-hint" :class="{ warn: ratioSum !== 100 }">合计 {{ ratioSum }}%</div>
                    </el-form-item>
                    <el-form-item label="组卷策略">
                      <el-checkbox v-model="form.mixBank">题库混组</el-checkbox>
                      <el-checkbox v-model="form.useSchoolBank">校本收藏参与</el-checkbox>
                      <div class="bank-sync-row">
                        <el-button
                          size="small"
                          plain
                          type="primary"
                          :icon="Refresh"
                          :loading="syncingQuestionBank"
                          :disabled="loading || updatingCurriculum"
                          @click="onSyncQuestionBank"
                        >
                          同步公开题库
                        </el-button>
                        <span class="field-hint">
                          本机已缓存 {{ publicBankResources.length }} 份素材
                        </span>
                      </div>
                      <div v-if="publicBankLastReport" class="field-hint">
                        最近同步：{{ publicBankLastReport.message }}
                      </div>
                    </el-form-item>
                  </el-collapse-item>
                </el-collapse>
              </template>

              <template v-else>
                <el-form-item label="单元">
                  <el-select v-model="form.unitId" class="w-full" placeholder="选择单元" filterable>
                    <el-option
                      v-for="u in unitOptions"
                      :key="u.id"
                      :label="u.name"
                      :value="u.id"
                    />
                  </el-select>
                </el-form-item>
                <el-form-item label="课时">
                  <el-select
                    v-model="lessonName"
                    class="w-full"
                    filterable
                    allow-create
                    default-first-option
                    placeholder="选择或输入课时名称"
                  >
                    <el-option v-for="n in lessonOptions" :key="n" :label="n" :value="n" />
                  </el-select>
                </el-form-item>
                <el-form-item label="课型">
                  <el-select v-model="lessonType" class="w-full">
                    <el-option label="新授课" value="new" />
                    <el-option label="练习课" value="practice" />
                    <el-option label="复习课" value="review" />
                    <el-option label="讲评课" value="feedback" />
                  </el-select>
                </el-form-item>
                <div class="form-row-2">
                  <el-form-item label="第几课时">
                    <el-input-number v-model="lessonPeriods" :min="1" :max="8" class="w-full" controls-position="right" />
                  </el-form-item>
                  <el-form-item label="时长(分)">
                    <el-input-number v-model="form.durationMin" :min="20" :max="90" class="w-full" controls-position="right" />
                  </el-form-item>
                </div>
                <el-form-item label="教案对象">
                  <el-checkbox v-model="includeParentGuide">同时生成家长辅导手册</el-checkbox>
                  <div class="field-hint">教师版用于上课；家长版用语通俗，方便回家陪读</div>
                </el-form-item>
              </template>
            </el-form>

            <div v-if="!currentPack" class="curriculum-bar warn compact-bar">
              未匹配课标，请同步课标或更换年级册次
            </div>
            <div v-else-if="form.templateId" class="template-chip">
              <span>模板：{{ activeTemplate?.name || form.templateId }}</span>
              <el-button size="small" text type="danger" @click="clearSelectedTemplate">取消</el-button>
            </div>
          </div>

          <div class="card-foot action-foot">
            <template v-if="workMode === 'exam'">
              <el-button
                type="primary"
                size="large"
                class="btn-primary-block"
                :icon="MagicStick"
                :loading="loading && !linking"
                :disabled="updatingCurriculum || linking"
                @click="onAiGenerate"
              >
                {{ loading && !linking ? '组卷中…' : structureModeLabel }}
              </el-button>
              <el-button
                class="btn-secondary-block"
                :loading="linking"
                :disabled="updatingCurriculum || loading || lessonLoading"
                @click="onGenerateLinked"
              >
                {{ linking ? `联动中（${linkProgress || '…'}）` : '一键：教案 + 练习卷' }}
              </el-button>

              <div class="action-grid four">
                <el-button
                  class="flex-1"
                  :disabled="loading || updatingCurriculum || linking"
                  @click="openTemplateMarket"
                >
                  模板市集
                </el-button>
                <el-button class="flex-1" @click="openCurriculumBrowser">
                  查看课标
                </el-button>
                <el-button
                  class="flex-1"
                  :icon="Upload"
                  :loading="updatingCurriculum"
                  @click="onUpdateCurriculum"
                >
                  同步课标
                </el-button>
                <el-dropdown
                  class="tools-dropdown flex-1"
                  trigger="click"
                  @command="onExamToolCommand"
                >
                  <el-button class="w-full tools-dropdown-btn">
                    更多工具
                    <el-icon class="el-icon--right"><ArrowDown /></el-icon>
                  </el-button>
                  <template #dropdown>
                    <el-dropdown-menu>
                      <el-dropdown-item command="shell" :disabled="loading || linking">快速空壳</el-dropdown-item>
                      <el-dropdown-item command="saveTpl" :disabled="!paper" divided>存为模板</el-dropdown-item>
                      <el-dropdown-item command="bank">校本库</el-dropdown-item>
                      <el-dropdown-item command="saveBank" :disabled="!paper">卷存入校本</el-dropdown-item>
                      <el-dropdown-item command="quality" :disabled="!paper" divided>质检</el-dropdown-item>
                      <el-dropdown-item command="spec" :disabled="!paper">细目表</el-dropdown-item>
                      <el-dropdown-item command="verify" :disabled="!paper">数学验算</el-dropdown-item>
                        <el-dropdown-item command="b" :disabled="!paper || makingB || loading">生成 B 卷</el-dropdown-item>
                      <el-dropdown-item command="parallel" :disabled="!paper || makingParallel">平行卷 A/B/C</el-dropdown-item>
                      <el-dropdown-item command="review" :disabled="!paper" divided>讲评稿</el-dropdown-item>
                      <el-dropdown-item command="redrill" :disabled="!paper || redrillLoading">错题再练卷</el-dropdown-item>
                      <el-dropdown-item command="browseKb" divided>查看课标</el-dropdown-item>
                      <el-dropdown-item command="ebookPrint">打印电子书单元</el-dropdown-item>
                      <el-dropdown-item command="prefs">设为默认参数</el-dropdown-item>
                    </el-dropdown-menu>
                  </template>
                </el-dropdown>
              </div>
            </template>

            <template v-else>
              <el-button
                type="primary"
                size="large"
                class="btn-primary-block"
                :icon="MagicStick"
                :loading="lessonLoading && !linking"
                :disabled="updatingCurriculum || linking"
                @click="onGenerateLesson"
              >
                {{ lessonLoading && !linking ? '生成中…' : '智能写教案' }}
              </el-button>
              <el-button
                class="btn-secondary-block"
                :loading="linking"
                :disabled="updatingCurriculum || loading || lessonLoading"
                @click="onGenerateLinked"
              >
                {{ linking ? `联动中（${linkProgress || '…'}）` : '一键：教案 + 练习卷' }}
              </el-button>
              <div class="action-grid">
                <el-button
                  class="flex-1"
                  :disabled="lessonLoading || updatingCurriculum || linking"
                  @click="onGenerateUnitAllLessons(true)"
                >
                  全课时教案
                </el-button>
                <el-button
                  class="flex-1"
                  :disabled="lessonLoading || linking"
                  @click="openTemplateMarket"
                >
                  模板市集
                </el-button>
              </div>
              <el-dropdown class="tools-dropdown" trigger="click" @command="onExamToolCommand">
                <el-button class="w-full tools-dropdown-btn">
                  更多
                  <el-icon class="el-icon--right"><ArrowDown /></el-icon>
                </el-button>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="lessonTpl">单课结构模板</el-dropdown-item>
                    <el-dropdown-item command="unitTpl">全课时结构模板</el-dropdown-item>
                    <el-dropdown-item command="sync" :disabled="updatingCurriculum" divided>同步课标</el-dropdown-item>
                    <el-dropdown-item command="browseKb">查看课标</el-dropdown-item>
                    <el-dropdown-item command="prefs">设为默认参数</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </template>
          </div>
        </div>
      </aside>

      <!-- 右侧预览 -->
      <main class="main">
        <div class="card preview-card">
          <div class="card-head preview-head">
            <div class="preview-title-row">
              <span class="card-title">预览</span>
              <el-radio-group
                v-if="paper && lessonPlan"
                v-model="previewTab"
                size="small"
                class="preview-tab"
              >
                <el-radio-button value="lesson">教案</el-radio-button>
                <el-radio-button value="exam">练习卷</el-radio-button>
              </el-radio-group>
              <div
                v-if="paper && (previewTab === 'exam' || !lessonPlan)"
                class="exam-view-controls"
              >
                <el-radio-group v-model="examViewMode" size="small" class="exam-view-switch">
                  <el-radio-button value="student">学生卷</el-radio-button>
                  <el-radio-button value="teacher">教师</el-radio-button>
                  <el-radio-button value="answers">答案</el-radio-button>
                </el-radio-group>
                <div class="preview-zoom" aria-label="预览缩放">
                  <el-tooltip content="缩小预览" placement="bottom">
                    <el-button
                      size="small"
                      text
                      circle
                      :icon="ZoomOut"
                      :disabled="previewZoom <= 0.75"
                      aria-label="缩小预览"
                      @click="setPreviewZoom(previewZoom - 0.1)"
                    />
                  </el-tooltip>
                  <button class="zoom-value" type="button" title="恢复 100%" @click="setPreviewZoom(1)">
                    {{ Math.round(previewZoom * 100) }}%
                  </button>
                  <el-tooltip content="放大预览" placement="bottom">
                    <el-button
                      size="small"
                      text
                      circle
                      :icon="ZoomIn"
                      :disabled="previewZoom >= 1.3"
                      aria-label="放大预览"
                      @click="setPreviewZoom(previewZoom + 0.1)"
                    />
                  </el-tooltip>
                </div>
              </div>
            </div>
            <div class="preview-tools" v-if="paper || lessonPlan || lessonBundle">
              <el-button
                size="small"
                type="primary"
                :icon="Download"
                @click="exportDocx"
              >
                导出 Word
              </el-button>
              <el-button
                size="small"
                type="success"
                plain
                :icon="Printer"
                @click="openPrintPreview(false)"
              >
                打印
              </el-button>
              <el-button
                v-if="paper && (previewTab === 'exam' || !displayLesson)"
                size="small"
                plain
                @click="openPrintPreview(true)"
              >
                打印答案
              </el-button>
              <el-button size="small" plain @click="openOutputCenter">产出中心</el-button>
              <el-dropdown
                v-if="paper && (previewTab === 'exam' || !lessonPlan)"
                trigger="click"
                size="small"
                @command="onExamToolCommand"
              >
                <el-button size="small" plain>
                  教研
                  <el-icon class="el-icon--right"><ArrowDown /></el-icon>
                </el-button>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="quality">质检</el-dropdown-item>
                    <el-dropdown-item command="spec">细目表</el-dropdown-item>
                    <el-dropdown-item command="review">讲评稿</el-dropdown-item>
                    <el-dropdown-item command="verify">数学验算</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
              <el-button size="small" text @click="headerPreviewVisible = true">卷头</el-button>
              <el-button size="small" text :icon="Refresh" @click="clearPaper">清空</el-button>
            </div>
          </div>

          <div class="card-body preview-body">
            <!-- 教案预览 -->
            <div
              v-if="displayLesson && (previewTab === 'lesson' || (!paper && workMode === 'lesson'))"
              class="exam-sheet"
            >
              <div v-if="lessonBundle?.plans?.length" class="bundle-tabs">
                <el-tag
                  v-for="(pl, i) in lessonBundle.plans"
                  :key="i"
                  class="bundle-tab"
                  :effect="bundleIndex === i ? 'dark' : 'plain'"
                  :type="pl.error ? 'danger' : 'primary'"
                  round
                  @click="selectBundlePlan(i)"
                >
                  {{ pl.meta?.lessonName || pl.meta?.title || `第${i + 1}课` }}
                </el-tag>
              </div>
              <div class="exam-paper lesson-paper">
                <div class="lesson-audience-bar" v-if="displayLesson.parentGuide || includeParentGuide">
                  <el-radio-group v-model="lessonAudienceView" size="small">
                    <el-radio-button value="teacher">教师版</el-radio-button>
                    <el-radio-button value="parent" :disabled="!displayLesson.parentGuide">
                      家长版{{ displayLesson.parentGuide ? '' : '（未生成）' }}
                    </el-radio-button>
                  </el-radio-group>
                </div>

                <!-- 家长版预览 -->
                <template v-if="lessonAudienceView === 'parent' && displayLesson.parentGuide">
                  <div class="exam-title">家长辅导手册</div>
                  <div class="exam-sub">
                    {{ displayLesson.meta.edition }} · {{ displayLesson.meta.subject }} ·
                    {{ displayLesson.meta.grade }}年级{{ displayLesson.meta.semester }} ·
                    {{ displayLesson.meta.unitName }}
                  </div>
                  <div class="lesson-topic">课题：{{ displayLesson.meta.title || displayLesson.meta.lessonName }}</div>
                  <p class="parent-summary">{{ displayLesson.parentGuide.summary }}</p>
                  <div class="lesson-block">
                    <h4>一、孩子这节课要会什么</h4>
                    <ul><li v-for="(t, i) in displayLesson.parentGuide.goalsInPlain || []" :key="'pg'+i">{{ t }}</li></ul>
                  </div>
                  <div class="lesson-block">
                    <h4>二、课前预习建议</h4>
                    <ul><li v-for="(t, i) in displayLesson.parentGuide.previewTips || []" :key="'pp'+i">{{ t }}</li></ul>
                  </div>
                  <div class="lesson-block">
                    <h4>三、怎么陪</h4>
                    <div
                      v-for="(st, i) in displayLesson.parentGuide.accompanySteps || []"
                      :key="'ps'+i"
                      class="parent-step"
                    >
                      <b>{{ st.step }}<span v-if="st.minutes">（约 {{ st.minutes }} 分钟）</span></b>
                      <p v-if="st.how">怎么做：{{ st.how }}</p>
                      <p v-if="st.say" class="parent-say">可以说：{{ st.say }}</p>
                    </div>
                  </div>
                  <div class="lesson-block">
                    <h4>四、可以这样问</h4>
                    <ul><li v-for="(t, i) in displayLesson.parentGuide.keyQuestions || []" :key="'pq'+i">{{ t }}</li></ul>
                  </div>
                  <div class="lesson-block">
                    <h4>五、常见卡点与纠正</h4>
                    <ul><li v-for="(t, i) in displayLesson.parentGuide.commonMistakes || []" :key="'pm'+i">{{ t }}</li></ul>
                  </div>
                  <div class="lesson-block">
                    <h4>六、家庭小练习</h4>
                    <ul><li v-for="(t, i) in displayLesson.parentGuide.homePractice || []" :key="'ph'+i">{{ t }}</li></ul>
                  </div>
                  <div class="lesson-block parent-encourage">
                    <h4>鼓励孩子</h4>
                    <p>{{ displayLesson.parentGuide.encourage || '今天你已经很努力了。' }}</p>
                  </div>
                </template>

                <!-- 教师版预览 -->
                <template v-else>
                  <div class="exam-title">教案（教师版）</div>
                  <div class="exam-sub">
                    {{ displayLesson.meta.edition }} · {{ displayLesson.meta.subject }} ·
                    {{ displayLesson.meta.grade }}年级{{ displayLesson.meta.semester }} ·
                    {{ displayLesson.meta.unitName }}
                  </div>
                  <div class="lesson-topic">课题：{{ displayLesson.meta.title || displayLesson.meta.lessonName }}</div>
                  <div class="exam-info">
                    <span v-if="displayLesson.meta.lessonType">课型：{{ displayLesson.meta.lessonType }}　　</span>
                    第 {{ displayLesson.meta.periods || 1 }} 课时　　约 {{ displayLesson.meta.durationMin || 40 }} 分钟
                  </div>
                  <div v-if="displayLesson.error" class="meta-line error">生成失败：{{ displayLesson.error }}</div>
                  <template v-else>
                    <div class="lesson-block">
                      <h4>一、教学目标</h4>
                      <p class="sub-h">知识与技能</p>
                      <ul><li v-for="(t, i) in displayLesson.objectives?.knowledge || []" :key="'k'+i">{{ t }}</li></ul>
                      <p class="sub-h">过程与方法</p>
                      <ul><li v-for="(t, i) in displayLesson.objectives?.ability || []" :key="'a'+i">{{ t }}</li></ul>
                      <p class="sub-h">情感态度与价值观</p>
                      <ul><li v-for="(t, i) in displayLesson.objectives?.emotion || []" :key="'e'+i">{{ t }}</li></ul>
                    </div>
                    <div class="lesson-block">
                      <h4>二、教学重难点</h4>
                      <p><b>重点：</b>{{ (displayLesson.keyPoints || []).join('；') }}</p>
                      <p><b>难点：</b>{{ (displayLesson.difficultPoints || []).join('；') }}</p>
                    </div>
                    <div class="lesson-block">
                      <h4>三、教学准备</h4>
                      <p>教师：{{ (displayLesson.preparation?.teacher || []).join('、') }}</p>
                      <p>学生：{{ (displayLesson.preparation?.student || []).join('、') }}</p>
                    </div>
                    <div class="lesson-block">
                      <h4>四、教学过程</h4>
                      <table class="lesson-process-table">
                        <thead>
                          <tr>
                            <th>环节</th><th>时间</th><th>内容</th><th>教师活动</th><th>学生活动</th>
                          </tr>
                        </thead>
                        <tbody>
                          <tr v-for="(step, i) in displayLesson.process || []" :key="i">
                            <td>{{ step.stage }}</td>
                            <td>{{ step.minutes != null ? step.minutes + '′' : '' }}</td>
                            <td class="pre-cell">{{ step.content }}<span v-if="step.intent" class="intent">（{{ step.intent }}）</span></td>
                            <td class="pre-cell">{{ step.teacherActivity }}</td>
                            <td class="pre-cell">{{ step.studentActivity }}</td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                    <div class="lesson-block">
                      <h4>五、板书设计</h4>
                      <pre class="board-pre">{{ displayLesson.boardDesign || '—' }}</pre>
                    </div>
                    <div class="lesson-block">
                      <h4>六、作业布置</h4>
                      <ul><li v-for="(t, i) in displayLesson.homework || []" :key="'h'+i">{{ t }}</li></ul>
                    </div>
                    <div class="lesson-block">
                      <h4>七、教学反思</h4>
                      <p>{{ displayLesson.reflection || '（课后填写）' }}</p>
                    </div>
                  </template>
                </template>
              </div>
              <div class="exam-foot-tip">
                可切换「教师版 / 家长版」后导出或打印。家长版用语通俗，适合家校共育。
              </div>
            </div>

            <div v-else-if="workMode === 'lesson' && !lessonPlan && !paper" class="empty-state">
              <div class="empty-icon">案</div>
              <div class="empty-title">还没有教案</div>
              <div class="empty-desc">
                1. 左侧选好单元与课时<br />
                2. 点「智能写教案」，或「一键：教案 + 练习卷」<br />
                3. 需要统一结构时，打开「模板市集」
              </div>
            </div>

            <!-- 试卷预览 -->
            <div v-else-if="!paper" class="empty-state">
              <div class="empty-icon">卷</div>
              <div class="empty-title">开始一份新试卷</div>
              <div v-if="historyList.length" class="recent-work">
                <div class="recent-work-title">最近使用</div>
                <button
                  v-for="h in historyList.slice(0, 3)"
                  :key="h.id"
                  type="button"
                  class="recent-work-item"
                  @click="loadHistoryEntry(h.id)"
                >
                  <span class="recent-work-name">{{ h.title }}</span>
                  <span class="recent-work-meta">{{ historyKindLabel(h) }} · {{ formatHistoryTime(h.createdAt) }}</span>
                </button>
              </div>
              <div v-else class="empty-paper-lines" aria-hidden="true">
                <span></span><span></span><span></span><span></span>
              </div>
              <el-button type="primary" :icon="MagicStick" :loading="loading" @click="onAiGenerate">
                智能组卷
              </el-button>
            </div>
            <div
              v-else-if="paper && (previewTab === 'exam' || !lessonPlan)"
              class="exam-sheet"
              :style="{ zoom: previewZoom }"
            >
              <div v-if="parallelSet?.papers?.length" class="bundle-tabs">
                <el-tag
                  v-for="(p, i) in parallelSet.papers"
                  :key="i"
                  class="bundle-tab"
                  :effect="parallelIndex === i ? 'dark' : 'plain'"
                  type="success"
                  round
                  @click="selectParallel(i)"
                >
                  {{ p.meta?.variant || ['A', 'B', 'C'][i] || i + 1 }} 卷
                </el-tag>
              </div>
              <div
                class="exam-paper"
                :class="{
                  'student-view': examViewMode === 'student',
                  'answer-view': examViewMode === 'answers',
                }"
              >
                <div class="exam-title">{{ paper.meta.title }}</div>
                <div class="exam-sub">
                  {{ paper.meta.edition }} · {{ paper.meta.subject }} ·
                  {{ paper.meta.grade }}年级{{ paper.meta.semester }} · {{ paper.meta.examType }}
                  <span class="sep">|</span>
                  满分 {{ paper.meta.totalScore }} 分
                  <span class="sep">|</span>
                  {{ paper.meta.durationMin }} 分钟
                </div>
                <div class="exam-info">
                  班级：________　　姓名：________　　学号：________　　得分：________
                </div>
                <div class="exam-body">
                  <div v-for="(sec, si) in paper.sections" :key="si" class="section-block">
                    <h4 class="sec-title">{{ sec.title }}</h4>
                    <div
                      v-for="(item, ii) in sec.items"
                      :key="ii"
                      class="item-line"
                      :class="{
                        'verify-mismatch': verifyItemOf(si, ii)?.status === 'mismatch',
                        'verify-ok': verifyItemOf(si, ii)?.status === 'ok',
                        'item-wrong': isWrong(si, ii),
                      }"
                    >
                      <div class="item-head">
                        <div class="stem">{{ item.stem }}</div>
                        <div v-if="examViewMode === 'teacher'" class="item-actions">
                          <el-checkbox
                            :model-value="isWrong(si, ii)"
                            size="small"
                            @change="toggleWrong(si, ii)"
                          >
                            错题
                          </el-checkbox>
                          <el-button size="small" text type="warning" @click="favoriteItem(si, ii)">
                            收藏
                          </el-button>
                          <el-button
                            size="small"
                            text
                            type="primary"
                            :loading="regeneratingKey === `${si}-${ii}`"
                            :disabled="!!regeneratingKey || loading"
                            @click="regenerateItem(si, ii)"
                          >
                            换题
                          </el-button>
                        </div>
                      </div>
                      <div v-if="item.options?.length" class="opts">
                        <span v-for="(op, oi) in item.options" :key="oi" class="opt">{{ op }}</span>
                      </div>
                      <div v-if="examViewMode !== 'student'" class="ans-hint">
                        答案：{{ item.answer || '—' }}
                        <span
                          v-if="examViewMode === 'teacher' && item.knowledgePoints?.length"
                          class="kp-hint"
                        >
                          · {{ item.knowledgePoints.join('、') }}
                        </span>
                        <template v-if="examViewMode === 'teacher' && verifyItemOf(si, ii)">
                          <el-tag
                            size="small"
                            class="verify-tag"
                            :type="verifyStatusType(verifyItemOf(si, ii)!.status)"
                            effect="plain"
                          >
                            {{ verifyStatusLabel(verifyItemOf(si, ii)!.status) }}
                            <template v-if="verifyItemOf(si, ii)?.computed">
                              · 验算 {{ verifyItemOf(si, ii)!.computed }}
                            </template>
                          </el-tag>
                        </template>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              <div v-if="examViewMode === 'teacher'" class="exam-foot-tip">
                顶部可直接「导出 Word」「打印」；勾选「错题」可做讲评/再练；单题可收藏到校本库。
              </div>
            </div>
          </div>
        </div>
      </main>
    </div>

    <AiProgressDialog
      v-model="aiPanelVisible"
      :status="aiStatus"
      :target="aiTargetLabel"
      :progress="aiProgress"
      :elapsed="aiElapsed"
      :steps="aiSteps"
      :step-index="aiStepIndex"
      :tips="aiTips"
      :tip-index="aiTipIndex"
      :success-summary="aiSuccessSummary"
      :error-text="aiErrorText"
      @settings="settingsVisible = true"
      @retry="onAiGenerate"
      @cancel="cancelAiGeneration"
    />

    <!-- 首次使用引导 -->
    <el-dialog
      v-model="onboardingVisible"
      title="欢迎使用试卷神器"
      width="480px"
      align-center
      :close-on-click-modal="false"
    >
      <div class="onboard-steps">
        <div class="onboard-step" :class="{ active: onboardingStep === 0, done: onboardingStep > 0 }">
          <div class="onboard-n">1</div>
          <div>
            <b>配置 AI 接口</b>
            <p class="field-hint">填写 API Base / Key / 模型。也可选「本地 Ollama」离线使用。</p>
          </div>
        </div>
        <div class="onboard-step" :class="{ active: onboardingStep === 1, done: onboardingStep > 1 }">
          <div class="onboard-n">2</div>
          <div>
            <b>选课标并同步</b>
            <p class="field-hint">左侧选学科年级；可点「同步课标」拉取 dzkbw.org 最新目录，用「查看课标」对照。</p>
          </div>
        </div>
        <div class="onboard-step" :class="{ active: onboardingStep === 2 }">
          <div class="onboard-n">3</div>
          <div>
            <b>生成 → 导出 / 打印</b>
            <p class="field-hint">智能组卷或写教案后，用预览顶栏「导出 Word」「打印」即可。</p>
          </div>
        </div>
      </div>
      <template #footer>
        <el-button v-if="onboardingStep > 0" @click="onboardingStep--">上一步</el-button>
        <el-button v-if="onboardingStep === 0" type="primary" @click="settingsVisible = true; onboardingStep = 1">
          去配置接口
        </el-button>
        <el-button v-else-if="onboardingStep === 1" type="primary" @click="onboardingStep = 2">
          下一步
        </el-button>
        <el-button v-else type="primary" @click="finishOnboarding">开始使用</el-button>
        <el-button text @click="finishOnboarding">跳过</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="settingsVisible"
      title="接口与偏好设置"
      width="560px"
      top="3vh"
      class="settings-dialog"
    >
      <el-form label-width="120px">
        <div class="settings-section-title">AI 接口</div>
        <el-form-item label="服务商">
          <el-select
            v-model="config.providerId"
            style="width: 100%"
            @change="applyPreset"
          >
            <el-option
              v-for="p in presets"
              :key="p.id"
              :label="p.name"
              :value="p.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="接口地址">
          <el-input
            v-model="config.apiBase"
            placeholder="https://api.example.com/v1"
          />
          <div class="field-tip">
            OpenAI 兼容根地址，一般以 <code>/v1</code> 结尾；勿填网站首页。
          </div>
        </el-form-item>
        <el-form-item label="API 密钥">
          <el-input
            v-model="config.apiKey"
            type="password"
            show-password
            :placeholder="config.apiKeyConfigured ? '已安全保存，留空表示不修改' : '使用当前 Windows 用户加密保存'"
          />
          <el-button
            v-if="config.apiKeyConfigured"
            type="danger"
            plain
            size="small"
            style="margin-top: 8px"
            @click="clearSavedApiKey"
          >
            清除已保存密钥
          </el-button>
          <div class="field-tip">
            {{ aiReadyHint }}。选择本地 Ollama 时可不填 API Key；云端服务仍需填写密钥。
          </div>
        </el-form-item>
        <el-form-item label="模型">
          <el-select
            v-model="config.model"
            filterable
            allow-create
            default-first-option
            style="width: 100%"
            placeholder="输入或选择模型名称"
          >
            <el-option v-for="m in modelOptions" :key="m" :label="m" :value="m" />
          </el-select>
        </el-form-item>
        <el-form-item label="随机度">
          <el-slider v-model="config.temperature" :min="0" :max="1" :step="0.1" show-input />
        </el-form-item>

        <div class="settings-section-title">校名卷头</div>
        <el-form-item label="学校名称">
          <el-input v-model="config.schoolName" placeholder="如：某某小学" />
        </el-form-item>
        <el-form-item label="学年度">
          <el-input v-model="config.academicYear" placeholder="如：2025—2026 学年度" />
        </el-form-item>
        <el-form-item label="学期">
          <el-select v-model="config.schoolTerm" clearable placeholder="可选" style="width: 100%">
            <el-option label="上学期" value="上学期" />
            <el-option label="下学期" value="下学期" />
          </el-select>
        </el-form-item>
        <el-form-item label="默认班级">
          <el-input v-model="config.defaultClassName" placeholder="如：三年级（2）班，可空" />
        </el-form-item>

        <div class="settings-section-title">导出与历史</div>
        <el-form-item label="导出档位">
          <el-select v-model="config.exportMode" style="width: 100%">
            <el-option label="仅学生卷" value="student" />
            <el-option label="学生卷 + 文末答案页" value="with_answers" />
            <el-option label="分文件：学生卷 + 答案（两次另存）" value="both" />
          </el-select>
        </el-form-item>
        <el-form-item label="文件名模板">
          <el-input
            v-model="config.exportFilenamePattern"
            placeholder="{school}{grade}年级-{subject}-{title}-{date}"
          />
          <div class="field-hint">可用：{'{school} {grade} {subject} {title} {date} {variant} {type}'}</div>
        </el-form-item>
        <el-form-item label="历史保留">
          <el-input-number v-model="config.historyMax" :min="5" :max="100" />
          <span class="field-tip-inline">条</span>
        </el-form-item>
        <el-alert
          type="info"
          :closable="false"
          show-icon
          title="保存时同步：当前组卷参数为默认；校名卷头用于导出与打印"
        />
      </el-form>
      <template #footer>
        <el-button @click="settingsVisible = false">取消</el-button>
        <el-button type="primary" @click="saveSettings">保存</el-button>
      </template>
    </el-dialog>

    <!-- 自有站电子书单元页图打印 -->
    <el-dialog
      v-model="ebookDialogVisible"
      title="打印电子书单元（自有站页图）"
      width="720px"
      top="6vh"
      class="ebook-print-dlg"
    >
      <p class="field-hint" style="margin-top: 0">
        对接自有资源站阅读页：根据链接中的 resId / bookId / contributeId 拉取目录与分页 JPG，再生成打印稿。
        默认示例为 100875 数学三年级上册。
      </p>
      <div class="form-grid" style="gap: 10px">
        <div class="field" style="grid-column: 1 / -1">
          <label>阅读页链接</label>
          <el-input
            v-model="ebookForm.url"
            type="textarea"
            :rows="2"
            placeholder="eBookAndTeacher.html?resId=...&bookId=...&contributeId=..."
          />
          <div class="btn-row" style="margin-top: 8px">
            <el-button type="primary" plain @click="parseEbookUrl">解析链接</el-button>
            <el-button :loading="ebookLoading" @click="loadEbookCatalog">加载目录</el-button>
          </div>
        </div>
        <div class="field">
          <label>站点 Base</label>
          <el-input v-model="ebookForm.baseUrl" placeholder="https://www.100875.com.cn" />
        </div>
        <div class="field">
          <label>resId</label>
          <el-input v-model="ebookForm.resId" />
        </div>
        <div class="field">
          <label>contributeId</label>
          <el-input v-model="ebookForm.contributeId" />
        </div>
        <div class="field">
          <label>最多页数（防过长）</label>
          <el-input-number v-model="ebookForm.maxPages" :min="1" :max="80" />
        </div>
        <div class="field" style="grid-column: 1 / -1" v-if="ebookCatalog">
          <label>
            选择单元
            <span class="field-hint" style="margin-left: 8px">
              {{ ebookCatalog.subjectName }} · {{ ebookCatalog.bookName }}
            </span>
          </label>
          <el-select v-model="ebookForm.bookId" filterable style="width: 100%" placeholder="单元">
            <el-option
              v-for="u in ebookUnitOptions"
              :key="u.bookId"
              :label="u.cataName"
              :value="u.bookId"
            />
          </el-select>
        </div>
        <div class="field" style="grid-column: 1 / -1" v-if="ebookUnitPages">
          <el-alert
            type="success"
            :closable="false"
            :title="`已缓存：${ebookUnitPages.unitName} · 第 ${ebookUnitPages.startPage}–${ebookUnitPages.endPage} 页 · ${ebookUnitPages.pages?.length || 0} 张`"
          />
        </div>
      </div>
      <template #footer>
        <el-button @click="ebookDialogVisible = false">关闭</el-button>
        <el-button
          type="primary"
          :icon="Printer"
          :loading="ebookFetchingPages"
          :disabled="!ebookForm.resId || !ebookForm.bookId || !ebookForm.contributeId"
          @click="fetchEbookUnitAndPreview"
        >
          拉取页图并预览打印
        </el-button>
      </template>
    </el-dialog>

    <!-- 课标浏览（对照目录） -->
    <el-dialog
      v-model="curriculumBrowserVisible"
      title="课标浏览 · 对照目录"
      width="920px"
      top="4vh"
      class="curriculum-browser-dlg"
    >
      <div class="cb-toolbar">
        <el-select v-model="browserSubject" size="small" style="width: 100px">
          <el-option label="数学" value="math" />
          <el-option label="语文" value="chinese" />
          <el-option label="英语" value="english" />
        </el-select>
        <el-select v-model="browserEdition" size="small" style="width: 120px">
          <el-option
            v-for="e in browserEditionOptions"
            :key="e.value"
            :label="e.label"
            :value="e.value"
          />
        </el-select>
        <el-select v-model="browserGrade" size="small" style="width: 100px">
          <el-option v-for="g in browserGradeOptions" :key="g" :label="`${g} 年级`" :value="g" />
        </el-select>
        <el-select v-model="browserSemester" size="small" style="width: 90px">
          <el-option label="上册" value="shang" />
          <el-option label="下册" value="xia" />
        </el-select>
        <el-input
          v-model="browserKeyword"
          size="small"
          clearable
          placeholder="搜索单元/课时/要点"
          style="width: 180px"
        />
        <el-tag v-if="browserPack" size="small" :type="browserPack.origin === 'user' ? 'success' : 'info'" effect="plain">
          {{ browserPack.origin === 'user' ? '已同步' : '内置' }}
        </el-tag>
        <span class="field-hint" style="margin: 0">共 {{ catalog.length }} 册</span>
      </div>

      <div class="cb-layout">
        <aside class="cb-pack-list">
          <div class="cb-side-title">本机课标包</div>
          <div
            v-for="c in catalogPackList"
            :key="c.path"
            class="cb-pack-item"
            :class="{
              active:
                c.subject === browserSubject &&
                c.edition === browserEdition &&
                c.grade === browserGrade &&
                c.semester === browserSemester,
            }"
            @click="selectBrowserPack(c)"
          >
            <div class="cb-pack-name">
              {{ c.subjectLabel }} · {{ c.editionLabel }}
            </div>
            <div class="cb-pack-meta">
              {{ c.grade }}年级{{ c.semesterLabel || (c.semester === 'shang' ? '上' : '下') }}
              · {{ c.units.length }} 单元
              <el-tag size="small" effect="plain" :type="c.origin === 'user' ? 'success' : 'info'" round>
                {{ c.origin === 'user' ? '同步' : '内置' }}
              </el-tag>
            </div>
          </div>
        </aside>

        <div class="cb-main" v-if="browserPack">
          <div class="cb-pack-head">
            <div>
              <div class="cb-title">{{ browserPack.title }}</div>
              <div class="field-hint">
                {{ browserPack.editionLabel }} · {{ browserPack.subjectLabel }}
                · path: {{ browserPack.path }}
              </div>
              <div class="field-hint" v-if="browserPack.source?.catalogRef">
                来源：{{ browserPack.source.catalogRef }}
              </div>
              <div class="field-hint" v-if="browserPack.source?.note">
                {{ browserPack.source.note }}
              </div>
            </div>
            <div class="cb-head-actions">
              <el-button size="small" plain :icon="Link" @click="openBrowserCatalogSource">打开网站目录</el-button>
              <el-button size="small" plain @click="copyBrowserAllText">复制全册</el-button>
              <el-button size="small" type="warning" plain :loading="diffLoading" @click="runCurriculumDiff">
                内置 vs 同步
              </el-button>
            </div>
          </div>
          <div v-if="curriculumDiff" class="cb-diff-bar">
            <el-tag size="small" effect="plain">{{ curriculumDiff.summary }}</el-tag>
            <span class="field-hint" v-if="curriculumDiff.hasBundled && curriculumDiff.hasUser">
              绿=一致 · 橙=有差异 · 蓝=仅同步 · 灰=仅内置
            </span>
          </div>

          <div class="cb-body">
            <div class="cb-units">
              <div
                v-for="u in browserUnits"
                :key="u.id"
                class="cb-unit-item"
                :class="{ active: browserActiveUnit?.id === u.id }"
                @click="browserUnitId = u.id"
              >
                <div class="cb-unit-name">
                  {{ u.name }}
                  <el-tag
                    v-if="curriculumDiff"
                    size="small"
                    class="cb-diff-tag"
                    :type="diffStatusType(curriculumDiff.units.find((d) => d.unitId === u.id || d.unitName === u.name)?.status || 'same')"
                    effect="plain"
                  >
                    {{ diffStatusLabel(curriculumDiff.units.find((d) => d.unitId === u.id || d.unitName === u.name)?.status || 'same') }}
                  </el-tag>
                </div>
                <div class="cb-unit-count">{{ u.lessons?.length || 0 }} 课时</div>
              </div>
              <div v-if="!browserUnits.length" class="field-hint" style="padding: 12px">无匹配单元</div>
            </div>
            <div class="cb-detail" v-if="browserActiveUnit">
              <div class="cb-detail-head">
                <b>{{ browserActiveUnit.name }}</b>
                <el-button size="small" type="primary" plain @click="copyBrowserUnitText">复制本单元</el-button>
              </div>
              <div
                v-if="curriculumDiff"
                class="cb-section cb-diff-detail"
              >
                <template v-for="d in curriculumDiff.units.filter((x) => x.unitId === browserActiveUnit.id || x.unitName === browserActiveUnit.name)" :key="d.unitId">
                  <div class="cb-section-title">对照差异</div>
                  <p v-if="d.added?.length" class="diff-add">+ 同步多出：{{ d.added.join('、') }}</p>
                  <p v-if="d.removed?.length" class="diff-del">− 内置有、同步无：{{ d.removed.join('、') }}</p>
                  <p v-if="!d.added?.length && !d.removed?.length && d.status === 'same'" class="field-hint">课时列表一致</p>
                  <p v-if="d.status === 'onlyUser'" class="diff-add">仅存在于同步课标</p>
                  <p v-if="d.status === 'onlyBundled'" class="diff-del">仅存在于内置课标</p>
                </template>
              </div>
              <div class="cb-section">
                <div class="cb-section-title">课时（{{ browserActiveUnit.lessons?.length || 0 }}）</div>
                <ol class="cb-lesson-list">
                  <li
                    v-for="(les, i) in browserActiveUnit.lessons || []"
                    :key="i"
                    :class="{
                      'diff-add-item': curriculumDiff?.units.some(
                        (d) =>
                          (d.unitId === browserActiveUnit.id || d.unitName === browserActiveUnit.name) &&
                          d.added?.includes(les),
                      ),
                      'diff-del-item': curriculumDiff?.units.some(
                        (d) =>
                          (d.unitId === browserActiveUnit.id || d.unitName === browserActiveUnit.name) &&
                          d.removed?.includes(les),
                      ),
                    }"
                  >
                    {{ les }}
                  </li>
                  <li v-if="!(browserActiveUnit.lessons || []).length" class="field-hint">无课时明细</li>
                </ol>
              </div>
              <div class="cb-section">
                <div class="cb-section-title">要点（{{ browserActiveUnit.points?.length || 0 }}）</div>
                <ul class="cb-point-list">
                  <li v-for="(pt, i) in browserActiveUnit.points || []" :key="i">{{ pt }}</li>
                  <li v-if="!(browserActiveUnit.points || []).length" class="field-hint">无要点</li>
                </ul>
              </div>
            </div>
            <div v-else class="cb-detail field-hint">请选择单元</div>
          </div>
        </div>
        <div v-else class="cb-main empty-desc" style="padding: 40px; text-align: center">
          未找到该册课标。可点「同步课标」从 dzkbw.org 拉取，或换年级/版本查看。
          <div style="margin-top: 12px">
            <el-button type="primary" :loading="updatingCurriculum" @click="onUpdateCurriculum">同步课标</el-button>
          </div>
        </div>
      </div>

      <template #footer>
        <div class="field-hint" style="flex: 1; text-align: left" v-if="curriculumDir">
          本机目录：{{ curriculumDir }}
        </div>
        <el-button @click="curriculumBrowserVisible = false">关闭</el-button>
        <el-button type="primary" :disabled="!browserPack" @click="applyBrowserToForm">
          套用到组卷参数
        </el-button>
      </template>
    </el-dialog>

    <!-- 历史记录 -->
    <el-dialog v-model="historyVisible" title="历史记录" width="760px" class="history-dialog">
      <div class="history-toolbar">
        <el-radio-group v-model="historyFilter" size="small" class="history-filter-tabs">
          <el-radio-button value="all">全部 {{ historyCounts.all }}</el-radio-button>
          <el-radio-button value="exam">试卷 {{ historyCounts.exam }}</el-radio-button>
          <el-radio-button value="lesson">教案 {{ historyCounts.lesson }}</el-radio-button>
          <el-radio-button v-if="historyCounts.other" value="other">其他 {{ historyCounts.other }}</el-radio-button>
        </el-radio-group>
        <el-input
          v-model="historyKeyword"
          clearable
          size="small"
          placeholder="搜索标题或摘要"
          style="width: 200px"
        />
      </div>
      <div v-if="!historyList.length" class="empty-desc" style="padding: 24px; text-align: center">
        暂无历史。完成组卷、教案或联动后将自动保存。
      </div>
      <div v-else-if="!filteredHistory.length" class="empty-desc" style="padding: 24px; text-align: center">
        当前分类下无记录。可切换到「试卷」或「教案」查看。
      </div>
      <div v-else class="history-list">
        <div v-for="h in filteredHistory" :key="h.id" class="history-item">
          <div class="history-main" @click="loadHistoryEntry(h.id)">
            <div class="history-title">
              <el-tag
                size="small"
                effect="dark"
                :type="historyKindTagType(h)"
                round
                style="margin-right: 6px"
              >
                {{ historyKindLabel(h) }}
              </el-tag>
              {{ h.title }}
            </div>
            <div class="history-sub">{{ h.summary }} · {{ formatHistoryTime(h.createdAt) }}</div>
          </div>
          <div class="history-actions">
            <el-button size="small" type="primary" plain @click="loadHistoryEntry(h.id)">打开</el-button>
            <el-button size="small" plain :icon="Download" @click="historyExport(h.id, false)">导出</el-button>
            <el-button
              v-if="historyKindOf(h) === 'exam'"
              size="small"
              plain
              @click="historyExport(h.id, true)"
            >
              答案
            </el-button>
            <el-button size="small" plain :icon="Printer" @click="historyPrint(h.id, false)">打印</el-button>
            <el-button size="small" text type="danger" @click="removeHistory(h.id)">删除</el-button>
          </div>
        </div>
      </div>
      <template #footer>
        <el-button v-if="historyList.length" @click="clearAllHistory">清空全部</el-button>
        <el-button type="primary" @click="historyVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <!-- 打印预览 -->
    <el-dialog
      v-model="printPreviewVisible"
      :title="printPreviewIsAnswer ? '打印预览 · 参考答案' : '打印预览'"
      width="860px"
      top="4vh"
      class="print-preview-dialog"
    >
      <iframe class="print-preview-frame" :srcdoc="printPreviewHtml" title="print-preview" />
      <template #footer>
        <el-button @click="printPreviewVisible = false">取消</el-button>
        <el-button type="primary" :icon="Printer" @click="confirmPrint">确认打印</el-button>
      </template>
    </el-dialog>

    <!-- 产出中心 -->
    <el-dialog v-model="outputCenterVisible" title="产出中心" width="520px">
      <div class="output-grid">
        <div class="output-block">
          <div class="output-label">试卷</div>
          <div class="btn-row">
            <el-button :disabled="!paper" @click="outputAction('exam')">导出试卷</el-button>
            <el-button :disabled="!paper" @click="outputAction('answers')">导出答案</el-button>
          </div>
          <div class="btn-row" style="margin-top: 8px">
            <el-button :disabled="!paper" plain @click="outputAction('print_exam')">打印试卷</el-button>
            <el-button :disabled="!paper" plain @click="outputAction('print_answers')">打印答案</el-button>
          </div>
        </div>
        <div class="output-block">
          <div class="output-label">教案</div>
          <div class="btn-row">
            <el-button :disabled="!displayLesson && !lessonPlan" @click="outputAction('lesson')">导出当前教案</el-button>
            <el-button :disabled="!lessonBundle?.plans?.length" @click="outputAction('bundle')">导出全课时</el-button>
          </div>
          <div class="btn-row" style="margin-top: 8px">
            <el-button :disabled="!displayLesson && !lessonPlan" plain @click="outputAction('print_lesson')">打印当前教案</el-button>
          </div>
        </div>
        <div class="output-block">
          <div class="output-label">批量</div>
          <el-button type="primary" :disabled="!paper && !lessonPlan && !lessonBundle" @click="outputAction('all')">
            导出全部（依次另存）
          </el-button>
        </div>
      </div>
      <template #footer>
        <el-button @click="headerPreviewVisible = true">卷头预览</el-button>
        <el-button type="primary" @click="outputCenterVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <!-- 卷头预览 -->
    <!-- 双向细目表 -->
    <el-dialog v-model="specDialogVisible" title="双向细目表" width="720px" top="6vh">
      <template v-if="specTable">
        <div class="spec-head">
          <div class="spec-title">{{ specTable.title }}</div>
          <div class="field-hint">{{ specTable.summary }}</div>
          <div class="field-hint">{{ specTable.uncoveredNote }}</div>
        </div>
        <div class="spec-block">
          <div class="output-label">知识点覆盖与分值</div>
          <el-table :data="specTable.knowledgeRows" size="small" max-height="280" stripe>
            <el-table-column prop="knowledgePoint" label="知识点" min-width="120" />
            <el-table-column prop="itemCount" label="题量" width="64" />
            <el-table-column prop="totalScore" label="分值" width="72" />
            <el-table-column prop="scoreRatio" label="占比%" width="72" />
            <el-table-column label="题号" min-width="100">
              <template #default="{ row }">{{ row.itemIds.join('、') }}</template>
            </el-table-column>
          </el-table>
        </div>
        <div class="spec-block" style="margin-top: 12px">
          <div class="output-label">大题分值分布</div>
          <el-table :data="specTable.sectionRows" size="small" max-height="200" stripe>
            <el-table-column prop="title" label="大题" min-width="160" />
            <el-table-column prop="itemCount" label="题量" width="64" />
            <el-table-column prop="score" label="分值" width="72" />
            <el-table-column prop="scoreRatio" label="占比%" width="72" />
          </el-table>
        </div>
      </template>
      <template #footer>
        <el-button @click="copySpecTableText">复制文本</el-button>
        <el-button type="primary" @click="specDialogVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <!-- 校本库 -->
    <el-dialog v-model="bankVisible" title="校本收藏库" width="720px" top="5vh">
      <div class="btn-row" style="margin-bottom: 12px">
        <el-button type="primary" plain @click="importSchoolPaper">导入本校卷 JSON</el-button>
        <el-button plain :disabled="!paper" @click="saveCurrentToBank">存入当前卷</el-button>
        <el-button plain @click="refreshBank">刷新</el-button>
      </div>
      <el-tabs v-model="bankTab">
        <el-tab-pane :label="`收藏题（${favorites.length}）`" name="items">
          <div v-if="!favorites.length" class="field-hint">暂无收藏。在试卷预览中点「收藏」即可加入。</div>
          <div v-else class="bank-list">
            <div v-for="f in favorites" :key="f.id" class="bank-item">
              <div class="bank-main">
                <div class="history-title">{{ f.stem.slice(0, 80) }}{{ f.stem.length > 80 ? '…' : '' }}</div>
                <div class="history-sub">
                  {{ f.subject || '—' }} · {{ f.grade || '?' }}年级
                  · {{ (f.knowledgePoints || []).join('、') || '无知识点' }}
                  · 答：{{ f.answer || '—' }}
                </div>
              </div>
              <el-button size="small" text type="danger" @click="removeFavorite(f.id)">删除</el-button>
            </div>
          </div>
          <el-button
            v-if="favorites.length"
            style="margin-top: 8px"
            text
            type="danger"
            @click="clearFavoritesAll"
          >
            清空收藏题
          </el-button>
        </el-tab-pane>
        <el-tab-pane :label="`校本卷（${bankPapers.length}）`" name="papers">
          <div v-if="!bankPapers.length" class="field-hint">暂无校本卷。可导入 JSON 或「当前卷存入校本」。</div>
          <div v-else class="bank-list">
            <div v-for="bp in bankPapers" :key="bp.id" class="bank-item">
              <div class="bank-main" @click="openBankPaper(bp.id)" style="cursor: pointer">
                <div class="history-title">{{ bp.title }}</div>
                <div class="history-sub">{{ bp.summary }}</div>
              </div>
              <div class="history-actions">
                <el-button size="small" type="primary" plain @click="openBankPaper(bp.id)">打开</el-button>
                <el-button size="small" text type="danger" @click="deleteBankPaperEntry(bp.id)">删除</el-button>
              </div>
            </div>
          </div>
        </el-tab-pane>
      </el-tabs>
      <template #footer>
        <el-button type="primary" @click="bankVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <!-- 作业讲评稿 -->
    <el-dialog v-model="reviewDialogVisible" title="作业讲评稿" width="680px" top="5vh">
      <div class="review-setup">
        <div class="field-hint">
          已勾选错题 {{ wrongKeys.length }} 道。可在预览中勾选「错题」，或下方填写知识点。
        </div>
        <el-form label-position="top" size="default">
          <el-form-item label="讲评知识点（逗号/顿号分隔）">
            <el-input
              v-model="reviewKpText"
              type="textarea"
              :rows="2"
              placeholder="如：表内乘法、脱式计算"
            />
          </el-form-item>
          <el-form-item label="生成方式">
            <el-checkbox v-model="reviewUseAi">使用 AI（未配置密钥时自动用本地模板）</el-checkbox>
          </el-form-item>
        </el-form>
        <div class="btn-row" style="margin-top: 8px">
          <el-button type="primary" :loading="reviewLoading" @click="onGenerateReview">
            {{ reviewLoading ? '生成中…' : '生成讲评提纲' }}
          </el-button>
          <el-button type="success" plain :loading="redrillLoading" @click="onGenerateRedrill">
            {{ redrillLoading ? '生成中…' : '一键再练卷' }}
          </el-button>
        </div>
      </div>
      <div v-if="reviewOutline" class="review-result">
        <div class="exam-title" style="font-size: 16px; margin-top: 16px">
          {{ reviewOutline.meta?.title || '讲评提纲' }}
        </div>
        <p class="field-hint">{{ reviewOutline.overview }}</p>
        <div v-if="reviewOutline.knowledgeFocus?.length" class="lesson-block">
          <h4>聚焦知识点</h4>
          <p>{{ reviewOutline.knowledgeFocus.join('、') }}</p>
        </div>
        <div v-if="reviewOutline.process?.length" class="lesson-block">
          <h4>教学过程</h4>
          <table class="lesson-process-table">
            <thead>
              <tr><th>环节</th><th>时间</th><th>内容</th><th>教师</th><th>学生</th></tr>
            </thead>
            <tbody>
              <tr v-for="(st, i) in reviewOutline.process" :key="i">
                <td>{{ st.stage }}</td>
                <td>{{ st.minutes || '—' }}</td>
                <td>{{ st.content }}</td>
                <td>{{ st.teacherActivity }}</td>
                <td>{{ st.studentActivity }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="reviewOutline.points?.length" class="lesson-block">
          <h4>知识点讲评要点</h4>
          <div v-for="(pt, i) in reviewOutline.points" :key="i" class="review-point">
            <b>{{ pt.knowledgePoint }}</b>
            <p>错因：{{ pt.errorPattern }}</p>
            <p>讲法：{{ pt.keyExplain }}</p>
            <p>板书：{{ pt.boardNote }}</p>
            <p v-if="pt.practice?.length">变式：{{ pt.practice.join('；') }}</p>
          </div>
        </div>
        <div v-if="reviewOutline.homework?.length" class="lesson-block">
          <h4>作业</h4>
          <ul><li v-for="(h, i) in reviewOutline.homework" :key="i">{{ h }}</li></ul>
        </div>
        <p v-if="reviewOutline.reflection" class="field-hint">反思：{{ reviewOutline.reflection }}</p>
      </div>
      <template #footer>
        <el-button :disabled="!reviewOutline" @click="copyReviewText">复制文本</el-button>
        <el-button type="primary" @click="reviewDialogVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <!-- 模板市集 -->
    <el-dialog v-model="templateMarketVisible" title="模板市集（本机）" width="820px" top="4vh" class="template-market-dlg">
      <div class="tmpl-toolbar">
        <el-radio-group v-model="templateFilter" size="small">
          <el-radio-button value="all">全部</el-radio-button>
          <el-radio-button value="paper">试卷结构</el-radio-button>
          <el-radio-button value="lesson">教案结构</el-radio-button>
          <el-radio-button value="mine">我的</el-radio-button>
        </el-radio-group>
        <el-input
          v-model="templateKeyword"
          size="small"
          clearable
          placeholder="搜索名称/标签"
          style="width: 180px; margin-left: 12px"
        />
        <el-button size="small" plain style="margin-left: auto" @click="importTemplateFile">导入 JSON</el-button>
        <el-button size="small" plain @click="refreshTemplates">刷新</el-button>
      </div>
      <div class="tmpl-grid">
        <div
          v-for="t in filteredTemplates"
          :key="t.id"
          class="tmpl-card"
          :class="{ active: selectedTemplateId === t.id || form.templateId === t.id }"
          role="button"
          tabindex="0"
          @click="selectTemplateCard(t)"
          @keydown.enter="selectTemplateCard(t)"
          @keydown.space.prevent="selectTemplateCard(t)"
        >
          <div class="tmpl-card-top">
            <span class="tmpl-name">{{ t.name }}</span>
            <el-tag size="small" :type="t.origin === 'user' ? 'success' : 'info'" effect="plain" round>
              {{ t.origin === 'user' ? '我的' : '内置' }}
            </el-tag>
          </div>
          <div class="tmpl-desc">{{ t.description || '—' }}</div>
          <div class="tmpl-meta">
            <el-tag size="small" effect="plain">{{ t.kind === 'lessonTemplate' ? '教案' : '试卷' }}</el-tag>
            <el-tag size="small" effect="plain">{{ subjectTag(t.subject) }}</el-tag>
            <span v-if="t.totalScore">{{ t.totalScore }}分</span>
            <span v-if="t.durationMin">{{ t.durationMin }}分钟</span>
          </div>
          <div v-if="t.sections?.length" class="tmpl-sections">
            {{ t.sections.map((s) => s.title.replace(/（.*?）/g, '')).join(' · ') }}
          </div>
          <div v-else-if="t.processStages?.length" class="tmpl-sections">
            {{ t.processStages.join(' → ') }}
          </div>
          <div class="tmpl-actions" @click.stop>
            <el-tooltip content="导出模板" placement="top">
              <el-button
                size="small"
                text
                circle
                :icon="Download"
                aria-label="导出模板"
                @click="exportTemplateFile(t.id)"
              />
            </el-tooltip>
            <el-tooltip v-if="t.origin === 'user'" content="删除模板" placement="top">
              <el-button
                size="small"
                text
                circle
                type="danger"
                :icon="Delete"
                aria-label="删除模板"
                @click="removeUserTemplate(t.id)"
              />
            </el-tooltip>
          </div>
        </div>
      </div>
      <div v-if="!filteredTemplates.length" class="field-hint">没有匹配的模板。</div>
      <template #footer>
        <span class="tmpl-footer-selection">
          {{ selectedTemplateId ? '已选择 1 个模板' : '选择一个模板后继续' }}
        </span>
        <el-button @click="templateMarketVisible = false">关闭</el-button>
        <el-button :disabled="!selectedTemplateId" :loading="applyingTemplate" @click="applyTemplateShell()">
          套用选中空壳
        </el-button>
        <el-button type="primary" :disabled="!selectedTemplateId" :loading="loading || lessonLoading" @click="generateWithSelectedTemplate">
          按选中模板生成
        </el-button>
      </template>
    </el-dialog>

    <!-- 质检报告 -->
    <el-dialog v-model="qualityDialogVisible" title="试卷质检" width="680px" top="6vh">
      <template v-if="qualityReport">
        <div class="verify-summary">
          <el-tag :type="qualityReport.score >= 80 ? 'success' : qualityReport.score >= 60 ? 'warning' : 'danger'" effect="dark">
            {{ qualityReport.score }} 分
          </el-tag>
          <el-tag type="danger" effect="plain">错误 {{ qualityReport.errorCount }}</el-tag>
          <el-tag type="warning" effect="plain">警告 {{ qualityReport.warnCount }}</el-tag>
          <el-tag type="info" effect="plain">提示 {{ qualityReport.infoCount }}</el-tag>
          <el-tag type="info" effect="plain">知识点 {{ qualityReport.knowledgeCount }}</el-tag>
          <el-tag v-if="qualityReport.mathChecked" type="info" effect="plain">
            验算 {{ qualityReport.mathMismatch }}/{{ qualityReport.mathChecked }} 不一致
          </el-tag>
        </div>
        <p class="field-hint">{{ qualityReport.summary }}</p>
        <p class="field-hint">细目：{{ qualityReport.specSummary }} · 未标知识点 {{ qualityReport.unmarkedCount }} 题</p>
        <div class="verify-list">
          <div
            v-for="(it, idx) in qualityReport.issues"
            :key="idx"
            class="verify-row"
            :class="it.level === 'error' ? 'mismatch' : it.level === 'warn' ? '' : 'ok'"
          >
            <el-tag
              size="small"
              :type="it.level === 'error' ? 'danger' : it.level === 'warn' ? 'warning' : 'info'"
              effect="dark"
            >
              {{ it.level === 'error' ? '错误' : it.level === 'warn' ? '警告' : '提示' }}
            </el-tag>
            <div class="verify-body">
              <div class="verify-stem">{{ it.message }}</div>
              <div class="verify-meta">{{ it.code }}{{ it.itemId ? ` · ${it.itemId}` : '' }}</div>
            </div>
          </div>
          <div v-if="!qualityReport.issues.length" class="field-hint">未发现问题。</div>
        </div>
      </template>
      <template #footer>
        <el-button @click="onBuildSpecTable">打开细目表</el-button>
        <el-button type="primary" :loading="qualityLoading" @click="onQualityCheck">重新质检</el-button>
        <el-button @click="qualityDialogVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="verifyDialogVisible" title="数学答案验算" width="640px">
      <template v-if="verifyReport">
        <div class="verify-summary">
          <el-tag type="info" effect="plain">共 {{ verifyReport.total }} 题</el-tag>
          <el-tag type="success" effect="plain">校验 {{ verifyReport.checked }}</el-tag>
          <el-tag type="success">一致 {{ verifyReport.ok }}</el-tag>
          <el-tag type="danger">不一致 {{ verifyReport.mismatch }}</el-tag>
          <el-tag type="info">跳过 {{ verifyReport.skipped }}</el-tag>
        </div>
        <div class="verify-list">
          <div
            v-for="(it, idx) in verifyReport.items.filter((x) => x.status === 'mismatch' || x.status === 'error' || x.status === 'ok')"
            :key="idx"
            class="verify-row"
            :class="it.status"
          >
            <el-tag size="small" :type="verifyStatusType(it.status)" effect="dark">
              {{ verifyStatusLabel(it.status) }}
            </el-tag>
            <div class="verify-body">
              <div class="verify-stem">{{ it.stem.slice(0, 120) }}{{ it.stem.length > 120 ? '…' : '' }}</div>
              <div class="verify-meta">
                答案：{{ it.answer || '—' }}
                <span v-if="it.computed"> · 计算得 {{ it.computed }}</span>
                <span v-if="it.message"> · {{ it.message }}</span>
              </div>
            </div>
          </div>
          <div v-if="!verifyReport.checked" class="field-hint">
            未找到可自动验算的口算/脱式题（题干需含明确算式，答案为数值）。
          </div>
        </div>
      </template>
      <template #footer>
        <el-button @click="verifyDialogVisible = false">关闭</el-button>
        <el-button type="primary" :loading="verifying" @click="onVerifyMath">重新验算</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="headerPreviewVisible" title="卷头预览" width="420px">
      <pre class="header-preview-box">{{ headerPreviewText }}</pre>
      <div class="field-tip">可在「接口设置 → 校名卷头」中修改，保存后导出/打印生效。</div>
      <template #footer>
        <el-button @click="settingsVisible = true">去设置</el-button>
        <el-button type="primary" @click="headerPreviewVisible = false">知道了</el-button>
      </template>
    </el-dialog>
  </div>
</template>
