/**
 * 应用内自动更新（Tauri plugin-updater）
 * 仅在桌面壳内可用；浏览器预览会静默跳过。
 */
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

export type UpdateCheckResult =
  | { available: false }
  | {
      available: true
      version: string
      body?: string
      date?: string
      /** 下载并安装，完成后可 relaunch */
      install: (onProgress?: (pct: number) => void) => Promise<void>
    }

/**
 * 检查更新。无更新返回 available:false。
 * 开发模式或非 Tauri 环境返回 available:false。
 */
export async function checkForAppUpdate(): Promise<UpdateCheckResult> {
  if (!isTauri()) {
    return { available: false }
  }
  try {
    const update = await check()
    if (!update) {
      return { available: false }
    }
    return {
      available: true,
      version: update.version,
      body: update.body,
      date: update.date,
      install: async (onProgress) => {
        let downloaded = 0
        let contentLength = 0
        await update.downloadAndInstall((event) => {
          switch (event.event) {
            case 'Started':
              contentLength = event.data.contentLength ?? 0
              onProgress?.(0)
              break
            case 'Progress':
              downloaded += event.data.chunkLength
              if (contentLength > 0) {
                onProgress?.(Math.min(99, Math.round((downloaded / contentLength) * 100)))
              }
              break
            case 'Finished':
              onProgress?.(100)
              break
          }
        })
        await relaunch()
      },
    }
  } catch (e) {
    const msg = String(e)
    // 端点未配置 / 网络失败时抛给调用方
    throw new Error(msg.includes('error sending request') || msg.includes('error trying to connect')
      ? `无法连接更新服务器。请检查网络，或确认 tauri.conf.json 中 updater.endpoints 已改为有效地址。\n原始错误：${msg}`
      : msg)
  }
}
