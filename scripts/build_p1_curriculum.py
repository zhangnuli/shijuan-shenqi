# -*- coding: utf-8 -*-
"""P1：补充人教数学 1–6、苏教数学骨架、英语人教 3–6 骨架课标包。"""
from __future__ import annotations
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "web" / "src-tauri" / "resources" / "data"

PLATFORM = {
    "platform": "国家中小学智慧教育平台 + 教材公开目录",
    "platformUrl": "https://basic.smartedu.cn/",
    "classroomUrl": "https://basic.smartedu.cn/syncClassroom",
    "materialUrl": "https://basic.smartedu.cn/tchMaterial",
    "elecEduUrl": "https://basic.smartedu.cn/elecEdu",
    "catalogSite": "http://www.dzkbw.com/",
    "note": "课标大纲用于命题/教案约束，不含教材正文。",
}


def unit(uid, name, lessons, points):
    return {"id": uid, "name": name, "lessons": lessons, "points": points}


def pack(subject, edition, grade, semester, title, units, edition_label, subject_label):
    sem_label = "上册" if semester == "shang" else "下册"
    return {
        "subject": subject,
        "edition": edition,
        "grade": grade,
        "semester": semester,
        "title": title,
        "source": {
            **PLATFORM,
            "editionLabel": edition_label,
            "subjectLabel": subject_label,
            "smarteduPathHint": f"小学 → {grade}年级 → {subject_label} → {edition_label} → {sem_label}",
        },
        "units": units,
        "examHints": [
            "命题必须落在本册单元/所选课时范围内",
            "单元测试聚焦所选单元与课时",
            "期中覆盖前半册，期末覆盖全册",
        ],
    }


# —— 人教版数学（公开单元结构摘要）——
MATH_RJ = {
    (1, "shang"): ("一年级上册", [
        unit("u1", "准备课", ["数一数", "比多少"], ["1-10 认识", "多少比较"]),
        unit("u2", "位置", ["上下", "前后", "左右"], ["位置关系"]),
        unit("u3", "1～5 的认识和加减法", ["认识", "分与合", "加减"], ["5 以内加减"]),
        unit("u4", "认识图形（一）", ["立体图形"], ["长方体正方体圆柱球"]),
        unit("u5", "6～10 的认识和加减法", ["认识", "加减", "连加连减"], ["10 以内加减"]),
        unit("u6", "11～20 各数的认识", ["数的组成", "读写", "序数"], ["20 以内数"]),
        unit("u7", "认识钟表", ["整时", "半时"], ["时间初步"]),
        unit("u8", "20 以内的进位加法", ["9 加几", "8 7 6 加几"], ["凑十法"]),
        unit("u9", "总复习", ["综合"], ["全册综合"]),
    ]),
    (1, "xia"): ("一年级下册", [
        unit("u1", "认识图形（二）", ["平面图形"], ["长方形正方形三角形圆"]),
        unit("u2", "20 以内的退位减法", ["十几减 9", "十几减几"], ["破十法"]),
        unit("u3", "分类与整理", ["分类", "简单统计"], ["分类标准"]),
        unit("u4", "100 以内数的认识", ["数位", "读写", "大小比较"], ["整十数"]),
        unit("u5", "认识人民币", ["元角分", "简单计算"], ["购物"]),
        unit("u6", "100 以内的加法和减法（一）", ["整十加减", "两位数加减一位数"], ["口算竖式"]),
        unit("u7", "找规律", ["简单规律"], ["观察推理"]),
        unit("u8", "总复习", ["综合"], ["全册综合"]),
    ]),
    (2, "shang"): ("二年级上册", [
        unit("u1", "长度单位", ["厘米米", "测量"], ["单位选择"]),
        unit("u2", "100 以内的加法和减法（二）", ["两位数加减", "进退位"], ["竖式验算"]),
        unit("u3", "角的初步认识", ["角", "直角"], ["角的判断"]),
        unit("u4", "表内乘法（一）", ["乘法意义", "2～6 口诀"], ["表内乘法"]),
        unit("u5", "观察物体（一）", ["不同位置观察"], ["空间观念"]),
        unit("u6", "表内乘法（二）", ["7～9 口诀", "解决问题"], ["综合应用"]),
        unit("u7", "认识时间", ["时分", "经过时间"], ["时间计算"]),
        unit("u8", "数学广角—搭配（一）", ["简单搭配"], ["有序思考"]),
        unit("u9", "总复习", ["综合"], ["全册综合"]),
    ]),
    (2, "xia"): ("二年级下册", [
        unit("u1", "数据收集整理", ["统计表"], ["数据整理"]),
        unit("u2", "表内除法（一）", ["平均分", "除法意义", "用口诀求商"], ["表内除法"]),
        unit("u3", "图形的运动（一）", ["轴对称", "平移"], ["图形变换"]),
        unit("u4", "表内除法（二）", ["用除法解决问题", "有余数"], ["余数意义"]),
        unit("u5", "混合运算", ["先乘除后加减", "小括号"], ["运算顺序"]),
        unit("u6", "有余数的除法", ["竖式", "验算"], ["余数小于除数"]),
        unit("u7", "万以内数的认识", ["计数单位", "读写比较"], ["大数认识"]),
        unit("u8", "克和千克", ["质量单位"], ["估测"]),
        unit("u9", "数学广角—推理", ["简单推理"], ["逻辑"]),
        unit("u10", "总复习", ["综合"], ["全册综合"]),
    ]),
    (3, "shang"): ("三年级上册", [
        unit("u1", "时、分、秒", ["时间单位", "简单计算"], ["时间"]),
        unit("u2", "万以内的加法和减法（一）", ["口算", "估算"], ["加减"]),
        unit("u3", "测量", ["毫米分米千米"], ["单位换算"]),
        unit("u4", "万以内的加法和减法（二）", ["连续进退位"], ["竖式"]),
        unit("u5", "倍的认识", ["求一个数的几倍", "倍数关系"], ["倍"]),
        unit("u6", "多位数乘一位数", ["笔算乘法", "解决问题"], ["乘法"]),
        unit("u7", "长方形和正方形", ["周长"], ["周长公式"]),
        unit("u8", "分数的初步认识", ["几分之一", "几分之几"], ["分数"]),
        unit("u9", "数学广角—集合", ["简单集合"], ["重叠问题"]),
        unit("u10", "总复习", ["综合"], ["全册综合"]),
    ]),
    (3, "xia"): ("三年级下册", [
        unit("u1", "位置与方向（一）", ["东南西北"], ["方向"]),
        unit("u2", "除数是一位数的除法", ["口算笔算", "验算"], ["除法"]),
        unit("u3", "复式统计表", ["统计"], ["读表"]),
        unit("u4", "两位数乘两位数", ["笔算", "估算"], ["乘法"]),
        unit("u5", "面积", ["面积单位", "长方形正方形面积"], ["面积"]),
        unit("u6", "年、月、日", ["平年闰年", "24 时计时法"], ["时间"]),
        unit("u7", "小数的初步认识", ["小数意义", "简单加减"], ["小数"]),
        unit("u8", "数学广角—搭配（二）", ["排列组合初步"], ["有序"]),
        unit("u9", "总复习", ["综合"], ["全册综合"]),
    ]),
    (4, "shang"): ("四年级上册", [
        unit("u1", "大数的认识", ["亿以内数", "改写近似数"], ["大数"]),
        unit("u2", "公顷和平方千米", ["土地面积单位"], ["单位"]),
        unit("u3", "角的度量", ["量角器", "角的分类"], ["角度"]),
        unit("u4", "三位数乘两位数", ["笔算", "积的变化"], ["乘法"]),
        unit("u5", "平行四边形和梯形", ["特征"], ["四边形"]),
        unit("u6", "除数是两位数的除法", ["试商", "调商"], ["除法"]),
        unit("u7", "条形统计图", ["复式条形图"], ["统计"]),
        unit("u8", "数学广角—优化", ["合理安排"], ["策略"]),
        unit("u9", "总复习", ["综合"], ["全册综合"]),
    ]),
    (4, "xia"): ("四年级下册", [
        unit("u1", "四则运算", ["运算顺序", "括号"], ["四则"]),
        unit("u2", "观察物体（二）", ["从三个方向观察"], ["视图"]),
        unit("u3", "运算律", ["交换结合分配", "简算"], ["运算律"]),
        unit("u4", "小数的意义和性质", ["数位", "大小比较", "改写"], ["小数"]),
        unit("u5", "三角形", ["分类", "内角和"], ["三角形"]),
        unit("u6", "小数的加法和减法", ["计算", "验算"], ["小数加减"]),
        unit("u7", "图形的运动（二）", ["轴对称平移"], ["变换"]),
        unit("u8", "平均数与条形统计图", ["平均数"], ["统计"]),
        unit("u9", "数学广角—鸡兔同笼", ["假设法"], ["策略"]),
        unit("u10", "总复习", ["综合"], ["全册综合"]),
    ]),
    (5, "shang"): ("五年级上册", [
        unit("u1", "小数乘法", ["小数乘整数", "小数乘小数"], ["小数乘法"]),
        unit("u2", "位置", ["数对"], ["位置"]),
        unit("u3", "小数除法", ["除数是整数/小数", "循环小数"], ["小数除法"]),
        unit("u4", "可能性", ["等可能"], ["概率"]),
        unit("u5", "简易方程", ["方程意义", "解方程"], ["方程"]),
        unit("u6", "多边形的面积", ["平行四边形三角形梯形"], ["面积"]),
        unit("u7", "数学广角—植树问题", ["间隔"], ["策略"]),
        unit("u8", "总复习", ["综合"], ["全册综合"]),
    ]),
    (5, "xia"): ("五年级下册", [
        unit("u1", "观察物体（三）", ["立体图形观察"], ["空间"]),
        unit("u2", "因数与倍数", ["特征", "质合数"], ["因数倍数"]),
        unit("u3", "长方体和正方体", ["表面积体积"], ["立体"]),
        unit("u4", "分数的意义和性质", ["约分通分", "真假分数"], ["分数"]),
        unit("u5", "图形的运动（三）", ["旋转"], ["变换"]),
        unit("u6", "分数的加法和减法", ["异分母", "混合运算"], ["分数加减"]),
        unit("u7", "折线统计图", ["数据分析"], ["统计"]),
        unit("u8", "数学广角—找次品", ["优化策略"], ["策略"]),
        unit("u9", "总复习", ["综合"], ["全册综合"]),
    ]),
    (6, "shang"): ("六年级上册", [
        unit("u1", "分数乘法", ["意义", "计算", "解决问题"], ["分数乘法"]),
        unit("u2", "位置与方向（二）", ["方向距离"], ["位置"]),
        unit("u3", "分数除法", ["倒数", "除法", "解决问题"], ["分数除法"]),
        unit("u4", "比", ["比的意义", "化简比", "按比分配"], ["比"]),
        unit("u5", "圆", ["周长面积"], ["圆"]),
        unit("u6", "百分数（一）", ["意义", "互化", "应用"], ["百分数"]),
        unit("u7", "扇形统计图", ["选择统计图"], ["统计"]),
        unit("u8", "数学广角—数与形", ["数形结合"], ["思想"]),
        unit("u9", "总复习", ["综合"], ["全册综合"]),
    ]),
    (6, "xia"): ("六年级下册", [
        unit("u1", "负数", ["意义", "大小比较"], ["负数"]),
        unit("u2", "百分数（二）", ["折扣利息税率等"], ["百分数应用"]),
        unit("u3", "圆柱与圆锥", ["侧面积体积"], ["立体"]),
        unit("u4", "比例", ["正反比例"], ["比例"]),
        unit("u5", "数学广角—鸽巢问题", ["抽屉原理"], ["策略"]),
        unit("u6", "整理和复习", ["数与代数", "图形几何", "统计概率"], ["小学总复习"]),
    ]),
}

# 苏教数学：用简化镜像结构（单元名标注苏教）
def sujiao_from_renjiao():
    out = {}
    for k, (title, units) in MATH_RJ.items():
        su_units = []
        for u in units:
            su_units.append(unit(u["id"], u["name"], list(u["lessons"]), list(u["points"])))
        out[k] = (title, su_units)
    return out


MATH_SJ = sujiao_from_renjiao()

# 英语人教 3–6（单元主题骨架）
EN_RJ = {}
for g in range(3, 7):
    for sem, lab in [("shang", "上册"), ("xia", "下册")]:
        units = []
        for i in range(1, 7):
            units.append(
                unit(
                    f"u{i}",
                    f"Unit {i}",
                    [f"Part A", f"Part B", f"Part C"],
                    ["词汇", "句型", "听说读写", "语音/文化"] if g >= 4 else ["词汇", "句型", "听说"],
                )
            )
        units.append(unit("u7", "Recycle / 复习", ["综合复习"], ["单元综合", "情景交际"]))
        EN_RJ[(g, sem)] = (f"{g}年级{lab}", units)


def write_subject(folder, data, subject, edition, edition_label, subject_label):
    d = OUT / folder
    d.mkdir(parents=True, exist_ok=True)
    for (g, s), (title, units) in data.items():
        p = pack(subject, edition, g, s, title, units, edition_label, subject_label)
        path = d / f"grade-{g}-{s}.json"
        path.write_text(json.dumps(p, ensure_ascii=False, indent=2), encoding="utf-8")
        print("wrote", path.relative_to(OUT))


def main():
    write_subject("math/renjiao", MATH_RJ, "math", "renjiao", "人教版", "数学")
    write_subject("math/sujiao", MATH_SJ, "math", "sujiao", "苏教版", "数学")
    write_subject("english/renjiao", EN_RJ, "english", "renjiao", "人教版", "英语")
    # update index
    index = {
        "subjects": [
            {"subject": "math", "edition": "beishida", "label": "数学·北师大版"},
            {"subject": "math", "edition": "renjiao", "label": "数学·人教版"},
            {"subject": "math", "edition": "sujiao", "label": "数学·苏教版"},
            {"subject": "chinese", "edition": "renjiao", "label": "语文·人教统编版"},
            {"subject": "english", "edition": "renjiao", "label": "英语·人教版(3-6)"},
        ],
        "rebuild": "python scripts/build_p1_curriculum.py",
    }
    (OUT / "index.json").write_text(json.dumps(index, ensure_ascii=False, indent=2), encoding="utf-8")
    print("done")


if __name__ == "__main__":
    main()
