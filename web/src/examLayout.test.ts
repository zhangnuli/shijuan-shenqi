import { describe, expect, it } from 'vitest'
import { isCompactCalculationItem } from './examLayout'

describe('isCompactCalculationItem', () => {
  it('treats one-point arithmetic as compact', () => {
    expect(
      isCompactCalculationItem(
        { stem: '3/4 + 1/4 =', score: 1 },
        'calc',
        '四、计算题（口算每题1分，脱式计算每题3分）',
      ),
    ).toBe(true)
  })

  it('keeps higher-scored expressions as worked calculations', () => {
    expect(
      isCompactCalculationItem(
        { stem: '1.2×(3.5+1.5)=', score: 3 },
        'calc',
        '四、计算题',
      ),
    ).toBe(false)
  })

  it('keeps equations and worded prompts out of the compact grid', () => {
    expect(isCompactCalculationItem({ stem: '3x+5=17', score: 5 }, 'calc', '解方程')).toBe(false)
    expect(isCompactCalculationItem({ stem: '用竖式计算：125÷5', score: 3 }, 'calc', '计算题')).toBe(false)
  })

  it('does not compact arithmetic outside a calculation section', () => {
    expect(isCompactCalculationItem({ stem: '3+4=', score: 1 }, 'fill', '填空题')).toBe(false)
  })
})
