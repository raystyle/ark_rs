//! docker：Windows 容器 Docker Engine 接管（自 ohmypwsh set-docker.ps1 完整迁移，2026-09-02）。
//! 形态：官方 static zip（download.docker.com CDN 直链，pin 驱动，dotnet/oscdimg 同族）+
//! Windows 服务注册 + daemon.json 合并 + docker-users 组 + compose 插件 + 机器级 PATH。
//! 需管理员：未提权且 gsudo 在位时经 gsudo 重跑 `ark install docker`（vsbuild 同模式）。
//! 与 vsbuild 的差异：docker 有版本与 pin（非 evergreen），resolve 走 cdn_url 分支。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::catalog::Tool;
use crate::download;
use crate::install::{InstallAction, InstallOutcome};
use crate::platform;
use crate::resolve::Resolution;
use crate::toolver;

const COMPOSE_VERSION: &str = "v5.5.0";
const COMPOSE_ASSET: &str = "docker-compose-windows-x86_64.exe";
const COMPOSE_URL: &str =
    "https://github.com/docker/compose/releases/download/{ver}/docker-compose-windows-x86_64.exe";
const SERVICE: &str = "docker";
const GROUP: &str = "docker-users";

/// 是否 docker 专用安装条目（extract 标记分发）。
pub fn is_docker(tool: &Tool) -> bool {
    tool.extract() == Some("docker-win")
}

/// 安装目录：<EnvRoot>\docker。
fn install_root(env_root: &Path) -> PathBuf {
    env_root.join("docker")
}

/// docker 安装主流程（cmd_install 在 resolve 之后分发到此）。
/// download：只解压二进制（无需管理员）。deploy（configure）：服务、组、daemon.json、PATH。
#[cfg(windows)]
pub fn install(
    def: &Tool,
    env_root: &Path,
    r: &Resolution,
    configure: bool,
) -> Result<InstallOutcome, String> {
    let bin_dir = install_root(env_root).join("bin");
    let docker_exe = bin_dir.join("docker.exe");

    let want = r.version.clone();
    let already = toolver::installed_version(&docker_exe, def).as_deref() == Some(want.as_str());

    if already && !configure {
        eprintln!("[INFO] docker {want} 已安装，跳过（download 不改服务与 PATH）");
        return Ok(InstallOutcome {
            action: InstallAction::Skipped,
            version: want,
            dir: Some(install_root(env_root)),
        });
    }

    // 服务 / 组 / daemon.json / 机器 PATH 要管理员；纯落盘不提权
    if configure && !platform::is_elevated() {
        return relaunch_elevated(def, env_root);
    }

    if !already {
        std::fs::create_dir_all(&bin_dir).map_err(|e| format!("创建目录失败: {e}"))?;

        let url = def
            .cdn_url()
            .ok_or_else(|| "docker 条目缺少 cdn_url 字段".to_string())?
            .replace("{version}", &r.version);
        let sha = def.pin_sha256();
        let zip = download::download_asset_with_mirror(
            env_root,
            &r.asset_name,
            &url,
            sha,
            false,
            "docker",
            &r.version,
        )?;
        extract_docker_bin(&zip, &bin_dir)?;
        eprintln!("[OK] docker/dockerd 就绪: {}", bin_dir.display());
        install_compose_plugin(env_root)?;
    }

    if configure {
        ensure_group()?;
        ensure_daemon_json(env_root)?;
        warn_if_containers_disabled();
        ensure_service(env_root, &bin_dir)?;
        ensure_cli_plugins_extra_dirs(env_root)?;
        ensure_machine_path(def, env_root, &bin_dir)?;
    }

    let version =
        toolver::installed_version_retried(&docker_exe, def).unwrap_or_else(|| r.version.clone());
    if already {
        eprintln!("[INFO] docker {version} 已安装，已核对服务与 PATH");
        return Ok(InstallOutcome {
            action: InstallAction::Skipped,
            version,
            dir: Some(install_root(env_root)),
        });
    }
    eprintln!("[OK] docker 安装完成: {version}");
    Ok(InstallOutcome {
        action: InstallAction::Installed,
        version,
        dir: Some(install_root(env_root)),
    })
}

/// 非 Windows 占位：docker-win 仅 Windows 语义。
///
/// # Errors
/// 非 Windows 平台一律返回 Err（docker 接管仅 Windows）。
#[cfg(not(windows))]
pub fn install(
    _def: &Tool,
    _env_root: &Path,
    _r: &Resolution,
    _configure: bool,
) -> Result<InstallOutcome, String> {
    Err("docker-win 安装类型仅在 Windows 可用".to_string())
}

/// 未提权时经 gsudo 重跑 `ark install docker`。
#[cfg(windows)]
fn relaunch_elevated(def: &Tool, env_root: &Path) -> Result<InstallOutcome, String> {
    let gsudo = which::which("gsudo").map_err(|_| {
        "docker 安装需管理员：以管理员终端重跑 `ark install docker`，或先 ark install gsudo 后自动提权"
            .to_string()
    })?;
    let exe = std::env::current_exe().map_err(|e| format!("获取当前 exe 失败: {e}"))?;
    eprintln!("[INFO] 需要管理员，经 gsudo 提权重跑: install docker");
    let status = Command::new(&gsudo)
        .arg(&exe)
        .arg("--env-root")
        .arg(env_root)
        .arg("install")
        .arg("docker")
        .status()
        .map_err(|e| format!("gsudo 启动失败: {}: {e}", gsudo.display()))?;
    if !status.success() {
        return Err(format!(
            "gsudo 提权执行失败 exit={}",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(InstallOutcome {
        action: InstallAction::Installed,
        version: toolver::installed_version(
            &install_root(env_root).join("bin").join("docker.exe"),
            def,
        )
        .unwrap_or_else(|| "unknown".to_string()),
        dir: Some(install_root(env_root)),
    })
}

/// 解压 zip 内 docker/docker.exe 与 docker/dockerd.exe 到 bin 目录（zip 顶层带 docker/ 包裹层）。
#[cfg(windows)]
fn extract_docker_bin(zip: &Path, bin_dir: &Path) -> Result<(), String> {
    let f = std::fs::File::open(zip).map_err(|e| format!("打开 zip 失败: {e}"))?;
    let mut archive = zip::ZipArchive::new(f).map_err(|e| format!("读 zip 失败: {e}"))?;
    for name in ["docker/docker.exe", "docker/dockerd.exe"] {
        let mut entry = archive
            .by_name(name)
            .map_err(|e| format!("官方 zip 缺少 {name}: {e}"))?;
        let dest = bin_dir.join(name.rsplit('/').next().unwrap_or(""));
        let mut out = std::fs::File::create(&dest).map_err(|e| format!("写文件失败: {e}"))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| format!("解压失败: {e}"))?;
    }
    Ok(())
}

/// docker-users 组与当前用户成员（net localgroup，幂等）。
#[cfg(windows)]
fn ensure_group() -> Result<(), String> {
    let run = |args: &[&str]| -> std::io::Result<std::process::Output> {
        Command::new("net").args(["localgroup"]).args(args).output()
    };
    let exists = run(&[GROUP]).map(|o| o.status.success()).unwrap_or(false);
    if !exists {
        run(&["/add", GROUP]).map_err(|e| format!("建组 {GROUP} 失败: {e}"))?;
    }
    let user = std::env::var("USERNAME").unwrap_or_default();
    if !user.is_empty() {
        let _ = run(&[GROUP, &user]);
    }
    Ok(())
}

/// daemon.json：保留用户键，仅确保 data-root 与缺省键（serde_json preserve_order 保持键序）。
#[cfg(windows)]
fn ensure_daemon_json(env_root: &Path) -> Result<(), String> {
    let path = PathBuf::from(r"C:\ProgramData\docker\config\daemon.json");
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("建目录失败: {e}"))?;
    }
    let mut obj: serde_json::Map<String, serde_json::Value> = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let mut changed = false;
    let data_root = env_root.join("docker-data").display().to_string();
    if obj.get("data-root").and_then(|v| v.as_str()) != Some(&data_root) {
        obj.insert("data-root".into(), serde_json::json!(data_root));
        changed = true;
    }
    for (k, v) in [
        ("group", serde_json::json!(GROUP)),
        ("exec-opts", serde_json::json!(["isolation=process"])),
    ] {
        if !obj.contains_key(k) {
            obj.insert(k.into(), v);
            changed = true;
        }
    }
    if changed {
        let text = serde_json::to_string_pretty(&serde_json::Value::Object(obj))
            .map_err(|e| format!("序列化 daemon.json 失败: {e}"))?
            + "\n";
        std::fs::write(&path, text).map_err(|e| format!("写 daemon.json 失败: {e}"))?;
        eprintln!("[OK] daemon.json 已更新: {}", path.display());
    } else {
        eprintln!("[INFO] daemon.json 已就绪（保留现有配置）");
    }
    Ok(())
}

/// Containers Windows 功能未启用时告警（不自动改系统功能）。
#[cfg(windows)]
fn warn_if_containers_disabled() {
    let out = Command::new("dism.exe")
        .args(["/Online", "/Get-FeatureInfo", "/FeatureName:Containers"])
        .output();
    if let Ok(o) = out {
        let text = String::from_utf8_lossy(&o.stdout);
        if !(text.contains("Enabled") || text.contains("已启用") || text.contains("挂起")) {
            eprintln!("[WARN] Windows 功能 Containers 未确认启用，docker 服务可能无法启动");
        }
    }
}

/// 服务：旧指向（非本 EnvRoot）先卸载再注册；自启 + 启动 + 状态核对。
#[cfg(windows)]
fn ensure_service(env_root: &Path, bin_dir: &Path) -> Result<(), String> {
    let qc = Command::new("sc.exe").args(["qc", SERVICE]).output();
    let qc_text = qc
        .map(|o| {
            format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            )
        })
        .unwrap_or_default();
    let exists = !(qc_text.contains("1060") || qc_text.contains("does not exist"));
    let points_to_bin = qc_text.contains(&bin_dir.display().to_string());

    if exists && !points_to_bin {
        eprintln!("[INFO] 检测到旧指向的 docker 服务，卸载后重注册");
        Command::new("sc.exe")
            .args(["delete", SERVICE])
            .output()
            .map_err(|e| format!("sc delete 失败: {e}"))?;
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    if !exists || !points_to_bin {
        let dockerd = bin_dir.join("dockerd.exe");
        let status = Command::new(&dockerd)
            .arg("--register-service")
            .status()
            .map_err(|e| format!("dockerd --register-service 启动失败: {e}"))?;
        if !status.success() {
            return Err(format!(
                "dockerd --register-service 失败 exit={}",
                status.code().unwrap_or(-1)
            ));
        }
    }
    Command::new("sc.exe")
        .args(["config", SERVICE, "start=", "auto"])
        .output()
        .map_err(|e| format!("sc config 失败: {e}"))?;
    Command::new("sc.exe")
        .args(["start", SERVICE])
        .output()
        .map_err(|e| format!("sc start 失败: {e}"))?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    let query = Command::new("sc.exe").args(["query", SERVICE]).output();
    let text = query
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    if !text.contains("RUNNING") {
        eprintln!("[WARN] docker 服务未运行（刚启用 Containers 功能的机器需重启一次）");
    }
    let _ = env_root;
    Ok(())
}

/// compose 插件：下载官方 .exe 与 .sha256 校验后落 cli-plugins。
#[cfg(windows)]
fn install_compose_plugin(env_root: &Path) -> Result<(), String> {
    let plugins_dir = install_root(env_root).join("cli-plugins");
    let dest = plugins_dir.join("docker-compose.exe");
    std::fs::create_dir_all(&plugins_dir).map_err(|e| format!("建目录失败: {e}"))?;
    if dest.exists() {
        return Ok(());
    }
    let url = COMPOSE_URL.replace("{ver}", COMPOSE_VERSION);
    let sha_url = format!("{url}.sha256");
    // compose sha 清单每次取新（force 跳缓存复用）；D44 对线 G2：镜像优先链统一
    // （镜像未播 compose 即秒回落官方，无静默例外）
    let sha_file = download::download_asset_with_mirror(
        env_root,
        &format!("{COMPOSE_ASSET}.sha256"),
        &sha_url,
        None,
        true,
        "docker-compose",
        "latest",
    )?;
    let sha_text =
        std::fs::read_to_string(&sha_file).map_err(|e| format!("读 sha 清单失败: {e}"))?;
    let sha = extract_sha64(&sha_text)?;
    let exe = download::download_asset_with_mirror(
        env_root,
        COMPOSE_ASSET,
        &url,
        Some(&sha),
        false,
        "docker-compose",
        "latest",
    )?;
    std::fs::copy(&exe, &dest).map_err(|e| format!("落插件失败: {e}"))?;
    eprintln!("[OK] compose 插件就绪: {}", dest.display());
    Ok(())
}

/// 从 sha 清单文本提取首个 64 位 hex（compose 官方清单格式「<sha>  <file>」）。
fn extract_sha64(text: &str) -> Result<String, String> {
    let mut cur = String::new();
    for c in text.chars() {
        if c.is_ascii_hexdigit() {
            cur.push(c);
            if cur.len() == 64 {
                return Ok(cur.to_uppercase());
            }
        } else if !cur.is_empty() {
            cur.clear();
        }
    }
    Err(format!("sha 清单未找到 64 位 hex: {text}"))
}

/// ~/.docker/config.json 追加 cliPluginsExtraDirs（插件发现不走环境变量，S017）。
#[cfg(windows)]
fn ensure_cli_plugins_extra_dirs(env_root: &Path) -> Result<(), String> {
    let config = dirs::home_dir()
        .ok_or_else(|| "无 HOME".to_string())?
        .join(".docker")
        .join("config.json");
    if let Some(dir) = config.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("建目录失败: {e}"))?;
    }
    let mut obj: serde_json::Map<String, serde_json::Value> = std::fs::read(&config)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let plugins_dir = install_root(env_root)
        .join("cli-plugins")
        .display()
        .to_string();
    let extra = obj
        .get_mut("cliPluginsExtraDirs")
        .and_then(|v| v.as_array_mut());
    if let Some(arr) = extra {
        let known = arr.iter().any(|v| v.as_str() == Some(&plugins_dir));
        if known {
            eprintln!("[INFO] cliPluginsExtraDirs 已含插件目录");
            return Ok(());
        }
        arr.push(serde_json::json!(plugins_dir));
    } else {
        obj.insert(
            "cliPluginsExtraDirs".into(),
            serde_json::json!([plugins_dir]),
        );
    }
    let text = serde_json::to_string_pretty(&serde_json::Value::Object(obj))
        .map_err(|e| format!("序列化 config.json 失败: {e}"))?
        + "\n";
    std::fs::write(&config, text).map_err(|e| format!("写 config.json 失败: {e}"))?;
    eprintln!("[OK] cliPluginsExtraDirs 已追加: {plugins_dir}");
    Ok(())
}

/// 机器级 PATH 前置 docker bin（幂等）。
#[cfg(windows)]
fn ensure_machine_path(def: &Tool, env_root: &Path, bin_dir: &Path) -> Result<(), String> {
    if platform::machine_path_contains(bin_dir)? {
        return Ok(());
    }
    if !platform::is_elevated() {
        // 已装且只差 PATH 的轻路径：仍需提权，走整体重跑
        return relaunch_elevated(def, env_root).map(|_| ());
    }
    platform::machine_path_add(&[bin_dir.to_path_buf()])?;
    eprintln!("[OK] 机器 PATH 已合并（新终端生效）");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha清单_提取64位hex() {
        let line = "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789  docker-compose-windows-x86_64.exe\n";
        assert_eq!(
            extract_sha64(line).unwrap(),
            "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789"
        );
        assert!(extract_sha64("no hex here").is_err());
    }
}
