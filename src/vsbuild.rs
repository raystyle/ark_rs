//! vsbuild：VS Build Tools 接管（自 ohmypwsh `scripts\set-vsbuild.ps1` 平移，2026-09-01）。
//! 永续引导器（aka.ms 直链，无版本无 sha 可 pin，属 evergreen 语义）；
//! 组件三件套 VCTools、VC.Tools.x86.x64、VC.CMake.Project（不带 --includeRecommended；
//! Windows SDK 走 ISO 分离装 Windows Kits，不进 VS 组件，见 set-windows-sdk.ps1）。
//! 需管理员：未提权且 gsudo 可用时经 gsudo 重跑 `ark install vsbuild`（退出码透传）；
//! PATH 写机器级（MSBuild 与 cl.exe 目录），非用户级。
//! 幂等：cl.exe 在位只补机器 PATH（PATH 齐备则完全无操作，无需管理员）。

use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
#[cfg(windows)]
use std::time::Duration;

use crate::catalog::Tool;
#[cfg(windows)]
use crate::download;
#[cfg(windows)]
use crate::install::InstallAction;
use crate::install::InstallOutcome;
#[cfg(windows)]
use crate::platform;
#[cfg(windows)]
use crate::toolver;

/// 引导器缓存文件名。
pub const BOOTSTRAPPER: &str = "vs_buildtools.exe";

/// 安装组件（VCTools/x86.x64/CMake）。
pub const COMPONENTS: [&str; 3] = [
    "Microsoft.VisualStudio.Workload.VCTools",
    "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
    "Microsoft.VisualStudio.Component.VC.CMake.Project",
];

/// 是否 vsbuild 型条目（extract = "vsbuild"）。
pub fn is_vsbuild(def: &Tool) -> bool {
    def.extract() == Some("vsbuild")
}

/// 安装根：`<EnvRoot>\vsbuild`。
pub fn install_root(env_root: &Path) -> PathBuf {
    env_root.join("vsbuild")
}

/// MSBuild.exe（路径跨版本稳定，版本探测用：`MSBuild -version` stdout 首行裸版本号）。
pub fn msbuild_exe(env_root: &Path) -> PathBuf {
    install_root(env_root)
        .join("MSBuild")
        .join("Current")
        .join("Bin")
        .join("MSBuild.exe")
}

/// 最新 MSVC 工具集的 cl.exe（存在即视为已安装）。
pub fn find_cl_exe(env_root: &Path) -> Option<PathBuf> {
    let msvc = install_root(env_root).join("VC").join("Tools").join("MSVC");
    let mut best: Option<String> = None;
    let mut cl: Option<PathBuf> = None;
    for entry in std::fs::read_dir(&msvc).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let candidate = entry
            .path()
            .join("bin")
            .join("Hostx64")
            .join("x64")
            .join("cl.exe");
        if !candidate.exists() {
            continue;
        }
        if best.as_ref().is_none_or(|b| name > *b) {
            best = Some(name);
            cl = Some(candidate);
        }
    }
    cl
}

/// 机器 PATH 应包含的目录（MSBuild 与 cl.exe 目录，存在才返回）。
pub fn machine_path_dirs(env_root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let msbuild = msbuild_exe(env_root);
    if msbuild.exists() {
        if let Some(parent) = msbuild.parent() {
            dirs.push(parent.to_path_buf());
        }
    }
    if let Some(cl) = find_cl_exe(env_root) {
        if let Some(dir) = cl.parent() {
            dirs.push(dir.to_path_buf());
        }
    }
    dirs
}

/// 引导器安装参数（纯函数可测；--wait 等安装器完成，3010 = 成功需重启）。
pub fn bootstrapper_args(install_path: &Path) -> Vec<String> {
    let mut args: Vec<String> = ["--quiet", "--norestart", "--wait", "--installPath"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    args.push(install_path.display().to_string());
    for c in COMPONENTS {
        args.push("--add".to_string());
        args.push(c.to_string());
    }
    args
}

/// 安装（幂等）。download 只跑引导器落二进制；deploy（configure）才写机器 PATH。
///
/// # Errors
/// 返回 Err（人读原因串）当：gsudo 提权执行失败 exit={} 等（完整失败面见函数体错误构造）。
pub fn install(def: &Tool, env_root: &Path, configure: bool) -> Result<InstallOutcome, String> {
    #[cfg(not(windows))]
    {
        let _ = (def, env_root, configure);
        Err("vsbuild 仅支持 Windows".to_string())
    }
    #[cfg(windows)]
    {
        let install_dir = install_root(env_root);
        let exe = msbuild_exe(env_root);

        // 已装：cl.exe 在位；download 不再碰 PATH，deploy 才补机器 PATH
        if find_cl_exe(env_root).is_some() {
            let version =
                toolver::installed_version(&exe, def).unwrap_or_else(|| "unknown".to_string());
            if !configure {
                eprintln!("[INFO] vsbuild 已安装，跳过（download 不改 PATH）");
                return Ok(InstallOutcome {
                    action: InstallAction::Skipped,
                    version,
                    dir: Some(install_dir),
                });
            }
            let dirs = machine_path_dirs(env_root);
            let mut path_missing = dirs.is_empty();
            for d in &dirs {
                if !platform::machine_path_contains(d)? {
                    path_missing = true;
                }
            }
            if path_missing {
                if !platform::is_elevated() {
                    return relaunch_elevated(def, env_root, InstallAction::Skipped);
                }
                platform::machine_path_add(&dirs)?;
                eprintln!("[OK] 机器 PATH 已合并（新终端生效）");
            } else {
                eprintln!("[INFO] vsbuild 已安装且机器 PATH 齐备，跳过");
            }
            return Ok(InstallOutcome {
                action: InstallAction::Skipped,
                version,
                dir: Some(install_dir),
            });
        }

        // 未装：引导器需管理员
        if !platform::is_elevated() {
            return relaunch_elevated(def, env_root, InstallAction::Installed);
        }
        install_elevated(def, env_root, configure)
    }
}

/// 未提权时经 gsudo 重跑当前动词（download 只落二进制，deploy 才写机器 PATH）。
#[cfg(windows)]
fn relaunch_elevated(
    def: &Tool,
    env_root: &Path,
    action: InstallAction,
) -> Result<InstallOutcome, String> {
    let gsudo = which::which("gsudo").map_err(|_| {
        "vsbuild 安装需管理员：以管理员终端重跑 `ark install vsbuild`，或先 `ark install gsudo` 后自动提权"
            .to_string()
    })?;
    let exe = std::env::current_exe().map_err(|e| format!("获取当前 exe 失败: {e}"))?;
    eprintln!("[INFO] 需要管理员，经 gsudo 提权重跑: install vsbuild");
    let status = Command::new(&gsudo)
        .arg(&exe)
        .arg("--env-root")
        .arg(env_root)
        .arg("install")
        .arg("vsbuild")
        .status()
        .map_err(|e| format!("gsudo 启动失败: {}: {e}", gsudo.display()))?;
    if !status.success() {
        return Err(format!(
            "gsudo 提权执行失败 exit={}",
            status.code().unwrap_or(-1)
        ));
    }
    // 子进程已完整执行安装与机器 PATH，父进程只探测结果透传
    Ok(InstallOutcome {
        action,
        version: toolver::installed_version(&msbuild_exe(env_root), def)
            .unwrap_or_else(|| "unknown".to_string()),
        dir: Some(install_root(env_root)),
    })
}

/// 已提权的安装主体：预建目录（TargetDirCheck 8004 坑）→ 下载引导器 → 静默安装 → 验证；
/// 机器 PATH 仅 configure（deploy）时写。
#[cfg(windows)]
fn install_elevated(
    def: &Tool,
    env_root: &Path,
    configure: bool,
) -> Result<InstallOutcome, String> {
    let url = def
        .cdn_url()
        .ok_or_else(|| "vsbuild 条目缺少 cdn_url 字段".to_string())?;
    let install_dir = install_root(env_root);

    // VS 预检要求 installPath 已存在（删掉旧目录后必须重建，否则 TargetDirCheck 失败 8004）
    std::fs::create_dir_all(&install_dir)
        .map_err(|e| format!("创建安装目录失败: {}: {e}", install_dir.display()))?;

    // 永续引导器：aka.ms 直链无版本无官方 sha（evergreen 语义）；
    // 镜像优先 latest 段（D44），边车 .sha256 即信任锚，未命中回落官方（D08 第二批）
    let boot = download::download_latest_with_sidecar(env_root, BOOTSTRAPPER, url, "vsbuild")?;

    eprintln!("[INFO] 运行 VS Build Tools 静默安装（VCTools 组件三件套）...");
    let status = Command::new(&boot)
        .args(bootstrapper_args(&install_dir))
        .status()
        .map_err(|e| format!("引导器启动失败: {}: {e}", boot.display()))?;
    let code = status.code().unwrap_or(-1);
    if code != 0 && code != 3010 {
        return Err(format!("VS Build Tools 安装失败 exit={code}"));
    }

    // 装后验证：cl.exe 就位（安装器 --wait 返回后应就绪，短重试兜底瞬态）
    let mut cl = find_cl_exe(env_root);
    for i in 1..=3u64 {
        if cl.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_secs(i));
        cl = find_cl_exe(env_root);
    }
    cl.ok_or_else(|| "安装后未找到 cl.exe（VC\\Tools\\MSVC\\*\\bin\\Hostx64\\x64）".to_string())?;

    if configure {
        platform::machine_path_add(&machine_path_dirs(env_root))?;
    }
    let version = toolver::installed_version(&msbuild_exe(env_root), def)
        .unwrap_or_else(|| "unknown".to_string());
    eprintln!("[OK] vsbuild 安装完成: {version}");
    eprintln!("[HINT] Windows SDK 不随 VS 组件：kernel32.lib 需 set-windows-sdk.ps1（ISO 分离）");
    Ok(InstallOutcome {
        action: InstallAction::Installed,
        version,
        dir: Some(install_dir),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 组件三件套（VCTools/x86.x64/CMake，无 SDK 无 includeRecommended）。
    #[test]
    fn 组件清单_与脚本一致() {
        assert_eq!(
            COMPONENTS,
            [
                "Microsoft.VisualStudio.Workload.VCTools",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "Microsoft.VisualStudio.Component.VC.CMake.Project",
            ]
        );
    }

    /// 引导器参数形态：静默四件 + installPath + 三组 --add（对齐 set-vsbuild.ps1）。
    #[test]
    fn 引导器参数_形态对齐脚本() {
        let args = bootstrapper_args(Path::new(r"D:\ohmyenv\vsbuild"));
        assert_eq!(
            args,
            vec![
                "--quiet".to_string(),
                "--norestart".to_string(),
                "--wait".to_string(),
                "--installPath".to_string(),
                r"D:\ohmyenv\vsbuild".to_string(),
                "--add".to_string(),
                "Microsoft.VisualStudio.Workload.VCTools".to_string(),
                "--add".to_string(),
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64".to_string(),
                "--add".to_string(),
                "Microsoft.VisualStudio.Component.VC.CMake.Project".to_string(),
            ]
        );
    }

    /// cl.exe 探测：多个 MSVC 工具集取字典序最新；无工具集为 None。
    #[test]
    fn cl探测_多工具集取最新() {
        let dir = tempfile::tempdir().expect("临时目录");
        // 按真实布局构造：<EnvRoot>\vsbuild\VC\Tools\MSVC\<工具集>\bin\Hostx64\x64\cl.exe
        let msvc = install_root(dir.path())
            .join("VC")
            .join("Tools")
            .join("MSVC");
        for (v, with_cl) in [
            ("14.40.111", true),
            ("14.44.35207", true),
            ("14.50.0", false),
        ] {
            let bin = msvc.join(v).join("bin").join("Hostx64").join("x64");
            std::fs::create_dir_all(&bin).expect("建目录");
            if with_cl {
                std::fs::write(bin.join("cl.exe"), b"stub").expect("写 cl");
            }
        }
        let cl = find_cl_exe(dir.path()).expect("应找到 cl.exe");
        assert!(cl.display().to_string().contains("14.44.35207"));
        // 机器 PATH 目录随探测联动
        assert!(machine_path_dirs(dir.path())
            .iter()
            .any(|d| d.display().to_string().contains("14.44.35207")));
    }
}
