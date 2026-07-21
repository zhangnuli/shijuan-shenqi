const { Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell,
        Header, Footer, AlignmentType, BorderStyle, WidthType, ShadingType,
        VerticalAlign, PageNumber, PageBreak } = require('docx');
const fs = require('fs');

// A4 page: 11906 x 16838 DXA
const PAGE_W = 11906;
const PAGE_H = 16838;
const MARGIN = 720; // 0.5 inch
const CONTENT_W = PAGE_W - MARGIN * 2; // 10466

const noBorder = { style: BorderStyle.NONE, size: 0, color: "FFFFFF" };
const noBorders = { top: noBorder, bottom: noBorder, left: noBorder, right: noBorder };
const thinBorder = { style: BorderStyle.SINGLE, size: 8, color: "333333" };
const thinBorders = { top: thinBorder, bottom: thinBorder, left: thinBorder, right: thinBorder };
const bottomLine = {
  top: noBorder, left: noBorder, right: noBorder,
  bottom: { style: BorderStyle.SINGLE, size: 12, color: "1a1a1a" }
};

function p(text, opts = {}) {
  const { bold = false, size = 22, center = false, indent = 0, spacingAfter = 80, spacingBefore = 0, color = "000000", font = "宋体" } = opts;
  return new Paragraph({
    alignment: center ? AlignmentType.CENTER : AlignmentType.LEFT,
    spacing: { after: spacingAfter, before: spacingBefore, line: 360 },
    indent: indent ? { left: indent } : undefined,
    children: [new TextRun({ text, bold, size, font, color })]
  });
}

function runs(parts, paraOpts = {}) {
  // parts: [{text, bold?, size?, font?, color?}]
  return new Paragraph({
    alignment: paraOpts.center ? AlignmentType.CENTER : AlignmentType.LEFT,
    spacing: { after: paraOpts.spacingAfter ?? 80, before: paraOpts.spacingBefore ?? 0, line: paraOpts.line ?? 360 },
    indent: paraOpts.indent ? { left: paraOpts.indent } : undefined,
    children: parts.map(part => new TextRun({
      text: part.text,
      bold: part.bold || false,
      size: part.size || 22,
      font: part.font || "宋体",
      color: part.color || "000000"
    }))
  });
}

function sectionTitle(text) {
  return new Paragraph({
    spacing: { before: 200, after: 120, line: 360 },
    border: { bottom: { style: BorderStyle.SINGLE, size: 6, color: "2E75B6", space: 4 } },
    children: [new TextRun({ text, bold: true, size: 24, font: "黑体", color: "1F4E79" })]
  });
}

function blankLine(width = 4) {
  return "　".repeat(width);
}

function cell(text, width, opts = {}) {
  return new TableCell({
    borders: opts.borders || thinBorders,
    width: { size: width, type: WidthType.DXA },
    shading: opts.fill ? { fill: opts.fill, type: ShadingType.CLEAR } : undefined,
    margins: { top: 60, bottom: 60, left: 80, right: 80 },
    verticalAlign: VerticalAlign.CENTER,
    children: [new Paragraph({
      alignment: opts.center ? AlignmentType.CENTER : AlignmentType.LEFT,
      children: [new TextRun({
        text,
        bold: opts.bold || false,
        size: opts.size || 20,
        font: opts.font || "宋体"
      })]
    })]
  });
}

function infoTable() {
  const w = Math.floor(CONTENT_W / 4);
  return new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: [w, w, w, CONTENT_W - w * 3],
    rows: [
      new TableRow({
        children: [
          cell("班级：________", w, { borders: noBorders, size: 20 }),
          cell("姓名：________", w, { borders: noBorders, size: 20 }),
          cell("学号：________", w, { borders: noBorders, size: 20 }),
          cell("得分：________", CONTENT_W - w * 3, { borders: noBorders, size: 20 })
        ]
      })
    ]
  });
}

function scoreTable() {
  const headers = ["题号", "一", "二", "三", "四", "五", "六", "总分"];
  const labels = ["得分", "", "", "", "", "", "", ""];
  const colW = Math.floor(CONTENT_W / headers.length);
  const widths = headers.map((_, i) => i === headers.length - 1 ? CONTENT_W - colW * (headers.length - 1) : colW);

  return new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({
        children: headers.map((h, i) => cell(h, widths[i], { center: true, bold: true, fill: "D6EAF8", size: 18 }))
      }),
      new TableRow({
        children: labels.map((h, i) => cell(h || "　", widths[i], { center: true, size: 18 }))
      })
    ]
  });
}

function calcBox(expr, w) {
  return new TableCell({
    borders: thinBorders,
    width: { size: w, type: WidthType.DXA },
    margins: { top: 100, bottom: 100, left: 100, right: 100 },
    children: [
      new Paragraph({
        alignment: AlignmentType.CENTER,
        spacing: { after: 60 },
        children: [new TextRun({ text: expr, size: 22, font: "宋体" })]
      }),
      new Paragraph({
        alignment: AlignmentType.CENTER,
        spacing: { after: 40 },
        children: [new TextRun({ text: "＝　　　", size: 22, font: "宋体" })]
      })
    ]
  });
}

const children = [];

// ========== 封面/标题 ==========
children.push(p("小学数学模拟考试卷", {
  bold: true, size: 36, center: true, font: "黑体", spacingAfter: 60, spacingBefore: 100
}));
children.push(p("北师大版 · 二年级（下册）期末", {
  bold: true, size: 28, center: true, font: "黑体", color: "1F4E79", spacingAfter: 60
}));
children.push(p("（真题模拟 · 满分100分 · 建议用时60分钟）", {
  size: 18, center: true, color: "666666", spacingAfter: 160
}));

children.push(infoTable());
children.push(new Paragraph({ spacing: { after: 120 }, children: [] }));
children.push(scoreTable());
children.push(new Paragraph({ spacing: { after: 160 }, children: [] }));

children.push(runs([
  { text: "温馨提示：", bold: true, size: 18, color: "C0392B" },
  { text: "请仔细审题，书写工整；计算题要验算；应用题要写清数量关系。", size: 18, color: "555555" }
], { spacingAfter: 200 }));

// ========== 一、填空题 ==========
children.push(sectionTitle("一、填空题（每空2分，共20分）"));

const fillIns = [
  "1. 一个三位数，最高位是百位，表示（　　）个百；最小的三位数是（　　）。",
  "2. 在○里填上“＞”“＜”或“＝”：  456 ○ 465　　  703 ○ 730　　  899 ○ 898＋1",
  "3. 3米＝（　　）分米＝（　　）厘米；  5千米＝（　　）米。",
  "4. 时针走一大格是（　　）小时，分针走一大格是（　　）分钟。",
  "5. 48÷6＝（　　），表示把48平均分成6份，每份是（　　）。",
  "6. 有余数的除法：17÷5＝（　　）……（　　），验算：商×除数＋余数＝被除数。",
  "7. 计算 100－36÷4 时，应先算（　　），结果是（　　）。",
  "8. 从东向西走，再向右转，现在面向（　　）方。",
  "9. 一个长方形有（　　）条对称轴；正方形有（　　）条对称轴。",
  "10. 图书角有故事书28本，科技书的本数是故事书的一半，科技书有（　　）本。"
];
fillIns.forEach(t => children.push(p(t, { size: 21, spacingAfter: 140 })));

// ========== 二、判断题 ==========
children.push(sectionTitle("二、判断题（对的打“√”，错的打“×”，每题2分，共10分）"));

const judges = [
  "1. 最大的两位数加1等于最小的三位数。（　　）",
  "2. 余数可以等于除数。（　　）",
  "3. 1千米比1000米长。（　　）",
  "4. 先乘除后加减，有括号的要先算括号里面的。（　　）",
  "5. 长方形对折后两边完全重合，所以长方形是轴对称图形。（　　）"
];
judges.forEach(t => children.push(p(t, { size: 21, spacingAfter: 120 })));

// ========== 三、选择题 ==========
children.push(sectionTitle("三、选择题（把正确答案的序号填在括号里，每题2分，共10分）"));

const choices = [
  { q: "1. 下面各数中，最大的是（　　）。", opts: "A. 908　　B. 980　　C. 890　　D. 809" },
  { q: "2. 25÷4 的商是（　　）。", opts: "A. 5……5　　B. 6……1　　C. 6……0　　D. 5……1" },
  { q: "3. 下面哪个长度最长？（　　）", opts: "A. 3米　　B. 280厘米　　C. 25分米　　D. 2千米" },
  { q: "4. 算式 36＋18÷6 的正确结果是（　　）。", opts: "A. 9　　B. 39　　C. 54　　D. 8" },
  { q: "5. 小明从家向东走200米到学校，放学回家应向（　　）走。", opts: "A. 东　　B. 南　　C. 西　　D. 北" }
];
choices.forEach(c => {
  children.push(p(c.q, { size: 21, spacingAfter: 40 }));
  children.push(p(c.opts, { size: 20, indent: 200, spacingAfter: 120 }));
});

// ========== 四、计算题 ==========
children.push(sectionTitle("四、计算题（共30分）"));

children.push(runs([
  { text: "1. 直接写出得数（每题1分，共8分）", bold: true, size: 21 }
], { spacingAfter: 100 }));

const directCalcs = [
  ["6×7＝", "9×8＝", "56÷7＝", "42÷6＝"],
  ["0×9＝", "1×5＝", "24÷3＝", "63÷9＝"]
];
directCalcs.forEach(row => {
  const w = Math.floor(CONTENT_W / 4);
  const widths = [w, w, w, CONTENT_W - w * 3];
  children.push(new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({
        children: row.map((expr, i) => new TableCell({
          borders: noBorders,
          width: { size: widths[i], type: WidthType.DXA },
          margins: { top: 40, bottom: 40, left: 40, right: 40 },
          children: [new Paragraph({
            children: [new TextRun({ text: expr + "　　", size: 22, font: "宋体" })]
          })]
        }))
      })
    ]
  }));
  children.push(new Paragraph({ spacing: { after: 80 }, children: [] }));
});

children.push(runs([
  { text: "2. 竖式计算（每题3分，共12分）注意验算。", bold: true, size: 21 }
], { spacingAfter: 100, spacingBefore: 80 }));

const vertical = ["368＋254＝", "705－279＝", "42×3＝", "56÷7＝"];
{
  const w = Math.floor(CONTENT_W / 4);
  const widths = [w, w, w, CONTENT_W - w * 3];
  children.push(new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({
        children: vertical.map((expr, i) => new TableCell({
          borders: thinBorders,
          width: { size: widths[i], type: WidthType.DXA },
          margins: { top: 120, bottom: 400, left: 80, right: 80 },
          children: [
            new Paragraph({
              alignment: AlignmentType.CENTER,
              children: [new TextRun({ text: expr, size: 20, font: "宋体" })]
            }),
            new Paragraph({ spacing: { after: 200 }, children: [] }),
            new Paragraph({ spacing: { after: 200 }, children: [] }),
            new Paragraph({ spacing: { after: 200 }, children: [] })
          ]
        }))
      })
    ]
  }));
}

children.push(runs([
  { text: "3. 脱式计算（每题2.5分，共10分）要写出计算步骤。", bold: true, size: 21 }
], { spacingAfter: 100, spacingBefore: 160 }));

const mixed = [
  ["28＋36÷6", "（45－9）÷6", "7×8－15", "100－6×9"]
];
mixed.forEach(row => {
  const w = Math.floor(CONTENT_W / 4);
  const widths = [w, w, w, CONTENT_W - w * 3];
  children.push(new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({
        children: row.map((expr, i) => new TableCell({
          borders: thinBorders,
          width: { size: widths[i], type: WidthType.DXA },
          margins: { top: 100, bottom: 300, left: 80, right: 80 },
          children: [
            new Paragraph({
              alignment: AlignmentType.CENTER,
              spacing: { after: 80 },
              children: [new TextRun({ text: expr, size: 20, font: "宋体" })]
            }),
            new Paragraph({
              alignment: AlignmentType.CENTER,
              children: [new TextRun({ text: "＝", size: 20, font: "宋体" })]
            }),
            new Paragraph({ spacing: { after: 120 }, children: [] }),
            new Paragraph({
              alignment: AlignmentType.CENTER,
              children: [new TextRun({ text: "＝", size: 20, font: "宋体" })]
            }),
            new Paragraph({ spacing: { after: 80 }, children: [] })
          ]
        }))
      })
    ]
  }));
});

// ========== 五、操作题 ==========
children.push(sectionTitle("五、操作与画图题（共10分）"));

children.push(p("1.（4分）画出下面图形的对称轴。（能画几条画几条）", { size: 21, spacingAfter: 80 }));

// Shape drawing boxes
{
  const labels = ["（1）长方形", "（2）正方形", "（3）等腰三角形", "（4）圆"];
  const w = Math.floor(CONTENT_W / 4);
  const widths = [w, w, w, CONTENT_W - w * 3];
  children.push(new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({
        children: labels.map((lab, i) => new TableCell({
          borders: thinBorders,
          width: { size: widths[i], type: WidthType.DXA },
          margins: { top: 80, bottom: 80, left: 60, right: 60 },
          children: [
            new Paragraph({
              alignment: AlignmentType.CENTER,
              spacing: { after: 60 },
              children: [new TextRun({ text: lab, size: 18, font: "宋体" })]
            }),
            new Paragraph({
              alignment: AlignmentType.CENTER,
              spacing: { after: 40 },
              children: [new TextRun({ text: "┌───┐", size: 16, font: "Consolas" })]
            }),
            new Paragraph({
              alignment: AlignmentType.CENTER,
              spacing: { after: 40 },
              children: [new TextRun({ text: "│　　　│", size: 16, font: "Consolas" })]
            }),
            new Paragraph({
              alignment: AlignmentType.CENTER,
              spacing: { after: 60 },
              children: [new TextRun({ text: "└───┘", size: 16, font: "Consolas" })]
            }),
            new Paragraph({
              alignment: AlignmentType.CENTER,
              children: [new TextRun({ text: "（请在框内画）", size: 14, font: "宋体", color: "888888" })]
            })
          ]
        }))
      })
    ]
  }));
}

children.push(p("2.（3分）在方格纸上画出从学校到公园的路线：从学校向东走3格，再向北走2格，就到公园。标出学校（S）和公园（P）。", {
  size: 21, spacingAfter: 80, spacingBefore: 160
}));

// Simple grid representation
{
  children.push(new Table({
    width: { size: 3600, type: WidthType.DXA },
    columnWidths: [720, 720, 720, 720, 720],
    rows: [0, 1, 2, 3, 4].map(r => new TableRow({
      height: { value: 400, rule: "atLeast" },
      children: [0, 1, 2, 3, 4].map(c => new TableCell({
        borders: thinBorders,
        width: { size: 720, type: WidthType.DXA },
        margins: { top: 20, bottom: 20, left: 20, right: 20 },
        children: [new Paragraph({
          alignment: AlignmentType.CENTER,
          children: [new TextRun({
            text: (r === 4 && c === 0) ? "S" : "　",
            size: 16,
            font: "宋体",
            bold: r === 4 && c === 0
          })]
        })]
      }))
    }))
  }));
}

children.push(p("3.（3分）用尺子量一量你的数学书大约长（　　）厘米，宽大约（　　）厘米。", {
  size: 21, spacingAfter: 80, spacingBefore: 160
}));

// ========== 六、解决问题 ==========
children.push(sectionTitle("六、解决问题（共20分）"));

const problems = [
  {
    title: "1.（4分）",
    body: "水果店运来苹果 240 千克，运来的梨比苹果少 85 千克。运来的梨有多少千克？"
  },
  {
    title: "2.（4分）",
    body: "妈妈买了 6 袋饼干，每袋 8 块。全家人一共吃了 30 块，还剩多少块？"
  },
  {
    title: "3.（4分）",
    body: "学校买来 48 本课外书，平均分给 6 个小组，每个小组分到多少本？如果每个小组再分到 2 本，一共需要多少本？"
  },
  {
    title: "4.（4分）",
    body: "一条小路长 300 米。小华从一端走到另一端用了 5 分钟。他平均每分钟走多少米？"
  },
  {
    title: "5.（4分）★稍难",
    body: "图书室原有故事书 180 本。又新买来 3 包，每包 40 本。现在故事书有多少本？如果平均放在 6 个书架上，每个书架放多少本？"
  }
];

problems.forEach((pr, idx) => {
  children.push(runs([
    { text: pr.title, bold: true, size: 21 },
    { text: pr.body, size: 21 }
  ], { spacingAfter: 60, spacingBefore: idx === 0 ? 0 : 120 }));
  // answer space
  children.push(p("解：", { size: 20, spacingAfter: 40 }));
  children.push(p("　", { size: 20, spacingAfter: 40 }));
  children.push(p("　", { size: 20, spacingAfter: 40 }));
  children.push(p("答：______________________________", { size: 20, spacingAfter: 80 }));
});

// ========== 参考答案（分页）==========
children.push(new Paragraph({ children: [new PageBreak()] }));

children.push(p("参考答案与评分说明", {
  bold: true, size: 32, center: true, font: "黑体", spacingAfter: 60, spacingBefore: 100
}));
children.push(p("小学数学 · 北师大版 · 二年级（下册）期末模拟卷", {
  size: 20, center: true, color: "666666", spacingAfter: 200
}));

children.push(sectionTitle("一、填空题（每空2分，共20分）"));
const ans1 = [
  "1. 1； 100",
  "2. ＜； ＜； ＝",
  "3. 30； 300； 5000",
  "4. 1； 5",
  "5. 8； 8",
  "6. 3； 2",
  "7. 除法（或 36÷4）； 91",
  "8. 北",
  "9. 2； 4",
  "10. 14"
];
ans1.forEach(t => children.push(p(t, { size: 20, spacingAfter: 60 })));

children.push(sectionTitle("二、判断题（每题2分，共10分）"));
children.push(p("1. √　　2. ×（余数必须小于除数）　　3. ×（1千米＝1000米）　　4. √　　5. √", {
  size: 20, spacingAfter: 100
}));

children.push(sectionTitle("三、选择题（每题2分，共10分）"));
children.push(p("1. B　　2. B　　3. D　　4. B　　5. C", {
  size: 20, spacingAfter: 100
}));

children.push(sectionTitle("四、计算题（共30分）"));
children.push(runs([{ text: "1. 直接写得数：", bold: true, size: 20 }], { spacingAfter: 40 }));
children.push(p("42， 72， 8， 7；　　0， 5， 8， 7", { size: 20, spacingAfter: 80 }));

children.push(runs([{ text: "2. 竖式计算：", bold: true, size: 20 }], { spacingAfter: 40 }));
children.push(p("368＋254＝622；　705－279＝426；　42×3＝126；　56÷7＝8", { size: 20, spacingAfter: 80 }));

children.push(runs([{ text: "3. 脱式计算：", bold: true, size: 20 }], { spacingAfter: 40 }));
children.push(p("28＋36÷6＝28＋6＝34", { size: 20, spacingAfter: 40 }));
children.push(p("（45－9）÷6＝36÷6＝6", { size: 20, spacingAfter: 40 }));
children.push(p("7×8－15＝56－15＝41", { size: 20, spacingAfter: 40 }));
children.push(p("100－6×9＝100－54＝46", { size: 20, spacingAfter: 100 }));

children.push(sectionTitle("五、操作与画图题（共10分）"));
children.push(p("1. 长方形画2条对称轴（横、竖中线）；正方形画4条（横、竖及两条对角线）；等腰三角形画1条（底边中线所在直线）；圆可画无数条（过圆心的任意直线），画对2条及以上给满分。", {
  size: 20, spacingAfter: 80
}));
children.push(p("2. 从S向右（东）3格，再向上（北）2格到达P，路线与标注正确即可。", {
  size: 20, spacingAfter: 80
}));
children.push(p("3. 按实际测量填写，合理即可（常见约长 26cm、宽 18cm 左右）。", {
  size: 20, spacingAfter: 100
}));

children.push(sectionTitle("六、解决问题（共20分）"));
children.push(p("1. 240－85＝155（千克）　答：运来的梨有155千克。", { size: 20, spacingAfter: 60 }));
children.push(p("2. 6×8＝48（块），48－30＝18（块）　答：还剩18块。", { size: 20, spacingAfter: 60 }));
children.push(p("3. 48÷6＝8（本）；每个小组再分2本：8＋2＝10（本），共需 10×6＝60（本）。（或：48＋2×6＝60）", {
  size: 20, spacingAfter: 60
}));
children.push(p("4. 300÷5＝60（米）　答：平均每分钟走60米。", { size: 20, spacingAfter: 60 }));
children.push(p("5. 3×40＝120（本），180＋120＝300（本）；300÷6＝50（本）　答：现在有300本，每个书架放50本。", {
  size: 20, spacingAfter: 120
}));

children.push(p("—— 试卷结束，祝同学们学习进步！——", {
  size: 18, center: true, color: "888888", spacingBefore: 200
}));
children.push(p("知识点覆盖：千以内数认识与比较 · 三位数加减 · 表内乘除与有余数除法 · 混合运算 · 长度单位 · 时间 · 方向位置 · 轴对称 · 测量 · 解决问题", {
  size: 16, center: true, color: "999999", spacingBefore: 80
}));

const doc = new Document({
  styles: {
    default: {
      document: {
        run: { font: "宋体", size: 22 }
      }
    }
  },
  sections: [{
    properties: {
      page: {
        size: { width: PAGE_W, height: PAGE_H },
        margin: { top: MARGIN, right: MARGIN, bottom: MARGIN, left: MARGIN }
      }
    },
    headers: {
      default: new Header({
        children: [new Paragraph({
          alignment: AlignmentType.RIGHT,
          children: [
            new TextRun({ text: "试卷神器 · 北师大版二年级数学", size: 16, font: "宋体", color: "999999" })
          ]
        })]
      })
    },
    footers: {
      default: new Footer({
        children: [new Paragraph({
          alignment: AlignmentType.CENTER,
          children: [
            new TextRun({ text: "第 ", size: 16, font: "宋体", color: "666666" }),
            new TextRun({ children: [PageNumber.CURRENT], size: 16, font: "宋体", color: "666666" }),
            new TextRun({ text: " 页", size: 16, font: "宋体", color: "666666" })
          ]
        })]
      })
    },
    children
  }]
});

Packer.toBuffer(doc).then(buffer => {
  const out = "北师大版二年级数学下册期末模拟考试卷.docx";
  fs.writeFileSync(out, buffer);
  console.log("Generated:", out);
});
