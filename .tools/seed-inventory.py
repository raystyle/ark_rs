# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""seed-inventory.py - 从 catalog/tools.toml 抽出 ohmycloud 镜像种子清单

唯一权威是 catalog pin（version + asset + sha256 三键齐才可入镜）。
输出 markdown，供 ISSUE 派给 raystyle/ohmycloud（流程见 R014）。

用法:
  uv run --script .tools/seed-inventory.py
  uv run --script .tools/seed-inventory.py --json

退出码: 0 成功; 2 读 catalog 失败。
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def _data_local_dir() -> Path:
    """对齐 Rust `dirs::data_local_dir()`：win 取 %LOCALAPPDATA%（缺省回落 ~/AppData/Local），
    mac 取 ~/Library/Application Support，其余取 $XDG_DATA_HOME（缺省 ~/.local/share）。"""
    home = Path.home()
    if sys.platform == "win32":
        return Path(os.environ.get("LOCALAPPDATA") or home / "AppData" / "Local")
    if sys.platform == "darwin":
        return home / "Library" / "Application Support"
    return Path(os.environ.get("XDG_DATA_HOME") or home / ".local" / "share")


def _catalog_path() -> Path:
    """对账清单源（D37 权威在 ohmycloud catalog-seed）：仓库件（开发态）优先，miss 则
    用户数据副本（云端同步件，ark 主名位）；两者皆缺提示先 ark catalog sync。"""
    data = _data_local_dir()
    cands = [
        ROOT / "catalog" / "tools.toml",
        data / "ark" / "catalog" / "tools.toml",
    ]
    for c in cands:
        if c.exists():
            return c
    raise SystemExit(
        "对账清单源缺失（仓库件与用户数据副本均无）：先跑 ark catalog sync 取云端件；" +
        "；".join(str(c) for c in cands)
    )

CATALOG = _catalog_path()
EVERGREEN_EXTRACT = {"ark-self", "vsbuild", "rustup"}


def trip(t: dict, prefix: str) -> tuple[str | None, str | None, str | None]:
    if prefix:
        return t.get(f"{prefix}version"), t.get(f"{prefix}asset"), t.get(f"{prefix}sha256")
    return t.get("version"), t.get("asset"), t.get("sha256")


def complete(v: tuple[str | None, str | None, str | None]) -> bool:
    return bool(v[0] and v[1] and v[2])


def partial(v: tuple[str | None, str | None, str | None]) -> bool:
    return bool((v[0] or v[1]) and not complete(v))


def collect(tools: dict) -> dict:
    seed: list[dict] = []
    missing: list[dict] = []
    evergreen: list[str] = []
    for name, t in tools.items():
        ext = t.get("extract") or ""
        if ext in EVERGREEN_EXTRACT:
            evergreen.append(name)
            continue
        plats = []
        for plat, prefix in (("win", ""), ("linux", "linux_"), ("mac", "mac_")):
            v = trip(t, prefix)
            if complete(v):
                plats.append(
                    {
                        "platform": plat,
                        "version": v[0],
                        "asset": v[1],
                        "sha256": v[2],
                    }
                )
            elif partial(v):
                missing.append(
                    {
                        "tool": name,
                        "platform": plat,
                        "version": v[0],
                        "asset": v[1],
                        "sha256": v[2],
                    }
                )
        if plats:
            seed.append({"tool": name, "hold": bool(t.get("hold")), "objects": plats})
    return {
        "tools_total": len(tools),
        "seed_tools": len(seed),
        "seed_objects": sum(len(x["objects"]) for x in seed),
        "seed": seed,
        "missing_sha": missing,
        "evergreen": evergreen,
    }


def emit_md(inv: dict) -> str:
    lines = [
        f"catalog 工具 {inv['tools_total']}；可入镜 {inv['seed_tools']} 工具、{inv['seed_objects']} 对象。",
        "",
        "路径：`https://env.ohmygh.com/<tool>/<version>/<asset>`，同名 `.sha256` 边车。",
        "信任锚即 ark catalog pin；无 sha 不入镜。",
        "",
        "### 可入镜",
        "",
        "| 工具 | 平台 | version | asset | sha256 |",
        "| --- | --- | --- | --- | --- |",
    ]
    for item in inv["seed"]:
        hold = " hold" if item["hold"] else ""
        for o in item["objects"]:
            lines.append(
                f"| {item['tool']}{hold} | {o['platform']} | {o['version']} | `{o['asset']}` | `{o['sha256']}` |"
            )
    lines += ["", "### 缺 sha 暂不入镜", ""]
    if inv["missing_sha"]:
        for m in inv["missing_sha"]:
            lines.append(
                f"- {m['tool']} {m['platform']} version={m['version'] or '-'} asset=`{m['asset'] or '-'}`"
            )
    else:
        lines.append("无。")
    lines += [
        "",
        "### evergreen（沙滚段，边车即锚）",
        "",
        "- `ark/dev/` 加 `ark/stable/`：`ark-*` 主名三资产（D41 B；ome 兼容段已收口 2026-09-18 剔除批）",
        "- `rust/latest/rustup-init.exe`",
        "- `vsbuild/latest/vs_buildtools.exe`",
        "",
        f"catalog evergreen 条目：{', '.join(inv['evergreen'])}。",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()
    try:
        data = tomllib.loads(CATALOG.read_text(encoding="utf-8"))
    except OSError as e:
        print(f"读 catalog 失败: {e}", file=sys.stderr)
        return 2
    inv = collect(data["tools"])
    if args.json:
        print(json.dumps(inv, ensure_ascii=False, indent=2))
    else:
        sys.stdout.write(emit_md(inv))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
