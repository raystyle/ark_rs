//! selfdeploy：自部署——复制当前 exe 到用户程序目录（Windows `%LOCALAPPDATA%\Programs\ome`，
//! Linux / macOS `~/.local/bin`），同步 catalog 到用户数据目录，并注册用户 PATH（幂等）。
//! Windows 顺带清理旧自部署位 `<EnvRoot>\ome\bin` 的 PATH 残留。
//! 幂等：目标与当前 exe 同路径则跳过复制；sha256 一致则跳过复制；PATH 注册由 envpath 幂等处理。

use std::path::{Path, PathBuf};

use crate::download::sha256_file;
use crate::platform;

/// 自部署结果。
pub struct SelfDeployOutcome {
    /// 部署位是否实际复制（幂等 false）
    pub copied: bool,
    /// bin 目录是否新注册 PATH
    pub path_registered: bool,
    /// 部署 bin 目录
    pub bin_dir: PathBuf,
    /// 部署位 exe 全路径
    pub exe: PathBuf,
    /// 同步到用户数据目录的 catalog 路径；无源可同步时为 None。
    pub catalog: Option<PathBuf>,
}

/// 复制 exe 到目标（纯文件逻辑，可测）：同路径跳过；sha256 一致跳过；否则覆盖复制。
/// 返回是否实际复制。
pub fn deploy_copy(src: &Path, dst: &Path) -> Result<bool, String> {
    let abs = |p: &Path| {
        std::path::absolute(p)
            .map(|x| x.to_string_lossy().to_lowercase())
            .map_err(|e| format!("取绝对路径失败: {}: {e}", p.display()))
    };
    if abs(src)? == abs(dst)? {
        return Ok(false); // 当前 exe 即目标（已在 bin 里运行）
    }
    if dst.exists() && sha256_file(src)? == sha256_file(dst)? {
        return Ok(false); // 内容一致，幂等跳过
    }
    if let Some(dir) = dst.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("创建目录失败: {}: {e}", dir.display()))?;
    }
    std::fs::copy(src, dst)
        .map_err(|e| format!("复制失败: {} -> {}: {e}", src.display(), dst.display()))?;
    Ok(true)
}

/// 同步 catalog 到用户数据目录 `<metadata>\catalog\tools.toml`（自部署即同步数据源，幂等）。
/// 当前活动 catalog 不存在（如任意目录运行无源二进制）时跳过，返回 None。
fn deploy_catalog() -> Result<Option<PathBuf>, String> {
    // D41 C：先把旧 ohmyenv 元数据七件套搬到新 ark 位（幂等 copy、旧位只读保留；
    // 搬完 metadata_dir 归位主名，本次同步落新位）
    if let Err(e) = platform::migrate_legacy_metadata() {
        eprintln!("[WARN] 元数据搬迁失败（继续读回旧位）: {e}");
    }
    // D41：POSIX profile 旧 ome env 块收口（Windows no-op）
    platform::migrate_legacy_env_block_once();
    let src = match crate::catalog::resolve_catalog_path() {
        Ok(p) if p.exists() => p,
        _ => return Ok(None),
    };
    let dst = platform::metadata_dir().join("catalog").join("tools.toml");
    let copied = deploy_copy(&src, &dst)?;
    // D34：签名件只在「与当前内容已不符」时撤掉。内容没变（self update 未动 catalog）就保留，
    // 免得同步后凭空报一份无签名运行态；内容变了（pin 回写或仓库改动）才撤，交由云端刷新补回。
    let sig = crate::catalog::signature_path(&dst);
    if sig.exists()
        && !matches!(
            crate::catalog::check_signature(&dst),
            crate::catalog::SignatureState::Valid
        )
    {
        let _ = std::fs::remove_file(&sig);
    }
    if copied {
        eprintln!(
            "[OK] 已同步 catalog: {} -> {}",
            src.display(),
            dst.display()
        );
    } else {
        eprintln!("[INFO] catalog 已是最新: {}", dst.display());
    }
    Ok(Some(dst))
}

/// 同步 SKILL.md 到用户数据目录（D09：agent 发现入口，自适应生成——本机实装清单与
/// 使用引导，非静态文件；生成快照随环境变化，`ark skill` 随时刷新，init 时顺带生成）。
pub fn deploy_skill() -> Result<PathBuf, String> {
    let dst = platform::metadata_dir().join("SKILL.md");
    std::fs::create_dir_all(dst.parent().unwrap_or(Path::new(".")))
        .map_err(|e| format!("创建数据目录失败: {e}"))?;
    // init 路径无现成渲染文本：写静态骨架（命令图与工作流），`ark skill` 再充实环境清单
    let want = include_str!("../SKILL.md");
    let cur = std::fs::read_to_string(&dst).unwrap_or_default();
    if cur != want {
        std::fs::write(&dst, want).map_err(|e| format!("写 SKILL.md 失败: {e}"))?;
        eprintln!("[OK] 已同步 SKILL.md: {}", dst.display());
    }
    Ok(dst)
}

/// 落盘自适应 SKILL 文本（cmd_skill 用；deploy_skill 的静态骨架仅作 init 兜底）。
pub fn write_skill(text: &str) -> Result<PathBuf, String> {
    let dst = platform::metadata_dir().join("SKILL.md");
    std::fs::create_dir_all(dst.parent().unwrap_or(Path::new(".")))
        .map_err(|e| format!("创建数据目录失败: {e}"))?;
    std::fs::write(&dst, text).map_err(|e| format!("写 SKILL.md 失败: {e}"))?;
    Ok(dst)
}

/// 自适应渲染环境 SKILL（D09）：本机实装依赖（十类分组、名称与版本）、类级使用引导、
/// 命令图与检测驱动工作流。agent 直读 stdout 或数据目录落盘件。
pub fn render_skill(cat: &crate::catalog::Catalog, env_root: &Path) -> Result<String, String> {
    let srows = crate::status::collect_status(cat, env_root)?;
    let mut out = String::new();
    out.push_str("# SKILL.md：ark 环境自适应清单\n\n> 由 `ark skill` 生成（快照随环境变化，缺什么 `ark install` 补）。\n\n");
    out.push_str("## 本机依赖与逐工具引导\n\n");
    for (cat_key, label) in crate::status::GROUPS {
        let group: Vec<&crate::status::StatusRow> =
            srows.iter().filter(|r| &r.category == cat_key).collect();
        if group.is_empty() {
            continue;
        }
        out.push_str(&format!("### {label}\n\n"));
        for r in &group {
            let def = match cat.tool(&r.name) {
                Ok(d) => d,
                Err(_) => continue,
            };
            // 三态标记：已装带版本；未装区分可装与本平台不适用空态
            let state = if let Some(v) = &r.installed {
                format!("已装 {v}")
            } else if crate::toolver::platform_managed(def) {
                "未装（`ark install` 补）".to_string()
            } else {
                "本平台不适用（空态）".to_string()
            };
            out.push_str(&format!("#### {} · {}\n\n", r.name, state));
            if !def.desc().is_empty() {
                out.push_str(&format!("- 用途：{}\n", def.desc()));
            }
            if r.installed.is_some() {
                if let Some(exe) = &r.exe {
                    out.push_str(&format!("- exe：{}\n", exe.display()));
                }
            }
            // env 实测（用户级优先、进程级兜底）；凭据类键只显在否不显值（doctor 同纪律）
            if !def.guide_env().is_empty() {
                let parts: Vec<String> = def
                    .guide_env()
                    .iter()
                    .map(|k| {
                        let up = k.to_uppercase();
                        let secret = up.contains("TOKEN")
                            || up.contains("KEY")
                            || up.contains("SECRET")
                            || up.contains("PASSWORD");
                        let val = crate::platform::get_user_env_var(k)
                            .ok()
                            .flatten()
                            .or_else(|| std::env::var(k).ok());
                        match val {
                            Some(_) if secret => format!("{k}=已设（值不显示）"),
                            Some(v) => format!("{k}={v}"),
                            None => format!("{k}=未设"),
                        }
                    })
                    .collect();
                out.push_str(&format!("- env：{}\n", parts.join("；")));
            }
            // 安装/数据目录实测（展开 ~ 与环境变量；在位与否如实标）
            if !def.guide_dirs().is_empty() {
                let parts: Vec<String> = def
                    .guide_dirs()
                    .iter()
                    .map(|d| {
                        let p = crate::platform::expand_install_path(
                            &crate::platform::expand_env_vars(d),
                        );
                        let mark = if Path::new(&p).exists() { "在" } else { "无" };
                        format!("{}（{mark}）", p.display())
                    })
                    .collect();
                out.push_str(&format!("- 目录：{}\n", parts.join("；")));
            }
            for line in def.guide_notes().lines().filter(|l| !l.trim().is_empty()) {
                out.push_str(&format!("- 注意：{}\n", line));
            }
            out.push('\n');
        }
    }
    out.push_str("## ark 命令与工作流\n\n- 诊断环境：`ark doctor`（系统/依赖两层 + check 节 + verdict 一锤定音：ready/degraded/broken）\n- 缺什么装什么：`ark install`（省略则全量，幂等，默认走 env.ohmygh.com 镜像、未命中回落官方）\n- 看三态：`ark status`；升级：`ark update`（省略则全量；agent 类走自更新）\n- 清单来源与刷新：`ark catalog`（看解析面与云端同步态；`ark catalog sync` 立即从云端刷新，新增软件不必换二进制）\n- 命令全图：`ark --llms`\n\n> 环境变化后重跑 `ark skill` 刷新本清单。\n");
    Ok(out)
}

/// 自部署：复制当前 exe 到用户程序目录，同步 catalog 到用户数据目录，注册 bin 目录进用户 PATH。
#[cfg(windows)]
pub fn self_deploy(env_root: &Path) -> Result<SelfDeployOutcome, String> {
    let src = std::env::current_exe().map_err(|e| format!("获取当前 exe 路径失败: {e}"))?;
    let dst = platform::self_deploy_target()?;
    let bin_dir = dst
        .parent()
        .ok_or("self-deploy 目标路径缺少父目录")?
        .to_path_buf();
    let copied = deploy_copy(&src, &dst)?;
    if copied {
        eprintln!("[OK] 已复制: {} -> {}", src.display(), dst.display());
    } else {
        eprintln!("[INFO] 目标已是最新，跳过复制: {}", dst.display());
    }
    let path_registered = platform::add_user_path(&bin_dir)?;
    // D41 C 收口（2026-09-14）：ome 别名停建，顺带清理既有副本（全舰队 ome 水位清零）
    if let Err(e) = platform::remove_ome_alias() {
        eprintln!("[WARN] ome 别名清理失败（不拦部署，下次再收）: {e}");
    }
    // 清理旧自部署位 <EnvRoot>\ome\bin 的 PATH 残留（一次性迁移，幂等）
    let legacy_bin = env_root.join("ome").join("bin");
    if platform::remove_user_path(&legacy_bin)? {
        eprintln!("[OK] 已移除旧 PATH 残留: {}", legacy_bin.display());
    }
    // D41 C：旧 ome 部署位纳管（Programs\ome）：PATH 条目移除加目录 best-effort 删除
    //（旧 exe 运行中会锁目录，删不动留待下次 init 收）
    if let Some(legacy) = platform::legacy_deploy_dir() {
        if platform::remove_user_path(&legacy)? {
            eprintln!("[OK] 已移除旧 ome 部署位 PATH 条目: {}", legacy.display());
        }
        if let Err(e) = std::fs::remove_dir_all(&legacy) {
            eprintln!(
                "[WARN] 旧 ome 部署位目录未删成（占用则下次 init 再收）: {}: {e}",
                legacy.display()
            );
        }
    }
    let catalog = deploy_catalog()?;
    let _skill = deploy_skill();
    Ok(SelfDeployOutcome {
        copied,
        path_registered,
        bin_dir,
        exe: dst,
        catalog,
    })
}

/// Linux / macOS：复制当前二进制到 `~/.local/bin/ark`，同步 catalog，并确保 `~/.local/bin` 在用户 PATH 中。
#[cfg(not(windows))]
pub fn self_deploy(_env_root: &Path) -> Result<SelfDeployOutcome, String> {
    let src = std::env::current_exe().map_err(|e| format!("获取当前二进制路径失败: {e}"))?;
    let dst = platform::self_deploy_target()?;
    let bin_dir = dst
        .parent()
        .ok_or("self-deploy 目标路径缺少父目录")?
        .to_path_buf();
    let copied = deploy_copy(&src, &dst)?;
    if copied {
        eprintln!("[OK] 已复制: {} -> {}", src.display(), dst.display());
    } else {
        eprintln!("[INFO] 目标已是最新，跳过复制: {}", dst.display());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&dst)
            .map_err(|e| format!("读取权限失败: {}: {e}", dst.display()))?
            .permissions();
        perm.set_mode(perm.mode() | 0o755);
        std::fs::set_permissions(&dst, perm)
            .map_err(|e| format!("设置可执行权限失败: {}: {e}", dst.display()))?;
    }
    let path_registered = platform::add_user_path(&bin_dir)?;
    // D41 C 收口（2026-09-14）：ome 别名停建，顺带清理既有副本（POSIX 落点 ~/.local/bin/ome）
    if let Err(e) = platform::remove_ome_alias() {
        eprintln!("[WARN] ome 别名清理失败（不拦部署，下次再收）: {e}");
    }
    let catalog = deploy_catalog()?;
    let _skill = deploy_skill();
    Ok(SelfDeployOutcome {
        copied,
        path_registered,
        bin_dir,
        exe: dst,
        catalog,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deploy_copy_复制与幂等() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let src = dir.path().join("src.exe");
        let dst = dir.path().join("bin").join("ome.exe");
        std::fs::write(&src, b"v1-binary").map_err(|e| e.to_string())?;

        assert!(deploy_copy(&src, &dst)?, "首次应复制");
        assert_eq!(
            std::fs::read(&dst).map_err(|e| e.to_string())?,
            b"v1-binary"
        );

        assert!(!deploy_copy(&src, &dst)?, "sha 一致应跳过");

        std::fs::write(&src, b"v2-binary").map_err(|e| e.to_string())?;
        assert!(deploy_copy(&src, &dst)?, "内容变化应覆盖复制");
        assert_eq!(
            std::fs::read(&dst).map_err(|e| e.to_string())?,
            b"v2-binary"
        );
        Ok(())
    }

    #[test]
    fn deploy_copy_同路径跳过() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let exe = dir.path().join("ome.exe");
        std::fs::write(&exe, b"self").map_err(|e| e.to_string())?;
        assert!(!deploy_copy(&exe, &exe)?, "同路径应跳过（自复制）");
        Ok(())
    }
}
