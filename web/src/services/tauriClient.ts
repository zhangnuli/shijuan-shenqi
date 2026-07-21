import { invoke } from '@tauri-apps/api/core'

export async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args)
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error ?? '未知错误')
    throw new Error(message, { cause: error })
  }
}
