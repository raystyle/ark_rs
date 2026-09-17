# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""seed.py - 软件资产域对账（--plan 只读）与 ark 自产产物灌段（D27/D41 路线 A）。
D37 完全解耦后资产播种与清单三件套运营归 ohmycloud catalog-seed，本件留对账面与自产段面。

口径（R014 延续）：
- 唯一权威 catalog\tools.toml：version + asset + sha256 三键齐才入镜；无 sha 进 pending 队列；
  hold 与 evergreen（ark-self/vsbuild/rustup）不入镜。
- 边车自算即锚：`<hex 小写>空两格<asset>` 同名 .sha256 边车，上传前资产先过 catalog sha 锚校验；
  边车经临时目录暂存后上传（不落源资产目录，源目录可能是仓库工作树，M015）。
- diff 走公网域面（GET 边车带 ?t= 时间戳击穿 + HEAD 资产），不需要 R2 凭据；上传走 rclone（CI 内）。
- rclone 配方：provider=Cloudflare、endpoint=R2_ENDPOINT、NO_CHECK_BUCKET 必带（受限 token 无建桶权）；
  段制缓存分治（REQ-0007 批一对齐标准）：版本段 immutable 长缓存、stable 加 dev 滚动段 max-age=60
  （消费侧 ?v=/?t= 击穿双保险）；stable 段 sync 清旧以排除清单保 D46 msvc 回退件一窗
  （rclone 语义实证 2026-09-17：excluded 默认不删、--delete-excluded 反清保护面，故不带该旗标）。

用法（uv 零安装，runner 预装 uv）：
  uv run --script .tools/seed.py --plan            # 全 catalog 域面 diff（只读，无凭据可跑）
  uv run --script .tools/seed.py                   # diff 加上传（需 R2_* 环境变量与 rclone）
  uv run --script .tools/seed.py --ark-dev --tag dev         # 路线 A：dev 产物灌 ark/dev 沙滚段（主名）
  uv run --script .tools/seed.py --ark-stable --tag v1.0.0   # 路线 A：v* 正式产物灌 ark/stable 段（主名）
  uv run --script .tools/seed.py --ark-published --tag v1.0.0  # 发布双段：版本段 copy（immutable）
                                                              # 加 stable sync（排除保 msvc 窗），
                                                              # 双段零上传红灯（r2-seed workflow 用）

环境变量：R2_ACCESS_KEY_ID / R2_SECRET_ACCESS_KEY / R2_ENDPOINT / R2_BUCKET（上传必需）；
GH_TOKEN 可选（公开仓不需要）；ARK_SEED_ALLOW_SKIP=1 豁免 --ark-stable/--ark-dev 的全 skip
红灯（旧 tag 窗口期重灌预期 skip 时用；--ark-published 严格面不适用——源空时防空段闸先红）。

退出码：0 全同步或 plan；1 有失败项（含 --ark-stable 零到件红灯、--ark-published
双段逐名核对缺件红灯）；2 catalog 解析失败或参数错。
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.request
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
    用户数据副本（云端同步件，D41 起 ark 主名、ohmyenv 旧位读回）；两者皆缺提示先 ark catalog sync。"""
    data = _data_local_dir()
    cands = [
        ROOT / "catalog" / "tools.toml",
        data / "ark" / "catalog" / "tools.toml",
        data / "ohmyenv" / "catalog" / "tools.toml",
    ]
    for c in cands:
        if c.exists():
            return c
    raise SystemExit(
        "对账清单源缺失（仓库件与用户数据副本均无）：先跑 ark catalog sync 取云端件；" +
        "；".join(str(c) for c in cands)
    )

DOMAIN = "https://env.ohmygh.com"
EVERGREEN_EXTRACT = {"ark-self", "vsbuild", "rustup"}
PLATFORMS = (("win", "", ""), ("linux", "linux_", "linux_"), ("mac", "mac_", "mac_"))
# 路线 A 的本仓三资产（CI 目标三元组；selfupdate 资产名同源）：主名 ark-*。
# ome-* 兼容名与 ome/ 段已撤（全舰队 ome 水位清零，2026-09-14 收口）。
_TRIPLES = (
    "x86_64-pc-windows-gnu.exe",
    "x86_64-unknown-linux-gnu",
    "aarch64-apple-darwin",
)
ARK_ASSETS = [f"ark-{t}" for t in _TRIPLES]
# REQ-0008 窗口包形名族（净 triple，hst 族形一致）：win zip 他 tar.gz。--ark-published
# 面到则灌（裸件终版窗的 release 无包形，missing_ok 跳过即 phase 无关，无需旗标与版本阈值）。
PACKAGE_ASSETS = [
    f"ark-{t.removesuffix('.exe')}.{'zip' if 'windows' in t else 'tar.gz'}" for t in _TRIPLES
]
RCLONE_ENV = {
    "RCLONE_CONFIG_SEED_TYPE": "s3",
    "RCLONE_CONFIG_SEED_PROVIDER": "Cloudflare",
    "RCLONE_CONFIG_SEED_ENDPOINT": "R2_ENDPOINT",
    "RCLONE_CONFIG_SEED_ACCESS_KEY_ID": "R2_ACCESS_KEY_ID",
    "RCLONE_CONFIG_SEED_SECRET_ACCESS_KEY": "R2_SECRET_ACCESS_KEY",
    "RCLONE_CONFIG_SEED_NO_CHECK_BUCKET": "true",
}
# stable 段 sync 的保窗排除清单（D46 msvc 回退件，selfupdate 回退读序依赖）：排除即保护
# （rclone 实证 2026-09-17：excluded 默认不删）。退役点 = 下一正式版封版窗口（总台核准
# 2026-09-17：五端 1.3.0+ 全实证后撤排除清单清段）。
STABLE_PROTECT = ("ark-x86_64-pc-windows-msvc.exe", "ark-x86_64-pc-windows-msvc.exe.sha256")


def http_get(url: str, timeout: int = 30) -> tuple[int, bytes]:
    req = urllib.request.Request(url, headers={"User-Agent": "ark-seed", "Cache-Control": "no-cache"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.status, resp.read()
    except urllib.error.HTTPError as e:
        return e.code, b""
    except Exception as e:  # 网络抖动如实报
        return -1, str(e).encode()


def http_head(url: str, timeout: int = 30) -> int:
    req = urllib.request.Request(url, method="HEAD", headers={"User-Agent": "ark-seed"})
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


def collect() -> tuple[list[dict], list[dict], list[str]]:
    """catalog -> （可入镜对象, pending_sha 队列, evergreen 排除名单）"""
    # 清单源用到时才解析（路线 A 的 --ark-* 不需要清单，不得因缺件而失败）
    data = tomllib.loads(_catalog_path().read_text(encoding="utf-8"))["tools"]
    objs: list[dict] = []
    pending: list[dict] = []
    evergreen: list[str] = []
    for name, t in data.items():
        ext = t.get("extract") or ""
        if ext in EVERGREEN_EXTRACT or t.get("hold"):
            evergreen.append(name)
            continue
        prefix_tag = t.get("tag_prefix") or ""
        for plat, pver, prel in PLATFORMS:
            repo = t.get(prel + "repo") or t.get("repo")
            version = t.get(pver + "version")
            asset = t.get(pver + "asset")
            sha = t.get(pver + "sha256")
            tag = t.get(pver + "tag") or (prefix_tag + version if version else None)
            if version and asset and sha:
                objs.append(
                    {"tool": name, "platform": plat, "repo": repo, "tag": tag,
                     "version": version, "asset": asset, "sha256": sha.upper()}
                )
            elif version or asset:
                pending.append({"tool": name, "platform": plat, "version": version, "asset": asset})
    return objs, pending, evergreen


def remote_state(tool: str, version: str, asset: str, sha: str) -> str:
    """域面状态：synced（边车值等且资产在）/ sidecar-missing / asset-missing / sha-drift / unreachable"""
    bust = str(int(time.time()))
    code, body = http_get(f"{DOMAIN}/{tool}/{version}/{asset}.sha256?t={bust}")
    if code == -1:
        return "unreachable"
    if code != 200:
        return "sidecar-missing"
    first = body.decode(errors="replace").split()
    if not first or first[0].lower() != sha.lower():
        return "sha-drift"
    if http_head(f"{DOMAIN}/{tool}/{version}/{asset}?v={sha[:16]}") != 200:
        return "asset-missing"
    return "synced"


def stage_sidecar(stage: Path, sha_hex: str, asset_name: str) -> Path:
    """边车写进暂存目录（M015：不落源资产所在目录，源目录可能是仓库工作树）"""
    side = stage / f"{asset_name}.sha256"
    side.write_text(sidecar_text(sha_hex, asset_name), encoding="utf-8", newline="\n")
    return side


def upload_pair(local_asset: Path, sha_hex: str, tool: str, version: str, dry: bool) -> bool:
    """资产加边车成对上传（rclone copyto；Cache-Control 双保险；边车经临时目录暂存；
    大件分账阈值 64MiB 沿用 B 面历史口径）"""
    ok = True
    with tempfile.TemporaryDirectory() as stage:
        stage_sidecar(Path(stage), sha_hex, local_asset.name)
        for src, name in ((local_asset, local_asset.name),
                          (Path(stage) / f"{local_asset.name}.sha256",
                           f"{local_asset.name}.sha256")):
            ok = upload_object(src, f"{tool}/{version}/{name}", dry,
                               extra=("--s3-upload-cutoff", "64MiB")) and ok
    return ok


def download_asset(repo: str, tag: str, asset: str, dest: Path, missing_ok: bool = False) -> str:
    """下载 release 资产，返回 "ok" / "skip" / "fail"。missing_ok 时 404 记 skip
    （D46 工具链切换窗口期：D46 前 tag 重灌时 gnu 资产不存在，段内存量 msvc 对象不动）。"""
    url = f"https://github.com/{repo}/releases/download/{tag}/{asset}"
    code, body = http_get(url, timeout=600)
    if code != 200 or not body:
        if missing_ok and code == 404:
            print(f"[skip] {asset} 在 {tag} 无此资产（工具链切换窗口期），不灌不红")
            return "skip"
        print(f"[FAIL] 下载 {url}: HTTP {code}")
        return "fail"
    dest.write_bytes(body)
    return True


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--plan", action="store_true", help="只 diff 不上传")
    ap.add_argument("--ark-dev", action="store_true", help="路线 A：dev 产物灌 ark/dev 沙滚段（主名）")
    ap.add_argument("--ark-stable", action="store_true", help="路线 A：v* 正式产物灌 ark/stable 段（主名）")
    ap.add_argument("--ark-published", action="store_true",
                    help="发布双段：版本段 copy（immutable）加 stable sync（排除保 msvc 窗），双段零上传红灯")
    ap.add_argument("--tag", default="dev", help="路线 A 的 release tag")
    args = ap.parse_args()
    dry = args.plan

    if args.ark_dev or args.ark_stable or args.ark_published:
        if args.ark_published and not args.tag.startswith("v"):
            print("[FAIL] --ark-published 需 v* tag（版本段名取自 tag）")
            return 2
        repo = "raystyle/ark_rs"
        # 段与资产族配套：ark/ 段配 ark-* 主名。
        # ome/ 段与 ome-* 兼容名已停写（全舰队 ome 水位清零，2026-09-14 收口）。
        if args.ark_dev:
            mode = "ark-dev"
        elif args.ark_stable:
            mode = "ark-stable"
        else:
            mode = "ark-published"
        results = {"staged": 0, "failed": 0}
        skipped: list[str] = []
        staged_names: list[str] = []
        with tempfile.TemporaryDirectory() as td:
            tdp = Path(td)
            stage = tdp / "seg"
            stage.mkdir()
            # published 面 release 已发布，裸件缺失即红（不走窗口期 skip 白名单）；
            # 包形到则灌（REQ-0008 窗口，裸件终版窗跳过即 phase 无关）
            fam = [(a, not args.ark_published) for a in ARK_ASSETS]
            fam += [(a, True) for a in PACKAGE_ASSETS]
            for asset, mok in fam:
                local = tdp / asset
                st = download_asset(repo, args.tag, asset, local, missing_ok=mok)
                if st == "fail":
                    results["failed"] += 1
                    continue
                if st == "skip":
                    skipped.append(asset)
                    continue
                sha = sha256_file(local)
                (stage / asset).write_bytes(local.read_bytes())
                stage_sidecar(stage, sha, asset)
                results["staged"] += 1
            if results["failed"]:
                # sync 形下部分灌段会清掉未到件（sync 以源为段终态），失败即整体不灌
                print("[FAIL] 有资产下载失败，跳过灌段（防半失败灌段）")
            elif args.ark_published:
                # 双段同灌（REQ-0007 批一，差4 裁 a 收归自家）：版本段 copy 加 immutable
                # 长缓存（差8）；stable 段 sync 清旧加排除保 msvc 窗（差5）。
                ver_seg = f"ark/{args.tag[1:]}"
                ok = all(
                    upload_object(p, f"{ver_seg}/{p.name}", dry,
                                  cache="public, max-age=31536000, immutable")
                    for p in sorted(stage.iterdir())
                )
                if not ok or not sync_segment(stage, "ark/stable", dry,
                                              protect=STABLE_PROTECT):
                    results["failed"] += 1
            elif args.ark_dev:
                if not sync_segment(stage, "ark/dev", dry):
                    results["failed"] += 1
            else:
                if not sync_segment(stage, "ark/stable", dry,
                                    protect=STABLE_PROTECT):
                    results["failed"] += 1
            staged_names = sorted(p.name for p in stage.iterdir())
        exit_code = 0 if results["failed"] == 0 else 1
        # skip 分流：裸件 skip 是真信号（清出话术与红灯判定）；包形 skip 在裸件终版窗
        # 是常态（REQ-0008 窗口前 release 无包形，phase 无关设计），只落 INFO 不进判定
        bare_skipped = [a for a in skipped if a not in PACKAGE_ASSETS]
        pkg_skipped = [a for a in skipped if a in PACKAGE_ASSETS]
        bare_staged = sum(1 for n in staged_names if n in ARK_ASSETS)
        if bare_skipped and mode in ("ark-stable", "ark-published"):
            names = "、".join(bare_skipped)
            counts = (f"staged {results['staged']}、skip {len(skipped)}、"
                      f"failed {results['failed']}")
            if not bare_staged and os.environ.get("ARK_SEED_ALLOW_SKIP") != "1":
                # 正常 tag run 应直灌（v1.2.3 实录三件齐灌）；裸件零到件即段未动
                # （多系 draft 窗口或资产名漂移），红灯拦静默丢段
                # （v1.3.0 漏切实录，2026-09-17 补审 F2）。
                print(f"[FAIL] {mode} 裸件零到件（{counts}，skip：{names}）：段未动。"
                      "发布后须 r2-seed dispatch 带 tag 补推；"
                      "窗口期旧 tag 重灌预期 skip 用 ARK_SEED_ALLOW_SKIP=1 豁免"
                      "（仅 --ark-stable/--ark-dev 面）。")
                exit_code = 1
            else:
                # sync 形下源即段终态：裸件 skip 不在源内，会被 sync 按源清出段
                print(f"[WARN] {mode} 裸件有 skip（{names}），sync 将按源清出段外")
        elif bare_skipped:
            print(f"[INFO] ark-dev 裸件有 skip（{'、'.join(bare_skipped)}），"
                  "sync 将按源清出 dev 段外")
        if pkg_skipped:
            print(f"[INFO] 包形未在 release（{len(pkg_skipped)} 件，裸件终版窗常态），"
                  "跳过灌段")
        if not dry and args.ark_published and exit_code == 0:
            # 双段齐备红灯（标准护栏三件之二，逐名核对收严：lsf 整段计数会把保窗
            # 排除件也计入非零，按暂存件名核对在位才算过）。
            missing: list[str] = []
            for seg in (f"ark/{args.tag[1:]}", "ark/stable"):
                have = segment_listing(seg)
                missing += [f"{seg}/{n}" for n in staged_names if n not in have]
            if missing:
                print(f"[FAIL] 段内核对缺件（{len(missing)}）：{', '.join(missing[:6])}")
                exit_code = 1
            else:
                print(f"mirror objects: ver 与 stable 各 {len(staged_names)} 件逐名在位")
        print(json.dumps({"mode": mode, **results}, ensure_ascii=False))
        return exit_code

    objs, pending, evergreen = collect()
    if objs is None:
        return 2
    results = {"total": len(objs), "synced": 0, "uploaded": 0, "failed": 0}
    fails: list[str] = []
    with tempfile.TemporaryDirectory() as td:
        tdp = Path(td)
        for o in objs:
            state = remote_state(o["tool"], o["version"], o["asset"], o["sha256"])
            if state == "synced":
                results["synced"] += 1
                continue
            print(f"[{'plan' if dry else 'sync'}] {o['tool']}/{o['version']}/{o['asset']}: {state}")
            if dry:
                continue
            local = tdp / o["asset"]
            if download_asset(o["repo"], o["tag"], o["asset"], local) != "ok":
                results["failed"] += 1
                fails.append(o["asset"])
                continue
            actual = sha256_file(local)
            if actual.upper() != o["sha256"]:
                print(f"[FAIL] 锚校验不过 {o['asset']}: catalog {o['sha256'][:12]}… 实际 {actual[:12]}…")
                results["failed"] += 1
                fails.append(o["asset"])
                continue
            if upload_pair(local, actual, o["tool"], o["version"], False):
                results["uploaded"] += 1
            else:
                results["failed"] += 1
                fails.append(o["asset"])
    # ome/catalog/ 三件套（清单加边车加 .minisig）自 B 承接起归 ohmycloud catalog-seed 流水
    # （R015 五：签名与播种运营托管 omc，2026-09-10 双轨收口）。本脚本不再上传 catalog 本体——
    # 防双轨竞态：本仓先推新清单而对方未补签时，云端会出现锚对但签名失配的窗口态。
    # 本脚本保留软件资产域（/<tool>/<version>/<asset> 加边车）的播种。
    print(f"[skip] ome/catalog/ 三件套归 ohmycloud catalog-seed（R015 B 承接）")
    print(json.dumps({**results, "pending_sha": len(pending), "evergreen": len(evergreen),
                      "fails": fails}, ensure_ascii=False))
    return 1 if (not dry and results["failed"]) else 0


def upload_object(local: Path, key: str, dry: bool,
                  cache: str = "public, max-age=60",
                  extra: tuple[str, ...] = ()) -> bool:
    """单对象上传（rclone copyto；cache 可覆写（版本段 immutable 长缓存））"""
    if dry:
        print(f"[plan] rclone copyto {local.name} -> seed:$R2_BUCKET/{key}")
        return True
    bucket = os.environ["R2_BUCKET"]
    proc = subprocess.run(
        ["rclone", "copyto", str(local), f"seed:{bucket}/{key}",
         "--header-upload", f"Cache-Control: {cache}", *extra],
        env=rclone_env(), capture_output=True, text=True,
    )
    if proc.returncode != 0:
        print(f"[FAIL] rclone copyto {key}: {proc.stderr.strip()[:300]}")
        return False
    return True


def rclone_env() -> dict[str, str]:
    """rclone 子进程环境（RCLONE_ENV 映射：命名 remote seed: 的 env 配置形）"""
    env = dict(os.environ)
    for k, v in RCLONE_ENV.items():
        env[k] = os.environ[v] if v.startswith("R2_") else v
    return env


def sync_segment(stage: Path, seg: str, dry: bool, protect: tuple[str, ...] = ()) -> bool:
    """整段 sync：源暂存目录即段终态（资产加边车成对）。不带 --delete-excluded：
    排除即保护（rclone 语义实证 2026-09-17），sync 自身已清源外旧件。protect 为
    保窗排除清单（D46 msvc 回退件）。源空即红拒绝 sync（防空段清段）。"""
    if not any(stage.iterdir()):
        print(f"[FAIL] {seg} 段源暂存空，拒绝 sync（防空段清段）")
        return False
    if dry:
        print(f"[plan] rclone sync {stage.name}/ -> seed:$R2_BUCKET/{seg}/"
              f"{' protect=' + ','.join(protect) if protect else ''}")
        return True
    bucket = os.environ["R2_BUCKET"]
    # --checksum 同族规（hst 参考实现同款）：源为当日下载件，size 加 modtime 风险本低，
    # checksum 校验多一道内容级保险
    cmd = ["rclone", "sync", str(stage), f"seed:{bucket}/{seg}/", "--checksum"]
    for p in protect:
        cmd += ["--exclude", p]
    proc = subprocess.run(cmd, env=rclone_env(), capture_output=True, text=True)
    if proc.returncode != 0:
        print(f"[FAIL] rclone sync {seg}: {proc.stderr.strip()[:300]}")
        return False
    return True


def segment_listing(seg: str) -> set[str]:
    """段内对象名集合（双段齐备红灯逐名核对用；lsf 失败按空集计即红）"""
    proc = subprocess.run(
        ["rclone", "lsf", f"seed:{os.environ['R2_BUCKET']}/{seg}/"],
        env=rclone_env(), capture_output=True, text=True,
    )
    if proc.returncode != 0:
        print(f"[WARN] rclone lsf {seg}: {proc.stderr.strip()[:200]}")
        return set()
    return {ln.strip() for ln in proc.stdout.splitlines() if ln.strip()}


if __name__ == "__main__":
    sys.exit(main())
