//! delegate：家族自研 CLI 的 update 委托腿（用户裁定 2026-09-19「我们自己维护的
//! 自升级的 CLI，ark 对齐也走 CLI 自升级渠道；其余不是我们维护的 CLI 对齐 omc 维护 env」）。
//! 家族五员（ark 自身、hst、browse、reader、officecli）在 `ark update <tool>` 面委托
//! 调用该 CLI 自身的自升级命令，吃其独立域通道与既有锚校验、回滚与自证（家族标准形）；
//! ark 不为五员自建镜像下载腿。其余非自研工具照旧镜像优先加官方兜底（D44 面不变）。
//! 让位契约联动（家族仓零改动）：家族自更新检出 ark-managed 落痕即拒自升，故委托前
//! 临时撤落痕（rename 挪开）、自升级完成（含失败）后恢复落痕。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::catalog::Tool;
use crate::toolver;

/// 委托腿结果。
pub struct DelegateOutcome {
    /// 家族 CLI 自升级进程是否成功退出
    pub ok: bool,
    /// 委托前探活版本（action 裁定对照面）
    pub version_before: Option<String>,
    /// 升级后探活版本（探测失败为 None）
    pub version: Option<String>,
    /// 委托的命令行（展示面）
    pub via: String,
}

/// 家族五员自升级命令路由（exe 后参数；改令 2026-09-19）：返回 None 即走镜像腿。
/// ark 自身是自管条目（update 面 is_ark_self 分支提示 `ark self update`），不入本表。
/// officecli 无用户面自升级子命令，其内部自升级入口 `__update-check__`（检新并自更，
/// OFFICECLI_SKIP_UPDATE 不设即执行）为委托命令（其仓 UpdateChecker 在册契约）。
pub fn self_update_args(tool: &str) -> Option<&'static [&'static str]> {
    match tool {
        "hst" => Some(&["self", "update"]),
        "browse" => Some(&["update"]),
        "reader" => Some(&["self", "update"]),
        "officecli" => Some(&["__update-check__"]),
        _ => None,
    }
}

/// 委托执行一次家族自升级：定位真身 exe → 临时撤 ark-managed 落痕 → 调家族自升级命令
/// （继承 stdio，家族自身日志为证）→ 恢复落痕（成功失败皆恢复）→ 探活新版本。
/// exe 缺位返回 Ok(None)（未装：调用方回落镜像安装腿做首装）。
///
/// # Errors
/// 返回 Err（人读原因串）当：定位 exe 路径失败或落痕恢复受阻（进程失败不 Err，走 ok=false）。
pub fn run(name: &str, def: &Tool, env_root: &Path) -> Result<Option<DelegateOutcome>, String> {
    let Some(args) = self_update_args(name) else {
        return Ok(None);
    };
    let locate = || {
        toolver::exe_path(def, env_root)
            .ok()
            .filter(|p| p.exists())
            .or_else(|| toolver::find_on_path(name).map(|p| p.to_path_buf()))
    };
    let exe: PathBuf = locate().ok_or_else(|| format!("{name} 无法定位真身 exe（委托腿需要）"))?;
    if !exe.exists() {
        return Ok(None); // 未装：回落镜像安装腿首装
    }
    let probe = || locate().and_then(|p| toolver::installed_version(&p, def));
    let version_before = probe();
    let exe_dir = exe
        .parent()
        .ok_or_else(|| format!("{name} exe 无父目录: {}", exe.display()))?;
    let via = format!("{name} {}", args.join(" "));
    // 撤落痕（家族让位判据即检 exe 同目录 ark-managed；挪开用 rename 保内容可原样恢复）
    let marker = exe_dir.join("ark-managed");
    let stashed = marker.exists().then(|| stash_marker(&marker));
    eprintln!("[INFO] {name} 家族自研 CLI，委托其自升级通道: {via}（临时撤 ark-managed 落痕让位）");
    let status = Command::new(&exe).args(args).status();
    // 恢复落痕（含失败路径：家族仓零改动契约靠 ark 侧复原）
    if let Some(stash) = stashed.as_ref() {
        restore_marker(stash, &marker)?;
    }
    let ok = status
        .map(|s| s.success())
        .map_err(|e| format!("启动 {via} 失败: {e}"))?;
    let version = probe();
    Ok(Some(DelegateOutcome {
        ok,
        version_before,
        version,
        via,
    }))
}

/// 落痕挪开（保内容原样恢复）：rename 至同目录隐藏名；内容读不出时记录 None 走重写恢复。
fn stash_marker(marker: &Path) -> PathBuf {
    let stash = marker.with_extension(format!("ark-managed.{}.bak", std::process::id()));
    if std::fs::rename(marker, &stash).is_ok() {
        return stash;
    }
    // rename 不动（Windows 竞态面）：读内容改走恢复重写
    PathBuf::from("")
}

/// 落痕恢复：stash 有效则 rename 回；空 stash 路径（rename 失败形）重写 ark 版本号。
fn restore_marker(stash: &Path, marker: &Path) -> Result<(), String> {
    if stash.as_os_str().is_empty() {
        std::fs::write(marker, format!("{}\n", env!("CARGO_PKG_VERSION")))
            .map_err(|e| format!("恢复 ark-managed 落痕失败: {}: {e}", marker.display()))?;
        return Ok(());
    }
    match std::fs::rename(stash, marker) {
        Ok(()) => Ok(()),
        Err(_) => {
            // rename 回失败（被占等）：stash 内容读出重写，读不出则以 ark 版本号落
            let content = std::fs::read_to_string(stash).ok();
            let body = content.unwrap_or_else(|| format!("{}\n", env!("CARGO_PKG_VERSION")));
            std::fs::write(marker, body)
                .map_err(|e| format!("恢复 ark-managed 落痕失败: {}: {e}", marker.display()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 改令验收 1：路由两面——家族员工具名路由到委托腿、非家族路由镜像腿（None）。
    #[test]
    fn 委托路由_家族与非家族两面() {
        // 期望值来源：四员本机 --help 实测（2026-09-19）：hst self update、browse update、
        /// reader self update、officecli __update-check__（内部自升级入口）
        assert_eq!(self_update_args("hst"), Some(&["self", "update"][..]));
        assert_eq!(self_update_args("browse"), Some(&["update"][..]));
        assert_eq!(self_update_args("reader"), Some(&["self", "update"][..]));
        assert_eq!(
            self_update_args("officecli"),
            Some(&["__update-check__"][..])
        );
        // 非家族（镜像腿）：ark 自研工具、env 镜像工具、随机名
        assert_eq!(self_update_args("ark"), None);
        assert_eq!(self_update_args("rg"), None);
        assert_eq!(self_update_args("jq"), None);
        assert_eq!(self_update_args("nonexistent"), None);
    }

    /// 落痕让位舞蹈（家族仓零改动方案）：委托执行期间落痕不在位、完成后恢复（含失败路径）。
    /// 用伪家族 CLI（脚本件）自证：脚本先断言落痕已撤（在则失败），再升级自身版本输出。
    #[test]
    fn 委托舞蹈_撤痕执行复痕() {
        let dir = tempfile::tempdir().expect("临时目录");
        // 伪真身：脚本 version 输出从 v1 升 v2；执行时断言 ark-managed 不在位
        let exe = dir.path().join("hst");
        std::fs::write(&exe, "#!/bin/sh\nif [ -f \"$(dirname \"$0\")/ark-managed\" ]; then echo 'marker still present' >&2; exit 9; fi\nver=$(cat \"$(dirname \"$0\")/ver\" 2>/dev/null || echo 1)\necho $((ver+1)) > \"$(dirname \"$0\")/ver\"\nexit 0\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(dir.path().join("ark-managed"), "1.4.1\n").unwrap();
        std::fs::write(dir.path().join("ver"), "1").unwrap();
        // catalog 条目：exe 直指沙盒（dir/bin/exe 相对 envroot 解析到脚本）
        let env_root = dir.path().join("envroot");
        std::fs::create_dir_all(env_root.join("bin")).unwrap();
        std::fs::copy(&exe, env_root.join("bin").join("hst")).unwrap();
        std::fs::set_permissions(
            env_root.join("bin").join("hst"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        std::fs::write(env_root.join("bin").join("ark-managed"), "1.4.1\n").unwrap();
        let def = Tool {
            linux_dir: Some("bin".into()),
            linux_bin: Some("bin".into()),
            linux_exe: Some("hst".into()),
            probe_pattern: Some("^v?(\\d+)".into()),
            ..Tool::default()
        };
        let out = run("hst", &def, &env_root)
            .expect("委托腿应可执行")
            .expect("hst 应路由委托腿");
        assert!(out.ok, "伪家族 CLI 应成功退出");
        // 落痕恢复在位（原内容）
        assert_eq!(
            std::fs::read_to_string(env_root.join("bin").join("ark-managed")).unwrap(),
            "1.4.1\n",
            "委托完成后落痕应原样恢复"
        );
        // 无 .bak 残留（目录枚举断言，glob 字面路径不展开）
        let leftovers: Vec<String> = std::fs::read_dir(env_root.join("bin"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".bak"))
            .collect();
        assert!(leftovers.is_empty(), "不应留 .bak 残留: {leftovers:?}");
    }
}
