<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import * as pdfjsLib from 'pdfjs-dist/build/pdf.mjs'
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url'
import type { PDFDocumentProxy } from 'pdfjs-dist/types/src/display/api'

pdfjsLib.GlobalWorkerOptions.workerSrc = workerUrl

const props = withDefaults(
  defineProps<{
    data: string
    loading?: boolean
  }>(),
  { loading: false },
)

const emit = defineEmits<{
  error: [message: string]
}>()

const pageCount = ref(0)
const canvasRefs = ref<HTMLCanvasElement[]>([])
const errorText = ref('')
let pdfDocument: PDFDocumentProxy | null = null
let renderToken = 0

function decodeBase64(data: string): Uint8Array {
  const binary = atob(data)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i)
  return bytes
}

function setCanvasRef(element: Element | null, index: number) {
  if (element instanceof HTMLCanvasElement) canvasRefs.value[index] = element
}

async function destroyDocument() {
  if (!pdfDocument) return
  const previous = pdfDocument
  pdfDocument = null
  try {
    await previous.destroy()
  } catch {
    // 旧文档已经被替换时无需阻断新预览。
  }
}

async function renderPdf(data: string) {
  const token = ++renderToken
  errorText.value = ''
  pageCount.value = 0
  canvasRefs.value = []
  await destroyDocument()
  if (!data) return

  try {
    const loadingTask = pdfjsLib.getDocument({ data: decodeBase64(data) })
    const documentProxy = await loadingTask.promise
    if (token !== renderToken) {
      await documentProxy.destroy()
      return
    }
    pdfDocument = documentProxy
    pageCount.value = documentProxy.numPages
    await nextTick()

    for (let pageNumber = 1; pageNumber <= documentProxy.numPages; pageNumber += 1) {
      if (token !== renderToken) return
      const canvas = canvasRefs.value[pageNumber - 1]
      if (!canvas) continue
      const page = await documentProxy.getPage(pageNumber)
      const viewport = page.getViewport({ scale: 1.3 })
      const context = canvas.getContext('2d')
      if (!context) throw new Error(`第 ${pageNumber} 页无法创建画布`)
      canvas.width = Math.ceil(viewport.width)
      canvas.height = Math.ceil(viewport.height)
      canvas.style.aspectRatio = `${viewport.width} / ${viewport.height}`
      await page.render({ canvasContext: context, viewport }).promise
    }
  } catch (error) {
    if (token !== renderToken) return
    errorText.value = error instanceof Error ? error.message : String(error)
    emit('error', errorText.value)
  }
}

watch(() => props.data, (data) => void renderPdf(data), { immediate: true })

onBeforeUnmount(() => {
  renderToken += 1
  void destroyDocument()
})
</script>

<template>
  <div class="pdf-preview-shell">
    <div v-if="loading" class="pdf-preview-state">正在生成 PDF 预览…</div>
    <div v-else-if="errorText" class="pdf-preview-state pdf-preview-error">
      PDF 预览失败：{{ errorText }}
    </div>
    <div v-else-if="!pageCount" class="pdf-preview-state">正在加载 PDF 页面…</div>
    <div v-else class="pdf-preview-pages">
      <section v-for="page in pageCount" :key="page" class="pdf-preview-page">
        <canvas :ref="(element) => setCanvasRef(element as Element | null, page - 1)" />
        <div class="pdf-preview-page-number">第 {{ page }} / {{ pageCount }} 页</div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.pdf-preview-shell {
  min-height: 68vh;
  overflow: auto;
  padding: 14px;
  background: #eef1f5;
}

.pdf-preview-state {
  display: grid;
  min-height: 300px;
  place-items: center;
  color: #64748b;
  font-size: 14px;
}

.pdf-preview-error {
  color: #b42318;
}

.pdf-preview-pages {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
}

.pdf-preview-page {
  position: relative;
  width: min(100%, 805px);
  padding-bottom: 24px;
  background: #fff;
  box-shadow: 0 2px 10px rgb(15 23 42 / 12%);
}

.pdf-preview-page canvas {
  display: block;
  width: 100%;
  height: auto;
}

.pdf-preview-page-number {
  position: absolute;
  right: 12px;
  bottom: 4px;
  color: #64748b;
  font-size: 11px;
}
</style>
