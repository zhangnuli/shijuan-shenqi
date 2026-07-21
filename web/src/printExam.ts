/**
 * 桌面端友好打印：
 * Tauri 里 window.open 常被拦截，改为隐藏 iframe 调起系统打印对话框。
 */

let activeFrame: HTMLIFrameElement | null = null

function cleanupFrame() {
  if (activeFrame && activeFrame.parentNode) {
    activeFrame.parentNode.removeChild(activeFrame)
  }
  activeFrame = null
}

/**
 * 将 HTML 送入 iframe 并调起打印
 */
export function printHtml(html: string): Promise<void> {
  return new Promise((resolve, reject) => {
    try {
      cleanupFrame()

      const iframe = document.createElement('iframe')
      iframe.setAttribute('title', 'print-frame')
      // 不可见，但保持一定尺寸（部分 WebView 对 0×0 不触发打印）
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
        // 稍等再移除，避免打印对话框未弹出就被销毁
        setTimeout(() => {
          cleanupFrame()
          resolve()
        }, 800)
      }

      const triggerPrint = () => {
        try {
          win.focus()
          // 打印结束后清理
          win.onafterprint = () => finish()
          win.print()
          // 部分环境不触发 afterprint，超时兜底
          setTimeout(finish, 60_000)
        } catch (e) {
          cleanupFrame()
          reject(e)
        }
      }

      // 图片/字体就绪
      if (doc.readyState === 'complete') {
        setTimeout(triggerPrint, 150)
      } else {
        iframe.onload = () => setTimeout(triggerPrint, 150)
        // 兜底
        setTimeout(triggerPrint, 500)
      }
    } catch (e) {
      cleanupFrame()
      reject(e)
    }
  })
}
