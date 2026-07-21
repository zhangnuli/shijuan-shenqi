import { describe, expect, it } from 'vitest'
import { formatFriendlyError } from './useAiProgress'

describe('formatFriendlyError', () => {
  it('turns authentication failures into an actionable message', () => {
    expect(formatFriendlyError('401 Unauthorized')).toContain('密钥无效')
  })

  it('explains an HTML response as an endpoint problem', () => {
    expect(formatFriendlyError('API 返回 HTML 网页')).toContain('接口地址配置有误')
  })

  it('limits unknown error details', () => {
    expect(formatFriendlyError('x'.repeat(600))).toHaveLength(503)
  })
})
