#!/usr/bin/env python3
"""mdcharlint.py：Markdown 技术文档禁用字符检查（G001 四类硬禁令的机械判定）。

规则来源：中英文 Markdown 技术文档写作规范 v1.0（2026-09-02，用户提供，G001 已吸收）。
四类 error 级：破折号与连接号、Unicode 箭头、emoji 与装饰符号、非法全角（白名单外）。
豁免区：围栏代码块、行内代码、链接目标、裸 URL；引文与规则定义区由人工核对。
用法：uv run --script .tools\\mdcharlint.py 文件1.md [文件2.md ...]（或目录，递归 .md）
"""

# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///

import re
import sys

# 输出端编码免疫（M006 家族）：CI hosted runner 新镜像 Python 默认 cp1252，print 中文
# （含「通过」行）即 UnicodeEncodeError 假红；stdout/stderr 统一 UTF-8 加损耗替换
for _s in (sys.stdout, sys.stderr):
    if hasattr(_s, "reconfigure"):
        _s.reconfigure(encoding="utf-8", errors="replace")

from pathlib import Path

RULES = [
    ("DASH", re.compile(r"[–—―−ー]")),
    ("ARROW", re.compile(r"[←-⇿➔➜➡⬅-⬇]")),
    ("EMOJI", re.compile(
        "[\U0001F000-\U0001FAFF☀-➿⬀-⬏"
        "‼⁉ℹ︀-️‍]")),
    ("FULLWIDTH", re.compile(r"[！-｠　‘-”]")),
]
CJK_OK = set("，。：；？！、（）《》「」『』·")
INLINE_CODE = re.compile(r"`[^`]*`")
LINK_TARGET = re.compile(r"\]\([^)]*\)")
BARE_URL = re.compile(r"https?://\S+")


def mask(line: str) -> str:
    line = INLINE_CODE.sub(lambda m: "`" + " " * (len(m.group()) - 2) + "`", line)
    line = LINK_TARGET.sub("]( )", line)
    return BARE_URL.sub(" ", line)


def scan(text: str):
    in_fence = False
    for no, raw in enumerate(text.splitlines(), 1):
        if raw.lstrip().startswith(("```", "$$")):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        for col, ch in enumerate(mask(raw), 1):
            for name, pattern in RULES:
                if pattern.match(ch) and not (name == "FULLWIDTH" and ch in CJK_OK):
                    yield no, col, name, ch
                    break


def main() -> int:
    import argparse

    # Windows CI 的 Python stdout 默认 cp1252，打印中文路径与总结行会 UnicodeEncodeError
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    ap = argparse.ArgumentParser(description="Markdown 禁用字符检查（G001 四类硬禁令）")
    ap.add_argument("paths", nargs="+", help="文件或目录（目录递归 .md）")
    ap.add_argument(
        "--all",
        action="store_true",
        help="连同历史归档（docs/diary、docs/proven）一起扫描；默认跳过（历史文档不回改惯例）",
    )
    args = ap.parse_args()
    excluded = {"diary", "proven", "aidoc"}
    targets: list[Path] = []
    for arg in args.paths:
        p = Path(arg)
        if p.is_dir():
            targets.extend(sorted(p.rglob("*.md")))
        else:
            targets.append(p)
    hits = 0
    for path in targets:
        if not args.all and any(seg in excluded for seg in path.parts[:-1]):
            continue
        for no, col, name, ch in scan(path.read_text(encoding="utf-8")):
            print(f"{path}:{no}:{col}: {name} U+{ord(ch):04X} {ch!r}")
            hits += 1
    print(f"违规 {hits} 处" if hits else "通过：未发现违规字符")
    return 1 if hits else 0


if __name__ == "__main__":
    sys.exit(main())
