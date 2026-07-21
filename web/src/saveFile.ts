/**
 * 保存 Word 文件：Tauri 桌面用「另存为」+ 写盘；浏览器环境降级 a 标签下载。
 */
import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function browserDownload(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.style.display = 'none'
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
}

/**
 * @returns 实际保存路径；用户取消返回 null
 */
export async function saveDocxFile(blob: Blob, defaultName: string): Promise<string | null> {
  const safeName = defaultName.replace(/[\\/:*?"<>|]/g, '_').replace(/\.docx$/i, '') + '.docx'

  if (!isTauri()) {
    browserDownload(blob, safeName)
    return safeName
  }

  const path = await save({
    title: '导出 Word 试卷',
    defaultPath: safeName,
    filters: [{ name: 'Word 文档', extensions: ['docx'] }],
  })

  if (!path) {
    return null // 用户取消
  }

  const bytes = new Uint8Array(await blob.arrayBuffer())
  await writeFile(path, bytes)
  return path
}
