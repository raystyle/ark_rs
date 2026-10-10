# /// script
# requires-python = ">=3.9"
# dependencies = []
# ///
"""llms-guard.py - 根 llms.txt 模块表与技能加载视图漂移守卫（ADR-0009，con-04 件5）

守两处手维护面的漂移（doc-gov「生成优于维护」的守卫形态）：
1. 根 llms.txt 代码文件位置表：src/*.rs 双向对账（每个 src 模块必有行、每行 src 路径必在盘），
   其余行（tests/、目录行）单向存在性对账；
2. 技能加载视图：docs/operations/skills/<名>/SKILL.md 与 .claude/skills/<名>/SKILL.md
   逐字节同步（投影形加载视图，ADR-0009），双向无孤儿。

退出码：0 守卫过；1 有漂移（错误行逐条列出）。
"""

import argparse
import io
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def read(path):
    with io.open(path, encoding="utf-8") as f:
        return f.read()


def table_rows(llms_text):
    """取代码文件位置表的首列路径（`| `src/x.rs` |` 形）。"""
    rows = []
    in_table = False
    for line in llms_text.split("\n"):
        if line.startswith("## 代码文件位置"):
            in_table = True
            continue
        if in_table:
            if line.startswith("## "):
                break
            m = re.match(r"^\|\s*`([^`]+)`\s*\|", line)
            if m:
                rows.append(m.group(1))
    return rows


def check_module_table(errors):
    llms = read(os.path.join(ROOT, "llms.txt"))
    rows = table_rows(llms)
    if not rows:
        errors.append("llms.txt 代码文件位置表为空或未解析到（表结构漂移）")
        return
    # 行侧：每行路径必须在盘（文件或目录）
    for r in rows:
        p = os.path.join(ROOT, r)
        if not os.path.exists(p):
            errors.append(f"模块表行指向不存在的路径: {r}")
    # 盘侧：src/*.rs 每个模块必须有一行（双向对账）
    row_set = set(rows)
    src_dir = os.path.join(ROOT, "src")
    for name in sorted(os.listdir(src_dir)):
        if name.endswith(".rs"):
            rel = f"src/{name}"
            if rel not in row_set:
                errors.append(f"src 模块缺 llms.txt 模块表行: {rel}")


def check_skills_projection(errors):
    ops = os.path.join(ROOT, "docs", "operations", "skills")
    view = os.path.join(ROOT, ".claude", "skills")
    ops_skills = sorted(os.listdir(ops)) if os.path.isdir(ops) else []
    view_skills = sorted(os.listdir(view)) if os.path.isdir(view) else []
    for name in ops_skills:
        src = os.path.join(ops, name, "SKILL.md")
        dst = os.path.join(view, name, "SKILL.md")
        if not os.path.isfile(src):
            continue
        if not os.path.isfile(dst):
            errors.append(f"技能加载视图缺投影: .claude/skills/{name}/SKILL.md")
        elif read(src) != read(dst):
            errors.append(f"技能加载视图与权威源不同步: .claude/skills/{name}/SKILL.md")
    for name in view_skills:
        if name not in ops_skills:
            errors.append(f"技能加载视图有孤儿（权威源无此技能）: .claude/skills/{name}/")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.parse_args()
    errors = []
    check_module_table(errors)
    check_skills_projection(errors)
    if errors:
        for e in errors:
            print(f"LLMS-GUARD FAIL: {e}")
        sys.exit(1)
    print("llms-guard: 模块表与技能投影守卫过（src 对账 + skills 同步）")
    sys.exit(0)


if __name__ == "__main__":
    main()
