//! selfupdate：ark 自身升级（`ark self update`），三通道：
//! - **dev**（默认）：pre-release tag `dev` 的滚动资产——CI push main 构建上传，本地测试期升级源；
//! - **stable**：`releases/latest` 正式版——CI 推 v* tag（封版）触发；
//! - **git**：源码安装——浅克隆仓库 cargo build 后替换（封版前无任何 release 时的通道，需 git 与 cargo）。
//!
//! D51 镜像默认通道：release 元数据镜像段读序先行（边车即锚），GitHub API 兜底；
//! `ARK_MIRROR=0` 官方优先逃逸阀反转读序（旧 =1 强制镜像已转正为默认）。
//! 升级判定：release 资产的 digest（sha256）与运行中 exe 的 sha256 对比，一致即已最新；
//! 不同则经 download_asset 下载到缓存（digest 校验）后替换部署位，并同步数据目录 catalog。
//! 包形资产（REQ-0008 窗口，ark-<target>.zip/.tar.gz）digest 为归档 sha：归档经锚校验下载后
//! 解包取内层二进制（拼名契约 ark-<target>/ark），判新等值在解包后的二进制间进行（机制不动）。
//! Windows 运行中 exe 可改名不可删：替换全程用 rename（备份 ark-old-<pid>、暂存 ark-new-<pid>），
//! 成功清备份、删不动留待启动收割。
//! REQ-0012 家族自更新统一标准：stable 通道官方 API 判新加 semver 门（localNewer 不动）加镜像
//! stable 段下载优先（镜像腿任一步失败整对回落官方、官方 digest 终腿硬校验；dev 腿镜像边车
//! 同源锚不符硬拒不回落）；自替换带 pid 锁、陈旧收割与 --version 自证回滚。

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

/// 是否自管条目（extract = "ark-self"）：无 pin 无资产，升级走 self update 三通道。
/// （旧值 ome-self 双接受已随 ome 关键字剔除批收口，2026-09-18；云端权威 catalog 自
/// 2026-09-14 起写 ark-self，旧值仅存在于长期未同步的端上副本，`ark catalog sync` 即迁。）
pub fn is_ark_self(def: &crate::catalog::Tool) -> bool {
    def.extract() == Some("ark-self")
}

/// 升级结果。
pub struct SelfUpdateOutcome {
    /// updated：已替换；current：已是最新；localNewer：本地领先远端不动（stable semver 门，REQ-0012）
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
/// 返回 Err（人读原因串）当：当前平台无 CI 构建资产（platform_triple 同源判）。
pub fn asset_for_this_platform() -> Result<String, String> {
    Ok(format!("ark-{}", platform_triple()?))
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
/// 返回 Err（人读原因串）当：通道缺件（git/cargo）、镜像与官方双链元数据全败、下载校验
/// 或替换失败等（完整失败面见函数体错误构造）。
pub fn self_update(env_root: &Path, channel: Channel) -> Result<SelfUpdateOutcome, String> {
    match channel {
        Channel::Git => self_update_git(env_root),
        Channel::Dev => self_update_release(env_root, "tags/dev"),
        Channel::Stable => self_update_release(env_root, "latest"),
    }
}

/// 元数据选源结果（D51）：digest 边车/API 锚、下载 URL、资产名、是否镜像命中（下载腿
/// 失败时据此补官方链）、官方 API 本轮是否已失败（逃逸路径不再重试 API）。
struct MetaPick {
    digest: String,
    dl_url: String,
    asset: String,
    from_mirror: bool,
    api_err: Option<String>,
}

/// release 通道分派：stable 走家族形（REQ-0012），dev 走 D51 镜像元数据优先。
fn self_update_release(env_root: &Path, endpoint: &str) -> Result<SelfUpdateOutcome, String> {
    if endpoint == "latest" {
        self_update_stable(env_root)
    } else {
        self_update_dev(env_root, endpoint)
    }
}

/// stable 通道（家族自更新统一标准形，REQ-0012 差异表 #1/#2/#3）：官方 latest API
/// 一次判新（tag 加 digest，镜像段资产名不带版本无法反查，browse 同构裁定）→
/// semver 门（相等 current、本地领先 localNewer 不动、远端新才升级）→ 下载腿镜像
/// stable 段优先（官方 digest 锚，CF 击穿 query）；镜像腿任一步失败（未命中或锚不符，
/// 含播种滞后常态）整对回落官方，官方终腿 digest 硬校验拒坏件（对线 F1 裁定）。
fn self_update_stable(env_root: &Path) -> Result<SelfUpdateOutcome, String> {
    let asset_name = asset_for_this_platform()?;
    let asset_pkg = asset_package_for_this_platform()?;
    let asset_msvc = asset_msvc_fallback();
    let mut names = vec![asset_pkg.as_str(), asset_name.as_str()];
    if let Some(m) = asset_msvc.as_deref() {
        names.push(m);
    }
    let (digest, official_url, asset, tag) = official_release_meta_with_tag("latest", &names)?;
    let remote_ver = strip_v(&tag);
    let exe = std::env::current_exe().map_err(|e| format!("定位自身 exe 失败: {e}"))?;
    let deploy = platform::self_deploy_target().unwrap_or(exe.clone());
    match semver_gate(remote_ver, env!("CARGO_PKG_VERSION")) {
        Verdict::Current => {
            eprintln!("[OK] 已是最新版（{remote_ver}）");
            return Ok(SelfUpdateOutcome {
                action: "current",
                channel: "stable",
                asset,
                sha256: sha8(&digest),
                exe: deploy,
                catalog_synced: sync_catalog_from_cloud(env_root),
            });
        }
        Verdict::LocalNewer => {
            eprintln!(
                "[INFO] 本地 {} 领先远端 {remote_ver}（localNewer），不动",
                env!("CARGO_PKG_VERSION")
            );
            return Ok(SelfUpdateOutcome {
                action: "localNewer",
                channel: "stable",
                asset,
                sha256: sha8(&digest),
                exe: deploy,
                catalog_synced: sync_catalog_from_cloud(env_root),
            });
        }
        Verdict::Upgrade => eprintln!(
            "[INFO] 远端 {remote_ver} 新于本地 {}，升级",
            env!("CARGO_PKG_VERSION")
        ),
    }
    // 下载腿（对线 F1/F2 修正）：ark 资产名不带版本，stable 段同名滚动，「镜像字节对官方
    // digest」不符含播种滞后常态（对源 browse 资产名带版本、同名 404 即整对回落），故镜像腿
    // 不符一律回落官方、由官方 digest 终腿硬校验拦坏件（官方件不符即终败，安全性等价）；
    // dev 腿的镜像边车锚才是同源对硬拒面（见 self_update_dev）。CF 击穿 query 缓解同名陈旧。
    // 逃逸阀 ARK_MIRROR=0：官方先行，镜像腿跳过（对齐 ADR-0008 开关语义）。
    let downloaded = if crate::download::mirror_off() {
        crate::download::download_asset(env_root, &asset, &official_url, Some(&digest), true)
            .map_err(|official_err| {
                format!("官方下载失败（ARK_MIRROR=0 官方优先）: {official_err}")
            })?
    } else {
        let mirror_dl = crate::download::with_query(
            &format!("{}/ark/stable/{asset}", crate::download::MIRROR_BASE),
            &format!("v={digest}"),
        );
        match crate::download::download_asset(env_root, &asset, &mirror_dl, Some(&digest), true) {
            Ok(p) => p,
            Err(mirror_err) => {
                if is_hash_mismatch(&mirror_err) {
                    // 对线 G-c：锚不符（可能篡改或滞后）升 WARN 保留报警反射；回落行为不变
                    eprintln!("[WARN] 镜像 stable 段资产与官方锚不符（篡改或滞后），回落官方（终腿官方锚校验）: {mirror_err}");
                } else {
                    eprintln!("[INFO] 镜像 stable 段未命中（{mirror_err}），整对回落官方");
                }
                crate::download::download_asset(
                    env_root,
                    &asset,
                    &official_url,
                    Some(&digest),
                    true,
                )
                .map_err(|official_err| {
                    format!("镜像与官方双链失败\n镜像段: {mirror_err}\n官方: {official_err}")
                })?
            }
        }
    };
    let mine = sha256_file(&exe)?;
    let candidate = package_candidate(&downloaded, &asset, &digest, &mine)?;
    let expect: Option<String> = Some(remote_ver.to_string());
    finish_update(
        env_root,
        "stable",
        candidate,
        digest,
        asset,
        expect.as_deref(),
    )
}

/// dev 通道（D51 镜像元数据优先 + REQ-0012 #2/#4 补齐）：镜像段读序（边车锚）先行、
/// 官方 API 兜底；镜像下载腿 digest 锚硬拒（不符 Err），网络性失败补官方链（对线 F2）；
/// 无版本语义（滚动 digest 锚，semver 门不适用），判新 digest 等值。
fn self_update_dev(env_root: &Path, endpoint: &str) -> Result<SelfUpdateOutcome, String> {
    let asset_name = asset_for_this_platform()?;
    let asset_pkg = asset_package_for_this_platform()?;
    let asset_msvc = asset_msvc_fallback();
    // 镜像段按通道分（oma 同型，段名与通道同名）。读序（REQ-0008 窗口扩包形层）：
    // ark/ 段包形主名先（双挂窗有包用包，存量迁移方向）、ark-<triple> gnu 裸件名次
    //（回退臂，dev 滚动源无包形落此层）、msvc 回退名再次（D46 窗口期）。ome-* 兼容
    // 层已收口（ome/ 段镜像删桶边车 404、stable 已全 ark-* 名，2026-09-18 剔除批）。
    // dev 通道禁止回落 stable；latest 段已退役。
    let mut names = vec![asset_pkg.as_str(), asset_name.as_str()];
    if let Some(m) = asset_msvc.as_deref() {
        names.push(m);
    }
    // D51 元数据双链选源（默认镜像段读序先行、未命中回落官方 API；逃逸阀 ARK_MIRROR=0 反转）：
    // api_err 记本轮官方 API 是否已失败（逃逸路径镜像回落命中时在位，二段回落不再重试 API，
    // 对线二轮 G-a2）；from_mirror 标下载腿失败时是否须补官方链（对线 F2）。
    let pick = if crate::download::mirror_off() {
        match official_asset_meta(endpoint, &names) {
            Ok((digest, dl_url, asset)) => MetaPick {
                digest,
                dl_url,
                asset,
                from_mirror: false,
                api_err: None,
            },
            Err(api_err) => {
                let (digest, dl_url, asset) = mirror_fallback_meta(
                    env_root,
                    "dev",
                    &asset_name,
                    asset_msvc.as_deref(),
                    &api_err,
                )?;
                MetaPick {
                    digest,
                    dl_url,
                    asset,
                    from_mirror: true,
                    api_err: Some(api_err),
                }
            }
        }
    } else {
        match mirror_meta(env_root, "dev", &asset_name, asset_msvc.as_deref()) {
            Ok((digest, dl_url, asset)) => MetaPick {
                digest,
                dl_url,
                asset,
                from_mirror: true,
                api_err: None,
            },
            Err(mirror_err) => {
                eprintln!("[WARN] 镜像段未命中（{mirror_err}），回落官方 API");
                match official_asset_meta(endpoint, &names) {
                    Ok((digest, dl_url, asset)) => MetaPick {
                        digest,
                        dl_url,
                        asset,
                        from_mirror: false,
                        api_err: None,
                    },
                    Err(api_err) => {
                        return Err(format!(
                            "镜像与官方双链失败\n镜像段: {mirror_err}\n官方: {api_err}"
                        ))
                    }
                }
            }
        }
    };

    let exe = std::env::current_exe().map_err(|e| format!("定位自身 exe 失败: {e}"))?;
    let mine = sha256_file(&exe)?;
    let pick_is_package = pick.asset.ends_with(".zip") || pick.asset.ends_with(".tar.gz");
    // 裸件：digest 即二进制 sha，先比后下（零下载判 current）。
    if !pick_is_package && mine == pick.digest {
        eprintln!("[OK] 已是最新构建（sha256 一致）");
        return Ok(SelfUpdateOutcome {
            action: "current",
            channel: "dev",
            asset: pick.asset,
            sha256: sha8(&pick.digest),
            exe: platform::self_deploy_target().unwrap_or(exe),
            catalog_synced: sync_catalog_from_cloud(env_root),
        });
    }
    if !pick_is_package {
        eprintln!("[INFO] 本地 {mine} 与远端 {} 不同，下载更新", pick.digest);
    }
    // 下载腿（REQ-0012 #2 硬拒形）：镜像命中腿直下镜像 URL（CF 击穿 query）加 digest 锚，
    // sha 不符硬拒 Err；网络性失败补官方链（F2），官方 API 本轮已失败不再重试（G-a2）。
    // 官方元数据命中腿沿用 download_asset_with_mirror（官方 URL 为主、镜像首试为辅，终态同锚硬校验）。
    let mut installed_digest = pick.digest.clone();
    let mut installed_asset = pick.asset.clone();
    let downloaded = if pick.from_mirror {
        let busted = crate::download::with_query(&pick.dl_url, &format!("v={}", pick.digest));
        match crate::download::download_asset(
            env_root,
            &pick.asset,
            &busted,
            Some(&pick.digest),
            true,
        ) {
            Ok(p) => p,
            Err(e) if is_hash_mismatch(&e) => {
                return Err(format!(
                    "镜像 dev 段资产与边车锚不符，硬拒不回落（家族标准，REQ-0012）:\n{e}"
                ))
            }
            Err(mirror_dl_err) => {
                if let Some(prev) = &pick.api_err {
                    return Err(format!(
                        "镜像与官方双链失败\n镜像段: {mirror_dl_err}\n官方: {prev}"
                    ));
                }
                eprintln!("[WARN] 镜像段下载失败（{mirror_dl_err}），回落官方 API 补官方链");
                match official_asset_meta(endpoint, &names) {
                    Ok((digest2, official_url, asset2)) => {
                        let p = crate::download::download_asset(
                            env_root,
                            &asset2,
                            &official_url,
                            Some(&digest2),
                            true,
                        )
                        .map_err(|official_err| {
                            format!(
                                "镜像与官方双链失败\n镜像段: {mirror_dl_err}\n官方: {official_err}"
                            )
                        })?;
                        installed_digest = digest2;
                        installed_asset = asset2;
                        p
                    }
                    Err(api_err) => {
                        return Err(format!(
                            "镜像与官方双链失败\n镜像段: {mirror_dl_err}\n官方: {api_err}"
                        ))
                    }
                }
            }
        }
    } else {
        crate::download::download_asset_with_mirror(
            env_root,
            &pick.asset,
            &pick.dl_url,
            Some(&pick.digest),
            false,
            "ark",
            "dev",
        )?
    };
    let candidate = package_candidate(&downloaded, &installed_asset, &installed_digest, &mine)?;
    finish_update(
        env_root,
        "dev",
        candidate,
        installed_digest,
        installed_asset,
        None,
    )
}

/// 包形候选解包与等值判新（两通道共用；包形 digest 是归档 sha，判新等值在解包后的
/// 二进制间进行，REQ-0008 机制不动）。返回 Some(候选件) 继续替换；None 表示包内二进制
/// 与本地等值（current，收尾由 `finish_update` 的 None 分支办）。
fn package_candidate(
    downloaded: &Path,
    asset: &str,
    digest: &str,
    mine: &str,
) -> Result<Option<PathBuf>, String> {
    let _ = digest;
    let is_package = asset.ends_with(".zip") || asset.ends_with(".tar.gz");
    if !is_package {
        return Ok(Some(downloaded.to_path_buf()));
    }
    let inner = unpack_candidate(downloaded)?;
    if sha256_file(&inner)? == mine {
        eprintln!("[OK] 已是最新构建（包内二进制 sha256 一致）");
        return Ok(None);
    }
    Ok(Some(inner))
}

/// 升级收尾（两通道共用）：替换自证（stable 断言远端版本、dev 断言可执行）、旧元数据
/// 搬迁、catalog 刷新。
#[allow(clippy::too_many_arguments)]
fn finish_update(
    env_root: &Path,
    channel: &'static str,
    candidate: Option<PathBuf>,
    digest: String,
    asset: String,
    expect_version: Option<&str>,
) -> Result<SelfUpdateOutcome, String> {
    let Some(candidate) = candidate else {
        // 包内二进制与本地等值：current（catalog 已在上层判新后未刷，这里补刷）
        return Ok(SelfUpdateOutcome {
            action: "current",
            channel,
            asset,
            sha256: sha8(&digest),
            exe: platform::self_deploy_target()
                .unwrap_or_else(|_| std::env::current_exe().unwrap_or_default()),
            catalog_synced: sync_catalog_from_cloud(env_root),
        });
    };
    let exe = replace_deployed_and_current(&candidate, expect_version)?;
    // 总台核收观察 a（2026-09-19）：自更新成功后刷新部署目录落痕内容为现版
    //（stable 有远端版本可写；dev 滚动源无版本语义不刷，留待下次装面幂等刷）
    if let Some(ver) = expect_version {
        if let Some(dir) = exe.parent() {
            if let Err(e) = crate::install::mark_ark_managed_with(dir, ver) {
                eprintln!("[WARN] {e}（不拦升级）");
            }
        }
    }
    // 解包目录收尾清理（G3：成功路径不留残，错误路径交下次进入或系统清理）
    let _ = std::fs::remove_dir_all(
        std::env::temp_dir().join(format!("ark-selfupdate-unpack-{}", std::process::id())),
    );
    // D41 C：升级后顺手搬旧元数据（幂等；失败只告警不拦升级收尾）
    if let Err(e) = platform::migrate_legacy_metadata() {
        eprintln!("[WARN] 元数据搬迁失败（旧位读回继续）: {e}");
    }
    // D41：POSIX profile 旧版 env 块收口（Windows no-op）
    platform::migrate_legacy_env_block_once();
    let catalog_synced = sync_catalog_from_cloud(env_root);
    Ok(SelfUpdateOutcome {
        action: "updated",
        channel,
        asset,
        sha256: sha8(&digest),
        exe,
        catalog_synced,
    })
}

/// 镜像段读序尝试表（纯函数，自测 3 三态矩阵的构造面）：包形主名先（REQ-0008 双挂窗
/// 有包用包）、gnu 裸件次（回退臂，dev 滚动源落此层）、msvc 回退名再次（D46 窗口期）；
/// 各层缺名即跳过，段恒 ark/（ome/ 兼容段已收口）。
fn mirror_attempts(
    base: &str,
    channel: &str,
    primary: &str,
    msvc_fallback: Option<&str>,
) -> Vec<(String, String, String, String)> {
    let mut out = vec![];
    let mut push = |asset: &str| {
        out.push((
            "ark".to_string(),
            asset.to_string(),
            format!("{base}/ark/{channel}/{asset}.sha256"),
            format!("{base}/ark/{channel}/{asset}"),
        ));
    };
    if let Ok(pkg) = asset_package_for_this_platform() {
        push(&pkg);
    }
    push(primary);
    if let Some(f) = msvc_fallback {
        push(f);
    }
    out
}

/// 镜像段读序核心（D51 起为默认通道）：按尝试表取首个在位边车为锚，命中返回
/// （锚、下载 URL、资产名）；全败返回末层错误串（调用方决定兜底方向与报错拼装）。
fn mirror_meta(
    env_root: &Path,
    channel: &str,
    primary: &str,
    msvc_fallback: Option<&str>,
) -> Result<(String, String, String), String> {
    let attempts = mirror_attempts(
        crate::download::MIRROR_BASE,
        channel,
        primary,
        msvc_fallback,
    );
    let mut last = String::new();
    for (_seg, asset, sidecar_url, dl) in attempts {
        match crate::download::mirror_sidecar_sha(env_root, &sidecar_url) {
            Ok(digest) => return Ok((digest, dl, asset)),
            Err(e) => last = format!("ark/{asset}: {e}"),
        }
    }
    Err(format!("镜像段读序全败: {last}"))
}

/// 官方 API 失败后的镜像回落（D51 逃逸阀路径用，D41 B/D46 读序不变）：报双链错误
/// （含官方 API 原因）。
fn mirror_fallback_meta(
    env_root: &Path,
    channel: &str,
    primary: &str,
    msvc_fallback: Option<&str>,
    api_err: &str,
) -> Result<(String, String, String), String> {
    eprintln!("[WARN] 官方 API 失败（{api_err}），镜像段读序试边车");
    mirror_meta(env_root, channel, primary, msvc_fallback)
        .map_err(|e| format!("镜像与官方双链失败\n官方: {api_err}\n镜像段: {e}"))
}

/// tag 去 v 前缀（stable 判新用）。
fn strip_v(tag: &str) -> &str {
    tag.trim().trim_start_matches('v').trim()
}

/// semver 门判定（家族标准：只升不降；REQ-0012 #3）。数值段比较复用 resolve 的
/// version_key（剥 v 加点分数值段）；解析失败的非常规 tag 保守按 Upgrade 放行
///（官方 latest 恒 v*，异常形装官方件自带 digest 校验无害）。
fn semver_gate(remote: &str, local: &str) -> Verdict {
    match (
        crate::resolve::version_key(remote),
        crate::resolve::version_key(local),
    ) {
        (Some(r), Some(l)) => match r.cmp(&l) {
            std::cmp::Ordering::Equal => Verdict::Current,
            std::cmp::Ordering::Less => Verdict::LocalNewer,
            std::cmp::Ordering::Greater => Verdict::Upgrade,
        },
        _ => Verdict::Upgrade,
    }
}

/// stable 判新三态：相等 current、本地领先 localNewer（不动）、远端新 Upgrade。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// 远端与本地同版：已最新
    Current,
    /// 本地领先远端：不动（semver 只升不降）
    LocalNewer,
    /// 远端更新：进下载腿
    Upgrade,
}

/// 下载错误判型：sha 锚不符（校验性失败，家族标准硬拒不回落）与网络性失败
///（连接错/未命中，整对回落官方）的分界（错误串判型，仓内惯例如 should_fallback_gh）。
fn is_hash_mismatch(err: &str) -> bool {
    // 对线 G3-1：判据引用 download 层常量（串改常量处单点同步，防文案漂移静默降级判型）
    err.contains(crate::download::HASH_MISMATCH_MARK)
}

/// 官方 release 元数据带 tag（stable 家族形判新用：tag 去 v 进 semver 门）。
fn official_release_meta_with_tag(
    endpoint: &str,
    names: &[&str],
) -> Result<(String, String, String, String), String> {
    let release = fetch_release(endpoint)?;
    let tag = release
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("release 缺少 tag_name: {endpoint}"))?
        .to_string();
    let (digest, url, asset) = asset_meta_in_release(&release, names)?;
    Ok((digest, url, asset, tag))
}

/// 官方 release 资产元数据（digest 大写 + 下载直链 + 命中资产名）；资产名读序：包形主名
/// 先（REQ-0008 窗口，双挂窗有包用包）、gnu 裸件名次（回退臂，双挂窗保旧量与 dev 滚动源）、
/// msvc 回退名再次（D46 窗口期，stable 段与历史 release 仅剩 msvc 资产），全 miss 报首名错；
/// API 段失败由调用方走镜像边车读序（ome 兼容名层已收口）。
fn official_asset_meta(endpoint: &str, names: &[&str]) -> Result<(String, String, String), String> {
    // release JSON 单拉一次（G2：原每层各拉一遍，窗口期 stable 稳定 2 次 API GET），
    // 本地按名序匹配，全 miss 报首名错。
    let release = fetch_release(endpoint)?;
    asset_meta_in_release(&release, names)
}

/// 在已拉取的 release JSON 内按名序匹配资产（official_asset_meta 的核，with_tag 变体共用）。
fn asset_meta_in_release(
    release: &Value,
    names: &[&str],
) -> Result<(String, String, String), String> {
    let mut primary_err: Option<String> = None;
    for name in names {
        match asset_in_release(release, name) {
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
    let exe = replace_deployed_and_current(&bin, None)?;
    // D41 C：同 release 通道（幂等搬迁，失败只告警）
    if let Err(e) = platform::migrate_legacy_metadata() {
        eprintln!("[WARN] 元数据搬迁失败（旧位读回继续）: {e}");
    }
    // D41：POSIX profile 旧版 env 块收口（Windows no-op）
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

/// 替换部署位并自证（家族自更新统一标准，REQ-0012 #4）：先替换自部署目标（用户 PATH
/// 上的 ark，带锁/暂存/备份/自证/回滚全链），当前进程 exe 不同再 best-effort 同步运行中
/// 副本（cargo run 场景，不背书权威位不自证）。
/// D41 C 收口：随替换清理旧 `ome` 别名（停建不重建；正以别名运行时 Windows 删不动，warn 下次再收）。
fn replace_deployed_and_current(
    new_file: &Path,
    expect_version: Option<&str>,
) -> Result<PathBuf, String> {
    let current = std::env::current_exe().map_err(|e| format!("定位自身 exe 失败: {e}"))?;
    let deploy = platform::self_deploy_target()?;
    if let Some(parent) = deploy.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建部署目录失败: {}: {e}", parent.display()))?;
    }
    replace_with_self_verify(&deploy, new_file, expect_version)?;
    if !path_same(&current, &deploy) && current.exists() {
        let _ = best_effort_sync_current(&current, new_file);
    }
    // D41 C 收口（2026-09-14）：ome 别名停建，升级顺带清理既有副本（水位清零；
    // 当前正以别名运行时 Windows 删不动，warn 留待下次再收）
    if let Err(e) = platform::remove_legacy_alias() {
        eprintln!("[WARN] 旧别名清理失败（不拦升级，下次再收）: {e}");
    }
    Ok(deploy)
}

/// 运行中副本 best-effort 同步（POSIX：写同目录暂存后 rename 覆盖；Windows 运行中
/// exe 不可写不可删，跳过留待下次）。失败不拦（部署位才是权威）。
fn best_effort_sync_current(current: &Path, new_file: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        let _ = (current, new_file);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        // 对线 G4-4：暂存名用收割面内精确形（with_extension 会拼出 ark.ark-new-<pid> 逃出谓词）
        let tmp = current
            .parent()
            .map(|d| d.join(format!("ark-new-{}", std::process::id())))
            .unwrap_or_else(|| current.with_extension("ark-new"));
        std::fs::copy(new_file, &tmp).map_err(|e| format!("写运行中副本暂存失败: {e}"))?;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod 755 失败: {e}"))?;
        std::fs::rename(&tmp, current).map_err(|e| format!("替换运行中副本失败: {e}"))?;
        Ok(())
    }
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

/// 部署目录自更新锁文件名（内容 = 持锁 pid；REQ-0012 #4 防并发互踩）。
fn selfupdate_lock_path(deploy_dir: &Path) -> PathBuf {
    deploy_dir.join(".ark-selfupdate.lock")
}

/// pid 活性检测（unix kill -0 / win tasklist 过滤；检测失败保守视为活，防误收割）。
fn pid_alive(pid: u32) -> bool {
    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(true)
    }
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output()
            .map(|o| {
                let text = String::from_utf8_lossy(&o.stdout);
                text.contains(&pid.to_string())
            })
            .unwrap_or(true)
    }
}

/// 取自更新锁：无锁直取；锁在位且持锁 pid 活 → 拒（并发互踩）；锁在位 pid 死 → 收割重取。
fn acquire_selfupdate_lock(lock: &Path) -> Result<(), String> {
    let mine = std::process::id().to_string();
    // 对线 G4-1：create_new 原子取锁（防并发双取互删互写）；已存在判持锁 pid 活性，
    // 死锁收割后重试一次
    for attempt in 0..2 {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock)
        {
            Ok(_) => {
                std::fs::write(lock, &mine)
                    .map_err(|e| format!("写自更新锁失败: {}: {e}", lock.display()))?;
                return Ok(());
            }
            Err(e) if attempt == 0 => {
                // 对线 G-a 加三轮 F-3-1/G-3-1：空/不可读内容一律视为持锁（保守向，防
                // create_new 与 write 间的空窗被误判死锁偷锁）；仅「可解析且 pid 已死」
                // 或「可解析且 pid 即本进程」（acquire 无重入面，必为上次残留，pid 复用
                // 撞旧锁不永久卡死）才收割重取；锁件不在位时报 create_new 真因。
                if !lock.exists() {
                    return Err(format!(
                        "取自更新锁失败（部署位不可写或锁件竞争）: {}: {e}",
                        lock.display()
                    ));
                }
                if let Ok(text) = std::fs::read_to_string(lock) {
                    if let Ok(pid) = text.trim().parse::<u32>() {
                        if pid != std::process::id() && pid_alive(pid) {
                            return Err(format!(
                                "另一自更新进程（pid {pid}）在跑，拒绝并发互踩；确属死锁可删 {}",
                                lock.display()
                            ));
                        }
                        let _ = std::fs::remove_file(lock); // 死锁/本 pid 残留收割
                    } else {
                        return Err(format!(
                            "自更新锁内容不可读（视为持锁，防误收割）：{}；确属死锁可手删",
                            lock.display()
                        ));
                    }
                } else {
                    return Err(format!(
                        "自更新锁不可读（视为持锁，防误收割）：{}；确属死锁可手删",
                        lock.display()
                    ));
                }
            }
            Err(e) => return Err(format!("取自更新锁失败: {}: {e}", lock.display())),
        }
    }
    Err("取自更新锁失败（收割重试后仍被占）".to_string())
}

/// 陈旧残留清扫（进入替换链时）：部署目录下他人 ark-new-* / ark-old-* 暂存与备份
/// 尝试删除（Windows 运行中件删不动则跳过，留待后续收割；v1.0.0 的 .exe.old 旧残留同扫）。
fn reap_stale_artifacts(deploy_dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(deploy_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(pid) = artifact_owner_pid(&name, std::process::id()) {
                // 对线 G-a 护栏：仅持件 pid 已死才清（在飞 peer 的暂存/备份不伤；
                // 偷锁极端下也不误删）；无 pid 的遗留精确名恒清
                if pid == 0 || !pid_alive(pid) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

/// 收割件归属 pid（Some=他人 pid 的暂存/备份、Some(0)=无 pid 遗留名恒清、None=非本链件）。
fn artifact_owner_pid(name: &str, my_pid: u32) -> Option<u32> {
    if name == "ark.exe.old" {
        return Some(0); // v1.0.0 遗留精确名：无 pid，恒清
    }
    for prefix in ["ark-new-", "ark-old-"] {
        if let Some(tail) = name.strip_prefix(prefix) {
            if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) {
                if let Ok(pid) = tail.parse::<u32>() {
                    if pid != my_pid {
                        return Some(pid);
                    }
                    return None; // 本 pid 件保留
                }
            }
        }
    }
    None
}

/// 自替换全链（家族自更新统一标准，REQ-0012 #4）：同目录暂存 `ark-new-<pid>`（防跨
/// 文件系统 rename EXDEV）→ 旧件备份 `ark-old-<pid>`（Windows 运行中 exe 可改名不可删，
/// 全程用 rename 避删）→ 暂存 rename 就位（同目录原子）→ `--version` 自证五次重试
/// （stable 断言含远端版本；dev 断言可执行非空；杀软瞬时锁面）→ 证败回滚备份并复核
/// 旧件在位，回滚受阻报自救路径。成功清备份（删不动留待收割）并释放锁。
fn replace_with_self_verify(
    deploy: &Path,
    new_file: &Path,
    expect: Option<&str>,
) -> Result<(), String> {
    let dir = deploy
        .parent()
        .ok_or_else(|| format!("部署位无父目录: {}", deploy.display()))?;
    let pid = std::process::id();
    let lock = selfupdate_lock_path(dir);
    acquire_selfupdate_lock(&lock)?;
    let result = replace_inner(deploy, new_file, expect, dir, pid);
    let _ = std::fs::remove_file(&lock); // 释放锁（best-effort；死锁收割兜底）
    result
}

fn replace_inner(
    deploy: &Path,
    new_file: &Path,
    expect: Option<&str>,
    dir: &Path,
    pid: u32,
) -> Result<(), String> {
    reap_stale_artifacts(dir);
    let staged = dir.join(format!("ark-new-{pid}"));
    let backup = dir.join(format!("ark-old-{pid}"));
    let _ = std::fs::remove_file(&staged); // 本 pid 上次残留
    std::fs::copy(new_file, &staged).map_err(|e| format!("暂存新件失败: {e}"))?;
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("暂存件 chmod 755 失败: {e}"))?;
    }
    let had_old = deploy.exists();
    if had_old {
        let _ = std::fs::remove_file(&backup);
        std::fs::rename(deploy, &backup).map_err(|e| format!("备份旧件失败: {e}"))?;
    }
    if let Err(e) = std::fs::rename(&staged, deploy) {
        // 就位失败：回滚备份并核验结果（对线 F4：不核验不得断言「已回滚」）
        if had_old {
            return match std::fs::rename(&backup, deploy) {
                Ok(()) => Err(format!(
                    "新件就位失败，旧件已回滚在位: {e}；新件留存 {} 供诊断",
                    staged.display()
                )),
                Err(re) => Err(format!(
                    "新件就位失败且回滚受阻: {e} / {re}\n自救：手动将 {} 移回 {}",
                    backup.display(),
                    deploy.display()
                )),
            };
        }
        let _ = std::fs::remove_file(&staged);
        return Err(format!("新件就位失败（首次落位无旧件，坏件已清）: {e}"));
    }
    match verify_by_version(deploy, expect, 5) {
        Ok(()) => {
            // 成功：清备份（Windows 旧件若被运行中进程占用删不动，留待收割）
            let _ = std::fs::remove_file(&backup);
            Ok(())
        }
        Err(verr) => {
            // 证败回滚：坏件挪开、备份回位、复核可执行
            let rolled = (|| -> Result<(), String> {
                if deploy.exists() {
                    let _ = std::fs::rename(deploy, &staged);
                }
                if had_old {
                    std::fs::rename(&backup, deploy).map_err(|e| format!("回滚备份失败: {e}"))?;
                } else {
                    std::fs::remove_file(deploy).map_err(|e| format!("清首次落位坏件失败: {e}"))?;
                }
                Ok(())
            })();
            match rolled {
                Ok(()) if verify_by_version(deploy, None, 1).is_ok() => Err(format!(
                    "新件自证失败已回滚旧件在位（{verr}）；坏新件留存 {} 供诊断，稍后重试",
                    staged.display()
                )),
                Ok(()) => Err(format!(
                    "新件自证失败已回滚旧件但复核未过（{verr}）；建议 `ark init` 重装部署位（{}）",
                    deploy.display()
                )),
                Err(re) => Err(format!(
                    "新件自证失败且回滚受阻（{verr} / {re}）；自救：手动将 {} 移回 {}",
                    backup.display(),
                    deploy.display()
                )),
            }
        }
    }
}

/// `--version` 自证（家族标准：五次重试对杀软瞬时锁面）。expect 在位断言输出含期望
/// 版本（stable 远端版本）；None 断言可执行且输出非空（dev 滚动源无版本语义）。
fn verify_by_version(exe: &Path, expect: Option<&str>, tries: u32) -> Result<(), String> {
    let mut last = String::new();
    for attempt in 1..=tries {
        if let Ok(out) = Command::new(exe).arg("--version").output() {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            let ok = out.status.success()
                && !text.trim().is_empty()
                && expect.is_none_or(|v| text.contains(v));
            if ok {
                return Ok(());
            }
            last = format!(
                "退出码 {:?}，输出 {text:?}（期望含 {expect:?}）",
                out.status.code()
            );
        } else {
            last = "启动失败".to_string();
        }
        if attempt < tries {
            std::thread::sleep(Duration::from_millis(400));
        }
    }
    Err(format!("--version 自证 {tries} 次失败: {last}"))
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

    /// 可执行假件：输出「ark <ver>」的脚本（unix shell 形；win 用批处理形）。
    fn fake_exec(dir: &Path, ver: &str) -> PathBuf {
        let p = dir.join(if cfg!(windows) { "fake.bat" } else { "fake" });
        #[cfg(not(windows))]
        std::fs::write(&p, format!("#!/bin/sh\necho ark {ver}\n")).unwrap();
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(windows)]
        std::fs::write(&p, format!("@echo off\necho ark {ver}\r\n")).unwrap();
        p
    }

    /// REQ-0012 #4：自替换全链（暂存/备份/自证/清备份/锁释放）——好件自证过。
    #[test]
    fn 自替换_好件自证过并清备份() {
        let dir = tempfile::tempdir().expect("临时目录");
        let deploy = dir.path().join("ark");
        let _ = std::fs::write(&deploy, b"old"); // 旧件（内容非可执行，证败用）
        let new = fake_exec(dir.path(), "9.9.9");
        replace_with_self_verify(&deploy, &new, Some("9.9.9")).expect("好件应自证过");
        let out = Command::new(&deploy).arg("--version").output().unwrap();
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("9.9.9"),
            "部署位应为新件"
        );
        // 备份已清、锁已释放、无暂存残留
        assert!(
            !dir.path().join(".ark-selfupdate.lock").exists(),
            "锁应释放"
        );
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with("ark-new-") || n.starts_with("ark-old-"))
            .collect();
        assert!(leftovers.is_empty(), "成功路径不留暂存/备份: {leftovers:?}");
    }

    /// REQ-0012 #4：坏件（自证必败）→ 回滚旧件并复核在位（旧件是好件才回滚得过去）。
    #[test]
    fn 自替换_坏件自证败回滚旧件() {
        let dir = tempfile::tempdir().expect("临时目录");
        let deploy = dir.path().join("ark");
        let good_old = fake_exec(dir.path(), "1.0.0");
        std::fs::copy(&good_old, &deploy).unwrap(); // 旧件是好件（回滚后复核要可执行）
                                                    // 坏件：文本件（rename 就位成功但 --version 不可执行）
        let bad = dir.path().join("bad");
        std::fs::write(&bad, b"not an executable").unwrap();
        let err =
            replace_with_self_verify(&deploy, &bad, Some("9.9.9")).expect_err("坏件自证应失败");
        assert!(err.contains("已回滚旧件"), "应报回滚: {err}");
        let out = Command::new(&deploy).arg("--version").output().unwrap();
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("1.0.0"),
            "回滚后部署位应为旧好件"
        );
    }

    /// REQ-0012 #4：锁语义——死锁与本 pid 残留锁均收割重取（三轮 F-3-1：acquire 无
    /// 重入面，本 pid 锁必为上次残留；pid 复用撞旧锁不得永久卡死）。
    #[test]
    fn 自更新锁_死锁与本pid残留收割() {
        let dir = tempfile::tempdir().expect("临时目录");
        let lock = selfupdate_lock_path(dir.path());
        // 死 pid 取超 pid_max 的普通值（u32::MAX 在 linux 会回绕成 -1 即「全体进程」，
        // kill -0 反而成功，不能当死 pid 用——本机 /proc/sys/kernel/pid_max 实证）
        std::fs::write(&lock, "300000000").unwrap();
        acquire_selfupdate_lock(&lock).expect("死锁应被收割并重取");
        assert_eq!(
            std::fs::read_to_string(&lock).unwrap().trim(),
            std::process::id().to_string()
        );
        // 本 pid 残留锁（上次升级中断）：同样收割重取，不报「另一进程在跑」
        std::fs::write(&lock, std::process::id().to_string()).unwrap();
        acquire_selfupdate_lock(&lock).expect("本 pid 残留锁应被收割并重取");
    }

    /// REQ-0012 #4：陈旧收割——他人 ark-new-*/ark-old-* 与旧 .exe.old 残留被清，本 pid 保留。
    #[test]
    fn 陈旧收割_他人残留清本pid保留() {
        let dir = tempfile::tempdir().expect("临时目录");
        let pid = std::process::id();
        let theirs = dir.path().join(format!("ark-new-{}", pid + 1));
        let old_legacy = dir.path().join("ark.exe.old");
        let mine = dir.path().join(format!("ark-old-{pid}"));
        std::fs::write(&theirs, b"x").unwrap();
        std::fs::write(&old_legacy, b"x").unwrap();
        std::fs::write(&mine, b"x").unwrap();
        reap_stale_artifacts(dir.path());
        assert!(!theirs.exists(), "他人暂存应清");
        assert!(!old_legacy.exists(), "旧 .exe.old 残留应清");
        assert!(mine.exists(), "本 pid 备份保留");
    }

    /// REQ-0012 #3：semver 门三态（数值段比较）与非 semver tag 保守放行。
    #[test]
    fn semver门_三态与非semver放行() {
        assert_eq!(semver_gate("1.4.1", "1.4.1"), Verdict::Current);
        assert_eq!(
            semver_gate("1.4.0", "1.4.1"),
            Verdict::LocalNewer,
            "本地领先不动"
        );
        assert_eq!(semver_gate("1.5.0", "1.4.1"), Verdict::Upgrade);
        // 数值段而非字典序：1.10 > 1.9
        assert_eq!(semver_gate("1.10.0", "1.9.9"), Verdict::Upgrade);
        assert_eq!(
            semver_gate("1.4.1.1", "1.4.1"),
            Verdict::Upgrade,
            "多段保守升级"
        );
        assert_eq!(
            semver_gate("nightly", "1.4.1"),
            Verdict::Upgrade,
            "非 semver 保守放行"
        );
        assert_eq!(strip_v("v1.4.1"), "1.4.1");
        assert_eq!(strip_v("1.4.1"), "1.4.1");
    }

    /// 对线 G4-2 加 G-a 护栏：收割件归属判定精确形——他人 pid 暂存/备份归属该 pid
    /// （由调用方判活性再清）、遗留 .exe.old 恒清（Some(0)）、本 pid 与非本链件保留。
    #[test]
    fn 收割归属_精确形() {
        let mine = 1000u32;
        assert_eq!(artifact_owner_pid("ark-new-1001", mine), Some(1001));
        assert_eq!(artifact_owner_pid("ark-old-999", mine), Some(999));
        assert_eq!(
            artifact_owner_pid("ark.exe.old", mine),
            Some(0),
            "遗留精确名恒清"
        );
        assert_eq!(
            artifact_owner_pid("ark-new-1000", mine),
            None,
            "本 pid 保留"
        );
        assert_eq!(
            artifact_owner_pid("ark-new-abc", mine),
            None,
            "非数字尾不认"
        );
        assert_eq!(artifact_owner_pid("ark-new-", mine), None, "空尾不认");
        assert_eq!(
            artifact_owner_pid("ark-new-1.tmp", mine),
            None,
            "带扩展不认"
        );
        assert_eq!(
            artifact_owner_pid("mytool.exe.old", mine),
            None,
            "他人 .exe.old 不认"
        );
        assert_eq!(
            artifact_owner_pid("ark-notes.txt", mine),
            None,
            "同前缀自有文件不认"
        );
    }

    /// REQ-0012 #2：错误判型——校验性（硬拒）与网络性（回落）分界。
    #[test]
    fn 错误判型_校验性与网络性() {
        // 判据引用 download 层常量（对线 G3-1）；「镜像对象 sha256 与锚不符」由
        // download 层镜像段内部转回落、不外泄到判型点（死分支已收）
        let mismatch_err = format!(
            "{}: /x\n期望 A\n实际 B",
            crate::download::HASH_MISMATCH_MARK
        );
        assert!(is_hash_mismatch(&mismatch_err));
        assert!(!is_hash_mismatch(
            "镜像对象 sha256 与锚不符（已删，回落官方）: 期望 A，实际 B"
        ));
        assert!(!is_hash_mismatch("HTTP 请求失败: https://x: 404"));
        assert!(!is_hash_mismatch("下载失败且 curl.exe 不可用"));
    }

    #[test]
    fn 资产名_当前平台必有映射() {
        // 本 CI 覆盖的三目标之一，或明确报不支持；D41 B 起主名 ark-（ome 兼容名层已收口）
        match asset_for_this_platform() {
            Ok(primary) => assert!(primary.starts_with("ark-"), "主名应带 ark- 前缀: {primary}"),
            Err(e) => assert!(e.contains("无 CI 构建资产")),
        }
    }

    #[test]
    fn 镜像段读序_尝试表三层构造() {
        // D41 自测 3（构造面）：包形主名先、gnu 裸件次、msvc 回退再次（D46）、URL 形态、
        // 缺名层跳过；ome/ 兼容段已收口（剔除批 2026-09-18），段恒 ark/
        let base = "https://mirror.example";
        let three = mirror_attempts(
            base,
            "dev",
            "ark-x86_64-pc-windows-gnu.exe",
            Some("ark-x86_64-pc-windows-msvc.exe"),
        );
        assert_eq!(three.len(), 3, "三层读序三尝试");
        let pkg = asset_package_for_this_platform().expect("测试平台有包名");
        assert_eq!(three[0].1, pkg, "包形主名先（有包用包）");
        assert_eq!(three[0].0, "ark", "段恒 ark");
        assert_eq!(
            three[1].1, "ark-x86_64-pc-windows-gnu.exe",
            "gnu 裸件回退臂次"
        );
        assert_eq!(
            three[2].1, "ark-x86_64-pc-windows-msvc.exe",
            "msvc 回退再次"
        );
        assert_eq!(
            three[1].2,
            format!("{base}/ark/dev/ark-x86_64-pc-windows-gnu.exe.sha256"),
            "边车 URL 形态"
        );
        let two_now = mirror_attempts(base, "stable", "ark-x.exe", None);
        assert_eq!(two_now.len(), 2, "无 msvc 回退名仅包加裸两层");
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
    fn 自管条目_extract单接受() {
        // ome 关键字剔除批（2026-09-18）：旧值 ome-self 双接受收口，仅认 ark-self
        //（云端权威 catalog 自 2026-09-14 起写 ark-self）
        let mk = |e: &str| crate::catalog::Tool {
            extract: Some(e.to_string()),
            ..Default::default()
        };
        assert!(is_ark_self(&mk("ark-self")), "ark-self 受认");
        assert!(!is_ark_self(&mk("ome-self")), "旧值已收口不再受认");
        assert!(!is_ark_self(&mk("zip")), "非自管不误判");
        assert!(!is_ark_self(&mk("npm-tgz")), "npm 型不误判");
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
