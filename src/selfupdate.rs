//! selfupdate：ome 自身升级（`ark self update`），三通道：
//! - **dev**（默认）：pre-release tag `dev` 的滚动资产——CI push main 构建上传，本地测试期升级源；
//! - **stable**：`releases/latest` 正式版——CI 推 v* tag（封版）触发；
//! - **git**：源码安装——浅克隆仓库 cargo build 后替换（封版前无任何 release 时的通道，需 git 与 cargo）。
//!
//! 升级判定：release 资产的 API digest（sha256）与运行中 exe 的 sha256 对比，一致即已最新；
//! 不同则经 download_asset 下载到缓存（digest 校验）后替换部署位，并同步数据目录 catalog。
//! 包形资产（REQ-0008 窗口，ark-<target>.zip/.tar.gz）digest 为归档 sha：归档经锚校验下载后
//! 解包取内层二进制（拼名契约 ark-<target>/ark），判新等值在解包后的二进制间进行（机制不动）。
//! Windows 运行中 exe 可改名不可删：旧 exe 改名 .old 保留、新 exe 就位，下次升级开头清理。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::Value;

use crate::download::sha256_file;
use crate::platform;

/// 自升级源仓库（D41 更名；旧 ohmyenv-rs 名 GitHub 301 兜底，官方路径不断）。
/// doctor 网络探针同源引用（自测 7 机检）。
pub const REPO: &str = "raystyle/ark_rs";
const UA: &str = "ark-selfupdate";

/// 升级通道。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    /// dev 滚动源（main push CI，pre-release tag `dev`）
    Dev,
    /// 正式版（v* tag CI，releases/latest）
    Stable,
    /// 源码安装（浅克隆 + cargo build）
    Git,
}

/// 是否自管条目（extract = "ome-self"；D41 起双接受 "ark-self"，数据面改名可单方回退）：无 pin 无资产，升级走 self update 三通道。
pub fn is_ome_self(def: &crate::catalog::Tool) -> bool {
    matches!(def.extract(), Some("ome-self") | Some("ark-self"))
}

/// 升级结果。
pub struct SelfUpdateOutcome {
    /// updated：已替换；current：已是最新
    pub action: &'static str,
    /// 升级通道（dev/stable/git）
    pub channel: &'static str,
    /// 命中的资产名
    pub asset: String,
    /// 远端构建 sha256（前 8 位展示）
    pub sha256: String,
    /// 替换后的部署位 exe
    pub exe: PathBuf,
    /// 升级后 catalog 同步是否成功
    pub catalog_synced: bool,
}

/// 编译目标三元组（build.yml 资产命名约定的公共段）。Windows 自 D46 切 gnu 交叉
/// （ubuntu 岗 mingw-w64 摆脱 VC；CRT 静态零 DLL 依赖，ohmycloud 侧实证）。
fn platform_triple() -> Result<&'static str, String> {
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        return Ok("x86_64-pc-windows-gnu.exe");
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Ok("x86_64-unknown-linux-gnu");
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok("aarch64-apple-darwin");
    }
    #[allow(unreachable_code)]
    Err("当前平台无 CI 构建资产（release 未覆盖此目标）".to_string())
}

/// 编译目标对应的 CI 资产主名（D41 B：`ark-<triple>`，release 双附主名）。
///
/// # Errors
/// 返回 Err（人读原因串）当：ARK_MIRROR=1 镜像优先，跳过官方 API 等（完整失败面见函数体错误构造）。
pub fn asset_for_this_platform() -> Result<String, String> {
    Ok(format!("ark-{}", platform_triple()?))
}

/// 兼容资产名（`ome-<triple>`，旧二进制认的名；ome/ 分发面已收口停写，仅历史
/// release 残量资产仍可命中，读序殿后）。Windows 下随 gnu 三元组派生的此名从未存在
/// （历史 ome 资产是 msvc 名），该层在 Windows 恒 miss、对应窗口已由 msvc 回退层覆盖，
/// 仅非 Windows 平台有效。
///
/// # Errors
/// 返回 Err（人读原因串）当：ARK_MIRROR=1 镜像优先，跳过官方 API 等（完整失败面见函数体错误构造）。
pub fn asset_compat_for_this_platform() -> Result<String, String> {
    Ok(format!("ome-{}", platform_triple()?))
}

/// 包形净三元组（platform_triple 去 `.exe` 尾）：包名与包内目录用净 triple
///（hst 族形一致：`ark-x86_64-pc-windows-gnu.zip`、内层 `ark.exe`），裸件名才含 .exe。
fn package_triple() -> Result<&'static str, String> {
    Ok(platform_triple()?.trim_end_matches(".exe"))
}

/// 包形资产名（REQ-0008 窗口，对线裁定）：`ark-<triple>` 单顶层目录（净 triple），win
/// zip 他 tar.gz，逐包 .sha256 边车。读序主名先于裸件（双挂窗存量迁移方向），裸件为
/// 回退臂（第三版退役删臂）。
///
/// # Errors
/// 返回 Err 当：当前平台无 CI 构建资产（platform_triple 同源判）。
pub fn asset_package_for_this_platform() -> Result<String, String> {
    let t = package_triple()?;
    let ext = if t.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    };
    Ok(format!("ark-{t}.{ext}"))
}

/// 包内二进制定位（拼名契约：`ark-<target>/ark(.exe)`，无版本段——REQ-0008 对线裁定，
/// 版本由 tag、边车、catalog pin 承载，路径不重复承载）。
///
/// # Errors
/// 返回 Err 当：解包目录内按拼名契约未找到二进制（包形不合规）。
fn package_inner_binary(dir: &Path) -> Result<PathBuf, String> {
    let t = package_triple()?;
    let name = if t.contains("windows") {
        "ark.exe"
    } else {
        "ark"
    };
    let p = dir.join(format!("ark-{t}")).join(name);
    if p.exists() {
        Ok(p)
    } else {
        Err(format!(
            "包内缺 {}（拼名契约 ark-<target>/ark，REQ-0008）",
            p.display()
        ))
    }
}

/// 归档解包候选（REQ-0008 窗口）：归档经下载层 digest 锚校验后解到临时目录，
/// 复用 extract 面（zip/tar.gz 与工具安装同源）；POSIX 补执行位（zip 形不保 mode）。
/// 目录生命周期至调用方 replace 完成（replace 后由调用方收尾清理，错误路径交系统清理）。
///
/// # Errors
/// 返回 Err 当：建目录、解包或拼名定位失败。
fn unpack_candidate(archive: &Path) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("ark-selfupdate-unpack-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("建解包目录失败: {e}"))?;
    if archive.extension().and_then(|e| e.to_str()) == Some("zip") {
        crate::extract::extract_zip(archive, &dir)?;
    } else {
        crate::extract::extract_targz(archive, &dir)?;
    }
    let inner = package_inner_binary(&dir)?;
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&inner, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("补执行位失败: {e}"))?;
    }
    Ok(inner)
}

/// 工具链回退资产名（D46 切 gnu）：新 gnu 二进制对旧源（stable 段与历史 release 仅剩
/// msvc 资产的窗口期）在主名未命中后试 msvc 名。仅 Windows 有此层。反向无回退：旧 msvc
/// 二进制对新 gnu 源全 miss（其读序无 gnu 名），升级走 omc catalog 通道重装。
fn asset_msvc_fallback() -> Option<String> {
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        Some("ark-x86_64-pc-windows-msvc.exe".to_string())
    }
    #[cfg(not(all(windows, target_arch = "x86_64")))]
    {
        None
    }
}

/// 自升级主流程。
///
/// # Errors
/// 返回 Err（人读原因串）当：ARK_MIRROR=1 镜像优先，跳过官方 API 等（完整失败面见函数体错误构造）。
pub fn self_update(env_root: &Path, channel: Channel) -> Result<SelfUpdateOutcome, String> {
    match channel {
        Channel::Git => self_update_git(env_root),
        Channel::Dev => self_update_release(env_root, "tags/dev"),
        Channel::Stable => self_update_release(env_root, "latest"),
    }
}

/// 镜像优先开关（`ARK_MIRROR=1`，读回 `OME_MIRROR`）：self update 跳过官方 API 直取镜像边车锚。
/// 供断源验收（远端不可构造官方断网）与未来默认切自建过渡；锚语义不变（边车取不到即拒绝）。
fn mirror_first() -> bool {
    crate::platform::env_var_or("ARK_MIRROR", "OME_MIRROR").as_deref() == Some("1")
}

/// release 通道（dev 滚动 / latest 正式）：元数据 → digest 对比 → 下载校验 → 替换 → 刷 catalog。
fn self_update_release(env_root: &Path, endpoint: &str) -> Result<SelfUpdateOutcome, String> {
    let channel = if endpoint == "latest" {
        "stable"
    } else {
        "dev"
    };
    let asset_name = asset_for_this_platform()?;
    let asset_pkg = asset_package_for_this_platform()?;
    let asset_compat = asset_compat_for_this_platform().ok();
    let asset_msvc = asset_msvc_fallback();
    // 镜像段按通道分（oma 同型，段名与通道同名）。读序（REQ-0008 窗口扩包形层）：
    // ark/ 段包形主名先（双挂窗有包用包，存量迁移方向）、ark-<triple> gnu 裸件名次
    //（回退臂，dev 滚动源无包形落此层）、msvc 回退名再次（D46 窗口期）、ome-* 兼容名
    // 殿后（分发面已停写，仅历史残量可命中）。dev 通道禁止回落 stable；latest 段已退役。
    let mirror_ver = if channel == "stable" { "stable" } else { "dev" };
    let mut names = vec![asset_pkg.as_str(), asset_name.as_str()];
    if let Some(m) = asset_msvc.as_deref() {
        names.push(m);
    }
    if let Some(c) = asset_compat.as_deref() {
        names.push(c);
    }
    let official = if mirror_first() {
        Err("ARK_MIRROR=1 镜像优先，跳过官方 API".to_string())
    } else {
        official_asset_meta(endpoint, &names)
    };
    let (digest, dl_url, seg_used, asset_used) = match official {
        Ok((d, u, name)) => (d, u, String::new(), name),
        Err(api_err) => mirror_fallback_meta(
            env_root,
            mirror_ver,
            &asset_name,
            asset_msvc.as_deref(),
            asset_compat.as_deref(),
            &api_err,
        )?,
    };

    let exe = std::env::current_exe().map_err(|e| format!("定位自身 exe 失败: {e}"))?;
    let mine = sha256_file(&exe)?;
    let is_package = asset_used.ends_with(".zip") || asset_used.ends_with(".tar.gz");
    // 镜像段内回落（官方 URL 失败时 download 层再兜一次）：镜像路径已命中则沿用其段；
    // 官方路径按命中资产名前缀取段（纯函数 fallback_seg，三态单测覆盖）
    let seg_for_fallback = fallback_seg(&seg_used, &asset_used);
    // 裸件：digest 即二进制 sha，先比后下（零下载判 current）。
    if !is_package && mine == digest {
        eprintln!("[OK] 已是最新构建（sha256 一致）");
        return Ok(SelfUpdateOutcome {
            action: "current",
            channel,
            asset: asset_used,
            sha256: sha8(&digest),
            exe: platform::self_deploy_target().unwrap_or(exe),
            catalog_synced: sync_catalog_from_cloud(env_root),
        });
    }
    if !is_package {
        eprintln!("[INFO] 本地 {mine} 与远端 {digest} 不同，下载更新");
    }
    let downloaded = crate::download::download_asset_with_mirror(
        env_root,
        &asset_used,
        &dl_url,
        Some(&digest),
        false,
        seg_for_fallback,
        mirror_ver,
    )?;
    // 包形：digest 是归档 sha（不与本地 exe 直比），归档经锚校验后解包取内层二进制，
    // 判新等值在解包后的二进制间进行——判新机制不动（REQ-0008 设计决策，解包取真身）。
    let candidate = if is_package {
        let inner = unpack_candidate(&downloaded)?;
        if sha256_file(&inner)? == mine {
            eprintln!("[OK] 已是最新构建（包内二进制 sha256 一致）");
            return Ok(SelfUpdateOutcome {
                action: "current",
                channel,
                asset: asset_used,
                sha256: sha8(&digest),
                exe: platform::self_deploy_target().unwrap_or(exe),
                catalog_synced: sync_catalog_from_cloud(env_root),
            });
        }
        inner
    } else {
        downloaded
    };
    let exe = replace_deployed_and_current(&candidate)?;
    // 解包目录收尾清理（G3：成功路径不留残，错误路径交下次进入或系统清理）
    let _ = std::fs::remove_dir_all(
        std::env::temp_dir().join(format!("ark-selfupdate-unpack-{}", std::process::id())),
    );
    // D41 C：升级后顺手搬旧元数据（幂等；失败只告警不拦升级收尾）
    if let Err(e) = platform::migrate_legacy_metadata() {
        eprintln!("[WARN] 元数据搬迁失败（旧位读回继续）: {e}");
    }
    // D41：POSIX profile 旧 ome env 块收口（Windows no-op）
    platform::migrate_legacy_env_block_once();
    let catalog_synced = sync_catalog_from_cloud(env_root);
    Ok(SelfUpdateOutcome {
        action: "updated",
        channel,
        asset: asset_used,
        sha256: sha8(&digest),
        exe,
        catalog_synced,
    })
}

/// 镜像段内回落的段名裁定（纯函数）：镜像路径已命中则沿用其段；官方路径按命中资产名
/// 前缀取段（ome-* 配 ome/ 段、ark-* 配 ark/ 段，D46 的 msvc 回退名属 ark- 族走主段）
/// ——过渡窗 release 仅 ome-* 时官方命中兼容名，回落段必须跟着兼容族走，
/// 否则 ark/ome-* 拼出 404 断保供。
fn fallback_seg<'a>(seg_used: &'a str, asset_used: &str) -> &'a str {
    if !seg_used.is_empty() {
        return seg_used;
    }
    if asset_used.starts_with("ome-") {
        "ome"
    } else {
        "ark"
    }
}

/// 镜像段读序尝试表（纯函数，自测 3 三态矩阵的构造面）：ark/ 段配 gnu 主名先、
/// ark/ 段 msvc 回退名次（D46 窗口期）、ome/ 段配 ome-* 兼容名殿后；各层缺名即跳过。
/// 每项含段、资产、边车与下载 URL。
fn mirror_attempts(
    base: &str,
    channel: &str,
    primary: &str,
    msvc_fallback: Option<&str>,
    compat: Option<&str>,
) -> Vec<(String, String, String, String)> {
    let mut out = vec![];
    let mut push = |seg: &str, asset: &str| {
        out.push((
            seg.to_string(),
            asset.to_string(),
            format!("{base}/{seg}/{channel}/{asset}.sha256"),
            format!("{base}/{seg}/{channel}/{asset}"),
        ));
    };
    // REQ-0008 窗口：包形主名先（有包用包，存量迁移方向），裸件为回退臂（双挂窗保旧），
    // 其后 D46 msvc 与 ome 兼容层照旧。dev 滚动源无包形，包名 miss 即落裸件。
    if let Ok(pkg) = asset_package_for_this_platform() {
        push("ark", &pkg);
    }
    push("ark", primary);
    if let Some(f) = msvc_fallback {
        push("ark", f);
    }
    if let Some(c) = compat {
        push("ome", c);
    }
    out
}

/// 镜像段读序落锚（D41 B，D46 扩 msvc 回退层）：按尝试表取首个在位边车为锚，命中返回
/// （锚、下载 URL、段、资产名）；全败报双链错误（含官方 API 原因）。
fn mirror_fallback_meta(
    env_root: &Path,
    channel: &str,
    primary: &str,
    msvc_fallback: Option<&str>,
    compat: Option<&str>,
    api_err: &str,
) -> Result<(String, String, String, String), String> {
    let attempts = mirror_attempts(
        crate::download::MIRROR_BASE,
        channel,
        primary,
        msvc_fallback,
        compat,
    );
    eprintln!(
        "[WARN] 官方 API 失败（{api_err}），镜像段读序试边车：{}",
        attempts
            .iter()
            .map(|(seg, a, _, _)| format!("{seg}/{channel}/{a}"))
            .collect::<Vec<_>>()
            .join(" 先、")
    );
    let mut last = String::new();
    for (seg, asset, sidecar_url, dl) in attempts {
        match crate::download::mirror_sidecar_sha(env_root, &sidecar_url) {
            Ok(digest) => return Ok((digest, dl, seg, asset)),
            Err(e) => last = format!("{seg}/{asset}: {e}"),
        }
    }
    Err(format!("镜像段读序全败（官方: {api_err}; {last}）"))
}

/// 官方 release 资产元数据（digest 大写 + 下载直链 + 命中资产名）；资产名读序：包形主名
/// 先（REQ-0008 窗口，双挂窗有包用包）、gnu 裸件名次（回退臂，双挂窗保旧量与 dev 滚动源）、
/// msvc 回退名再次（D46 窗口期，stable 段与历史 release 仅剩 msvc 资产）、ome 兼容名殿后
///（历史 release 残量），全 miss 报首名错；API 段失败由调用方走镜像边车读序。
fn official_asset_meta(endpoint: &str, names: &[&str]) -> Result<(String, String, String), String> {
    // release JSON 单拉一次（G2：原每层各拉一遍，窗口期 stable 稳定 2 次 API GET），
    // 本地按名序匹配，全 miss 报首名错。
    let release = fetch_release(endpoint)?;
    let mut primary_err: Option<String> = None;
    for name in names {
        match asset_in_release(&release, name) {
            Ok(hit) => return Ok(hit),
            Err(e) => {
                primary_err.get_or_insert(e);
            }
        }
    }
    Err(primary_err.unwrap_or_else(|| {
        format!(
            "release 缺资产 {}（CI 是否已跑完？）",
            names.first().copied().unwrap_or("")
        )
    }))
}

/// 在已拉取的 release JSON 内按名找资产（digest 大写 + 下载直链 + 名）。
fn asset_in_release(release: &Value, asset_name: &str) -> Result<(String, String, String), String> {
    let assets = release
        .get("assets")
        .and_then(Value::as_array)
        .ok_or_else(|| "release 无资产列表".to_string())?;
    let asset = assets
        .iter()
        .find(|a| a.get("name").and_then(Value::as_str) == Some(asset_name))
        .ok_or_else(|| format!("release 缺资产 {asset_name}（CI 是否已跑完？）"))?;
    let digest = asset
        .get("digest")
        .and_then(Value::as_str)
        .and_then(|d| d.strip_prefix("sha256:"))
        .ok_or_else(|| format!("资产 {asset_name} 无 sha256 digest，拒绝无校验升级"))?
        .to_uppercase();
    let dl_url = asset
        .get("browser_download_url")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("资产 {asset_name} 无下载地址"))?
        .to_string();
    Ok((digest, dl_url, asset_name.to_string()))
}

/// git 通道：浅克隆仓库构建后替换（封版前无 release 的源码安装；需 git 与 cargo）。
fn self_update_git(env_root: &Path) -> Result<SelfUpdateOutcome, String> {
    let git = which::which("git").map_err(|_| "git 通道需要 git 在 PATH".to_string())?;
    let cargo = which::which("cargo").map_err(|_| {
        "git 通道需要 cargo 在 PATH（无 Rust 工具链时用 dev/stable 通道）".to_string()
    })?;

    let work = std::env::temp_dir().join(format!("ark-selfupdate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    let url = format!("https://github.com/{REPO}");
    eprintln!("[INFO] 浅克隆 {url}");
    let out = Command::new(&git)
        .args(["clone", "--depth", "1"])
        .arg(&url)
        .arg(&work)
        .output()
        .map_err(|e| format!("git clone 启动失败: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git clone 失败: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    eprintln!("[INFO] cargo build --release（源码构建耗时较长）");
    let out = Command::new(&cargo)
        .args(["build", "--release", "--locked"])
        .current_dir(&work)
        .output()
        .map_err(|e| format!("cargo build 启动失败: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo build 失败: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    // 产物名随 crate 名派生（D41 更名后随 CARGO_PKG_NAME 走，不再硬编码）
    #[cfg(windows)]
    let bin = work
        .join("target")
        .join("release")
        .join(format!("{}.exe", env!("CARGO_PKG_NAME")));
    #[cfg(not(windows))]
    let bin = work
        .join("target")
        .join("release")
        .join(env!("CARGO_PKG_NAME"));

    let exe = std::env::current_exe().map_err(|e| format!("定位自身 exe 失败: {e}"))?;
    let (mine, built) = (sha256_file(&exe)?, sha256_file(&bin)?);
    if mine == built {
        eprintln!("[OK] 已是最新构建（sha256 一致）");
        let catalog_synced = sync_catalog_from_cloud(env_root);
        let _ = std::fs::remove_dir_all(&work);
        return Ok(SelfUpdateOutcome {
            action: "current",
            channel: "git",
            asset: "source".to_string(),
            sha256: sha8(&built),
            exe,
            catalog_synced,
        });
    }
    let exe = replace_deployed_and_current(&bin)?;
    // D41 C：同 release 通道（幂等搬迁，失败只告警）
    if let Err(e) = platform::migrate_legacy_metadata() {
        eprintln!("[WARN] 元数据搬迁失败（旧位读回继续）: {e}");
    }
    // D41：POSIX profile 旧 ome env 块收口（Windows no-op）
    platform::migrate_legacy_env_block_once();
    let catalog_synced = sync_catalog_from_cloud(env_root);
    let _ = std::fs::remove_dir_all(&work);
    Ok(SelfUpdateOutcome {
        action: "updated",
        channel: "git",
        asset: "source".to_string(),
        sha256: sha8(&built),
        exe,
        catalog_synced,
    })
}

/// 先替换自部署目标（用户 PATH 上的 ark），若当前进程 exe 不同再替换运行中副本（cargo run）。
/// D41 C：随替换重建 `ome` 别名（部署位同目录同内容副本，best-effort 不拦升级）。
fn replace_deployed_and_current(new_file: &Path) -> Result<PathBuf, String> {
    let current = std::env::current_exe().map_err(|e| format!("定位自身 exe 失败: {e}"))?;
    let deploy = platform::self_deploy_target()?;
    if let Some(parent) = deploy.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建部署目录失败: {}: {e}", parent.display()))?;
    }
    replace_exe(&deploy, new_file)?;
    if !path_same(&current, &deploy) && current.exists() {
        let _ = replace_exe(&current, new_file);
    }
    // D41 C 收口（2026-09-14）：ome 别名停建，升级顺带清理既有副本（水位清零；
    // 当前正以别名运行时 Windows 删不动，warn 留待下次再收）
    if let Err(e) = platform::remove_ome_alias() {
        eprintln!("[WARN] ome 别名清理失败（不拦升级，下次再收）: {e}");
    }
    Ok(deploy)
}

fn path_same(a: &Path, b: &Path) -> bool {
    let na = a
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', "\\");
    let nb = b
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', "\\");
    na.eq_ignore_ascii_case(&nb)
}

/// 替换部署位 exe：Windows 改名旧的为 .old 再 copy 新的（运行中 exe 不可删；
/// 目标尚无前代即首次落位，直接写新件——v1.0.0 验收实证：新装机 self update 无前代可改名即败）；
/// Unix copy 到同目录临时文件后 chmod 755 再 rename 原子覆盖。
fn replace_exe(exe: &Path, new_file: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        if !exe.exists() {
            // 首次落位（部署位无前代，如新装机直接 self update）：无旧可改名，直接写
            if let Some(parent) = exe.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("建部署目录失败: {}: {e}", parent.display()))?;
            }
            std::fs::copy(new_file, exe).map_err(|e| format!("写入新 exe 失败: {e}"))?;
            return Ok(());
        }
        let old = exe.with_extension("exe.old");
        let _ = std::fs::remove_file(&old); // 上次升级残留（进程已退出才删得掉）
        std::fs::rename(exe, &old).map_err(|e| format!("改名旧 exe 失败: {e}"))?;
        if let Err(e) = std::fs::copy(new_file, exe) {
            // 回滚：把旧名改回来，不留半损状态
            let _ = std::fs::rename(&old, exe);
            return Err(format!("写入新 exe 失败（已回滚）: {e}"));
        }
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let tmp = exe.with_extension("ark-new");
        std::fs::copy(new_file, &tmp).map_err(|e| format!("写临时文件失败: {e}"))?;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod 755 失败: {e}"))?;
        std::fs::rename(&tmp, exe).map_err(|e| format!("替换 exe 失败: {e}"))?;
    }
    Ok(())
}

/// 刷新数据目录 catalog（D37 终态：一律云端权威，走边车锚加解析加验签三重门）。
/// best-effort：失败只提示，用户可稍后 `ark catalog sync`；不再从已退役的仓库件取源。
fn sync_catalog_from_cloud(env_root: &Path) -> bool {
    let target = crate::catalog::user_data_catalog_path();
    match crate::catalog::sync_to(env_root, &target, true, crate::catalog::auto_ttl()) {
        Ok(outcome) => {
            eprintln!(
                "[OK] catalog 已同步（云端验签）: {} [{}]",
                target.display(),
                outcome.action()
            );
            true
        }
        Err(e) => {
            eprintln!("[WARN] catalog 云端刷新失败（不影响升级，可稍后 `ark catalog sync`）: {e}");
            false
        }
    }
}

/// 取 release 元数据：直连 api.github.com（带 GH_TOKEN 注入），403/限流回退 gh api。
fn fetch_release(endpoint: &str) -> Result<Value, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/{endpoint}");
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout(Duration::from_secs(30))
        .build();
    let mut req = agent
        .get(&url)
        .set("User-Agent", UA)
        .set("Accept", "application/vnd.github+json");
    if let Ok(tok) = std::env::var("GH_TOKEN") {
        req = req.set("Authorization", &format!("Bearer {tok}"));
    }
    match req.call() {
        Ok(resp) => {
            let body = resp
                .into_string()
                .map_err(|e| format!("读取响应体失败: {e}"))?;
            serde_json::from_str(&body).map_err(|e| format!("JSON 解析失败: {e}"))
        }
        Err(e) => {
            let msg = format!("{e}");
            if msg.contains("403") || msg.to_lowercase().contains("rate limit") {
                eprintln!("[INFO] api.github.com 直连受限，改用 gh api（认证通道）");
                gh_api(&url)
            } else if msg.contains("404") {
                Err(format!(
                    "尚无对应 release（未封版无正式版；dev 通道需先有 main push 的 CI）: {endpoint}"
                ))
            } else {
                Err(format!("查询 release 失败: {msg}"))
            }
        }
    }
}

/// gh api 兜底（认证通道）：与 resolve.rs 同思路，selfupdate 独立持有。
fn gh_api(url: &str) -> Result<Value, String> {
    let path = url
        .strip_prefix("https://api.github.com")
        .ok_or_else(|| format!("非 api.github.com 地址: {url}"))?;
    let gh = which::which("gh").map_err(|_| "gh 不可用，无法回退 gh api".to_string())?;
    let out = Command::new(gh)
        .arg("api")
        .arg(path)
        .output()
        .map_err(|e| format!("gh api 执行失败: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "gh api 失败: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("gh api 输出解析失败: {e}"))
}

/// 摘要展示用短 sha（前 8 位）。
fn sha8(s: &str) -> String {
    s.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn 替换exe_首次落位无前代直接写() {
        // v1.0.0 验收实证回归：新装机 self update（部署位无前代）不得因 rename 缺文件而败
        let dir = tempfile::tempdir().expect("临时目录");
        let src = dir.path().join("new.exe");
        std::fs::write(&src, b"fresh").expect("写新件");
        let dst = dir.path().join("deploy").join("ark.exe");
        replace_exe(&dst, &src).expect("首次落位应直接写成功");
        assert_eq!(std::fs::read(&dst).expect("读部署位"), b"fresh");
        // 二次替换：有前代走改名链，内容更新；.old 残留属设计（下次升级起手清）
        std::fs::write(&src, b"v2").expect("写二代");
        replace_exe(&dst, &src).expect("二次替换应成功");
        assert_eq!(std::fs::read(&dst).expect("读部署位"), b"v2");
        assert!(
            dst.with_extension("exe.old").exists(),
            "旧件改名保留（下次升级起手清）"
        );
        // 三次替换：起手清上次 .old 残留
        std::fs::write(&src, b"v3").expect("写三代");
        replace_exe(&dst, &src).expect("三次替换应成功");
        assert_eq!(std::fs::read(&dst).expect("读部署位"), b"v3");
    }

    #[test]
    fn 资产名_当前平台必有映射() {
        // 本 CI 覆盖的三目标之一，或明确报不支持；D41 B 起主名 ark-、兼容名 ome-
        match (asset_for_this_platform(), asset_compat_for_this_platform()) {
            (Ok(primary), Ok(compat)) => {
                assert!(primary.starts_with("ark-"), "主名应带 ark- 前缀: {primary}");
                assert!(compat.starts_with("ome-"), "兼容名应带 ome- 前缀: {compat}");
                assert_eq!(
                    primary.trim_start_matches("ark-"),
                    compat.trim_start_matches("ome-"),
                    "主名与兼容名共用三元组"
                );
            }
            (Err(e), _) => assert!(e.contains("无 CI 构建资产")),
            _ => panic!("主名可解析则兼容名必可解析"),
        }
    }

    #[test]
    fn 段内回落裁定_三态() {
        // 镜像命中沿用其段；官方命中按资产族；默认主段
        assert_eq!(
            fallback_seg("ome", "ome-x.exe"),
            "ome",
            "镜像命中的段直接沿用"
        );
        assert_eq!(
            fallback_seg("ark", "ark-x.exe"),
            "ark",
            "镜像命中的段直接沿用"
        );
        assert_eq!(
            fallback_seg("", "ome-x.exe"),
            "ome",
            "官方命中兼容名回落兼容段"
        );
        assert_eq!(fallback_seg("", "ark-x.exe"), "ark", "官方命中主名回落主段");
        // D46：msvc 回退名属 ark- 族，回落主段（gnu 主名 miss 后窗口期资产仍按 ark/ 段取）
        assert_eq!(
            fallback_seg("", "ark-x86_64-pc-windows-msvc.exe"),
            "ark",
            "官方命中 msvc 回退名回落主段"
        );
    }

    #[test]
    fn 镜像段读序_尝试表四层构造() {
        // D41 自测 3（构造面）：ark 主先、ome 兼容回落、URL 形态、缺名层跳过；
        // D46 补 msvc 回退层：gnu 主名先、msvc 回退次、ome 兼容殿后
        let base = "https://mirror.example";
        let four = mirror_attempts(
            base,
            "dev",
            "ark-x86_64-pc-windows-gnu.exe",
            Some("ark-x86_64-pc-windows-msvc.exe"),
            Some("ome-x86_64-pc-windows-gnu.exe"),
        );
        // REQ-0008 窗口扩包形层：包形主名先、gnu 裸件次、msvc 回退再次、ome 段殿后
        assert_eq!(four.len(), 4, "四层读序四尝试");
        let pkg = asset_package_for_this_platform().expect("测试平台有包名");
        assert_eq!(four[0].1, pkg, "包形主名先（有包用包）");
        assert_eq!(four[0].0, "ark", "包形走 ark 主段");
        assert_eq!(
            four[1].1, "ark-x86_64-pc-windows-gnu.exe",
            "gnu 裸件回退臂次"
        );
        assert_eq!(four[2].1, "ark-x86_64-pc-windows-msvc.exe", "msvc 回退再次");
        assert_eq!(four[2].0, "ark", "msvc 回退名走主段");
        assert_eq!(four[3].0, "ome", "ome 段殿后");
        assert_eq!(
            four[1].2,
            format!("{base}/ark/dev/ark-x86_64-pc-windows-gnu.exe.sha256"),
            "边车 URL 形态"
        );
        assert_eq!(
            four[3].3,
            format!("{base}/ome/dev/ome-x86_64-pc-windows-gnu.exe"),
            "下载 URL 形态"
        );
        let three_now = mirror_attempts(base, "dev", "ark-x.exe", None, Some("ome-x.exe"));
        assert_eq!(three_now.len(), 3, "无 msvc 回退名跳过该层（包加裸加兼容）");
        let two_now = mirror_attempts(base, "stable", "ark-x.exe", None, None);
        assert_eq!(two_now.len(), 2, "无回退名仅包加裸两层");
        assert!(
            two_now[0].2.contains("/ark/stable/"),
            "stable 通道段名随通道"
        );
    }

    #[test]
    fn 资产包名_平台形() {
        // 对齐 资产名_当前平台必有映射 的 Err 分支形：非三目标宿主显式报不支持后跳过
        let (Ok(pkg), Ok(t)) = (asset_package_for_this_platform(), package_triple()) else {
            assert!(platform_triple().is_err(), "无包名须因平台无 CI 覆盖");
            return;
        };
        if t.contains("windows") {
            assert_eq!(
                pkg,
                format!("ark-{t}.zip"),
                "win 形 zip（净 triple 无 .exe 尾）"
            );
            assert!(!pkg.ends_with(".exe.zip"), "不出现 exe.zip 疵名");
        } else {
            assert_eq!(pkg, format!("ark-{t}.tar.gz"), "他形 tar.gz");
        }
    }

    #[test]
    fn 包内定位_拼名契约() {
        let t = package_triple().expect("测试平台有净三元组");
        let name = if t.contains("windows") {
            "ark.exe"
        } else {
            "ark"
        };
        let tmp = std::env::temp_dir().join("ark-test-pkg-inner");
        let _ = std::fs::remove_dir_all(&tmp);
        let inner_dir = tmp.join(format!("ark-{t}"));
        std::fs::create_dir_all(&inner_dir).unwrap();
        std::fs::write(inner_dir.join(name), b"bin").unwrap();
        let hit = package_inner_binary(&tmp).expect("拼名命中");
        assert_eq!(hit, inner_dir.join(name));
        // 空目录报拼名契约错
        let empty = std::env::temp_dir().join(format!("ark-test-pkg-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&empty);
        std::fs::create_dir_all(&empty).unwrap();
        assert!(package_inner_binary(&empty)
            .unwrap_err()
            .contains("拼名契约 ark-<target>/ark"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn 解包候选_归档往返() {
        let t = package_triple().expect("测试平台有净三元组");
        let name = if t.contains("windows") {
            "ark.exe"
        } else {
            "ark"
        };
        let payload = b"#!/bin/sh\necho ark-test\n";
        let stage = std::env::temp_dir().join("ark-test-pkg-archives");
        let _ = std::fs::remove_dir_all(&stage);
        let src_dir = stage.join("src").join(format!("ark-{t}"));
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join(name), payload).unwrap();

        // tar.gz 形
        let tgz = stage.join("pkg.tar.gz");
        let f = std::fs::File::create(&tgz).unwrap();
        let enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
        let mut tb = tar::Builder::new(enc);
        tb.append_dir_all(
            format!("ark-{t}"),
            stage.join("src").join(format!("ark-{t}")),
        )
        .unwrap();
        tb.into_inner().unwrap().finish().unwrap();
        let inner = unpack_candidate(&tgz).expect("tar.gz 解包取内层");
        assert_eq!(std::fs::read(&inner).unwrap(), payload, "tar.gz 内层逐字等");

        // zip 形（win 资产形；全平台跑写读往返）
        let zp = stage.join("pkg.zip");
        let zf = std::fs::File::create(&zp).unwrap();
        use std::io::Write as _;
        let mut zw = zip::ZipWriter::new(zf);
        let opts: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
        zw.start_file(format!("ark-{t}/{name}"), opts).unwrap();
        zw.write_all(payload).unwrap();
        zw.finish().unwrap();
        let inner2 = unpack_candidate(&zp).expect("zip 解包取内层");
        assert_eq!(std::fs::read(&inner2).unwrap(), payload, "zip 内层逐字等");
        let _ = std::fs::remove_dir_all(&stage);
        let _ = std::fs::remove_dir_all(
            std::env::temp_dir().join(format!("ark-selfupdate-unpack-{}", std::process::id())),
        );
    }

    #[test]
    fn 自管条目_新旧extract双接受() {
        // D41（R8）：数据面改名可单方回退——引擎双接受，六消费分支同判定
        let mk = |e: &str| crate::catalog::Tool {
            extract: Some(e.to_string()),
            ..Default::default()
        };
        assert!(
            is_ome_self(&mk("ome-self")),
            "旧标记仍受认（数据面未改名期）"
        );
        assert!(is_ome_self(&mk("ark-self")), "新标记受认");
        assert!(!is_ome_self(&mk("zip")), "非自管不误判");
        assert!(!is_ome_self(&mk("npm-tgz")), "npm 型不误判");
    }

    #[cfg(not(all(windows, target_arch = "x86_64")))]
    #[test]
    fn msvc回退层_非windows无此层() {
        // G5 对线补：msvc 回退层仅 Windows 有（linux/mac 走单层主名读序）
        assert!(
            asset_msvc_fallback().is_none(),
            "非 Windows 不应有 msvc 回退层"
        );
    }

    #[test]
    fn 短sha_取前8位() {
        assert_eq!(sha8("ABCDEF1234"), "ABCDEF12");
    }
}
