import { describe, expect, it } from 'vitest'
import { formatFriendlyError } from './useAiProgress'

describe('formatFriendlyError', () => {
  it('turns authentication failures into an actionable message', () => {
    expect(formatFriendlyError('401 Unauthorized')).toContain('密钥无效')
  })

  it('explains an HTML response as an endpoint problem', () => {
    expect(formatFriendlyError('API 返回 HTML 网页')).toContain('接口地址配置有误')
  })

  it('does not mislabel transport failures as HTML responses', () => {
    const message = formatFriendlyError(
      '请求失败: error sending request for url (https://qianfan.baidubce.com/v2/chat/completions)',
    )
    expect(message).toContain('无法连接接口')
    expect(message).not.toContain('服务器返回了网页')
  })

  it('distinguishes encrypted key storage failures from missing keys', () => {
    expect(formatFriendlyError('API Key 解密失败: 当前用户无权读取')).toContain('无法读取')
  })

  it('limits unknown error details', () => {
    expect(formatFriendlyError('x'.repeat(600))).toHaveLength(503)
  })
})
