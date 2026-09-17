//! rustup：Rust 接管（自 ohmypwsh `scripts\set-rust.ps1` 平移，2026-09-02；POSIX 接管 D42，2026-09-13）。
//! rsproxy 引导器直链（无版本无 sha，evergreen 语义）：rustup-init 引导 stable 工具链，
//! 无 pin（`rustup update stable` 即更新语义，官方 6 周滚动，stable 只收安全补丁）。
//! 安装根双形态：Windows 重定位 EnvRoot（RUSTUP_HOME=`<EnvRoot>\rustup`、CARGO_HOME=`<EnvRoot>\cargo`，
//! 用户环境变量）；POSIX 尊重既有（进程 env > 用户级 > 默认 `~/.rustup` 与 `~/.cargo`，
//! 不持久化两 HOME 变量、进程内钉解析值供引导器）。
//! rsproxy 双镜像：rustup 分发（RUSTUP_DIST_SERVER/RUSTUP_UPDATE_ROOT，win 注册表 / POSIX profile env 块）
//! 加 cargo sparse（config.toml，Windows 落 EnvRoot 重定位位、POSIX 落 `~/.cargo`，rsproxy 全量形态）；
//! cargo bin 进用户 PATH（ark 惯例尾部追加；set-rust 为前置，已装机器由 ps1 前置位保持不变）。
//! 幂等：rustc 在位不重跑 init（update stable 照跑保最新）；config.toml 内容一致不重写。

use std::path::{Path, PathBuf};

use crate::catalog::Tool;
use crate::download;
use crate::install::InstallAction;
use crate::install::InstallOutcome;
use crate::platform;
use crate::toolver;

/// 引导器缓存文件名（平台各异）。
pub const INIT_EXE: &str = if cfg!(windows) {
    "rustup-init.exe"
} else {
    "rustup-init"
};

/// rsproxy rustup 分发镜像（用户环境变量两键）。
const DIST_SERVER: &str = "https://rsproxy.cn";
const UPDATE_ROOT: &str = "https://rsproxy.cn/rustup";

/// cargo 镜像配置（与 set-rust.ps1 逐字节一致：crates-io 换 rsproxy sparse、git 走 cli、http 两开关；
/// 全平台同内容，rsproxy 全量形态）。
const CARGO_CONFIG: &str = r#"[source.crates-io]
replace-with = "rsproxy"

[source.rsproxy]
registry = "sparse+https://rsproxy.cn/index/"

[net]
git-fetch-with-cli = true

[http]
check-revoke = false
multiplexing = true
"#;

/// 是否 rustup 引导器型条目（extract = "rustup"）。
pub fn is_rustup(def: &Tool) -> bool {
    def.extract() == Some("rustup")
}

/// RUSTUP_HOME：Windows `<EnvRoot>\rustup`；POSIX 尊重既有（进程 env > 用户级 > 默认 `~/.rustup`）。
pub fn rustup_home(env_root: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        env_root.join("rustup")
    }
    #[cfg(not(windows))]
    {
        let _ = env_root;
        if let Some(v) = std::env::var_os("RUSTUP_HOME") {
            return PathBuf::from(v);
        }
        if let Ok(Some(v)) = platform::get_user_env_var("RUSTUP_HOME") {
            return platform::expand_install_path(&platform::expand_env_vars(&v));
        }
        home_dir().join(".rustup")
    }
}

/// CARGO_HOME：Windows `<EnvRoot>\cargo`；POSIX 与 manifest `cargo_home_path` 同一解析序
/// （进程 env > 用户级 > 默认 `~/.cargo`，对线 R3：单一解析序防双目录）。
pub fn cargo_home(env_root: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        env_root.join("cargo")
    }
    #[cfg(not(windows))]
    {
        let _ = env_root;
        crate::manifest::cargo_home_path().unwrap_or_else(|_| home_dir().join(".cargo"))
    }
}

/// rustc 可执行（`<cargo home>\bin\rustc[.exe]`，verify dev-rust 维度断言位）。
pub fn rustc_exe(env_root: &Path) -> PathBuf {
    cargo_home(env_root)
        .join("bin")
        .join(format!("rustc{}", platform::exe_suffix()))
}

#[cfg(not(windows))]
fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// rustup-init 引导参数（纯函数可测）。Windows 对齐 set-rust.ps1（stable + msvc host + 不动 PATH）；
/// POSIX host 三元组由 rustup-init 自检（x86_64/aarch64 不写死）。
pub fn init_args() -> Vec<String> {
    #[cfg(windows)]
    let args = [
        "-y",
        "--default-toolchain",
        "stable",
        "--default-host",
        "x86_64-pc-windows-msvc",
        "--no-modify-path",
    ];
    #[cfg(not(windows))]
    let args = ["-y", "--default-toolchain", "stable", "--no-modify-path"];
    args.iter().map(|s| s.to_string()).collect()
}

/// 安装（幂等）：download 只落二进制（进程内重定位供 rustup-init 写入安装根）；
/// deploy（configure）才持久化用户环境变量、cargo 镜像与 PATH。
///
/// # Errors
/// 返回 Err（人读原因串）当：rustup-init 失败 exit={} 等（完整失败面见函数体错误构造）。
pub fn install(def: &Tool, env_root: &Path, configure: bool) -> Result<InstallOutcome, String> {
    #[cfg(windows)]
    {
        install_windows(def, env_root, configure)
    }
    #[cfg(not(windows))]
    {
        install_posix(def, env_root, configure)
    }
}

/// Windows：EnvRoot 重定位模型（set-rust.ps1 平移原语义）。
#[cfg(windows)]
fn install_windows(def: &Tool, env_root: &Path, configure: bool) -> Result<InstallOutcome, String> {
    use std::process::Command;

    let rustup_home = rustup_home(env_root);
    let cargo_home = cargo_home(env_root);
    let rustc = rustc_exe(env_root);
    let url = def
        .cdn_url()
        .ok_or_else(|| "rust 条目缺少 cdn_url 字段".to_string())?;

    // ── 1. 目录 + 进程内重定位（供 rustup-init 把工具链写入 EnvRoot）──
    for dir in [&rustup_home, &cargo_home] {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("创建目录失败: {}: {e}", dir.display()))?;
    }
    for (k, v) in [
        ("RUSTUP_DIST_SERVER", DIST_SERVER.to_string()),
        ("RUSTUP_UPDATE_ROOT", UPDATE_ROOT.to_string()),
        ("RUSTUP_HOME", rustup_home.display().to_string()),
        ("CARGO_HOME", cargo_home.display().to_string()),
    ] {
        std::env::set_var(k, &v);
        if configure {
            let cur = platform::get_user_env_var(k)?;
            if cur.as_deref() != Some(v.as_str()) {
                platform::set_user_env_var(k, &v)?;
                eprintln!("[OK] 用户环境变量已设: {k}={v}（新终端生效）");
            } else {
                eprintln!("[INFO] {k} 已是 {v}");
            }
        }
    }

    // ── 2. rustup-init 引导（rustc 在位则跳过；引导器读取进程内重定位变量）──
    let had_rustc = toolver::installed_version(&rustc, def).is_some();
    if !had_rustc {
        // 永续引导器：rsproxy 直链无版本无官方 sha（evergreen 语义）；
        // 镜像优先 latest 段（D44），边车 .sha256 即信任锚，未命中回落官方（D08 第二批）
        let init = download::download_latest_with_sidecar(env_root, INIT_EXE, url, "rust")?;
        eprintln!("[INFO] 运行 rustup-init（stable / x86_64-pc-windows-msvc）...");
        let status = Command::new(&init)
            .args(init_args())
            .status()
            .map_err(|e| format!("rustup-init 启动失败: {}: {e}", init.display()))?;
        if !status.success() {
            return Err(format!(
                "rustup-init 失败 exit={}",
                status.code().unwrap_or(-1)
            ));
        }
    } else {
        eprintln!("[INFO] rustc 已安装，跳过 rustup-init");
    }

    // ── 3. 保持最新 stable（幂等；失败不拦——网络抖动不应让在位安装报错，末尾版本校验兜底）──
    let rustup = cargo_home.join("bin").join("rustup.exe");
    if rustup.exists() {
        for args in [vec!["update", "stable"], vec!["default", "stable"]] {
            if let Ok(st) = Command::new(&rustup).args(&args).status() {
                if !st.success() {
                    eprintln!("[WARN] rustup {:?} 未成功（继续）", args);
                }
            }
        }
    }

    if configure {
        // ── 4. cargo 镜像（内容一致不重写；UTF-8 无 BOM）──
        let cargo_cfg = cargo_home.join("config.toml");
        let existing = std::fs::read_to_string(&cargo_cfg).unwrap_or_default();
        if existing != CARGO_CONFIG {
            std::fs::write(&cargo_cfg, CARGO_CONFIG)
                .map_err(|e| format!("写 cargo config 失败: {}: {e}", cargo_cfg.display()))?;
            eprintln!("[OK] cargo 镜像已写入 config.toml");
        } else {
            eprintln!("[INFO] cargo 镜像已是最新");
        }

        // ── 5. 用户 PATH（cargo bin）──
        let bin_dir = cargo_home.join("bin");
        if !platform::user_path_contains(&bin_dir)? {
            platform::add_user_path(&bin_dir)?;
            eprintln!("[OK] 用户 PATH 已加 cargo bin（新终端生效）");
        } else {
            eprintln!("[INFO] cargo bin 已在 PATH");
        }
    }

    // ── 6. 校验 ──
    let version = toolver::installed_version_retried(&rustc, def)
        .ok_or_else(|| "rustc 版本校验失败（安装后未探测到）".to_string())?;
    eprintln!("[OK] rust 安装完成: {version}");
    Ok(InstallOutcome {
        action: if had_rustc {
            InstallAction::Skipped
        } else {
            InstallAction::Installed
        },
        version,
        dir: Some(cargo_home),
    })
}

/// POSIX：系统标准位模型（R010；D42）。安装根尊重既有（进程 env > 用户级 > 默认 `~/.rustup`
/// 与 `~/.cargo`），解析值**钉进进程**（rustup-init 只读进程 env，用户级值不钉则装错位，
/// 对线 R3）；两 HOME 变量**不持久化**（POSIX 不重定位语义）；持久镜像变量经 profile env
/// 标记块（shell rc）。
#[cfg(not(windows))]
fn install_posix(def: &Tool, env_root: &Path, configure: bool) -> Result<InstallOutcome, String> {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let rustup_home = rustup_home(env_root);
    let cargo_home = cargo_home(env_root);
    let rustc = rustc_exe(env_root);
    let url = def.cdn_url().ok_or_else(|| {
        "rust 条目缺少本平台 cdn_url 字段（linux_cdn_url / mac_cdn_url）".to_string()
    })?;

    // ── 1. 目录 + 进程内变量（安装根钉进程防引导器落错位；镜像变量引导期走 rsproxy）──
    for dir in [&rustup_home, &cargo_home] {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("创建目录失败: {}: {e}", dir.display()))?;
    }
    std::env::set_var("RUSTUP_HOME", &rustup_home);
    std::env::set_var("CARGO_HOME", &cargo_home);
    for (k, v) in [
        ("RUSTUP_DIST_SERVER", DIST_SERVER),
        ("RUSTUP_UPDATE_ROOT", UPDATE_ROOT),
    ] {
        std::env::set_var(k, v);
        if configure {
            let cur = platform::get_user_env_var(k)?;
            if cur.as_deref() != Some(v) {
                platform::set_user_env_var(k, v)?;
                eprintln!("[OK] 用户环境变量已设: {k}={v}（新终端生效）");
            } else {
                eprintln!("[INFO] {k} 已是 {v}");
            }
        }
    }

    // ── 2. rustup-init 引导（rustc 在位则跳过；host 三元组引导器自检）──
    let had_rustc = toolver::installed_version(&rustc, def).is_some();
    if !had_rustc {
        let init = download::download_latest_with_sidecar(env_root, INIT_EXE, url, "rust")?;
        std::fs::set_permissions(&init, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod 755 失败: {}: {e}", init.display()))?;
        eprintln!("[INFO] 运行 rustup-init（stable，host 自检）...");
        let status = Command::new(&init)
            .args(init_args())
            .status()
            .map_err(|e| format!("rustup-init 启动失败: {}: {e}", init.display()))?;
        if !status.success() {
            return Err(format!(
                "rustup-init 失败 exit={}",
                status.code().unwrap_or(-1)
            ));
        }
    } else {
        eprintln!("[INFO] rustc 已安装，跳过 rustup-init");
    }

    // ── 3. 保持最新 stable（幂等；失败不拦，末尾版本校验兜底）──
    let rustup = cargo_home.join("bin").join("rustup");
    if rustup.exists() {
        for args in [vec!["update", "stable"], vec!["default", "stable"]] {
            if let Ok(st) = Command::new(&rustup).args(&args).status() {
                if !st.success() {
                    eprintln!("[WARN] rustup {:?} 未成功（继续）", args);
                }
            }
        }
    }

    if configure {
        // ── 4. cargo 镜像（~/.cargo/config.toml，内容一致不重写）──
        let cargo_cfg = cargo_home.join("config.toml");
        let existing = std::fs::read_to_string(&cargo_cfg).unwrap_or_default();
        if existing != CARGO_CONFIG {
            std::fs::write(&cargo_cfg, CARGO_CONFIG)
                .map_err(|e| format!("写 cargo config 失败: {}: {e}", cargo_cfg.display()))?;
            eprintln!("[OK] cargo 镜像已写入 config.toml");
        } else {
            eprintln!("[INFO] cargo 镜像已是最新");
        }

        // ── 5. 用户 PATH（~/.cargo/bin，profile 标记块）──
        let bin_dir = cargo_home.join("bin");
        if !platform::user_path_contains(&bin_dir)? {
            platform::add_user_path(&bin_dir)?;
            eprintln!("[OK] 用户 PATH 已加 cargo bin（新终端生效）");
        } else {
            eprintln!("[INFO] cargo bin 已在 PATH");
        }
    }

    // ── 6. 校验 ──
    let version = toolver::installed_version_retried(&rustc, def)
        .ok_or_else(|| "rustc 版本校验失败（安装后未探测到）".to_string())?;
    eprintln!("[OK] rust 安装完成: {version}");
    Ok(InstallOutcome {
        action: if had_rustc {
            InstallAction::Skipped
        } else {
            InstallAction::Installed
        },
        version,
        dir: Some(cargo_home),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 引导器参数形态：静默 + stable 工具链 + 不动 PATH；Windows 加 msvc host（对齐 set-rust.ps1）。
    #[cfg(windows)]
    #[test]
    fn 引导器参数_形态对齐脚本() {
        assert_eq!(
            init_args(),
            vec![
                "-y".to_string(),
                "--default-toolchain".to_string(),
                "stable".to_string(),
                "--default-host".to_string(),
                "x86_64-pc-windows-msvc".to_string(),
                "--no-modify-path".to_string(),
            ]
        );
    }

    /// POSIX 引导器参数：host 三元组不自检写死（aarch64 linux 与 mac 同链路）。
    #[cfg(not(windows))]
    #[test]
    fn 引导器参数_posix不带host() {
        assert_eq!(
            init_args(),
            vec![
                "-y".to_string(),
                "--default-toolchain".to_string(),
                "stable".to_string(),
                "--no-modify-path".to_string(),
            ]
        );
    }

    /// cargo 镜像配置：crates-io 替换源 + rsproxy sparse + git cli + http 两开关（脚本语义关键标志）。
    /// A13 断言 `grep -c rsproxy` ≥ 3：replace-with、节名、sparse URL 三处命中。
    #[test]
    fn cargo镜像_关键标志齐全() {
        assert!(CARGO_CONFIG.contains(r#"replace-with = "rsproxy""#));
        assert!(CARGO_CONFIG.contains(r#"registry = "sparse+https://rsproxy.cn/index/""#));
        assert!(CARGO_CONFIG.contains("git-fetch-with-cli = true"));
        assert!(CARGO_CONFIG.contains("check-revoke = false"));
        assert!(CARGO_CONFIG.contains("multiplexing = true"));
        assert!(
            CARGO_CONFIG.matches("rsproxy").count() >= 3,
            "rsproxy 关键标志至少三处（wsl 总台 A13 判据）"
        );
    }

    /// 安装根双形态：Windows 重定位 EnvRoot；POSIX 尊重既有变量、缺省走系统标准位（R010）。
    #[test]
    fn 安装根_平台形态() {
        let root = Path::new("/env-root-or-d:\\ohmyenv");
        if cfg!(windows) {
            assert!(rustup_home(root).starts_with(root));
            assert!(cargo_home(root).starts_with(root));
            assert!(rustc_exe(root).ends_with("rustc.exe"));
        } else {
            // 默认位断言须清进程变量（本用例独占这两个变量，无并行竞用）
            std::env::remove_var("RUSTUP_HOME");
            std::env::remove_var("CARGO_HOME");
            assert!(
                rustup_home(root).ends_with(".rustup"),
                "POSIX 缺省应为 ~/.rustup"
            );
            assert!(
                cargo_home(root).ends_with(".cargo"),
                "POSIX 缺省应为 ~/.cargo"
            );
            assert!(rustc_exe(root).ends_with("rustc"), "POSIX 无 .exe 后缀");
            // 尊重既有（对线 R3）：进程变量优先于默认位
            std::env::set_var("RUSTUP_HOME", "/custom/rustup");
            std::env::set_var("CARGO_HOME", "/custom/cargo");
            assert!(
                rustup_home(root).starts_with("/custom/rustup"),
                "既有 RUSTUP_HOME 应被尊重"
            );
            assert!(
                cargo_home(root).starts_with("/custom/cargo"),
                "既有 CARGO_HOME 应被尊重"
            );
            std::env::remove_var("RUSTUP_HOME");
            std::env::remove_var("CARGO_HOME");
        }
    }
}
