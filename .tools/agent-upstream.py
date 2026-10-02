# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""agent-upstream.py - 官方上游最新版预播 R2 版本段（REQ-0017，用户三裁 2026-10-02）。

口径：
- 清单驱动（.tools/agent-upstream.toml）：tier=agent（claude/codex/kimi）每日层、
  tier=cli（gh/rclone/uv 等 19 件）每周一层；层裁在 workflow 面（date -u +%u 判
  周一），本脚本收 --tier agent|all 与 --tools 名单过滤。
- 同版绿色跳过：域面比对 <domain>/<tool>/<version>/ 资产加边车双 HEAD 200 即已在播。
- 更新则逐件下载、官方 SHASUMS 逐件校验（combined 合并清单与 sidecar 逐件边车
  两形；none 形官方无清单，自产边车为唯一锚并 WARN 注记）；先全下全验后统一
  上传（防半灌，同 seed.py 先例）。
- rclone copyto 灌版本段 <tool>/<version>/（Cache-Control immutable）加自产
  sha256 边车（<hex 小写>空两格<asset>，同 seed.py 契约）。
- 预播成功后 repository_dispatch 通知 ai-cloud 仓（event_type=agent-upstream-seeded，
  client_payload 出工具加版本加逐件 sha 加 run_url；token 缺则 WARN 跳过不红）。
  R008 分工边界：catalog bump 与 stable 段归 ai-cloud catalog-seed，本面绝不触碰。
- 红灯：任一资产缺或校验不符即该工具整体不灌记失败，终局退出码 1（workflow
  侧失败自动落 gh issue 写明资产名与期望差）。

用法（uv 零安装，runner 预装 uv）：
  uv run --script .tools/agent-upstream.py --tier all --plan    # 只读预演（零下载零上传，无需 R2 凭据）
  uv run --script .tools/agent-upstream.py --tier agent          # agent 层真跑
  uv run --script .tools/agent-upstream.py --tier all --tools gh,uv  # 名单过滤真跑

环境变量：GH_TOKEN（gh api 用，runner 予 github.token，本机用 gh 登录态）；
上传需 R2_ACCESS_KEY_ID / R2_SECRET_ACCESS_KEY / R2_ENDPOINT / R2_BUCKET 与 rclone；
AGENT_UPSTREAM_DISPATCH_TOKEN 可选（ai-cloud repository_dispatch）；
AGENT_UPSTREAM_DOMAIN 可选（域面比对锚，缺省 https://env.ohmygh.com）。

退出码：0 全跳过或全成；1 有失败项；2 清单解析或参数错。
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import tomllib
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / ".tools" / "agent-upstream.toml"
DOMAIN = os.environ.get("AGENT_UPSTREAM_DOMAIN", "https://env.ohmygh.com")
DISPATCH_REPO = os.environ.get("AGENT_UPSTREAM_DISPATCH_REPO", "raystyle/ai-cloud")
DISPATCH_EVENT = "agent-upstream-seeded"
RCLONE_ENV = {
    "RCLONE_CONFIG_SEED_TYPE": "s3",
    "RCLONE_CONFIG_SEED_PROVIDER": "Cloudflare",
    "RCLONE_CONFIG_SEED_ENDPOINT": "R2_ENDPOINT",
    "RCLONE_CONFIG_SEED_ACCESS_KEY_ID": "R2_ACCESS_KEY_ID",
    "RCLONE_CONFIG_SEED_SECRET_ACCESS_KEY": "R2_SECRET_ACCESS_KEY",
    "RCLONE_CONFIG_SEED_NO_CHECK_BUCKET": "true",
}
# 官方清单两形：GNU（hex 空两格名 或 hex 空格*名）与 BSD（SHA256 (名) = hex）
GNU_RE = re.compile(r"^([0-9a-fA-F]{64})\s+\*?(.+?)\s*$")
BSD_RE = re.compile(r"^SHA256 \((.+?)\) = ([0-9a-fA-F]{64})$")


def http_get(url: str, timeout: int = 30) -> tuple[int, bytes]:
    req = urllib.request.Request(url, headers={"User-Agent": "ark-agent-upstream"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.status, resp.read()
    except urllib.error.HTTPError as e:
        return e.code, b""
    except Exception as e:  # 网络抖动如实报
        return -1, str(e).encode()


def http_head(url: str, timeout: int = 30) -> int:
    req = urllib.request.Request(url, method="HEAD", headers={"User-Agent": "ark-agent-upstream"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.status
    except urllib.error.HTTPError as e:
        return e.code
    except Exception:
        return -1


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def sidecar_text(sha_hex: str, asset: str) -> str:
    return f"{sha_hex.lower()}  {asset}\n"


def rclone_env() -> dict[str, str]:
    """rclone 子进程环境（RCLONE_ENV 映射：命名 remote seed: 的 env 配置形，同 seed.py）"""
    env = dict(os.environ)
    for k, v in RCLONE_ENV.items():
        env[k] = os.environ[v] if v.startswith("R2_") else v
    return env


def upload_object(local: Path, key: str, cache: str) -> bool:
    bucket = os.environ["R2_BUCKET"]
    proc = subprocess.run(
        ["rclone", "copyto", str(local), f"seed:{bucket}/{key}",
         "--header-upload", f"Cache-Control: {cache}", "--s3-upload-cutoff", "64MiB"],
        env=rclone_env(), capture_output=True, text=True,
    )
    if proc.returncode != 0:
        print(f"[FAIL] rclone copyto {key}: {proc.stderr.strip()[:300]}")
        return False
    return True


def upload_pair(local_asset: Path, sha_hex: str, tool: str, version: str) -> bool:
    """资产加边车成对上传（版本段 immutable 长缓存；边车经临时目录暂存，M015 同律）"""
    with tempfile.TemporaryDirectory() as stage:
        side = Path(stage) / f"{local_asset.name}.sha256"
        side.write_text(sidecar_text(sha_hex, local_asset.name), encoding="utf-8", newline="\n")
        cache = "public, max-age=31536000, immutable"
        ok = upload_object(local_asset, f"{tool}/{version}/{local_asset.name}", cache)
        ok2 = upload_object(side, f"{tool}/{version}/{side.name}", cache)
        return ok and ok2


def gh_latest_tag(repo: str) -> str:
    proc = subprocess.run(
        ["gh", "api", f"repos/{repo}/releases/latest", "--jq", ".tag_name"],
        capture_output=True, text=True,
    )
    if proc.returncode != 0 or not proc.stdout.strip():
        raise RuntimeError(f"gh api latest {repo}: {proc.stderr.strip()[:200]}")
    return proc.stdout.strip()


def parse_sums(text: str) -> dict[str, str]:
    """官方清单解析（两形兼容）：名到小写 hex 的映射"""
    table: dict[str, str] = {}
    for ln in text.splitlines():
        g = GNU_RE.match(ln)
        b = BSD_RE.match(ln) if not g else None
        if g:
            table[g.group(2).strip()] = g.group(1).lower()
        elif b:
            table[b.group(1).strip()] = b.group(2).lower()
    return table


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tier", choices=["agent", "all"], default="agent",
                    help="层：agent（默认）或 all（cli 层加跑）")
    ap.add_argument("--tools", default="", help="逗号名单过滤（可选）")
    ap.add_argument("--plan", action="store_true",
                    help="只读预演：解析与域面比对，零下载零上传")
    args = ap.parse_args()

    try:
        manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    except Exception as e:
        print(f"[FAIL] 清单解析失败 {MANIFEST}: {e}")
        return 2
    wanted = {t.strip() for t in args.tools.split(",") if t.strip()}
    picked = []
    for t in manifest.get("tool") or []:
        if args.tier == "agent" and t.get("tier") != "agent":
            continue
        if wanted and t.get("name") not in wanted:
            continue
        picked.append(t)
    if not picked:
        print("[FAIL] 层裁后清单为空（检查 tier 与 --tools 过滤）")
        return 2

    stats = {"skip": 0, "seeded": 0, "failed": 0}
    fails: list[str] = []
    seeded: list[dict] = []
    bust = str(int(time.time()))

    for t in picked:
        name, repo = t["name"], t["repo"]
        prefix = t.get("tag_prefix") or ""
        tier = t.get("tier", "cli")
        sums = t.get("sums") or {}
        strategy = sums.get("strategy", "none")
        try:
            tag = gh_latest_tag(repo)
        except Exception as e:
            print(f"[FAIL] {name}: {e}")
            stats["failed"] += 1
            fails.append(f"{name}: {e}")
            continue
        version = tag.removeprefix(prefix)
        assets = [a.replace("{version}", version) for a in t.get("assets") or []]
        if not assets:
            print(f"[FAIL] {name}: 清单 assets 为空")
            stats["failed"] += 1
            fails.append(f"{name}: assets 空")
            continue
        base = f"{DOMAIN}/{name}/{version}"
        # 同版绿跳判据 = 资产加边车双在位（F6：只比资产会把边车传失败的半灌态永久
        # 掩盖；对齐 seed.py remote_state 的 synced 语义）
        if all(http_head(f"{base}/{a}?v={bust}") == 200
               and http_head(f"{base}/{a}.sha256?t={bust}") == 200 for a in assets):
            stats["skip"] += 1
            print(f"[skip] {name} {version}（域面资产加边车全在位，同版绿色跳过）")
            continue
        print(f"[{'plan' if args.plan else 'seed'}] {name} {version}（tag {tag}，{len(assets)} 件）")
        if args.plan:
            continue

        # 先全下全验后统一上传（防半灌：任一缺或校验不符即整体不灌）
        problem = ""
        sha_map: dict[str, str] = {}
        with tempfile.TemporaryDirectory() as td:
            tdp = Path(td)
            for a in assets:
                code, body = http_get(
                    f"https://github.com/{repo}/releases/download/{tag}/{a}", timeout=900)
                if code != 200 or not body:
                    problem = f"{a}: 下载 HTTP {code}"
                    break
                (tdp / a).write_bytes(body)
                sha_map[a] = sha256_file(tdp / a)
            if not problem and strategy == "combined":
                sfile = (sums.get("file") or "").replace("{version}", version)
                code, body = http_get(
                    f"https://github.com/{repo}/releases/download/{tag}/{sfile}", timeout=120)
                if code != 200:
                    problem = f"{sfile}: 官方清单下载 HTTP {code}"
                else:
                    table = parse_sums(body.decode(errors="replace"))
                    # 键形回退（F3）：官方清单键可能带目标目录前缀（mq 形：名/名.exe），
                    # 按 basename 对齐裸资产名
                    by_base = {k.rsplit("/", 1)[-1]: v for k, v in table.items()}
                    for a in assets:
                        exp = table.get(a) or by_base.get(a)
                        if exp is None:
                            problem = f"{a}: 官方清单缺行（{sfile}）"
                            break
                        if exp != sha_map[a]:
                            problem = (f"{a}: 校验不符（官方 {exp[:12]}… "
                                       f"实际 {sha_map[a][:12]}…）")
                            break
            elif not problem and strategy == "sidecar":
                for a in assets:
                    code, body = http_get(
                        f"https://github.com/{repo}/releases/download/{tag}/{a}.sha256",
                        timeout=120)
                    if code != 200:
                        problem = f"{a}.sha256: 官方边车下载 HTTP {code}"
                        break
                    text = body.decode(errors="replace")
                    exp = parse_sums(text).get(a)
                    if not exp and text.split():
                        exp = text.split()[0].lower()
                    if exp != sha_map[a]:
                        problem = (f"{a}: 校验不符（官方 {str(exp)[:12]}… "
                                   f"实际 {sha_map[a][:12]}…）")
                        break
            elif not problem and strategy == "none":
                print(f"[WARN] {name}: 官方无清单（none 形），自产边车为唯一锚")
            if problem:
                print(f"[FAIL] {name} {version}: {problem}（整体不灌，防半灌）")
                stats["failed"] += 1
                fails.append(f"{name}@{version}: {problem}")
                continue
            up_ok = all(upload_pair(tdp / a, sha_map[a], name, version) for a in assets)
            if not up_ok:
                stats["failed"] += 1
                fails.append(f"{name}@{version}: rclone 上传失败")
                continue
        stats["seeded"] += 1
        seeded.append({
            "name": name, "tier": tier, "repo": repo, "tag": tag, "version": version,
            "assets": [{"name": a, "sha256": sha_map[a].upper()} for a in assets],
        })
        print(f"[ok] {name} {version}: {len(assets)} 件灌版本段（immutable）加边车")

    # 预播成功通知 ai-cloud（catalog bump 面，R008 分工；token 缺则 WARN 跳过不红）。
    # F5：该端点必填 event_type 加 client_payload（对象形，顶层不超 10 键、64KB 内），
    # gh -F 传 string 会被拒，必须 --input - 喂整 JSON。
    if seeded:
        token = os.environ.get("AGENT_UPSTREAM_DISPATCH_TOKEN")
        run_url = ""
        if os.environ.get("GITHUB_REPOSITORY"):
            run_url = (f"{os.environ.get('GITHUB_SERVER_URL', 'https://github.com')}/"
                       f"{os.environ['GITHUB_REPOSITORY']}/actions/runs/"
                       f"{os.environ.get('GITHUB_RUN_ID', '')}")
        payload = json.dumps({
            "event_type": DISPATCH_EVENT,
            "client_payload": {
                "tools": seeded,
                "run_url": run_url,
                "seeded_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            },
        }, ensure_ascii=False)
        if not token:
            print("[WARN] AGENT_UPSTREAM_DISPATCH_TOKEN 未配置，跳过 repository_dispatch"
                  "（预播已成功不红；配置后下次生效）")
        else:
            proc = subprocess.run(
                ["gh", "api", f"repos/{DISPATCH_REPO}/dispatches", "--method", "POST",
                 "--input", "-", "-H", "Content-Type: application/json"],
                input=payload, capture_output=True, text=True,
                env={**os.environ, "GH_TOKEN": token},
            )
            if proc.returncode != 0:
                print(f"[FAIL] repository_dispatch {DISPATCH_REPO}: {proc.stderr.strip()[:200]}")
                stats["failed"] += 1
                fails.append(f"dispatch: {proc.stderr.strip()[:120]}")
            else:
                print(f"[ok] repository_dispatch {DISPATCH_EVENT} -> {DISPATCH_REPO}"
                      f"（{len(seeded)} 工具）")

    print(json.dumps({"tier": args.tier, **stats, "fails": fails}, ensure_ascii=False))
    return 1 if stats["failed"] else 0


if __name__ == "__main__":
    sys.exit(main())
