import { describe, expect, it } from 'vitest'
import { buildAnswerSheetPrintHtml } from './answerSheetPrint'

describe('buildAnswerSheetPrintHtml', () => {
  it('renders large, answer-only rows without question content', () => {
    const html = buildAnswerSheetPrintHtml(
      {
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
              { id: '1', stem: '下列哪个答案正确？', options: ['A', 'B'], answer: 'B', analysis: '解析不应显示' },
              { id: '2', stem: '第二题', answer: '24\n36' },
            ],
          },
          {
            type: 'judge',
            title: '二、判断题',
            score: 5,
            items: [
              { id: '1', stem: '第三题', answer: '正确' },
              { id: '2', stem: '第四题', answer: '错误' },
            ],
          },
        ],
      },
      { schoolName: '实验小学' },
    )

    expect(html).toContain('第一单元测试卷 · 大字答案')
    expect(html).toContain('实验小学')
    expect(html).toContain('B')
    expect(html).toContain('24<br />36')
    expect(html).not.toContain('下列哪个答案正确')
    expect(html).not.toContain('解析不应显示')
    expect(html).not.toContain('class="opts"')
    expect(html).toContain('font-size: 16pt')
    expect(html).toContain('class="answer-rows cols-4"')
    expect(html).toContain('class="answer-row wide"')
  })
})
