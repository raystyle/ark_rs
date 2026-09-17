//! platform：跨平台抽象层。
//!
//! Windows：EnvRoot 为 `D:\ohmyenv` 或 `C:\ohmyenv`，工具集中安装；PATH 通过注册表管理。
//! Linux / macOS：各软件按系统标准目录安装；PATH 通过当前 shell 的 profile 文件（如 `~/.bashrc`）管理。
//! ark 自身与 EnvRoot 解耦：二进制进用户程序目录（Windows `%LOCALAPPDATA%\Programs\ark`）、
//! 元数据进用户数据目录（Windows `%LOCALAPPDATA%\ark`；Linux `~/.local/share/ark`；
//! macOS `~/Library/Application Support/ark`）；迁移过渡期旧 `ohmyenv` 目录在而新目录未建时读回旧位。

use std::path::{Path, PathBuf};

/// 默认 EnvRoot：显式参数与环境变量已在 `catalog::resolve_env_root` 处理，此处仅返回平台默认值。
pub fn default_env_root() -> PathBuf {
    #[cfg(windows)]
    {
        if Path::new(r"D:\").exists() {
            PathBuf::from(r"D:\ohmyenv")
        } else {
            PathBuf::from(r"C:\ohmyenv")
        }
    }
    #[cfg(not(windows))]
    {
        data_dir().join("ohmyenv")
    }
}

/// ark 自身元数据目录（独立 app 数据目录，与 EnvRoot 解耦）：
/// Windows `%LOCALAPPDATA%\ark`；Linux `~/.local/share/ark`；macOS `~/Library/Application Support/ark`。
/// D41 迁移过渡：新 `ark` 目录未建而旧 `ohmyenv` 目录在（旧二进制所写）时读回旧位，
/// 令新旧二进制共享同一份 catalog 与 seq 记录；C 阶段搬迁七件套后自然归位主名。
pub fn metadata_dir() -> PathBuf {
    resolve_metadata_dir(&data_dir())
}

/// 元数据目录解析（传 data 便于测）：主名 `ark`；未建且旧 `ohmyenv` 在则读回旧位。
fn resolve_metadata_dir(data: &Path) -> PathBuf {
    let ark = data.join("ark");
    if !ark.exists() {
        let old = data.join("ohmyenv");
        if old.exists() {
            return old;
        }
    }
    ark
}

/// 旧元数据目录（D41 迁移源 `<data>\ohmyenv`；只在存在时 Some，旧目录始终只读保留）。
pub fn legacy_metadata_dir() -> Option<PathBuf> {
    let old = data_dir().join("ohmyenv");
    old.exists().then_some(old)
}

/// 元数据七件套（D41 C 搬迁清单：catalog 域运行态全套）。
pub const METADATA_MIGRATION_FILES: [&str; 7] = [
    "tools.toml",
    "tools.toml.minisig",
    "manifest.toml",
    "manifest.toml.minisig",
    ".tools.toml.seq",
    ".manifest.toml.seq",
    ".last-sync",
];

/// 元数据七件套搬迁（D41 C）：旧 `ohmyenv\catalog` 在而新 `ark\catalog` 缺件时逐件复制
/// （copy 不 move：旧目录只读保留，旧二进制并行期仍读旧位；幂等：新位在即跳过）。
/// 返回是否有搬迁动作。init 与 self update 后接（搬迁后 metadata_dir 自然归位主名）。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn migrate_legacy_metadata() -> Result<bool, String> {
    let Some(old) = legacy_metadata_dir() else {
        return Ok(false);
    };
    migrate_legacy_metadata_in(&old, &data_dir().join("ark"))
}

/// 搬迁核心（传新旧根便于测）。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn migrate_legacy_metadata_in(old: &Path, new: &Path) -> Result<bool, String> {
    let from = old.join("catalog");
    let to = new.join("catalog");
    if !from.is_dir() {
        return Ok(false);
    }
    let mut changed = false;
    for name in METADATA_MIGRATION_FILES {
        let (src, dst) = (from.join(name), to.join(name));
        if src.exists() && !dst.exists() {
            std::fs::create_dir_all(&to)
                .map_err(|e| format!("建目录失败: {}: {e}", to.display()))?;
            std::fs::copy(&src, &dst)
                .map_err(|e| format!("搬迁失败: {} -> {}: {e}", src.display(), dst.display()))?;
            changed = true;
        }
    }
    Ok(changed)
}

/// 可执行文件后缀。
pub fn exe_suffix() -> &'static str {
    #[cfg(windows)]
    {
        ".exe"
    }
    #[cfg(not(windows))]
    {
        ""
    }
}

/// 展开安装路径中的 `~` 与环境变量引用。
pub fn expand_install_path(raw: &str) -> PathBuf {
    if raw.starts_with("~/") || raw == "~" {
        if let Some(home) = dirs::home_dir() {
            return if raw == "~" {
                home
            } else {
                home.join(&raw[2..])
            };
        }
    }
    PathBuf::from(expand_env_vars(raw))
}

/// 展开后的路径若为相对路径则拼到 EnvRoot 下（Windows 名录是相对 dir/bin；Linux/macOS 多为 ~/ 绝对）。
pub fn join_if_relative(env_root: &Path, p: PathBuf) -> PathBuf {
    if p.is_absolute() {
        p
    } else {
        env_root.join(p)
    }
}

/// PATH 环境变量条目分隔符。
pub fn path_separator() -> char {
    #[cfg(windows)]
    {
        ';'
    }
    #[cfg(not(windows))]
    {
        ':'
    }
}

/// 用户级本地数据目录（Windows `%LOCALAPPDATA%`；Linux XDG_DATA_HOME 或 `~/.local/share`；macOS `~/Library/Application Support`）。
fn data_dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .expect("无法确定用户主目录")
            .join(".local")
            .join("share")
    })
}

/// 自部署目标路径（D41：ark 接管部署位；旧 ome 位与 `ome` 别名已于 2026-09-14 收口停建）。
/// Windows：`%LOCALAPPDATA%\Programs\ark\ark.exe`
/// Linux / macOS：`~/.local/bin/ark`
///
/// # Errors
/// 返回 Err（人读原因串）当：{}: {e} 等（完整失败面见函数体错误构造）。
pub fn self_deploy_target() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        Ok(data_dir().join("Programs").join("ark").join("ark.exe"))
    }
    #[cfg(not(windows))]
    {
        let bin_dir = dirs::home_dir()
            .ok_or("无法确定用户主目录")?
            .join(".local")
            .join("bin");
        Ok(bin_dir.join("ark"))
    }
}

/// 读环境变量：空白值视同未设，返回 None（旧名读回已随 ome 关键字剔除批撤除，2026-09-18）。
pub fn env_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// 旧 ome 部署位（D41 C 接管清单：Windows `Programs\ome`，在位时 Some）。
/// POSIX 旧位 `~/.local/bin/ome` 即别名落点，由 `remove_legacy_alias` 清理。
pub fn legacy_deploy_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let d = data_dir().join("Programs").join("ome");
        d.exists().then_some(d)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// `ome` 别名落点（D41 C 过渡载体已停建，2026-09-14 全舰队 ome 水位清零收口；
/// 落点保留供 `remove_legacy_alias` 定位清理既有副本）。
///
/// # Errors
/// 返回 Err（人读原因串）当：{}: {e} 等（完整失败面见函数体错误构造）。
pub fn legacy_alias_target() -> Result<PathBuf, String> {
    let t = self_deploy_target()?;
    let name = if cfg!(windows) { "ome.exe" } else { "ome" };
    Ok(t.with_file_name(name))
}

/// 清理 `ome` 别名副本（幂等：不在位返回 false 静默；best-effort：失败返回 Err 由调用方告警，
/// Windows 文件占用时留待下次 init / self update 再收）。
///
/// # Errors
/// 返回 Err（人读原因串）当：{}: {e} 等（完整失败面见函数体错误构造）。
pub fn remove_legacy_alias() -> Result<bool, String> {
    let alias = legacy_alias_target()?;
    remove_file_if_exists(&alias)
}

/// 删除文件，不存在视为已清理（幂等）。返回是否实际删除。
fn remove_file_if_exists(path: &Path) -> Result<bool, String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// 用户面写入总闸门（测试隔离）：`ARK_TEST_NO_PATH_REG=1` 时，**所有**用户环境写入面
/// 一律跳过（PATH 注册、用户级变量、profile 钩子、用户 bin 直链）。变量名沿历史（PATH 注册
/// 是最早的那面），覆盖面已扩到四面——只守一面会让沙盒测试从别的门漏进真实环境。
pub fn user_env_write_blocked() -> bool {
    env_var("ARK_TEST_NO_PATH_REG").as_deref() == Some("1")
}

/// 路径是否位于系统临时目录下。闸的是「注册面」不是安装面：Temp 下装得
/// （staging 本就常用 Temp），持久 PATH 不进。
fn is_temp_path(dir: &Path) -> bool {
    normalize(dir).starts_with(normalize(&std::env::temp_dir()))
}

/// 尽力归一（macOS 实证：`$TMPDIR` 是 `/var/folders/...`，真身在 `/private/var/...`；
/// 不存在的子路径直接 canonicalize 会失败退字面，符号链接前缀对不上——从深往浅找
/// 第一个可 canonicalize 的祖先，规范后拼回余段）。
fn normalize(p: &Path) -> PathBuf {
    let mut ancestor = p.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        match ancestor.canonicalize() {
            Ok(c) => {
                let mut out = c;
                for seg in tail.iter().rev() {
                    out = out.join(seg);
                }
                return out;
            }
            Err(_) => match (ancestor.parent(), ancestor.file_name()) {
                (Some(parent), Some(name)) => {
                    tail.push(name.to_os_string());
                    ancestor = parent.to_path_buf();
                }
                _ => return p.to_path_buf(),
            },
        }
    }
}

/// 将 dir 注册进用户 PATH；返回是否实际新增。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn add_user_path(dir: &Path) -> Result<bool, String> {
    // O5（S017）：测试隔离开关——沙盒 EnvRoot 的集成测试不得污染真实注册面/Profile
    // （M002「沙盒漏写面」同型回归的根治；与 ARK_TEST_REAL/MIRROR 同为闸门口径）
    if user_env_write_blocked() {
        eprintln!("[INFO] ARK_TEST_NO_PATH_REG=1：跳过用户 PATH 注册（测试隔离）");
        return Ok(false);
    }
    // temp 闸（2026-09-14，ohmycloud 舰队回执报障）：临时 envroot 的探测/测试装不得把
    // Temp 段写进用户持久 PATH（lan-win 实证：注册表沉淀 Temp 下 zig 与 jq 多条）。
    if is_temp_path(dir) {
        eprintln!(
            "[WARN] 临时目录不注册用户 PATH（探测/测试装不落持久面）: {}",
            dir.display()
        );
        return Ok(false);
    }
    #[cfg(windows)]
    {
        windows::add_user_path(&dir.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        unix::add_user_path(dir)
    }
}

/// 从用户 PATH 移除 dir；返回是否实际移除。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn remove_user_path(dir: &Path) -> Result<bool, String> {
    #[cfg(windows)]
    {
        windows::remove_user_path(&dir.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        unix::remove_user_path(dir)
    }
}

/// 用户 PATH 是否已含 dir。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn user_path_contains(dir: &Path) -> Result<bool, String> {
    #[cfg(windows)]
    {
        windows::user_path_contains(&dir.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        unix::user_path_contains(dir)
    }
}

/// 用户 PATH 原始条目（保序不去空；Windows 读 HKCU\Environment，非 Windows 读 profile 标记块）。
/// doctor 诊断死链与重复条目用。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn user_path_entries() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    {
        windows::read_user_path_raw().map(|raw| raw.split(';').map(str::to_string).collect())
    }
    #[cfg(not(windows))]
    {
        unix::profile_path_entries()
    }
}

/// PATH 条目合并（纯函数）：把缺失目录追加到 raw 尾部（大小写不敏感、忽略尾反斜杠比较）。
/// 返回新值与是否变化；供用户级与机器级 PATH 写入共用。
pub fn merge_path_entries(raw: &str, dirs: &[String]) -> (String, bool) {
    let mut parts: Vec<String> = raw
        .split(';')
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.trim().to_string())
        .collect();
    let norm = |s: &str| s.trim_end_matches('\\').to_lowercase();
    let mut changed = false;
    for d in dirs {
        let ds = d.trim().to_string();
        if ds.is_empty() {
            continue;
        }
        if !parts.iter().any(|p| norm(p) == norm(&ds)) {
            parts.push(ds);
            changed = true;
        }
    }
    (parts.join(";"), changed)
}

/// profile 环境变量块合并（纯函数）：读序双块（`# >>> ark env` 主、`# >>> ome env` 旧），
/// 同 KEY 行跨块合并替换，结果写 ark 块并退役旧块（幂等迁移，D41）。无块则追加到文末。
/// Linux/macOS 写用户环境变量用。
/// env 标记块常量（D41：写 ark 块，读序 ark 与旧 ome 块）。
const ARK_ENV_MARKER: &str = "# >>> ark env";
const ARK_ENV_END: &str = "# <<< ark env";
const LEGACY_ENV_MARKER: &str = "# >>> ome env";
const LEGACY_ENV_END: &str = "# <<< ome env";

/// 逐块收行（marker..end 区间体，保序）。
fn env_block_body(text: &str, marker: &str, end: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let (Some(s), Some(e)) = (
        lines.iter().position(|l| l.trim_start() == marker),
        lines.iter().position(|l| l.trim_start() == end),
    ) else {
        return Vec::new();
    };
    if e <= s {
        return Vec::new();
    }
    lines[s + 1..e].iter().map(|l| l.to_string()).collect()
}

/// 摘除指定标记块（保留其余原文，尾保换行）。
fn env_strip_block(text: &str, marker: &str, end: &str) -> String {
    let mut out = Vec::new();
    let mut skip = false;
    for l in text.lines() {
        if l.trim_start() == marker {
            skip = true;
            continue;
        }
        if skip && l.trim_start() == end {
            skip = false;
            continue;
        }
        if !skip {
            out.push(l);
        }
    }
    let mut joined = out.join("\n");
    if !joined.is_empty() && !joined.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// 行的 KEY 前缀（`export K="v"` 取 K）。
fn env_key_of(s: &str) -> String {
    s.trim_start().split('=').next().unwrap_or("").to_string()
}

/// 双块读序合并体：ark 块先，旧 ome 块只补差（同 KEY 以 ark 块值为准）。
fn merged_env_body(text: &str) -> Vec<String> {
    let mut body = env_block_body(text, ARK_ENV_MARKER, ARK_ENV_END);
    for l in env_block_body(text, LEGACY_ENV_MARKER, LEGACY_ENV_END) {
        if !body.iter().any(|b| env_key_of(b) == env_key_of(&l)) {
            body.push(l);
        }
    }
    body
}

/// 以给定体重组文本：两块全退，ark 块追加文末。
fn rebuild_env_block(text: &str, body: &[String]) -> String {
    let base = env_strip_block(
        &env_strip_block(text, ARK_ENV_MARKER, ARK_ENV_END),
        LEGACY_ENV_MARKER,
        LEGACY_ENV_END,
    );
    let mut out = base;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(ARK_ENV_MARKER);
    out.push('\n');
    for l in body {
        out.push_str(l);
        out.push('\n');
    }
    out.push_str(ARK_ENV_END);
    out.push('\n');
    out
}

/// 向 profile env 块合并 export 行（已有键行级 upsert，无则追加；幂等不重写）。
pub fn merge_env_exports(text: &str, key: &str, value: &str) -> String {
    let line = format!("export {key}=\"{value}\"");
    let prefix_tag = format!("export {key}=");
    let mut body = merged_env_body(text);
    body.retain(|l| !l.trim_start().starts_with(&prefix_tag));
    body.push(line);
    rebuild_env_block(text, &body)
}

/// D41：旧 ome env 块收口迁移（值不变的块体搬迁；无旧标记时原样返回）。纯函数。
pub fn absorb_legacy_env_block(text: &str) -> String {
    if !text.contains(LEGACY_ENV_MARKER) {
        return text.to_string();
    }
    rebuild_env_block(text, &merged_env_body(text))
}

/// O3（S017）：自定义钩子块 upsert（fnm 等；纯函数）。幂等且块体感知：
/// 块在位而块体与目标不一致时原位重写（存量旧块体自愈升级，M027 防再犯），
/// 同标记重复块收敛为一块，旧 ome 标记块迁移为 ark 标记，块外原文与顺序不动。
/// 起标记未闭合（有头无尾）按既有家族语义截断收口：该行之后的原文一并丢弃。
pub fn merge_hook_block(text: &str, marker: &str, lines: &[&str]) -> String {
    let end = marker.replace(">>>", "<<<");
    let legacy = marker.replace("ark", "ome");
    let legacy_end = legacy.replace(">>>", "<<<");
    // 旧 ome 标记行先迁移为 ark 标记（块体行不动，D41 口径）
    let migrated = text
        .replace(&format!("{legacy}\n"), &format!("{marker}\n"))
        .replace(&format!("{legacy_end}\n"), &format!("{end}\n"));
    let desired_body = lines.join("\n");
    let mut out: Vec<String> = Vec::new();
    let mut inserted = false;
    let mut in_block = false;
    for line in migrated.lines() {
        let t = line.trim();
        if t == marker || t == legacy {
            in_block = true;
            if !inserted {
                out.push(marker.to_string());
                out.push(desired_body.clone());
                out.push(end.clone());
                inserted = true;
            }
            continue;
        }
        if in_block {
            if t == end || t == legacy_end {
                in_block = false;
            }
            continue; // 旧块体行与结束标记行都不外带（换目标块体）
        }
        out.push(line.to_string());
    }
    if inserted {
        let mut result = out.join("\n");
        if migrated.ends_with('\n') {
            result.push('\n');
        }
        return result;
    }
    // 无块：追加到文末（空文本直接以块起，避免文件首空行）
    if migrated.is_empty() {
        return format!("{marker}\n{desired_body}\n{end}\n");
    }
    format!("{migrated}\n{marker}\n{desired_body}\n{end}\n")
}

/// D41：检测旧 ome env 标记即收口迁移一次（init 与 self update 收尾钩子；
/// POSIX 写 profile，Windows 注册表面无块概念、no-op）。
pub fn migrate_legacy_env_block_once() {
    #[cfg(not(windows))]
    if let Err(e) = unix::migrate_legacy_env_block_once() {
        eprintln!("[WARN] profile 旧 env 块收口失败: {e}");
    }
}

/// 设置用户级环境变量（幂等）。Windows 写 HKCU\Environment 并同步当前进程；
/// O3（S017）：写 profile 自定义钩子块（POSIX；Windows no-op）。幂等且块体感知：
/// 块体与目标不一致时原位重写（存量旧钩子自愈升级）。
pub fn ensure_profile_hook(marker: &str, lines: &[&str]) {
    if user_env_write_blocked() {
        return;
    }
    #[cfg(not(windows))]
    if let Err(e) = unix::ensure_profile_hook(marker, lines) {
        eprintln!("[WARN] profile 钩子写入失败: {e}");
    }
    #[cfg(windows)]
    let _ = (marker, lines);
}

/// Linux/macOS 写 profile 的 ome 标记块。用于装后遥测关闭等运行时开关。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn set_user_env_var(key: &str, value: &str) -> Result<(), String> {
    if user_env_write_blocked() {
        eprintln!("[INFO] ARK_TEST_NO_PATH_REG=1：跳过用户级变量写入（测试隔离）: {key}");
        return Ok(());
    }
    #[cfg(windows)]
    {
        windows::set_user_env_var(key, value)
    }
    #[cfg(not(windows))]
    {
        unix::set_user_env_var(key, value)
    }
}

/// 读用户级环境变量（未设置返回 None）。Windows 读 HKCU\Environment；
/// Linux/macOS 读 profile 的 ome 标记块。heal 判 OK/HEALED 用。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn get_user_env_var(key: &str) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        windows::get_user_env_var(key)
    }
    #[cfg(not(windows))]
    {
        unix::get_user_env_var(key)
    }
}

/// 撤除用户级环境变量（D42 mirror env_unset 通道；幂等）。返回是否真的撤了。
/// Windows 删 HKCU\Environment 值并同步当前进程；POSIX 摘 profile env 标记块内行。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn remove_user_env_var(key: &str) -> Result<bool, String> {
    if user_env_write_blocked() {
        eprintln!("[INFO] ARK_TEST_NO_PATH_REG=1：跳过用户级变量撤除（测试隔离）: {key}");
        return Ok(false);
    }
    #[cfg(windows)]
    {
        windows::remove_user_env_var(key)
    }
    #[cfg(not(windows))]
    {
        unix::remove_user_env_var(key)
    }
}

/// env 块摘除单键（纯函数，D42 env_unset 通道）：**全文件撤变量不分写入者**——
/// 托管块内（读序双块合并体）摘行、体空则两块整撤不残留空标记；托管块之外
/// 的同名 `export KEY=` 行（遗产脚本或用户手写形态）同样摘除，
/// 否则旧变量继续生效而日志已报「已撤」（对线 R1）。
/// 快速路径：全文件无该键的 export 行时**原样返回**——重建本身会重排块位
/// （env 块不在文末时后续 install 追加 PATH 块即此常态），恒等才能让
/// 「文本变化」等价「真撤了」（对线 R1 复审残留）。
pub fn remove_env_export(text: &str, key: &str) -> String {
    let prefix_tag = format!("export {key}=");
    if !text
        .lines()
        .any(|l| l.trim_start().starts_with(&prefix_tag))
    {
        return text.to_string();
    }
    let body: Vec<String> = merged_env_body(text)
        .into_iter()
        .filter(|l| !l.trim_start().starts_with(&prefix_tag))
        .collect();
    let base = if body.is_empty() {
        env_strip_block(
            &env_strip_block(text, ARK_ENV_MARKER, ARK_ENV_END),
            LEGACY_ENV_MARKER,
            LEGACY_ENV_END,
        )
    } else {
        rebuild_env_block(text, &body)
    };
    // 块外同名行摘除（幂等：重建后的 base 块内已无该键，此处只清块外残留）
    let mut out: Vec<&str> = Vec::new();
    for l in base.lines() {
        if !l.trim_start().starts_with(&prefix_tag) {
            out.push(l);
        }
    }
    let mut joined = out.join("\n");
    if !joined.is_empty() && !joined.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// 当前进程是否管理员（以写权限打开 HKLM Environment 判定；非 Windows 恒 false）。
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        windows::is_elevated()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// 机器 PATH 是否已含 dir（非 Windows 恒 false）。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn machine_path_contains(dir: &Path) -> Result<bool, String> {
    #[cfg(windows)]
    {
        windows::machine_path_contains(&dir.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        let _ = dir;
        Ok(false)
    }
}

/// 把缺失目录追加进机器 PATH（REG_EXPAND_SZ，需管理员）；返回是否写入。非 Windows 恒不写入。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn machine_path_add(dirs: &[PathBuf]) -> Result<bool, String> {
    #[cfg(windows)]
    {
        windows::machine_path_add(dirs)
    }
    #[cfg(not(windows))]
    {
        let _ = dirs;
        Ok(false)
    }
}

/// 展开环境变量引用。
/// Windows：`%VAR%`；Linux / macOS：`$VAR` 与 `${VAR}`。
///
/// # Panics
/// 内嵌正则以 `expect` 构造，仅当静态正则模式非法时 panic（常量模式，实践中不可达）。
pub fn expand_env_vars(s: &str) -> String {
    #[cfg(windows)]
    {
        use regex::Regex;
        let re = Regex::new("%([^%]+)%").expect("静态正则应合法");
        re.replace_all(s, |caps: &regex::Captures| {
            std::env::var(&caps[1]).unwrap_or_else(|_| caps[0].to_string())
        })
        .into_owned()
    }
    #[cfg(not(windows))]
    {
        use regex::Regex;
        let re = Regex::new(r"\$\{([^}]+)\}|\$([A-Za-z_][A-Za-z0-9_]*)").expect("静态正则应合法");
        re.replace_all(s, |caps: &regex::Captures| {
            let name = caps
                .get(1)
                .or_else(|| caps.get(2))
                .map(|m| m.as_str())
                .unwrap_or("");
            std::env::var(name).unwrap_or_else(|_| caps[0].to_string())
        })
        .into_owned()
    }
}

/// 判定 exe 字段是否表示 official 布局（使用环境变量展开 / 绝对路径，不纳入 EnvRoot 管理）。
/// Windows：含 `%`；Linux / macOS：含 `$` 或为绝对路径。
pub fn is_official_exe(exe: &str) -> bool {
    #[cfg(windows)]
    {
        exe.contains('%')
    }
    #[cfg(not(windows))]
    {
        exe.contains('$') || Path::new(exe).is_absolute()
    }
}

/// 标准化 PATH 条目比较：展开环境变量、大小写不敏感（Windows）/ 敏感（Unix）。
pub fn path_entries_eq(a: &str, b: &str) -> bool {
    let a_exp = expand_env_vars(a);
    let b_exp = expand_env_vars(b);
    #[cfg(windows)]
    {
        a_exp.eq_ignore_ascii_case(&b_exp)
    }
    #[cfg(not(windows))]
    {
        a_exp == b_exp
    }
}

// ── Windows 实现 ──

#[cfg(windows)]
mod windows {
    use super::*;
    use winreg::enums::{RegType, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
    use winreg::{RegKey, RegValue};

    /// 机器级 Environment 键（Session Manager）。
    const MACHINE_ENV: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

    /// 原始用户 PATH（未展开值；doctor 诊断死链与重复条目用，PATH 写入共用）。
    pub fn read_user_path_raw() -> Result<String, String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu
            .open_subkey_with_flags("Environment", KEY_READ)
            .map_err(|e| format!("打开 HKCU\\Environment 失败: {e}"))?;
        env.get_value::<String, _>("Path").or(Ok(String::new()))
    }

    fn write_user_path_raw(raw: &str) -> Result<(), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu
            .open_subkey_with_flags("Environment", KEY_WRITE)
            .map_err(|e| format!("打开 HKCU\\Environment 失败: {e}"))?;
        let bytes: Vec<u8> = raw
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .flat_map(u16::to_le_bytes)
            .collect();
        env.set_raw_value(
            "Path",
            &RegValue {
                vtype: RegType::REG_EXPAND_SZ,
                bytes,
            },
        )
        .map_err(|e| format!("写用户 PATH 失败: {e}"))
    }

    pub fn add_user_path(dir: &str) -> Result<bool, String> {
        let raw = read_user_path_raw()?;
        let added = if let Some(new_raw) = crate::envpath::add_path_entry(&raw, dir) {
            write_user_path_raw(&new_raw)?;
            notify_env_change();
            true
        } else {
            false
        };
        ensure_process_path(dir);
        Ok(added)
    }

    /// 当前进程 PATH 缺 dir 时前置插入（注册表已有但本会话未刷新时仍可在子进程里找到）。
    fn ensure_process_path(dir: &str) {
        let cur = std::env::var("PATH").unwrap_or_default();
        if let Some(new) = crate::envpath::add_path_entry(&cur, dir) {
            std::env::set_var("PATH", new);
        }
    }

    /// 通知 Explorer / 新进程重读环境（已打开的终端不会自动刷新）。
    fn notify_env_change() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
        };
        let mut name: Vec<u16> = "Environment".encode_utf16().collect();
        name.push(0);
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                name.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                1000,
                std::ptr::null_mut(),
            );
        }
    }

    pub fn remove_user_path(dir: &str) -> Result<bool, String> {
        let raw = read_user_path_raw()?;
        let new_raw = crate::envpath::remove_path_entry(&raw, dir);
        if new_raw == raw {
            return Ok(false);
        }
        write_user_path_raw(&new_raw)?;
        let expanded_dir = expand_env_vars(dir);
        let cur = std::env::var("PATH").unwrap_or_default();
        let kept = cur
            .split(';')
            .filter(|p| !p.is_empty() && !expand_env_vars(p).eq_ignore_ascii_case(&expanded_dir))
            .collect::<Vec<_>>()
            .join(";");
        std::env::set_var("PATH", kept);
        Ok(true)
    }

    pub fn user_path_contains(dir: &str) -> Result<bool, String> {
        let raw = read_user_path_raw()?;
        let norm = |s: &str| {
            expand_env_vars(s)
                .trim_end_matches(['\\', '/'])
                .to_lowercase()
        };
        let want = norm(dir);
        Ok(raw
            .split(';')
            .any(|p| !p.trim().is_empty() && norm(p) == want))
    }

    /// 管理员判定：以写权限打开 HKLM Environment（无管理员时 OpenKey 报权限错）。
    pub fn is_elevated() -> bool {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        hklm.open_subkey_with_flags(MACHINE_ENV, KEY_READ | KEY_WRITE)
            .is_ok()
    }

    fn read_machine_path_raw() -> Result<String, String> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let env = hklm
            .open_subkey_with_flags(MACHINE_ENV, KEY_READ)
            .map_err(|e| format!("读取 HKLM Environment 失败: {e}"))?;
        env.get_value::<String, _>("Path").or(Ok(String::new()))
    }

    pub fn machine_path_contains(dir: &str) -> Result<bool, String> {
        let raw = read_machine_path_raw()?;
        let norm = |s: &str| s.trim_end_matches('\\').to_lowercase();
        Ok(raw
            .split(';')
            .any(|p| !p.trim().is_empty() && norm(p) == norm(dir)))
    }

    pub fn machine_path_add(dirs: &[PathBuf]) -> Result<bool, String> {
        let raw = read_machine_path_raw()?;
        let dirs: Vec<String> = dirs
            .iter()
            .map(|d| d.to_string_lossy().to_string())
            .collect();
        let (new_raw, changed) = merge_path_entries(&raw, &dirs);
        if !changed {
            return Ok(false);
        }
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let env = hklm
            .open_subkey_with_flags(MACHINE_ENV, KEY_WRITE)
            .map_err(|e| format!("打开 HKLM Environment 写失败（需管理员）: {e}"))?;
        let bytes: Vec<u8> = new_raw
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .flat_map(u16::to_le_bytes)
            .collect();
        env.set_raw_value(
            "Path",
            &RegValue {
                vtype: RegType::REG_EXPAND_SZ,
                bytes,
            },
        )
        .map_err(|e| format!("写机器 PATH 失败: {e}"))?;
        notify_env_change();
        Ok(true)
    }

    /// 用户级环境变量（REG_SZ，幂等覆盖），并同步当前进程。
    pub fn set_user_env_var(key: &str, value: &str) -> Result<(), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu
            .open_subkey_with_flags("Environment", KEY_WRITE)
            .map_err(|e| format!("打开 HKCU\\Environment 失败: {e}"))?;
        env.set_value(key, &value)
            .map_err(|e| format!("写用户环境变量失败: {key}: {e}"))?;
        std::env::set_var(key, value);
        notify_env_change();
        Ok(())
    }

    /// 读用户级环境变量（未设置返回 None；heal 判 OK/HEALED 用）。
    pub fn get_user_env_var(key: &str) -> Result<Option<String>, String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu
            .open_subkey_with_flags("Environment", KEY_READ)
            .map_err(|e| format!("打开 HKCU\\Environment 失败: {e}"))?;
        Ok(env.get_value(key).ok())
    }

    /// 删用户级环境变量（值不存在返回 false 不报错）并同步当前进程与广播。
    pub fn remove_user_env_var(key: &str) -> Result<bool, String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu
            .open_subkey_with_flags("Environment", KEY_WRITE)
            .map_err(|e| format!("打开 HKCU\\Environment 失败: {e}"))?;
        match env.delete_value(key) {
            Ok(()) => {
                std::env::remove_var(key);
                notify_env_change();
                Ok(true)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(format!("删用户环境变量失败: {key}: {e}")),
        }
    }
}

// ── Linux / macOS 实现 ──

#[cfg(not(windows))]
mod unix {
    use super::*;

    /// PATH 标记块（D41：写 ark 块，读序 ark 与旧 ome 块合并；旧块在 upsert 时退役）
    const ARK_PATH_MARKER: &str = "# >>> ark PATH";
    const ARK_PATH_END: &str = "# <<< ark PATH";
    const LEGACY_PATH_MARKER: &str = "# >>> ome PATH";
    const LEGACY_PATH_END: &str = "# <<< ome PATH";

    fn profile_path() -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or("无法确定用户主目录")?;
        // 优先按当前 shell 选择；bash 为默认。
        if let Ok(shell) = std::env::var("SHELL") {
            if shell.contains("zsh") {
                return Ok(home.join(".zshrc"));
            }
            if shell.contains("fish") {
                return Ok(home.join(".config").join("fish").join("config.fish"));
            }
        }
        Ok(home.join(".bashrc"))
    }

    fn read_profile() -> Result<String, String> {
        let path = profile_path()?;
        if !path.exists() {
            return Ok(String::new());
        }
        std::fs::read_to_string(&path)
            .map_err(|e| format!("读取 profile 失败: {}: {e}", path.display()))
    }

    fn write_profile(text: &str) -> Result<(), String> {
        let path = profile_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建 profile 目录失败: {}: {e}", parent.display()))?;
        }
        std::fs::write(&path, text).map_err(|e| format!("写 profile 失败: {}: {e}", path.display()))
    }

    /// D41：旧 ome env 块收口（检测到标记即搬迁，幂等；测试隔离闸门同守）。
    pub(super) fn migrate_legacy_env_block_once() -> Result<(), String> {
        if super::user_env_write_blocked() {
            return Ok(());
        }
        let text = read_profile()?;
        let migrated = super::absorb_legacy_env_block(&text);
        if migrated != text {
            write_profile(&migrated)?;
        }
        Ok(())
    }

    /// O3（S017）：写自定义钩子块（幂等且块体感知：目标块体不一致原位重写，
    /// 旧 ome 标记块迁移为 ark；逻辑在纯函数 merge_hook_block，此处只做 IO）。
    pub(super) fn ensure_profile_hook(marker: &str, lines: &[&str]) -> Result<(), String> {
        let text = read_profile()?;
        let new_text = super::merge_hook_block(&text, marker, lines);
        if new_text != text {
            write_profile(&new_text)?;
        }
        Ok(())
    }

    /// 摘除指定标记块（通用；保留其余原文）。
    fn remove_block(text: &str, marker: &str, end: &str) -> String {
        let mut out = Vec::new();
        let mut skip = false;
        for line in text.lines() {
            if line.trim().starts_with(marker) {
                skip = true;
                continue;
            }
            if skip && line.trim().starts_with(end) {
                skip = false;
                continue;
            }
            if !skip {
                out.push(line);
            }
        }
        out.join("\n")
    }

    fn remove_ome_path_block(text: &str) -> String {
        remove_block(
            &remove_block(text, ARK_PATH_MARKER, ARK_PATH_END),
            LEGACY_PATH_MARKER,
            LEGACY_PATH_END,
        )
    }

    fn profile_is_fish() -> bool {
        std::env::var("SHELL")
            .map(|s| s.contains("fish"))
            .unwrap_or(false)
    }

    fn format_export(dir: &str) -> String {
        if profile_is_fish() {
            format!("fish_add_path -P {dir}")
        } else {
            format!(r#"export PATH="{}:$PATH""#, dir)
        }
    }

    fn parse_export_dir(line: &str) -> Option<String> {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("export PATH=") {
            let v = rest.trim().trim_matches('"');
            return Some(v.trim_end_matches(":$PATH").to_string());
        }
        t.strip_prefix("fish_add_path -P ")
            .or_else(|| t.strip_prefix("fish_add_path "))
            .map(|s| s.trim().to_string())
    }

    /// 单块解析（marker..end 区间内的 export 行）。
    fn block_path_dirs(text: &str, marker: &str, end: &str) -> Vec<String> {
        let mut dirs = Vec::new();
        let mut in_block = false;
        for line in text.lines() {
            if line.trim().starts_with(marker) {
                in_block = true;
                continue;
            }
            if in_block && line.trim().starts_with(end) {
                break;
            }
            if in_block {
                if let Some(d) = parse_export_dir(line) {
                    dirs.push(d);
                }
            }
        }
        dirs
    }

    /// PATH 目录读序（D41）：ark 块先，旧 ome 块只补差（等价项不重复）。
    pub(super) fn ome_path_dirs(text: &str) -> Vec<String> {
        let mut dirs = block_path_dirs(text, ARK_PATH_MARKER, ARK_PATH_END);
        for d in block_path_dirs(text, LEGACY_PATH_MARKER, LEGACY_PATH_END) {
            if !dirs.iter().any(|x| path_entries_eq(x, &d)) {
                dirs.push(d);
            }
        }
        dirs
    }

    /// upsert：写 ark 块（两块目录合并结果），旧 ome 块退役（其项已并入，不丢配置）。
    pub(super) fn upsert_ome_path_block(text: &str, dirs: &[String]) -> String {
        let body: String = dirs
            .iter()
            .map(|d| format!("{}\n", format_export(d)))
            .collect();
        let block = format!("{ARK_PATH_MARKER}\n{body}{ARK_PATH_END}\n");
        let stripped = remove_ome_path_block(text);
        let mut out = stripped;
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&block);
        out
    }

    pub fn add_user_path(dir: &Path) -> Result<bool, String> {
        let dir_str = dir.to_string_lossy().to_string();
        let text = read_profile()?;
        let mut dirs = ome_path_dirs(&text);
        let added = if dirs.iter().any(|d| path_entries_eq(d, &dir_str)) {
            false
        } else {
            dirs.push(dir_str.clone());
            true
        };
        // D41：无新增但旧 ome 块在位也落写一次（收口迁移，防旧块长期残留；目录已并入不丢）
        if added || text.contains(LEGACY_PATH_MARKER) {
            write_profile(&upsert_ome_path_block(&text, &dirs))?;
        }
        let cur = std::env::var("PATH").unwrap_or_default();
        if !cur.split(':').any(|p| path_entries_eq(p, &dir_str)) {
            std::env::set_var("PATH", format!("{dir_str}:{cur}"));
        }
        Ok(added)
    }

    pub fn remove_user_path(dir: &Path) -> Result<bool, String> {
        let dir_str = dir.to_string_lossy().to_string();
        let text = read_profile()?;
        let dirs = ome_path_dirs(&text);
        if !dirs.iter().any(|d| path_entries_eq(d, &dir_str)) {
            return Ok(false);
        }
        let kept: Vec<String> = dirs
            .into_iter()
            .filter(|d| !path_entries_eq(d, &dir_str))
            .collect();
        let new_text = if kept.is_empty() {
            remove_ome_path_block(&text)
        } else {
            upsert_ome_path_block(&text, &kept)
        };
        write_profile(&new_text)?;
        let cur = std::env::var("PATH").unwrap_or_default();
        let kept_path = cur
            .split(':')
            .filter(|p| !p.is_empty() && !path_entries_eq(p, &dir_str))
            .collect::<Vec<_>>()
            .join(":");
        std::env::set_var("PATH", kept_path);
        Ok(true)
    }

    pub fn user_path_contains(dir: &Path) -> Result<bool, String> {
        let dir_str = dir.to_string_lossy().to_string();
        let text = read_profile()?;
        Ok(ome_path_dirs(&text)
            .iter()
            .any(|d| path_entries_eq(d, &dir_str)))
    }

    /// PATH 原始条目（profile 的 ome PATH 标记块内各目录；doctor 诊断用）。
    pub fn profile_path_entries() -> Result<Vec<String>, String> {
        let text = read_profile()?;
        Ok(ome_path_dirs(&text))
    }

    /// 用户级环境变量：profile 的 ome env 标记块内幂等 upsert，并同步当前进程。
    pub fn set_user_env_var(key: &str, value: &str) -> Result<(), String> {
        let text = read_profile()?;
        let new_text = merge_env_exports(&text, key, value);
        if new_text != text {
            write_profile(&new_text)?;
        }
        std::env::set_var(key, value);
        Ok(())
    }

    /// 读用户级环境变量：profile 全文件找 `export KEY="value"`（标记块内外皆认，
    /// 遗产脚本手写形态同样读到；未设置返回 None）。
    pub fn get_user_env_var(key: &str) -> Result<Option<String>, String> {
        let text = read_profile()?;
        let prefix = format!("export {key}=\"");
        for line in text.lines() {
            if let Some(rest) = line.trim_start().strip_prefix(&prefix) {
                if let Some(v) = rest.strip_suffix('"') {
                    return Ok(Some(v.to_string()));
                }
            }
        }
        Ok(None)
    }

    /// 撤除用户级环境变量：profile 全文件摘同名 export 行（remove_env_export 纯函数做文本，
    /// 块内外不分写入者）；**按写后文本是否真变化报告**（防「报已撤而文件没动」，
    /// 对线 R1）；撤后同步当前进程。
    pub fn remove_user_env_var(key: &str) -> Result<bool, String> {
        let text = read_profile()?;
        let new_text = super::remove_env_export(&text, key);
        if new_text == text {
            return Ok(false);
        }
        write_profile(&new_text)?;
        std::env::remove_var(key);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_空白视同未设() {
        // 单名读取语义：未设 None、纯空白视同未设、常规值 trim 后返回
        std::env::remove_var("ARK_TEST_ENVVAR_X");
        assert_eq!(env_var("ARK_TEST_ENVVAR_X"), None, "未设为 None");
        std::env::set_var("ARK_TEST_ENVVAR_X", "  ");
        assert_eq!(env_var("ARK_TEST_ENVVAR_X"), None, "纯空白视同未设");
        std::env::set_var("ARK_TEST_ENVVAR_X", " val ");
        assert_eq!(
            env_var("ARK_TEST_ENVVAR_X").as_deref(),
            Some("val"),
            "trim 后返回"
        );
        std::env::remove_var("ARK_TEST_ENVVAR_X");
    }

    #[cfg(windows)]
    #[test]
    fn legacy别名_部署位同目录() {
        // D41 C 收口：别名停建，落点仅供清理定位（旧位实名 ome）
        let alias = legacy_alias_target().expect("别名应可解析");
        let deploy = self_deploy_target().expect("部署位应可解析");
        assert_eq!(alias.parent(), deploy.parent(), "别名与部署位同目录");
        assert!(
            alias.ends_with("ome.exe"),
            "别名文件名: {}",
            alias.display()
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn legacy别名_部署位同目录() {
        let alias = legacy_alias_target().expect("别名应可解析");
        let deploy = self_deploy_target().expect("部署位应可解析");
        assert_eq!(alias.parent(), deploy.parent(), "别名与部署位同目录");
        assert!(alias.ends_with("ome"), "别名文件名: {}", alias.display());
    }

    #[cfg(unix)]
    #[test]
    fn 路径归一_符号链接前缀对齐() {
        // macOS 实证形态同构复刻：$TMPDIR 为链接形、canonicalize 后为真身形；
        // 不存在的子路径经归一后两形前缀必须对齐（CI mac 岗红根因）
        use std::os::unix::fs::symlink;
        let base = std::env::temp_dir();
        let real = base.join(format!("ark-norm-real-{}", std::process::id()));
        let link = base.join(format!("ark-norm-link-{}", std::process::id()));
        std::fs::create_dir_all(&real).expect("建真身");
        let _ = std::fs::remove_file(&link);
        symlink(&real, &link).expect("建符号链接");
        let via_link = link.join("probe").join("zig");
        let via_real = real.join("probe").join("zig");
        assert!(
            normalize(&via_link).starts_with(&normalize(&real)),
            "链接形子路径应归一到真身前缀: {:?} vs {:?}",
            normalize(&via_link),
            normalize(&real)
        );
        assert_eq!(normalize(&via_link), normalize(&via_real), "两形同点");
        let _ = std::fs::remove_file(&link);
        let _ = std::fs::remove_dir_all(&real);
    }

    #[test]
    fn temp路径判定_闸注册面() {
        let tmp = std::env::temp_dir();
        assert!(
            is_temp_path(&tmp.join("ark-probe").join("zig")),
            "temp 子路径应命中闸"
        );
        assert!(
            !is_temp_path(&dirs::home_dir().expect("home").join(".local").join("bin")),
            "正式泊位不误伤"
        );
    }

    #[test]
    fn 清理文件_幂等双态() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let f = dir.path().join("ome");
        std::fs::write(&f, b"x").map_err(|e| e.to_string())?;
        assert!(remove_file_if_exists(&f)?, "在位应删并返回 true");
        assert!(!remove_file_if_exists(&f)?, "不在位应静默返回 false");
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn self_deploy_target_windows_用户程序目录与envroot解耦() {
        let t = self_deploy_target().expect("self-deploy 目标应可解析");
        assert!(
            t.ends_with("Programs\\ark\\ark.exe"),
            "目标应在用户程序目录: {}",
            t.display()
        );
        assert!(
            !t.starts_with(default_env_root()),
            "目标不应在 EnvRoot 下: {}",
            t.display()
        );
    }

    #[test]
    fn 元数据目录_主名ark_旧ohmyenv在位读回() {
        // D41 迁移过渡三态：皆无取主名、仅旧在取旧位、新在归位主名
        let dir = tempfile::tempdir().expect("临时目录");
        assert_eq!(
            resolve_metadata_dir(dir.path()),
            dir.path().join("ark"),
            "两者皆无应取主名 ark"
        );
        std::fs::create_dir_all(dir.path().join("ohmyenv")).expect("建旧目录");
        assert_eq!(
            resolve_metadata_dir(dir.path()),
            dir.path().join("ohmyenv"),
            "旧 ohmyenv 在而新未建应读回旧位（共享 catalog 与 seq）"
        );
        std::fs::create_dir_all(dir.path().join("ark")).expect("建新目录");
        assert_eq!(
            resolve_metadata_dir(dir.path()),
            dir.path().join("ark"),
            "新 ark 建成后应归位主名"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn self_deploy_target_posix_用户bin目录() {
        let t = self_deploy_target().expect("self-deploy 目标应可解析");
        assert!(
            t.ends_with(".local/bin/ark"),
            "目标应在用户 bin 目录: {}",
            t.display()
        );
        assert!(
            !t.starts_with(default_env_root()),
            "目标不应在 EnvRoot 下: {}",
            t.display()
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn path块_旧ome块读序合并与退役() {
        // D41：旧 ome PATH 块单独在位可读；upsert 并入旧项写 ark 块、旧块退役、不重复
        let legacy = "# >>> ome PATH\nexport PATH=\"/a/bin:$PATH\"\n# <<< ome PATH\n";
        assert_eq!(
            super::unix::ome_path_dirs(legacy),
            vec!["/a/bin".to_string()],
            "旧块单独在位应可读出"
        );
        let t = super::unix::upsert_ome_path_block(
            legacy,
            &["/a/bin".to_string(), "/b/bin".to_string()],
        );
        assert!(t.contains("# >>> ark PATH"), "应写 ark 块");
        assert!(!t.contains("# >>> ome PATH"), "旧块应退役");
        assert_eq!(
            super::unix::ome_path_dirs(&t),
            vec!["/a/bin".to_string(), "/b/bin".to_string()],
            "合并后保序不重复"
        );
    }

    #[test]
    fn 元数据七件套搬迁_幂等且旧位只读保留() {
        // D41 C：首搬七件全复制、复搬幂等（新位在即跳过）、旧位 copy 不 move 保留、旧 catalog 缺无动作
        let dir = tempfile::tempdir().expect("临时目录");
        let (old, new) = (dir.path().join("ohmyenv"), dir.path().join("ark"));
        let from = old.join("catalog");
        std::fs::create_dir_all(&from).expect("建旧目录");
        for name in METADATA_MIGRATION_FILES {
            std::fs::write(from.join(name), format!("payload-{name}")).expect("写件");
        }
        assert!(
            migrate_legacy_metadata_in(&old, &new).expect("首搬应成功"),
            "首搬应有动作"
        );
        for name in METADATA_MIGRATION_FILES {
            assert_eq!(
                std::fs::read_to_string(new.join("catalog").join(name)).expect("新位应有件"),
                format!("payload-{name}"),
                "件 {name} 内容应一致"
            );
            assert!(
                from.join(name).exists(),
                "旧位件 {name} 应保留（copy 不 move）"
            );
        }
        std::fs::write(from.join("tools.toml"), "changed").expect("改旧位");
        assert!(
            !migrate_legacy_metadata_in(&old, &new).expect("复搬应成功"),
            "复搬应无动作（新位在即跳过）"
        );
        assert_eq!(
            std::fs::read_to_string(new.join("catalog").join("tools.toml")).expect("读新位"),
            "payload-tools.toml",
            "复搬不得覆盖新位"
        );
        let empty_old = dir.path().join("no-catalog");
        std::fs::create_dir_all(&empty_old).expect("建空旧根");
        assert!(
            !migrate_legacy_metadata_in(&empty_old, &new).expect("应成功"),
            "旧 catalog 目录缺应无动作"
        );
    }

    #[cfg(windows)]
    #[test]
    fn metadata_dir_windows_独立于envroot() {
        let m = metadata_dir();
        assert!(
            !m.starts_with(default_env_root()),
            "元数据目录应独立于 EnvRoot: {}",
            m.display()
        );
    }

    #[test]
    fn path条目合并_缺失追加存在跳过() {
        let raw = r"C:\win;D:\ohmyenv\jq";
        // 已存在（尾反斜杠与大小写不敏感）不重复追加
        let (out, changed) = merge_path_entries(raw, &[r"d:\OHMYENV\jq\".to_string()]);
        assert!(!changed);
        assert_eq!(out, raw);
        // 缺失追加到尾部，保序
        let (out, changed) = merge_path_entries(
            raw,
            &[r"D:\ohmyenv\vsbuild\MSBuild\Current\Bin".to_string()],
        );
        assert!(changed);
        assert_eq!(
            out,
            r"C:\win;D:\ohmyenv\jq;D:\ohmyenv\vsbuild\MSBuild\Current\Bin"
        );
        // 空条目被清理后合并
        let (out, _) = merge_path_entries("C:\\win;;", &["E:\\x".to_string()]);
        assert_eq!(out, r"C:\win;E:\x");
    }

    #[test]
    fn profile环境变量块_幂等upsert() {
        // 无块：追加 ark 标记块到文末
        let t1 = merge_env_exports("export A=1\n", "DOTNET_CLI_TELEMETRY_OPTOUT", "1");
        assert!(t1.contains("# >>> ark env"));
        assert!(t1.contains("export DOTNET_CLI_TELEMETRY_OPTOUT=\"1\""));
        // 同 KEY 同值：幂等不变
        assert_eq!(
            merge_env_exports(&t1, "DOTNET_CLI_TELEMETRY_OPTOUT", "1"),
            t1
        );
        // 块内追加第二个变量：首个保留、不重复
        let t3 = merge_env_exports(&t1, "POWERSHELL_UPDATECHECK", "Off");
        assert!(t3.contains("export POWERSHELL_UPDATECHECK=\"Off\""));
        assert_eq!(t3.matches("export DOTNET_CLI_TELEMETRY_OPTOUT").count(), 1);
        // 同 KEY 改值：替换旧值
        let t4 = merge_env_exports(&t3, "POWERSHELL_UPDATECHECK", "On");
        assert!(t4.contains("export POWERSHELL_UPDATECHECK=\"On\""));
        assert!(!t4.contains("export POWERSHELL_UPDATECHECK=\"Off\""));
    }

    #[test]
    fn 旧env块收口_值不变搬迁() {
        // D41：absorb 无旧标记原样返回；有旧标记搬迁且值不动、ark 块既有值不被旧值覆盖
        let clean = "export A=1\n";
        assert_eq!(absorb_legacy_env_block(clean), clean, "无旧标记零动作");
        let mixed = "# >>> ark env\nexport K=\"new\"\n# <<< ark env\nother\n# >>> ome env\nexport K=\"old\"\nexport ONLY_OLD=\"1\"\n# <<< ome env\n";
        let t = absorb_legacy_env_block(mixed);
        assert!(
            t.contains("# >>> ark env") && !t.contains("# >>> ome env"),
            "旧块退役"
        );
        assert!(t.contains("export K=\"new\""), "同 KEY 以 ark 值为准");
        assert!(!t.contains("export K=\"old\""), "旧值不重复带");
        assert!(t.contains("export ONLY_OLD=\"1\""), "旧块独有键保留");
        assert!(t.contains("other"), "块外原文不动");
    }

    #[test]
    fn env块_摘除单键与空块收口() {
        // D42 mirror env_unset 的纯函数面：摘目标键、块外原文不动、块内他键保留
        let t1 = merge_env_exports(&merge_env_exports("export A=1\n", "K1", "v1"), "K2", "v2");
        let t2 = remove_env_export(&t1, "K1");
        assert!(!t2.contains("export K1="), "目标键应摘除");
        assert!(t2.contains("export K2=\"v2\""), "块内他键保留");
        assert!(t2.contains("export A=1"), "块外原文不动");
        assert!(t2.contains("# >>> ark env"), "仍有他键时块保留");
        // 摘掉最后一个键：整块收口，不残留空标记块
        let t3 = merge_env_exports("export A=1\n", "ONLY", "x");
        let t4 = remove_env_export(&t3, "ONLY");
        assert!(!t4.contains("# >>> ark env"), "体空应整块收口");
        assert!(t4.contains("export A=1"), "块外原文不动");
        // 摘不存在的键：块在文末的规范形态下幂等不变
        assert_eq!(remove_env_export(&t1, "NOPE"), t1);
        // 摘不存在的键：env 块不在文末（后续 install 追加 PATH 块的常态）也必须恒等，
        // 不得因块重排而误报「已撤」（对线 R1 复审残留）
        let mut mid = t1.clone();
        mid.push_str("# >>> ark PATH\nexport PATH=\"/x:$PATH\"\n# <<< ark PATH\n");
        assert_eq!(remove_env_export(&mid, "NOPE"), mid, "无该键时不得重排");
        // 块外同名行（遗产脚本/用户手写形态）同样摘除（对线 R1：撤变量不分写入者）
        let legacy = "export UV_INDEX_URL=\"https://old.example\"\nalias ll='ls -l'\n";
        let t5 = remove_env_export(legacy, "UV_INDEX_URL");
        assert!(!t5.contains("UV_INDEX_URL"), "块外旧变量行应摘除");
        assert!(t5.contains("alias ll='ls -l'"), "无关行不动");
    }

    #[test]
    fn profile环境变量块_旧ome块迁移() {
        // D41：旧 ome env 块在位时，upsert 并入其行、写 ark 块、旧块退役
        let legacy = "# >>> ome env\nexport KEEPME=\"1\"\n# <<< ome env\n";
        let t = merge_env_exports(legacy, "DOTNET_CLI_TELEMETRY_OPTOUT", "1");
        assert!(t.contains("# >>> ark env"), "应写 ark 块");
        assert!(!t.contains("# >>> ome env"), "旧块应退役");
        assert!(t.contains("export KEEPME=\"1\""), "旧块既有行应并入不丢失");
        assert!(t.contains("export DOTNET_CLI_TELEMETRY_OPTOUT=\"1\""));
        // 同 KEY 旧值在新块与旧块并存时：旧块行弃、新值唯一
        let mixed = "# >>> ark env\nexport K=\"new\"\n# <<< ark env\n# >>> ome env\nexport K=\"old\"\n# <<< ome env\n";
        let t2 = merge_env_exports(mixed, "K", "new");
        assert_eq!(t2.matches("export K=").count(), 1, "同 KEY 跨块不重复");
        assert!(t2.contains("export K=\"new\""));
    }

    /// M027：fnm 钩子块目标形态单一权威在 install.rs（FNM_HOOK_LINES），
    /// 测试引用同源常量，块体漂移测试必红。
    fn fnm_hook_lines() -> [&'static str; 2] {
        crate::install::FNM_HOOK_LINES
    }

    #[test]
    fn profile钩子块_无块追加与幂等() {
        let marker = "# >>> ark fnm >>>";
        let lines = fnm_hook_lines();
        // 无块：追加到文末，原文在前，PATH 导出先于守卫 eval
        let t1 = merge_hook_block("export A=1\n", marker, &lines);
        assert!(t1.starts_with("export A=1\n"), "块外原文在前不动");
        assert!(t1.contains(marker) && t1.contains("# <<< ark fnm <<<"));
        assert!(
            t1.find(lines[0]).unwrap() < t1.find(lines[1]).unwrap(),
            "先导 PATH 再 eval"
        );
        assert!(lines.iter().all(|l| t1.contains(l)), "块体为单一权威形态");
        // 幂等：二调逐字不变（防每装一次重写 profile）
        assert_eq!(merge_hook_block(&t1, marker, &lines), t1);
        // 空文本：只剩块
        let empty = merge_hook_block("", marker, &lines);
        assert!(empty.starts_with(marker));
        assert_eq!(merge_hook_block(&empty, marker, &lines), empty);
    }

    #[test]
    fn profile钩子块_存量裸eval块体原位升级() {
        // M027 实弹存量形态：块在位但块体是裸 eval（执行序早于 PATH 含 ~/.local/bin）
        let marker = "# >>> ark fnm >>>";
        let lines = fnm_hook_lines();
        let old = "alias ll='ls -l'\n\
                   # >>> ark fnm >>>\n\
                   eval \"$(fnm env)\"\n\
                   # <<< ark fnm <<<\n\
                   export B=2\n";
        let t = merge_hook_block(old, marker, &lines);
        assert!(
            !t.lines().any(|l| l.trim() == "eval \"$(fnm env)\""),
            "旧裸钩子行应被替换（守卫内 eval 除外）"
        );
        assert!(t.contains(lines[1]), "新块体在位");
        assert_eq!(t.matches(marker).count(), 1, "不重复建块");
        // 原位升级：块的前后原文与相对顺序不动
        assert!(t.find("alias ll").unwrap() < t.find(marker).unwrap());
        assert!(t.find(marker).unwrap() < t.find("export B=2").unwrap());
        // 升级后幂等
        assert_eq!(merge_hook_block(&t, marker, &lines), t);
    }

    #[test]
    fn profile钩子块_旧ome标记迁移与重复块收敛() {
        let marker = "# >>> ark fnm >>>";
        let lines = fnm_hook_lines();
        // 旧 ome 标记块：迁移为 ark 标记且块体一并升级
        let old = "# >>> ome fnm >>>\neval \"$(fnm env)\"\n# <<< ome fnm <<<\n";
        let t = merge_hook_block(old, marker, &lines);
        assert!(!t.contains(">>> ome fnm"), "旧标记退役");
        assert!(t.contains(marker) && t.contains(lines[1]));
        assert_eq!(merge_hook_block(&t, marker, &lines), t, "迁移后幂等");
        // 同标记重复块收敛为一块
        let block = "# >>> ark fnm >>>\nwhatever\n# <<< ark fnm <<<\n";
        let dup = format!("{block}{block}");
        let t2 = merge_hook_block(&dup, marker, &lines);
        assert_eq!(t2.matches(marker).count(), 1, "重复块收敛");
        assert!(!t2.contains("whatever"), "残体不带");
    }

    #[test]
    fn profile钩子块_未闭合截断与标记空白容错() {
        // 未闭合起标记：既有家族语义，该行之后原文截断收口（doc comment 已钉）
        let marker = "# >>> ark fnm >>>";
        let lines = fnm_hook_lines();
        let unclosed = "A=1\n# >>> ark fnm >>>\nold body\nB=2\n";
        let t = merge_hook_block(unclosed, marker, &lines);
        assert!(
            !t.contains("B=2") && !t.contains("old body"),
            "未闭合截断收口"
        );
        assert!(
            t.contains("A=1\n") && t.contains(lines[1]),
            "闭标记前原文与新块体保留"
        );
        assert_eq!(merge_hook_block(&t, marker, &lines), t, "截断后幂等");
        // 标记行带行首行尾空白：识别并规范重写
        let padded = "A=1\n  # >>> ark fnm >>>  \nold\n\t# <<< ark fnm <<< \nB=2\n";
        let t2 = merge_hook_block(padded, marker, &lines);
        assert!(!t2.contains("old"), "空白标记块体同样替换");
        assert!(t2.contains("A=1\n") && t2.contains("B=2\n"), "块外原文不动");
        // 原文无尾换行：插入路径保持无尾换行态且幂等
        let no_nl = "A=1\n# >>> ark fnm >>>\nold\n# <<< ark fnm <<<";
        let t3 = merge_hook_block(no_nl, marker, &lines);
        assert!(!t3.ends_with('\n'), "无尾换行态保持");
        assert_eq!(merge_hook_block(&t3, marker, &lines), t3, "无尾换行幂等");
    }

    #[cfg(not(windows))]
    #[test]
    fn expand_env_vars_linux_语法() {
        std::env::set_var("ARK_TEST_PLAT_X", "/home/demo");
        assert_eq!(expand_env_vars("$ARK_TEST_PLAT_X/bin"), "/home/demo/bin");
        assert_eq!(expand_env_vars("${ARK_TEST_PLAT_X}/bin"), "/home/demo/bin");
        assert_eq!(expand_env_vars("$ARK_NO_SUCH_VAR/x"), "$ARK_NO_SUCH_VAR/x");
    }

    #[cfg(not(windows))]
    #[test]
    fn is_official_exe_linux_判定() {
        assert!(is_official_exe("$HOME/.local/bin/jq"));
        assert!(is_official_exe("/usr/local/bin/jq"));
        assert!(!is_official_exe("jq/bin/jq"));
    }

    #[cfg(not(windows))]
    #[test]
    fn path_entries_eq_大小写敏感_仅linux() {
        assert!(path_entries_eq("/home/a/bin", "/home/a/bin"));
        assert!(!path_entries_eq("/home/a/bin", "/home/A/bin"));
    }
}
