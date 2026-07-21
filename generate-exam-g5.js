const { Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell,
        Header, Footer, AlignmentType, BorderStyle, WidthType, ShadingType,
        VerticalAlign, PageNumber, PageBreak } = require('docx');
const fs = require('fs');

const PAGE_W = 11906;
const PAGE_H = 16838;
const MARGIN = 720;
const CONTENT_W = PAGE_W - MARGIN * 2;

const noBorder = { style: BorderStyle.NONE, size: 0, color: "FFFFFF" };
const noBorders = { top: noBorder, bottom: noBorder, left: noBorder, right: noBorder };
const thinBorder = { style: BorderStyle.SINGLE, size: 8, color: "333333" };
const thinBorders = { top: thinBorder, bottom: thinBorder, left: thinBorder, right: thinBorder };

function p(text, opts = {}) {
  const { bold = false, size = 21, center = false, indent = 0, spacingAfter = 80, spacingBefore = 0, color = "000000", font = "宋体" } = opts;
  return new Paragraph({
    alignment: center ? AlignmentType.CENTER : AlignmentType.LEFT,
    spacing: { after: spacingAfter, before: spacingBefore, line: 340 },
    indent: indent ? { left: indent } : undefined,
    children: [new TextRun({ text, bold, size, font, color })]
  });
}

function runs(parts, paraOpts = {}) {
  return new Paragraph({
    alignment: paraOpts.center ? AlignmentType.CENTER : AlignmentType.LEFT,
    spacing: { after: paraOpts.spacingAfter ?? 80, before: paraOpts.spacingBefore ?? 0, line: paraOpts.line ?? 340 },
    indent: paraOpts.indent ? { left: paraOpts.indent } : undefined,
    children: parts.map(part => new TextRun({
      text: part.text,
      bold: part.bold || false,
      size: part.size || 21,
      font: part.font || "宋体",
      color: part.color || "000000"
    }))
  });
}

function sectionTitle(text) {
  return new Paragraph({
    spacing: { before: 200, after: 120, line: 340 },
    border: { bottom: { style: BorderStyle.SINGLE, size: 6, color: "2E75B6", space: 4 } },
    children: [new TextRun({ text, bold: true, size: 24, font: "黑体", color: "1F4E79" })]
  });
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

function calcCell(expr, w, lines = 3) {
  const kids = [
    new Paragraph({
      alignment: AlignmentType.CENTER,
      spacing: { after: 60 },
      children: [new TextRun({ text: expr, size: 20, font: "宋体" })]
    })
  ];
  for (let i = 0; i < lines; i++) {
    kids.push(new Paragraph({
      alignment: AlignmentType.CENTER,
      spacing: { after: 40 },
      children: [new TextRun({ text: i === 0 ? "＝" : "　", size: 20, font: "宋体" })]
    }));
  }
  return new TableCell({
    borders: thinBorders,
    width: { size: w, type: WidthType.DXA },
    margins: { top: 80, bottom: 120, left: 60, right: 60 },
    children: kids
  });
}

const children = [];

// ===== 标题 =====
children.push(p("小学数学模拟考试卷", {
  bold: true, size: 36, center: true, font: "黑体", spacingAfter: 60, spacingBefore: 80
}));
children.push(p("北师大版 · 五年级（下册）期末", {
  bold: true, size: 28, center: true, font: "黑体", color: "1F4E79", spacingAfter: 60
}));
children.push(p("（真题模拟 · 满分100分 · 建议用时90分钟）", {
  size: 18, center: true, color: "666666", spacingAfter: 140
}));

children.push(infoTable());
children.push(new Paragraph({ spacing: { after: 100 }, children: [] }));
children.push(scoreTable());
children.push(new Paragraph({ spacing: { after: 140 }, children: [] }));

children.push(runs([
  { text: "温馨提示：", bold: true, size: 18, color: "C0392B" },
  { text: "仔细审题，步骤完整；能约分的要约分；带单位的题别忘写单位；注意验算。", size: 18, color: "555555" }
], { spacingAfter: 180 }));

// ===== 一、填空题 =====
children.push(sectionTitle("一、填空题（每空1分，共20分）"));

const fillIns = [
  "1. 在（　　）里填上合适的数：  0.8＝（　　）/10＝（　　）%；  3/4＝（　　）/12＝（　　）%。",
  "2. 把 2.4 扩大到原来的 10 倍是（　　），缩小到原来的 1/100 是（　　）。",
  "3. 一个长方体长 8 cm、宽 5 cm、高 4 cm，它的体积是（　　）cm³，表面积是（　　）cm²。",
  "4. 正方体的棱长是 6 dm，它的体积是（　　）dm³＝（　　）cm³。",
  "5. 3/5 × 10/9 ＝（　　）（写成最简分数）；  7/8 ÷ 14/15 ＝（　　）。",
  "6. 一台洗衣机原价 2000 元，现打八折出售，现价是（　　）元；比原价便宜（　　）元。",
  "7. 把 3.6 米长的绳子平均分成 8 段，每段长（　　）米，合（　　）分米。",
  "8. 一个数的 3/4 是 36，这个数是（　　）；36 的 3/4 是（　　）。",
  "9. 从 1、2、3、4、5、6 这六个数字中任意摸出一个，摸到偶数的可能性是（　　），摸到大于 4 的数的可能性是（　　）。",
  "10. 分数 5/12 和 7/18 的公分母可以是（　　），通分后分别是（　　）和（　　）。"
];
fillIns.forEach(t => children.push(p(t, { size: 20, spacingAfter: 120 })));

// ===== 二、判断题 =====
children.push(sectionTitle("二、判断题（对的打“√”，错的打“×”，每题2分，共10分）"));

const judges = [
  "1. 小数点向右移动两位，原数就扩大到原来的 100 倍。（　　）",
  "2. 两个分数相乘，积一定大于每一个乘数。（　　）",
  "3. 长方体有 6 个面、12 条棱、8 个顶点。（　　）",
  "4. 甲数比乙数多 20%，乙数就比甲数少 20%。（　　）",
  "5. 把一个数除以分数，等于乘这个分数的倒数。（　　）"
];
judges.forEach(t => children.push(p(t, { size: 20, spacingAfter: 100 })));

// ===== 三、选择题 =====
children.push(sectionTitle("三、选择题（把正确答案的序号填在括号里，每题2分，共10分）"));

const choices = [
  {
    q: "1. 下列各数中，最大的是（　　）。",
    opts: "A. 0.99　　B. 9/10　　C. 0.909　　D. 99%"
  },
  {
    q: "2. 一个长方体长 8 cm、宽 6 cm、高 5 cm，它的棱长总和是（　　）。",
    opts: "A. 76 cm　　B. 38 cm　　C. 19 cm　　D. 152 cm"
  },
  {
    q: "3. 计算 2/3 ÷ 4/9 的正确结果是（　　）。",
    opts: "A. 8/27　　B. 3/2　　C. 2/3　　D. 1/6"
  },
  {
    q: "4. 一杯果汁，喝了 2/5，又倒入半杯果汁，现在杯中果汁相当于满杯的（　　）。",
    opts: "A. 9/10　　B. 11/10　　C. 1　　D. 4/5"
  },
  {
    q: "5. 把棱长 4 cm 的正方体铁块熔化后铸成一个高 2 cm 的长方体，长方体的底面积是（　　）。",
    opts: "A. 32 cm²　　B. 16 cm²　　C. 64 cm²　　D. 8 cm²"
  }
];
choices.forEach(c => {
  children.push(p(c.q, { size: 20, spacingAfter: 40 }));
  children.push(p(c.opts, { size: 19, indent: 200, spacingAfter: 100 }));
});

// ===== 四、计算题 =====
children.push(sectionTitle("四、计算题（共30分）"));

children.push(runs([
  { text: "1. 直接写出得数（每题1分，共8分）", bold: true, size: 20 }
], { spacingAfter: 80 }));

const directRows = [
  ["1.5×0.4＝", "3.6÷0.9＝", "2/5＋3/10＝", "5/6－1/3＝"],
  ["7/8×4/21＝", "9/10÷3/5＝", "25%×40＝", "0.8×1/4＝"]
];
directRows.forEach(row => {
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
            children: [new TextRun({ text: expr + "　　", size: 20, font: "宋体" })]
          })]
        }))
      })
    ]
  }));
  children.push(new Paragraph({ spacing: { after: 60 }, children: [] }));
});

children.push(runs([
  { text: "2. 用竖式计算（每题3分，共6分）", bold: true, size: 20 }
], { spacingAfter: 80, spacingBefore: 80 }));

{
  const exprs = ["12.48 ÷ 2.4", "3.75 × 1.6"];
  const w = Math.floor(CONTENT_W / 2);
  const widths = [w, CONTENT_W - w];
  children.push(new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({
        children: exprs.map((expr, i) => new TableCell({
          borders: thinBorders,
          width: { size: widths[i], type: WidthType.DXA },
          margins: { top: 100, bottom: 300, left: 100, right: 100 },
          children: [
            new Paragraph({
              alignment: AlignmentType.CENTER,
              children: [new TextRun({ text: expr, size: 20, font: "宋体" })]
            }),
            new Paragraph({ spacing: { after: 200 }, children: [] }),
            new Paragraph({ spacing: { after: 200 }, children: [] }),
            new Paragraph({ spacing: { after: 200 }, children: [] }),
            new Paragraph({ spacing: { after: 120 }, children: [] })
          ]
        }))
      })
    ]
  }));
}

children.push(runs([
  { text: "3. 脱式计算（能简算的要简算，每题2分，共8分）", bold: true, size: 20 }
], { spacingAfter: 80, spacingBefore: 140 }));

{
  const exprs = [
    "2.5×3.2＋2.5×6.8",
    "7/12＋5/18",
    "3/4 × (2/5 ＋ 2/15)",
    "（5.6－1.8）÷0.4"
  ];
  const w = Math.floor(CONTENT_W / 2);
  const widths = [w, CONTENT_W - w];
  // 2x2 grid
  for (let r = 0; r < 2; r++) {
    children.push(new Table({
      width: { size: CONTENT_W, type: WidthType.DXA },
      columnWidths: widths,
      rows: [
        new TableRow({
          children: [0, 1].map(c => {
            const expr = exprs[r * 2 + c];
            return new TableCell({
              borders: thinBorders,
              width: { size: widths[c], type: WidthType.DXA },
              margins: { top: 80, bottom: 200, left: 80, right: 80 },
              children: [
                new Paragraph({
                  alignment: AlignmentType.CENTER,
                  spacing: { after: 60 },
                  children: [new TextRun({ text: expr, size: 19, font: "宋体" })]
                }),
                new Paragraph({ alignment: AlignmentType.CENTER, children: [new TextRun({ text: "＝", size: 19, font: "宋体" })] }),
                new Paragraph({ spacing: { after: 80 }, children: [] }),
                new Paragraph({ alignment: AlignmentType.CENTER, children: [new TextRun({ text: "＝", size: 19, font: "宋体" })] }),
                new Paragraph({ spacing: { after: 60 }, children: [] })
              ]
            });
          })
        })
      ]
    }));
    children.push(new Paragraph({ spacing: { after: 60 }, children: [] }));
  }
}

children.push(runs([
  { text: "4. 解方程（每题2分，共8分）", bold: true, size: 20 }
], { spacingAfter: 80, spacingBefore: 100 }));

{
  const eqs = ["x＋3.5＝8.2", "2.4x＝7.2", "x－2/5＝1/2", "3/4 x＝9/10"];
  const w = Math.floor(CONTENT_W / 2);
  const widths = [w, CONTENT_W - w];
  for (let r = 0; r < 2; r++) {
    children.push(new Table({
      width: { size: CONTENT_W, type: WidthType.DXA },
      columnWidths: widths,
      rows: [
        new TableRow({
          children: [0, 1].map(c => {
            const expr = eqs[r * 2 + c];
            return new TableCell({
              borders: thinBorders,
              width: { size: widths[c], type: WidthType.DXA },
              margins: { top: 80, bottom: 160, left: 80, right: 80 },
              children: [
                new Paragraph({
                  alignment: AlignmentType.CENTER,
                  spacing: { after: 40 },
                  children: [new TextRun({ text: expr, size: 20, font: "宋体" })]
                }),
                new Paragraph({ children: [new TextRun({ text: "解：", size: 18, font: "宋体" })] }),
                new Paragraph({ spacing: { after: 100 }, children: [] }),
                new Paragraph({ spacing: { after: 60 }, children: [] })
              ]
            });
          })
        })
      ]
    }));
    children.push(new Paragraph({ spacing: { after: 60 }, children: [] }));
  }
}

// ===== 五、操作与实践 =====
children.push(sectionTitle("五、操作与实践题（共10分）"));

children.push(p("1.（4分）下图是一个长方体展开图的一部分（单位：cm）。请补全展开图，并求这个长方体的体积和表面积。", {
  size: 20, spacingAfter: 80
}));
children.push(p("已知：底面是长 6 cm、宽 4 cm 的长方形，高是 5 cm。", {
  size: 19, spacingAfter: 60, color: "444444"
}));

// Simple unfold sketch with labels
{
  const w = Math.floor(CONTENT_W / 3);
  const widths = [w, w, CONTENT_W - w * 2];
  children.push(new Table({
    width: { size: CONTENT_W, type: WidthType.DXA },
    columnWidths: widths,
    rows: [
      new TableRow({
        children: ["前面 6×5", "上面 6×4", "后面 6×5"].map((lab, i) => new TableCell({
          borders: thinBorders,
          width: { size: widths[i], type: WidthType.DXA },
          margins: { top: 120, bottom: 120, left: 60, right: 60 },
          children: [
            new Paragraph({
              alignment: AlignmentType.CENTER,
              children: [new TextRun({ text: lab, size: 18, font: "宋体" })]
            }),
            new Paragraph({
              alignment: AlignmentType.CENTER,
              spacing: { before: 60 },
              children: [new TextRun({ text: "（请补画侧面）", size: 14, color: "888888", font: "宋体" })]
            })
          ]
        }))
      })
    ]
  }));
}
children.push(p("体积 V＝____________　　表面积 S＝____________", {
  size: 20, spacingAfter: 120, spacingBefore: 100
}));

children.push(p("2.（3分）在下面的方格中画出一个面积为 12 格的轴对称图形，并画出对称轴。", {
  size: 20, spacingAfter: 80
}));

// 6x4 grid
{
  const n = 8;
  const cellW = 480;
  const totalW = cellW * n;
  children.push(new Table({
    width: { size: totalW, type: WidthType.DXA },
    columnWidths: Array(n).fill(cellW),
    rows: [0, 1, 2, 3, 4].map(() => new TableRow({
      height: { value: 360, rule: "atLeast" },
      children: Array(n).fill(0).map(() => new TableCell({
        borders: thinBorders,
        width: { size: cellW, type: WidthType.DXA },
        margins: { top: 10, bottom: 10, left: 10, right: 10 },
        children: [new Paragraph({
          alignment: AlignmentType.CENTER,
          children: [new TextRun({ text: "　", size: 12, font: "宋体" })]
        })]
      }))
    }))
  }));
}

children.push(p("3.（3分）把一张长 20 cm、宽 15 cm 的长方形纸，剪成尽可能多的边长 5 cm 的小正方形。最多能剪（　　）个，剩余纸片的面积是（　　）cm²。", {
  size: 20, spacingAfter: 80, spacingBefore: 140
}));

// ===== 六、解决问题 =====
children.push(sectionTitle("六、解决问题（共20分）"));

const problems = [
  {
    title: "1.（4分）",
    body: "果园里有桃树 120 棵，梨树的棵数是桃树的 3/4，苹果树比梨树多 25 棵。苹果树有多少棵？"
  },
  {
    title: "2.（4分）",
    body: "一项工程，甲单独做 10 天完成，乙单独做 15 天完成。两人合作，每天完成这项工程的几分之几？合作几天可以完成？"
  },
  {
    title: "3.（4分）",
    body: "一个长方体玻璃鱼缸（无盖），长 5 dm、宽 3 dm、高 4 dm。制作这个鱼缸至少需要玻璃多少平方分米？如果里面的水深 3 dm，水的体积是多少立方分米？（1 dm³＝1 L）"
  },
  {
    title: "4.（4分）",
    body: "商场促销：原价 80 元的书包现价 64 元。现价是原价的百分之几？便宜了百分之几？如果按现价购买，买 5 个比原价一共节省多少元？"
  },
  {
    title: "5.（4分）★稍难",
    body: "一杯牛奶，第一次喝了全部的 1/3，第二次喝了余下的 1/4，这时杯中还剩 150 毫升。这杯牛奶原来有多少毫升？"
  }
];

problems.forEach((pr, idx) => {
  children.push(runs([
    { text: pr.title, bold: true, size: 20 },
    { text: pr.body, size: 20 }
  ], { spacingAfter: 50, spacingBefore: idx === 0 ? 0 : 100 }));
  children.push(p("解：", { size: 19, spacingAfter: 40 }));
  children.push(p("　", { size: 19, spacingAfter: 30 }));
  children.push(p("　", { size: 19, spacingAfter: 30 }));
  children.push(p("　", { size: 19, spacingAfter: 30 }));
  children.push(p("答：________________________________________", { size: 19, spacingAfter: 60 }));
});

// ===== 参考答案 =====
children.push(new Paragraph({ children: [new PageBreak()] }));

children.push(p("参考答案与评分说明", {
  bold: true, size: 32, center: true, font: "黑体", spacingAfter: 60, spacingBefore: 80
}));
children.push(p("小学数学 · 北师大版 · 五年级（下册）期末模拟卷", {
  size: 20, center: true, color: "666666", spacingAfter: 180
}));

children.push(sectionTitle("一、填空题（每空1分，共20分）"));
[
  "1. 8； 80； 9； 75",
  "2. 24； 0.024",
  "3. 160； 184　（S＝2×(8×5＋8×4＋5×4)＝2×(40＋32＋20)＝184）",
  "4. 216； 216000",
  "5. 2/3； 15/16",
  "6. 1600； 400",
  "7. 0.45； 4.5",
  "8. 48； 27",
  "9. 1/2（或 3/6）； 1/3（或 2/6）",
  "10. 36（或其他公倍数亦可）； 15/36； 14/36"
].forEach(t => children.push(p(t, { size: 19, spacingAfter: 50 })));

children.push(sectionTitle("二、判断题（每题2分，共10分）"));
children.push(p("1. √　　2. ×（如 1/2×1/3＝1/6，积比乘数小）　　3. √　　4. ×（乙比甲少的百分数＝20%÷120%＝1/6≈16.7%）　　5. √", {
  size: 19, spacingAfter: 80
}));

children.push(sectionTitle("三、选择题（每题2分，共10分）"));
children.push(p("1. A（0.99＞0.9＝9/10＝99%＞0.909）", { size: 19, spacingAfter: 40 }));
children.push(p("2. A（棱长总和＝4×(8＋6＋5)＝4×19＝76 cm）", { size: 19, spacingAfter: 40 }));
children.push(p("3. B（2/3÷4/9＝2/3×9/4＝3/2）", { size: 19, spacingAfter: 40 }));
children.push(p("4. B（喝剩 3/5，再倒入半杯：3/5＋1/2＝11/10）", { size: 19, spacingAfter: 40 }));
children.push(p("5. A（体积 4³＝64 cm³，底面积＝64÷2＝32 cm²）", { size: 19, spacingAfter: 40 }));
children.push(p("汇总：1.A　2.A　3.B　4.B　5.A", { size: 20, spacingAfter: 100 }));

children.push(sectionTitle("四、计算题（共30分）"));
children.push(runs([{ text: "1. 直接写得数：", bold: true, size: 19 }], { spacingAfter: 40 }));
children.push(p("0.6； 4； 7/10； 1/2；　　1/6； 3/2（或1.5）； 10； 0.2", { size: 19, spacingAfter: 70 }));

children.push(runs([{ text: "2. 竖式：", bold: true, size: 19 }], { spacingAfter: 40 }));
children.push(p("12.48÷2.4＝5.2；　3.75×1.6＝6", { size: 19, spacingAfter: 70 }));

children.push(runs([{ text: "3. 脱式：", bold: true, size: 19 }], { spacingAfter: 40 }));
children.push(p("2.5×3.2＋2.5×6.8＝2.5×(3.2＋6.8)＝2.5×10＝25", { size: 19, spacingAfter: 40 }));
children.push(p("7/12＋5/18＝21/36＋10/36＝31/36", { size: 19, spacingAfter: 40 }));
children.push(p("3/4×(2/5＋2/15)＝3/4×(6/15＋2/15)＝3/4×8/15＝2/5", { size: 19, spacingAfter: 40 }));
children.push(p("（5.6－1.8）÷0.4＝3.8÷0.4＝9.5", { size: 19, spacingAfter: 70 }));

children.push(runs([{ text: "4. 解方程：", bold: true, size: 19 }], { spacingAfter: 40 }));
children.push(p("x＝8.2－3.5＝4.7；　x＝7.2÷2.4＝3；　x＝1/2＋2/5＝9/10；　x＝9/10×4/3＝6/5", {
  size: 19, spacingAfter: 100
}));

children.push(sectionTitle("五、操作与实践题（共10分）"));
children.push(p("1. 展开图补全：在侧面位置补画两个 4×5 的长方形即可。体积 V＝6×4×5＝120 cm³；表面积 S＝2×(6×4＋6×5＋4×5)＝2×(24＋30＋20)＝148 cm²。", {
  size: 19, spacingAfter: 60
}));
children.push(p("2. 图形面积为 12 格且轴对称即可，对称轴画正确给满分。", {
  size: 19, spacingAfter: 60
}));
children.push(p("3. 最多剪 4×3＝12 个；剩余面积 20×15－12×25＝300－300＝0 cm²。（若理解为不可拼接则同样刚好剪完）", {
  size: 19, spacingAfter: 100
}));

children.push(sectionTitle("六、解决问题（共20分）"));
children.push(p("1. 梨树：120×3/4＝90（棵）；苹果树：90＋25＝115（棵）。答：苹果树有115棵。", {
  size: 19, spacingAfter: 50
}));
children.push(p("2. 每天完成：1/10＋1/15＝1/6；合作天数：1÷1/6＝6（天）。答：每天完成1/6，合作6天完成。", {
  size: 19, spacingAfter: 50
}));
children.push(p("3. 无盖表面积：5×3＋2×(5×4＋3×4)＝15＋2×(20＋12)＝15＋64＝79（dm²）；水体积：5×3×3＝45（dm³）＝45 L。", {
  size: 19, spacingAfter: 50
}));
children.push(p("4. 现价是原价：64÷80＝0.8＝80%；便宜了：1－80%＝20%（或(80－64)÷80＝20%）；节省：16×5＝80（元）。", {
  size: 19, spacingAfter: 50
}));
children.push(p("5. 第二次喝后剩余：1－1/4＝3/4（相对第一次喝后）；第一次喝后剩：2/3。故最终剩余占原来：2/3×3/4＝1/2。原来：150÷1/2＝300（毫升）。答：原来有300毫升。", {
  size: 19, spacingAfter: 120
}));

children.push(p("—— 试卷结束，祝同学们学习进步！——", {
  size: 18, center: true, color: "888888", spacingBefore: 160
}));
children.push(p("知识点覆盖：小数乘除 · 分数四则 · 百分数 · 长方体与正方体 · 方程 · 可能性 · 轴对称 · 工程问题 · 折扣与百分数应用", {
  size: 15, center: true, color: "999999", spacingBefore: 60
}));

const doc = new Document({
  styles: {
    default: {
      document: {
        run: { font: "宋体", size: 21 }
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
            new TextRun({ text: "试卷神器 · 北师大版五年级数学", size: 16, font: "宋体", color: "999999" })
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
  const out = "北师大版五年级数学下册期末模拟考试卷.docx";
  fs.writeFileSync(out, buffer);
  console.log("Generated:", out);
});
