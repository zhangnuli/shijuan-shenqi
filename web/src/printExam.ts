/**
 * 桌面端友好打印：
 * - Tauri：用 Edge/Chrome 无头导出 PDF（--no-pdf-header-footer），避免页脚出现 tauri.localhost
 * - 浏览器预览：回退为隐藏 iframe + window.print()
 */

import { invoke } from '@tauri-apps/api/core'
import { openPath } from '@tauri-apps/plugin-opener'

let activeFrame: HTMLIFrameElement | null = null

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function cleanupFrame() {
  if (activeFrame && activeFrame.parentNode) {
    activeFrame.parentNode.removeChild(activeFrame)
  }
  activeFrame = null
}

/** iframe 回退（非 Tauri 或 PDF 导出失败时） */
function printHtmlViaIframe(html: string): Promise<void> {
  return new Promise((resolve, reject) => {
    try {
      cleanupFrame()

      const iframe = document.createElement('iframe')
      iframe.setAttribute('title', 'print-frame')
      iframe.style.cssText = [
        'position:fixed',
        'right:0',
        'bottom:0',
        'width:1px',
        'height:1px',
        'opacity:0',
        'pointer-events:none',
        'border:0',
        'z-index:-1',
      ].join(';')

      document.body.appendChild(iframe)
      activeFrame = iframe

      const win = iframe.contentWindow
      const doc = iframe.contentDocument || win?.document
      if (!win || !doc) {
        cleanupFrame()
        reject(new Error('无法创建打印文档'))
        return
      }

      doc.open()
      doc.write(html)
      doc.close()

      let done = false
      const finish = () => {
        if (done) return
        done = true
        setTimeout(() => {
          cleanupFrame()
          resolve()
        }, 800)
      }

      const triggerPrint = () => {
        try {
          win.focus()
          win.onafterprint = () => finish()
          win.print()
          setTimeout(finish, 60_000)
        } catch (e) {
          cleanupFrame()
          reject(e)
        }
      }

      if (doc.readyState === 'complete') {
        setTimeout(triggerPrint, 150)
      } else {
        iframe.onload = () => setTimeout(triggerPrint, 150)
        setTimeout(triggerPrint, 500)
      }
    } catch (e) {
      cleanupFrame()
      reject(e)
    }
  })
}

export type PrintHtmlResult = {
  /** pdf：已打开无系统页眉的 PDF；iframe：回退系统打印（可能带 URL 页脚） */
  mode: 'pdf' | 'iframe'
  pdfPath?: string
}

/**
 * 打印 HTML。Tauri 下优先生成 PDF 并用系统默认程序打开，再由用户打印。
 */
export async function printHtml(html: string): Promise<PrintHtmlResult> {
  if (isTauri()) {
    try {
      const pdfPath = await invoke<string>('print_html_document', { html })
      await openPath(pdfPath)
      return { mode: 'pdf', pdfPath }
    } catch (e) {
      console.warn('[print] PDF 导出失败，回退 iframe 打印', e)
      await printHtmlViaIframe(html)
      return { mode: 'iframe' }
    }
  }
  await printHtmlViaIframe(html)
  return { mode: 'iframe' }
}
