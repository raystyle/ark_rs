//! selfdeploy：自部署——复制当前 exe 到用户程序目录（Windows `%LOCALAPPDATA%\Programs\ark`，
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
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
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

/// 清理数据目录已部署的旧 SKILL.md（D50 撤 skill 面的幂等收尾；仿旧别名清理模式）。
/// 在则删返回 true；不在返回 false（幂等静默；仅 NotFound 视为不在，其余元数据错误照常上抛）。
/// 旧位语义：metadata_dir 读回规则（ark 位未建且 ohmyenv 位在则读回旧位）使「仅旧位」机器
/// 删的是 ohmyenv/SKILL.md（与 deploy_catalog 同款收口先例）；ark 位在时旧位副本代码永不收，
/// 归一次性清扫与旧二进制退役收敛。
///
/// # Errors
/// 返回 Err（人读原因串）当：读取元数据或删除失败（文件占用等） 等（完整失败面见函数体错误构造）。
pub fn remove_legacy_skill() -> Result<bool, String> {
    remove_legacy_skill_at(&platform::metadata_dir().join("SKILL.md"))
}

/// remove_legacy_skill 的可测核（传目标路径，便于 tmpdir 断言三态）。
///
/// # Errors
/// 返回 Err（人读原因串）当：读取元数据或删除失败 等（完整失败面见函数体错误构造）。
pub fn remove_legacy_skill_at(dst: &Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(dst) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("读取 SKILL.md 元数据失败: {}: {e}", dst.display())),
        Ok(_) => {}
    }
    std::fs::remove_file(dst).map_err(|e| format!("删除 SKILL.md 失败: {}: {e}", dst.display()))?;
    Ok(true)
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
    if let Err(e) = platform::remove_legacy_alias() {
        eprintln!("[WARN] 旧别名清理失败（不拦部署，下次再收）: {e}");
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
    // D50 收口：skill 面撤除，顺带清理数据目录已部署的旧 SKILL.md（幂等 best-effort）
    match remove_legacy_skill() {
        Ok(true) => eprintln!("[OK] 已清理旧 SKILL.md（skill 面已撤，D50）"),
        Ok(false) => {}
        Err(e) => eprintln!("[WARN] 旧 SKILL.md 清理失败（不拦部署，下次再收）: {e}"),
    }
    Ok(SelfDeployOutcome {
        copied,
        path_registered,
        bin_dir,
        exe: dst,
        catalog,
    })
}

/// Linux / macOS：复制当前二进制到 `~/.local/bin/ark`，同步 catalog，并确保 `~/.local/bin` 在用户 PATH 中。
///
/// # Errors
/// 返回 Err（人读原因串）当：获取当前二进制路径失败 等（完整失败面见函数体错误构造）。
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
    if let Err(e) = platform::remove_legacy_alias() {
        eprintln!("[WARN] ome 别名清理失败（不拦部署，下次再收）: {e}");
    }
    let catalog = deploy_catalog()?;
    // D50 收口：skill 面撤除，顺带清理数据目录已部署的旧 SKILL.md（幂等 best-effort）
    match remove_legacy_skill() {
        Ok(true) => eprintln!("[OK] 已清理旧 SKILL.md（skill 面已撤，D50）"),
        Ok(false) => {}
        Err(e) => eprintln!("[WARN] 旧 SKILL.md 清理失败（不拦部署，下次再收）: {e}"),
    }
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

    #[test]
    fn remove_legacy_skill_三态() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let f = dir.path().join("SKILL.md");
        std::fs::write(&f, b"# old").map_err(|e| e.to_string())?;
        assert!(remove_legacy_skill_at(&f)?, "在则删应返 true");
        assert!(!remove_legacy_skill_at(&f)?, "不在应返 false（幂等静默）");
        // 目录占名：symlink_metadata 可读但 remove_file 必败，应进 Err 而非静默 Ok(false)
        std::fs::create_dir(&f).map_err(|e| e.to_string())?;
        assert!(remove_legacy_skill_at(&f).is_err(), "目录占名应进 Err");
        Ok(())
    }
}
