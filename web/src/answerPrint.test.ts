import { describe, expect, it } from 'vitest'
import { buildAnswerPrintHtml } from './answerPrint'

describe('buildAnswerPrintHtml', () => {
  it('prints only answers with compact layouts based on question type', () => {
    const html = buildAnswerPrintHtml({
      meta: {
        edition: '人教版',
        subject: '数学',
        grade: 3,
        semester: '上册',
        examType: '单元测试',
        title: '第一单元测试卷',
        totalScore: 100,
        durationMin: 60,
      },
      sections: [
        {
          type: 'choice',
          title: '一、选择题',
          score: 10,
          items: [
            { id: '1', stem: '不应显示的题干', options: ['A', 'B'], answer: 'B', analysis: '不应显示的解析' },
            { id: '2', stem: '第二题', answer: 'C' },
          ],
        },
        {
          type: 'fill',
          title: '二、填空题',
          score: 10,
          items: [
            { id: '1', stem: '填空题干', answer: '24' },
            { id: '2', stem: '多空题干', answer: '第一空：24\n第二空：36' },
          ],
        },
      ],
    })

    expect(html).toContain('第一单元测试卷 · 参考答案')
    expect(html).toContain('class="answer-grid cols-6"')
    expect(html).toContain('class="answer-grid cols-4"')
    expect(html).toContain('class="answer-item wide"')
    expect(html).toContain('第一空：24<br />第二空：36')
    expect(html).not.toContain('不应显示的题干')
    expect(html).not.toContain('不应显示的解析')
    expect(html).not.toContain('class="opts"')
    expect(html).toContain('font-size: 11pt')
  })
})
