import { computed, onUnmounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import type { ExamPaper } from '../types'

export const aiSteps = [
  { title: '加载课标', desc: '确定年级、册次与范围' },
  { title: '请求模型', desc: '连接已配置的 AI 接口' },
  { title: '生成内容', desc: '撰写教案或试题' },
  { title: '整理输出', desc: '汇总结构，准备预览' },
]

export const aiTips = [
  '预计 20-90 秒，请保持窗口打开。',
  '试题依据当前教材版本与单元知识点生成。',
  '完成后可在下方预览，并导出 Word。',
  '若失败，请核对接口地址（通常以 /v1 结尾）与密钥。',
  '网络较慢时等待时间会相应增加。',
]

export function formatFriendlyError(err: unknown): string {
  const raw = String(err ?? '未知错误')
  const message = raw
    .replace(/^Error:\s*/i, '')
    .replace(/^AI 组卷失败:\s*/i, '')
    .trim()

  if (/HTML|网页|doctype|API Base|网站首页|\/v1/i.test(message)) {
    return (
      '接口地址配置有误，服务器返回了网页而非数据。\n\n' +
      '请在「接口设置」中检查：\n' +
      '1. 接口地址应为 API 根路径（多为 .../v1），勿填写网站首页；\n' +
      '2. 密钥与模型名称是否正确。\n\n' +
      `技术信息：\n${message.slice(0, 400)}`
    )
  }
  if (/API Key|api key|未填写|请先在设置/i.test(message)) {
    return '尚未配置 API 密钥，请先在「接口设置」中填写。'
  }
  if (/timeout|超时|TIMED_OUT|deadline/i.test(message)) {
    return '请求超时。请检查网络后重试，或更换响应更快的模型。'
  }
  if (/401|Unauthorized|invalid.*key|鉴权|认证/i.test(message)) {
    return '密钥无效或已过期，请核对 API 密钥及账户额度。'
  }
  if (/404|Not Found/i.test(message)) {
    return '接口地址不存在（404），请核对路径是否完整（常见需包含 /v1）。'
  }
  if (/network|连接|Failed to fetch|dns|refused/i.test(message)) {
    return '无法连接接口，请检查网络或接口地址是否可访问。'
  }
  return message.length > 500 ? `${message.slice(0, 500)}...` : message
}

export function useAiProgress() {
  const panelVisible = ref(false)
  const status = ref<'running' | 'success' | 'error'>('running')
  const stepIndex = ref(0)
  const tipIndex = ref(0)
  const elapsed = ref(0)
  const errorText = ref('')
  const successSummary = ref('')
  let elapsedTimer: ReturnType<typeof setInterval> | null = null
  let tipTimer: ReturnType<typeof setInterval> | null = null
  let stepTimer: ReturnType<typeof setInterval> | null = null

  const progress = computed(() => {
    if (status.value === 'success') return 100
    if (status.value === 'error') return Math.min(90, 15 + stepIndex.value * 20)
    const seconds = elapsed.value
    if (seconds < 8) return 8 + seconds * 4
    if (seconds < 30) return 40 + (seconds - 8) * 1.2
    if (seconds < 60) return 66 + (seconds - 30) * 0.4
    return Math.min(92, 78 + (seconds - 60) * 0.15)
  })

  function clearTimers() {
    if (elapsedTimer) clearInterval(elapsedTimer)
    if (tipTimer) clearInterval(tipTimer)
    if (stepTimer) clearInterval(stepTimer)
    elapsedTimer = null
    tipTimer = null
    stepTimer = null
  }

  function start() {
    panelVisible.value = true
    status.value = 'running'
    stepIndex.value = 0
    tipIndex.value = 0
    elapsed.value = 0
    errorText.value = ''
    successSummary.value = ''
    clearTimers()
    elapsedTimer = setInterval(() => (elapsed.value += 1), 1000)
    tipTimer = setInterval(() => (tipIndex.value = (tipIndex.value + 1) % aiTips.length), 4000)
    stepTimer = setInterval(() => {
      if (stepIndex.value < aiSteps.length - 1) stepIndex.value += 1
    }, 6500)
  }

  function succeed(paper: ExamPaper) {
    clearTimers()
    status.value = 'success'
    stepIndex.value = aiSteps.length - 1
    const itemCount = paper.sections?.reduce((sum, section) => sum + (section.items?.length || 0), 0) || 0
    successSummary.value = `已生成 ${paper.sections?.length || 0} 道大题、${itemCount} 道小题，用时 ${elapsed.value} 秒。`
    ElMessage.success({ message: '组卷完成', duration: 2500 })
    setTimeout(() => {
      if (status.value === 'success') panelVisible.value = false
    }, 1800)
  }

  function fail(err: unknown) {
    clearTimers()
    status.value = 'error'
    errorText.value = formatFriendlyError(err)
  }

  onUnmounted(clearTimers)

  return {
    panelVisible,
    status,
    stepIndex,
    tipIndex,
    elapsed,
    errorText,
    successSummary,
    progress,
    start,
    succeed,
    fail,
  }
}
