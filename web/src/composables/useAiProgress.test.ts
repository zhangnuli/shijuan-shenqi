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

  it('explains a Coding Plan endpoint mismatch', () => {
    expect(formatFriendlyError('401 coding_plan_api_key_not_allowed')).toContain('Coding Plan')
    expect(formatFriendlyError('401 coding_plan_api_key_not_allowed')).toContain('Token Plan')
  })

  it('explains a Token Plan endpoint mismatch', () => {
    const message = formatFriendlyError('401 token_plan_person_api_key_not_allowed')
    expect(message).toContain('Token Plan')
    expect(message).toContain('/v2/tokenplan/personal')
    expect(message).not.toContain('请在「接口设置」将服务商切换')
  })

  it('does not turn a Token Plan transport URL into a migration hint', () => {
    const message = formatFriendlyError(
      '请求失败: error sending request for url (https://qianfan.baidubce.com/v2/tokenplan/personal/chat/completions)',
    )
    expect(message).toContain('无法连接接口')
    expect(message).not.toContain('请不要重复切换')
  })

  it('does not turn server key errors into missing-key errors', () => {
    const message = formatFriendlyError('API 错误 (401): API key is invalid')
    expect(message).toContain('密钥无效')
    expect(message).not.toContain('尚未配置')
  })

  it('keeps missing runtime keys distinguishable', () => {
    expect(formatFriendlyError('AI_CONFIG_MISSING: runtime key is empty')).toContain('尚未配置')
  })

  it('limits unknown error details', () => {
    expect(formatFriendlyError('x'.repeat(600))).toHaveLength(503)
  })
})
