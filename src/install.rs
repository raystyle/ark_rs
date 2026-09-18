//! install：安装主编排，对齐 helpers.ps1 的 Install-ToolVersion（1019-1267 行）。
//! 流程：防穿越校验 → 幂等跳过（顺带补 PATH/sha 回填/滞后锁定）→ sha 基准（pin > 官方源）
//! → 下载（含 bootstrap 资产与 MZ 头校验）→ 删旧目录全量重建 → extract 分派
//! → 装后验版本（5 次递增重试）→ 回写 lock（write_pin / write_sha256）。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::catalog::{self, Catalog, Tool};
use crate::checksum;
use crate::download;
use crate::extract;
use crate::resolve::Resolution;
use crate::toolver;

/// 安装选项（对齐 Install-ToolVersion 的 -RegisterPath / -UpdateLock / -Force）。
/// `configure` 为 deploy 侧：PATH、用户环境变量、注册表与配置；download 为 false。
#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    /// 安装后落 manifest 配置（env_set/shims/mirror）
    pub configure: bool,
    /// 回写 pin 锁（update 传 false）
    pub update_lock: bool,
    /// 强制重装跳过幂等检查
    pub force: bool,
}

/// 安装结果动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallAction {
    /// 新装落位
    Installed,
    /// 幂等跳过（已在位同版）
    Skipped,
}

impl InstallAction {
    /// 动作短名（emit 块 value）。
    pub fn as_str(&self) -> &'static str {
        match self {
            InstallAction::Installed => "installed",
            InstallAction::Skipped => "skipped",
        }
    }
}

/// 安装结果：动作、版本、安装目录（msi 无绿色目录，为 None）。
pub struct InstallOutcome {
    /// 安装动作（installed/skipped）
    pub action: InstallAction,
    /// 安装落定版本
    pub version: String,
    /// 安装目录（msi 无绿色目录为 None）
    pub dir: Option<PathBuf>,
}

/// 防穿越：path 必须在允许的安全根之下。
/// Windows：在 EnvRoot 之下（大小写不敏感）；
/// Linux / macOS：在 $HOME 之下，或显式在 EnvRoot 之下。
pub fn is_safe_under_root(root: &Path, path: &Path) -> bool {
    let (Ok(full_root), Ok(full_path)) = (std::path::absolute(root), std::path::absolute(path))
    else {
        return false;
    };
    #[cfg(windows)]
    {
        let root_str = full_root.to_string_lossy().replace('/', "\\");
        let root_str = root_str.trim_end_matches('\\').to_lowercase() + "\\";
        let path_str = full_path
            .to_string_lossy()
            .replace('/', "\\")
            .to_lowercase();
        path_str.starts_with(&root_str)
    }
    #[cfg(not(windows))]
    {
        let home = dirs::home_dir().unwrap_or_else(|| full_root.clone());
        let allowed_roots = [home, full_root];
        allowed_roots.iter().any(|r| {
            let r_str = r.to_string_lossy().trim_end_matches('/').to_string() + "/";
            full_path.to_string_lossy().starts_with(&r_str)
        })
    }
}

/// 安装目录解析：msi 无绿色目录；official 走 exe 上两级（官方目录）；其余 EnvRoot\dir。
fn install_dir(
    def: &Tool,
    env_root: &Path,
    is_msi: bool,
    is_official: bool,
) -> Result<Option<PathBuf>, String> {
    if is_msi {
        return Ok(None);
    }
    if is_official {
        let exe = toolver::exe_path(def, env_root)?;
        let dir = exe
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| "official exe 路径无法上溯两级".to_string())?;
        return Ok(Some(dir.to_path_buf()));
    }
    let dir = def.dir().ok_or_else(|| "工具缺少 dir 字段".to_string())?;
    // {version} 占位仅 bin/exe 支持（对线 Z2）：dir 写占位会静默落字面 {version} 目录
    // 并与 exe 布局错位，显式拒绝
    if dir.contains("{version}") {
        return Err("dir 字段不支持 {version} 占位（仅 bin/exe 支持）".to_string());
    }
    Ok(Some(crate::platform::join_if_relative(
        env_root,
        crate::platform::expand_install_path(dir),
    )))
}

/// 注册 PATH 的目录：official 取 exe 上一级（展开后），其余按平台字段解析（支持 `~` 与 `$VAR`）。
fn bin_dir(
    def: &Tool,
    env_root: &Path,
    is_official: bool,
    version: &str,
) -> Result<Option<PathBuf>, String> {
    if is_official {
        let exe = toolver::exe_path(def, Path::new("."))?;
        return Ok(exe.parent().map(Path::to_path_buf));
    }
    // {version} 占位（D43 zig 版本目录布局）：以解析版本直替换（PATH 指向本版目录）
    Ok(def.bin().map(|b| {
        crate::platform::join_if_relative(
            env_root,
            crate::platform::expand_install_path(&b.replace("{version}", version)),
        )
    }))
}

/// 安装单工具（下载 → 校验 → 解压 → 验版本 → 回写）。
///
/// # Errors
/// 返回 Err（人读原因串）当：危险路径，拒绝操作: {} 等（完整失败面见函数体错误构造）。
pub fn install_tool(
    cat: &Catalog,
    env_root: &Path,
    name: &str,
    res: &Resolution,
    opts: &InstallOptions,
) -> Result<InstallOutcome, String> {
    let def = cat.tool(name)?;
    // D39 R016：安装配置部署逻辑数据面——与 tools.toml 同目录的 manifest.toml，
    // 工具节有原语用数据、无节走内建回退（双轨过渡，omc manifest 上线后撤内建）
    let mf = crate::manifest::load(&cat.path).unwrap_or_else(|e| {
        eprintln!("[WARN] {e}（忽略，本次零原语）");
        crate::manifest::ManifestFile::default()
    });
    // manifest 节键：catalog 的 manifest 字段声明的节键优先，缺省同名节（R016 二）
    let ms = mf.manifest.get(def.manifest.as_deref().unwrap_or(name));
    let is_msi = def.extract() == Some("msi");
    let is_official = toolver::is_official(def);
    let install_dir = install_dir(def, env_root, is_msi, is_official)?;
    let exe_path = toolver::exe_path(def, env_root)?;
    // shim 落点：目标二进制所在目录（R016「同 bin 目录」；POSIX 嵌套布局与 official 型下
    // install_dir 可能不是二进制所在目录，取 exe 父目录才与 PATH 注册面一致）。msi 无 EnvRoot 落点，跳过。
    let shim_dir = install_dir.as_deref().and_then(|_| exe_path.parent());

    // 防穿越：绿色目录必须在 EnvRoot 下（official/msi 除外）
    if !is_msi && !is_official {
        let dir = install_dir
            .as_ref()
            .ok_or_else(|| format!("{name} 无法确定安装目录"))?;
        if !is_safe_under_root(env_root, dir) {
            return Err(format!("危险路径，拒绝操作: {}", dir.display()));
        }
    }

    // ── agent 类存量纳管（D07，2026-09-05 用户三裁延续）：PATH 任意位在位即跳过，
    //    不迁移不重装（对齐 oma agents install 的「已装任何来源即跳过」判定）；--force 才装 EnvRoot ──
    //  注意：非 agent 工具不在此纳管（install 是「显式装入管理面」意图，PATH 同版不拦，
    //  REQ-0003 收窄裁定：PATH 存量已达锁定版的纳管只在 update 面）。
    if def.category.as_deref() == Some("agent") && !opts.force {
        if let Some(found) = toolver::find_on_path(name) {
            eprintln!(
                "[INFO] {name} 已在 PATH 安装（{}），存量原地纳管跳过（--force 装进 EnvRoot）",
                found.display()
            );
            return Ok(InstallOutcome {
                action: InstallAction::Skipped,
                version: res.version.clone(),
                dir: install_dir,
            });
        }
    }

    // ── 幂等跳过：已装版本 == 解析版本且非 force ──
    let cur = toolver::installed_version(&exe_path, def);
    if !opts.force && cur.as_deref() == Some(res.version.as_str()) {
        eprintln!(
            "[INFO] {name} {} 已安装，跳过（--force 强制重装）",
            res.version
        );
        let cache = download::cache_path(env_root, &res.asset_name);
        // 顺带回填 sha256（命中缓存时）
        if def.pin_sha256().is_none() && cache.exists() {
            let sha = download::sha256_file(&cache)?;
            catalog::write_sha256(&cat.path, name, &sha)?;
            eprintln!("[OK] 已回填 sha256（命中缓存）");
        }
        // REQ-0012：skip 分支补落痕（标记随目录重建自然消失，此处幂等补齐）
        if !is_msi {
            if let Some(dir) = exe_path.parent() {
                let _ = mark_ark_managed(dir);
            }
        }
        if opts.configure {
            register_bin(def, env_root, is_official, &res.version)?;
            // 老环境补别名（bun 已存在但同目录缺 bunx.exe）：manifest shims 节唯一来源
            // （omc 数据面已上线并验收，2026-09-11 撤 ensure_bunx_shim 内建双轨）。
            // 幂等分支同样执行 post_install：既是补装漏，也是主分支失败后的重试路径（共识③）
            apply_manifest_primitives(ms, name, shim_dir)?;
            ensure_user_bin_link(name, &exe_path);
            ensure_fnm_shell_hook(name);
        }
        if opts.update_lock && def.pin_tag() != Some(res.tag.as_str()) {
            // 已安装版本与解析一致但锁定滞后（如上次安装中断）：补齐锁定
            catalog::write_pin(&cat.path, name, res)?;
            if cache.exists() {
                let sha = download::sha256_file(&cache)?;
                catalog::write_sha256(&cat.path, name, &sha)?;
            }
            eprintln!("[OK] {name} 已锁定: {}（补齐滞后锁定）", res.version);
        }
        return Ok(InstallOutcome {
            action: InstallAction::Skipped,
            version: res.version.clone(),
            dir: install_dir,
        });
    }

    // uv-git 型：uv tool install git 安装（无下载资产无 sha，幂等逻辑上面已覆盖）
    if def.extract() == Some("uv-git") {
        let out = install_uv_git(cat, name, def, res, opts, &exe_path, install_dir)?;
        // D39 共识②：早退通道的 manifest 原语应用点留在调用侧（通道签名不必为 manifest 增参）
        if opts.configure {
            apply_manifest_primitives(ms, name, exe_path.parent())?;
            ensure_user_bin_link(name, &exe_path);
        }
        return Ok(out);
    }

    // npm-tgz 型：release tgz 过锚下载后 npm install -g（Node CLI；幂等逻辑上面已覆盖）
    if def.extract() == Some("npm-tgz") {
        let out = install_npm_tgz(cat, name, def, res, opts, env_root, install_dir)?;
        // D39 共识②：同上（npm 全局 bin 的 exe 父目录即 shims 落点，落点可漂移见 R016 注）。
        // O7（S017）：外层 exe_path 是注入前解析的裸名（npm-tgz 的 PATH 现查形态），
        // symlink 会建出字面自引用；install_npm_tgz 已注入 fnm 静态位进进程 PATH，
        // 此处重解析得绝对路径（锚定静态位，multishell 只做执行期）
        if opts.configure {
            let exe_abs = toolver::exe_path(def, env_root)?;
            apply_manifest_primitives(ms, name, exe_abs.parent())?;
            ensure_user_bin_link(name, &exe_abs);
        }
        return Ok(out);
    }

    // ── sha 校验优先级：pin 的 sha256 > 官方校验源三型 ──
    let expected_sha = checksum::expected_sha256(def, res, env_root)?;

    // 下载（tag 与锁定不一致时强制重下，对齐 -Force:$forceDownload）；
    // 镜像优先（D44）：env.ohmygh.com 单次首试，未命中或锚不符回落官方；有锚必校验（pin、镜像版本段边车或官方 sums）；
    // D51：pin 驱动解析的 asset_url 是镜像主通道 URL，官方回落地址取 fallback_url（在位时）
    let force_download = def.pin_tag() != Some(res.tag.as_str());
    let official_url = res.fallback_url.as_deref().unwrap_or(&res.asset_url);
    let cache = download::download_asset_with_mirror(
        env_root,
        &res.asset_name,
        official_url,
        expected_sha.as_deref(),
        force_download,
        name,
        &res.version,
    )?;

    // 额外 bootstrap 资产（如 7z 的 7zr.exe）：仅 Windows 下 7z-extra 使用；先下载最小解压器，MZ 头校验
    #[cfg(windows)]
    if let Some(bootstrap) = def.bootstrap_asset() {
        let repo = def
            .repo()
            .ok_or_else(|| format!("{name} 使用 bootstrap_asset 但缺少 repo 字段"))?;
        let boot_url = format!(
            "https://github.com/{repo}/releases/download/{}/{bootstrap}",
            res.tag
        );
        let boot_path = download::download_asset_with_mirror(
            env_root,
            bootstrap,
            &boot_url,
            None,
            false,
            name,
            &res.version,
        )?;
        let head = fs::read(&boot_path)
            .map_err(|e| format!("读取 BootstrapAsset 失败: {}: {e}", boot_path.display()))?;
        if head.len() < 2 {
            return Err(format!("{name} BootstrapAsset 缺失或为空: {bootstrap}"));
        }
        if head[0] != 0x4D || head[1] != 0x5A {
            return Err(format!(
                "{name} BootstrapAsset 不是有效 Windows 可执行文件: {bootstrap}"
            ));
        }
    }

    // 下载后 sha：同 tag 且同资产才用 pin sha 核对；仅同发行缺 sha 时回填。
    // D37 起 update 与 install 都不回写 pin（锁定单源归数据面），跨 tag 新 sha 自然不落 pin 字段。
    let sha = download::sha256_file(&cache)?;
    let pinned_asset = def.pin_asset().unwrap_or("");
    let same_asset = pinned_asset.is_empty() || pinned_asset == res.asset_name;
    let same_release = def.pin_tag() == Some(res.tag.as_str()) && same_asset;
    let sha_backfilled = if same_release {
        if let Some(pinned) = def.pin_sha256() {
            if !sha.eq_ignore_ascii_case(pinned) {
                return Err(format!("{name} 缓存 sha256 与锁定不符"));
            }
            false
        } else {
            true
        }
    } else {
        false
    };

    // 删旧目录全量重建（Windows 绿色目录类）。Linux 多个工具可能共享 ~/.local/bin，不整删。
    #[cfg(windows)]
    if !is_msi && !is_official {
        if let Some(dir) = &install_dir {
            if dir.exists() {
                fs::remove_dir_all(dir)
                    .map_err(|e| format!("删除旧目录失败: {}: {e}", dir.display()))?;
            }
            fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {}: {e}", dir.display()))?;
        }
    }
    #[cfg(not(windows))]
    if !is_msi && !is_official {
        if let Some(dir) = &install_dir {
            fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {}: {e}", dir.display()))?;
            // 仅删除目标 exe 文件（不清理共享 bin 目录下的其他工具）
            let _ = fs::remove_file(&exe_path);
        }
    }

    // extract 分派（msi/rmux 分支不使用 install_dir，传 env_root 占位）
    let target_dir = install_dir.as_deref().unwrap_or(env_root);
    extract::extract_asset(name, def, &cache, target_dir, env_root)?;

    // 装后验版本（5 次递增重试：7zsfx 等解包后文件/杀软可能瞬态未就绪）
    // D43：装后验证锚定刚装版本（占位布局 glob 取 max 在降级场景会锚错）
    let verify_exe = toolver::exe_path_for_version(def, env_root, &res.version)?;
    let installed = toolver::installed_version_retried(&verify_exe, def).ok_or_else(|| {
        format!(
            "{name} 安装后未找到可执行文件或无法读取版本: {}",
            verify_exe.display()
        )
    })?;
    if installed != res.version {
        return Err(format!(
            "{name} 版本不符: 期望 {}，实际 {installed}",
            res.version
        ));
    }
    eprintln!(
        "[OK] {name} 安装完成: {installed} @ {}",
        verify_exe.display()
    );

    // 成功才回写 lock
    if sha_backfilled {
        catalog::write_sha256(&cat.path, name, &sha)?;
    }
    if opts.configure {
        register_bin(def, env_root, is_official, &res.version)?;
        // D39 R016 L1/L2：manifest 节的原语在装成后统一应用（env_set、shims、post_install）
        apply_manifest_primitives(ms, name, shim_dir)?;
        ensure_user_bin_link(name, &exe_path);
        ensure_fnm_shell_hook(name);
    }
    if opts.update_lock {
        catalog::write_pin(&cat.path, name, res)?;
        catalog::write_sha256(&cat.path, name, &sha)?;
        eprintln!("[OK] {name} 已锁定: {}", res.version);
    }

    // REQ-0012：真身落痕（绿色与 official 型；msi 真身在系统位不落）
    if !is_msi {
        if let Some(dir) = exe_path.parent() {
            if let Err(e) = mark_ark_managed(dir) {
                eprintln!("[WARN] {e}（不拦安装）");
            }
        }
    }

    Ok(InstallOutcome {
        action: InstallAction::Installed,
        version: installed,
        dir: install_dir,
    })
}

/// deploy 侧：应用 manifest 节的用户级环境变量（幂等；download 不写注册表）。
/// D39 R016：唯一来源是 manifest env_set（omc 数据面已上线并验收，2026-09-11 撤内建双轨）；
/// 无节零动作。
fn ensure_user_env_overrides(ms: Option<&crate::manifest::ToolManifest>) -> Result<(), String> {
    // D39 R016：用户级配置唯一来源是 manifest 节 env_set（omc 数据面已上线三节并验收，
    // 2026-09-11 撤内建双轨收口）；无节零动作
    if let Some(m) = ms {
        return crate::manifest::apply_env_set(m);
    }
    Ok(())
}

/// POSIX 嵌套布局可发现性兜底的核心（纯函数化便于测）：保证 `user_bin/<name>` 指向装好的 exe。
/// 靶场（ohmycloud lan-linux 实测 2026-09-11）：skip 分支注册的 PATH 目录写在 profile，
/// 非交互 shell（omc hostExec）不加载而形同虚设；`~/.local/bin` 是 XDG 用户 bin 基建，
/// **多数发行版与 hostExec 的 PATH 都含**（注意：只剩最小 PATH 的 env 里同样无效，本兜底
/// 不等于把 PATH 送进去）。
/// 语义：已指对即跳过（幂等）；悬空或指向旧 target（版本目录型布局升级后）先删再建
/// ——**直链以 ark 装的那份为准**，既存链接指向别处即重指；落点若是非链接的真文件
/// （用户自装）绝不覆盖；exe 本就落该目录（多数 POSIX 绿色工具）时不建自指链接。
/// 返回是否真的建/重指了链接（false = 已指对、真文件、或本就同路径，供调用方决定是否出声）。
#[cfg(not(windows))]
fn link_into_user_bin(user_bin: &Path, name: &str, exe: &Path) -> Result<bool, String> {
    let dst = user_bin.join(name);
    if exe == dst {
        return Ok(false);
    }
    match std::fs::read_link(&dst) {
        Ok(target) if target == exe => return Ok(false),
        Ok(_) => {
            std::fs::remove_file(&dst)
                .map_err(|e| format!("清旧直链失败 {}: {e}", dst.display()))?;
        }
        Err(_) if dst.exists() => return Ok(false),
        Err(_) => {}
    }
    std::fs::create_dir_all(user_bin)
        .map_err(|e| format!("建目录失败 {}: {e}", user_bin.display()))?;
    std::os::unix::fs::symlink(exe, &dst)
        .map_err(|e| format!("直链失败 {} -> {}: {e}", dst.display(), exe.display()))?;
    Ok(true)
}

/// 生产入口：落点固定 `~/.local/bin`，失败只 WARN（可发现性兜底不该拦安装）。
#[cfg(not(windows))]
fn ensure_user_bin_link(name: &str, exe: &Path) {
    if crate::platform::user_env_write_blocked() {
        return;
    }
    let Some(home) = dirs::home_dir() else { return };
    match link_into_user_bin(&home.join(".local").join("bin"), name, exe) {
        Ok(true) => eprintln!(
            "[OK] 用户 bin 直链已建: ~/.local/bin/{name} -> {}",
            exe.display()
        ),
        Ok(false) => {}
        Err(e) => eprintln!("[WARN] {e}"),
    }
}

#[cfg(windows)]
fn ensure_user_bin_link(_name: &str, _exe: &Path) {}

/// manifest 原语应用点（L1 env_set、mirror 与 shims、L2 post_install）：**全链唯一实现**，
/// 主链（幂等分支与成功尾）与 uv-git/npm-tgz 早退通道共用，避免多份并行漂移。
/// 失败语义与主链一致：env_set、mirror 与 shims 硬错（`?`，L1 写不进就是没配上），
/// post_install 降 WARN 不拦安装收尾（D39 共识③）。
/// mirror 在 post_install 之前落源：镜像 env 进程内即设，fnm install 等 L2 命令
/// 立即经 npmmirror 拉 node（D42）。
fn apply_manifest_primitives(
    ms: Option<&crate::manifest::ToolManifest>,
    name: &str,
    shim_dir: Option<&Path>,
) -> Result<(), String> {
    let Some(m) = ms else { return Ok(()) };
    ensure_user_env_overrides(ms)?;
    if m.mirror.is_some() {
        let home = dirs::home_dir().ok_or("无法确定用户主目录（mirror 落源）")?;
        crate::manifest::apply_mirror(m, name, &home)?;
    }
    if m.shims.is_some() {
        match shim_dir.filter(|d| d.is_absolute()) {
            Some(dir) => crate::manifest::apply_shims(m, dir)?,
            // 相对/空落点（npm-tgz 的 exe 未落 PATH 时 exe_path 只有裸名）不可用，
            // 绝不退化成「按 CWD 拼相对路径」（防在工作目录里造出别名）
            None => eprintln!(
                "[WARN] manifest shims 落点不可定位（exe 不在 PATH 或为相对路径），跳过别名生成"
            ),
        }
    }
    if let Err(e) = crate::manifest::run_post_install(m, name) {
        eprintln!("[WARN] {e}；工具本体已装成，重跑 ark install {name} 可重试 post_install");
    }
    Ok(())
}

/// 注册 bin 目录进用户 PATH（Windows 注册表 / Linux profile）。
/// ark-managed 落痕（REQ-0012 生产者契约，browse 等家族自更新让位判据依赖）：
/// ark 安装的真身二进制同目录落 `ark-managed` 标记文件，内容 = ark 版本号；
/// 幂等（同内容零重写）。卸载语义：ark 无 uninstall 命令，标记随工具目录重建重写、
/// 手删目录即连标记消失（doctor 孤儿检测只扫 cache 目录不受影响）。
/// 特型（npm-tgz/uv-git/msi 与 rustup/vsbuild 专用模块）真身在他管域不落。
fn mark_ark_managed(exe_dir: &Path) -> Result<(), String> {
    let marker = exe_dir.join("ark-managed");
    let version = env!("CARGO_PKG_VERSION");
    if let Ok(existing) = std::fs::read_to_string(&marker) {
        if existing.trim() == version {
            return Ok(()); // 幂等：同内容零重写
        }
    }
    std::fs::write(&marker, format!("{version}\n"))
        .map_err(|e| format!("写 ark-managed 落痕失败: {}: {e}", marker.display()))
}

fn register_bin(
    def: &Tool,
    env_root: &Path,
    is_official: bool,
    version: &str,
) -> Result<(), String> {
    // npm-tgz：bin 落 npm 全局 bin（npm 自管 PATH 面），不注册 EnvRoot 目录（防死链）
    if def.extract() == Some("npm-tgz") {
        return Ok(());
    }
    if let Some(dir) = bin_dir(def, env_root, is_official, version)? {
        if crate::platform::add_user_path(&dir)? {
            eprintln!("[OK] PATH 已注册: {}（新终端生效）", dir.display());
        } else {
            eprintln!("[INFO] PATH 已含: {}", dir.display());
        }
    }
    Ok(())
}

/// uv-git 安装：uv tool install --force git+repo@tag（依赖 uv 运行时；shim 落 uv 管的
/// UV_TOOL_BIN_DIR，register 走 official 分支注册该目录）。
/// 升级占用解锁（browser-harness M102 方案吸收）：Windows 运行中进程锁 venv Scripts\，
/// uv 无法替换——升级前用工具自身 CLI 停守护（--reload 停 daemon、rmux kill-server 停
/// worker 会话），装后提示 x-monitor 恢复值守栈（ark 不擅自拉起）。
fn install_uv_git(
    cat: &Catalog,
    name: &str,
    def: &Tool,
    res: &Resolution,
    opts: &InstallOptions,
    exe_path: &Path,
    install_dir: Option<PathBuf>,
) -> Result<InstallOutcome, String> {
    let uv = which::which("uv").map_err(|_| {
        format!("{name} 为 uv tool 安装型，需要 uv 运行时在位（先 ark install uv）")
    })?;
    let repo = def
        .repo()
        .ok_or_else(|| format!("{name} 缺少 repo 字段（uv-git 型必需）"))?;
    let url = format!("git+https://github.com/{repo}@{}", res.tag);

    // 升级（旧 exe 在位）：先停工具自己的守护栈解锁 venv（best-effort，CLI 子命令以工具为准）
    let upgrading = exe_path.exists();
    if upgrading {
        for args in [["--reload"].as_slice(), ["rmux", "kill-server"].as_slice()] {
            if let Ok(status) = Command::new(exe_path).args(args).status() {
                if !status.success() {
                    eprintln!("[WARN] {} {:?} 停止未成功（继续尝试解锁）", name, args);
                }
            }
        }
    }

    eprintln!("[INFO] uv tool install {url}（依赖 uv 运行时，首次较慢）");
    let status = Command::new(&uv)
        .args(["tool", "install", "--force"])
        .arg(&url)
        .status()
        .map_err(|e| format!("uv 启动失败: {e}"))?;
    if !status.success() {
        return Err(format!(
            "{name} uv tool install 失败 exit={}（venv 可能被运行中进程锁定，停掉后重试）",
            status.code().unwrap_or(-1)
        ));
    }
    let version = toolver::installed_version_retried(exe_path, def)
        .ok_or_else(|| format!("{name} 装后版本探测失败（检查 catalog probe_pattern 字段）"))?;
    if opts.update_lock && def.pin_tag() != Some(res.tag.as_str()) {
        catalog::write_pin(&cat.path, name, res)?;
        eprintln!("[OK] {name} 已锁定: {}", res.version);
    }
    if opts.configure {
        register_bin(def, Path::new("."), true, &res.version)?;
    }
    if upgrading {
        eprintln!("[HINT] 升级已停守护栈；恢复值守: {name} x-monitor");
    }
    Ok(InstallOutcome {
        action: InstallAction::Installed,
        version,
        dir: install_dir,
    })
}

/// npm-tgz 安装：release tgz 下载过锚后 npm install -g（Node CLI，如 browser-harness 的 bh）。
/// bin 落 npm 全局 bin（随各端 node 生态走，无静态路径）：exe 定位与幂等探测走 PATH 现查
/// （toolver::exe_path 的 npm-tgz 分支）；需 node 与 npm 在 PATH（fnm 供给）。
/// O3（ohmycloud S017）：npm 不在 PATH 时经 fnm 解析 node 的安装 bin 目录
/// （默认别名优先，否则最高版本目录），返回应 prepend 到子进程 PATH 的路径。
/// 非交互 shell（omc hostExec）不加载 profile，fnm 钩子对其无效——进程内解析才是治本。
#[cfg(not(windows))]
fn fnm_node_bin() -> Option<PathBuf> {
    fnm_node_bin_in(&dirs::home_dir()?.join(".local").join("share").join("fnm"))
}

/// 解析核心（传 base 便于测；纯逻辑无副作用）：默认别名优先，否则取含 npm 的**数值段最大**版本目录。
#[cfg(not(windows))]
fn fnm_node_bin_in(base: &Path) -> Option<PathBuf> {
    // aliases/default 符号链接 -> node-versions/<v>/installation（readlink 可能给相对路径）
    let via_alias = std::fs::read_link(base.join("aliases").join("default"))
        .ok()
        .and_then(|p| {
            let p = if p.is_absolute() {
                p
            } else {
                base.join("aliases").join(&p)
            };
            p.join("bin").canonicalize().ok()
        })
        // 别名指向的版本也得真含 npm，否则落回版本扫描（别名指向残缺目录时不至于就断链）
        .filter(|bin| bin.join("npm").exists());
    if via_alias.is_some() {
        return via_alias;
    }
    // 无别名：取数值段最大的版本目录。**不能用字典序**：字符串序会把 v9 排在 v24 之前
    // （与 resolve::pick_max_semver 同源教训，二者互指）。
    let mut best: Option<(Vec<u64>, PathBuf)> = None;
    let rd = std::fs::read_dir(base.join("node-versions")).ok()?;
    for e in rd.flatten() {
        let bin = e.path().join("installation").join("bin");
        if !bin.join("npm").exists() {
            continue;
        }
        let Some(key) = crate::resolve::version_key(&e.file_name().to_string_lossy()) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(k, _)| crate::resolve::semver_cmp(&key, k) == std::cmp::Ordering::Greater)
        {
            best = Some((key, bin));
        }
    }
    best.map(|(_, b)| b)
}

#[cfg(windows)]
fn fnm_node_bin() -> Option<PathBuf> {
    None
}

/// M027：fnm profile 钩子块体（单一权威，platform 测试引用）。自含 PATH 导出
/// （不依赖 rc 内相对执行序：裸 `eval "$(fnm env)"` 在 PATH 含 ~/.local/bin 之前
/// 执行会报 fnm not found）；`command -v` 守卫缺席静默，且 if 形态不改变 source
/// 退出码（errexit 下 source rc 仍返回 0）。
pub(crate) const FNM_HOOK_LINES: [&str; 2] = [
    "export PATH=\"$HOME/.local/bin:$PATH\"",
    "if command -v fnm >/dev/null 2>&1; then eval \"$(fnm env)\"; fi",
];

/// O3-b：fnm 装后写 profile 钩子（ark fnm 标记块，幂等且块体感知）。
/// 交互 shell 经钩子取 node；非交互（omc hostExec）由 fnm_node_bin 进程内解析治本。
/// node 版本供给归数据面（manifest fnm 节 post_install：`fnm install <ver>` 加
/// `fnm default <ver>`，fnm 在 ~/.local/bin，非交互同样可跑）。
fn ensure_fnm_shell_hook(name: &str) {
    if name == "fnm" {
        crate::platform::ensure_profile_hook("# >>> ark fnm >>>", &FNM_HOOK_LINES);
    }
}

/// 会话级 shim 判据（O6）：fnm multishell 目录（`fnm_multishells/<pid>_<ts>/bin`，Linux 在
/// `/run/user/<uid>/` 下、Windows 在 `%LOCALAPPDATA%` 下）随会话注销即回收，凡锚定必悬空。
fn is_session_shim(path: &Path) -> bool {
    path.to_string_lossy()
        .split(['/', '\\'])
        .any(|seg| seg.starts_with("fnm_multishells"))
}

/// 子进程 PATH 与 npm 落点：PATH 里是真安装的 npm 直用（不抢用户既定的 node 生态）；
/// npm 缺席、或命中的是会话级 shim 时改用 fnm 静态位（`node-versions/<v>/installation/bin`），
/// 会话目录只做执行期环境（O6：交互 shell 的 fnm hook 会把它前置，锚上去注销即悬空）。
fn npm_cmd_env() -> (std::path::PathBuf, Option<std::ffi::OsString>) {
    let on_path = which::which("npm").ok();
    if let Some(npm) = &on_path {
        if !is_session_shim(npm) {
            return (npm.clone(), None);
        }
    }
    if let Some(bin) = fnm_node_bin() {
        if let Ok(npm) = which::which_in(
            "npm",
            Some(&bin),
            std::env::current_dir().unwrap_or_default().as_path(),
        ) {
            let path = std::env::var_os("PATH").unwrap_or_default();
            let mut parts: Vec<PathBuf> = std::env::split_paths(&path).collect();
            parts.insert(0, bin.clone());
            if let Ok(joined) = std::env::join_paths(parts) {
                // 同时注入自身进程 PATH：装后版本探测（exe_path 的 PATH 现查）同链生效
                std::env::set_var("PATH", &joined);
                eprintln!(
                    "[INFO] 经 fnm 静态位解析 node（multishell 只做执行期）: {}",
                    npm.display()
                );
                return (npm, Some(joined));
            }
        }
    }
    // 静态位也不可用（fnm 损坏、未装 node）时的兜底：仍用 PATH 现查（哪怕是会话 shim，
    // 至少能装），都没有才回落裸名由调用方报「先 ark install fnm」。
    if let Some(npm) = on_path {
        return (npm, None);
    }
    (std::path::PathBuf::from("npm"), None)
}

fn install_npm_tgz(
    cat: &Catalog,
    name: &str,
    def: &Tool,
    res: &Resolution,
    opts: &InstallOptions,
    env_root: &Path,
    install_dir: Option<PathBuf>,
) -> Result<InstallOutcome, String> {
    let (npm, npm_path_env) = npm_cmd_env();
    if npm_path_env.is_none() && which::which("npm").is_err() {
        return Err(format!(
            "{name} 为 npm 全局装型，需要 node 与 npm 在 PATH（先 ark install fnm 装 node，npm 型安装会自动经 fnm 解析）"
        ));
    }
    // exe_path 在主链开头解析（fnm 注入前），npm-tgz 的 PATH 现查形态此时是裸名；
    // 注入后重解析（find_on_path 现可命中 fnm node bin）
    let exe_path = toolver::exe_path(def, env_root)?;

    let expected = checksum::expected_sha256(def, res, env_root)?;
    // D51：官方回落地址取 fallback_url（pin 驱动解析在位），asset_url 为镜像主通道
    let official_url = res.fallback_url.as_deref().unwrap_or(&res.asset_url);
    let cache = download::download_asset_with_mirror(
        env_root,
        &res.asset_name,
        official_url,
        expected.as_deref(),
        true,
        name,
        &res.version,
    )?;

    eprintln!(
        "[INFO] npm install -g {}（依赖拉取走 npm registry，首次较慢）",
        cache.display()
    );
    let mut cmd = Command::new(&npm);
    if let Some(p) = &npm_path_env {
        cmd.env("PATH", p);
    }
    let status = cmd
        .args(["install", "-g", "--no-fund", "--no-audit"])
        .arg(&cache)
        .status()
        .map_err(|e| format!("npm 启动失败: {e}"))?;
    if !status.success() {
        return Err(format!(
            "{name} npm install -g 失败 exit={}",
            status.code().unwrap_or(-1)
        ));
    }

    let version = toolver::installed_version_retried(&exe_path, def).ok_or_else(|| {
        format!("{name} 装后版本探测失败（检查 PATH 与 catalog probe_pattern 字段）")
    })?;
    if opts.update_lock && def.pin_tag() != Some(res.tag.as_str()) {
        catalog::write_pin(&cat.path, name, res)?;
        if cache.exists() {
            let sha = download::sha256_file(&cache)?;
            catalog::write_sha256(&cat.path, name, &sha)?;
        }
        eprintln!("[OK] {name} 已锁定: {}", res.version);
    }
    eprintln!("[OK] {name} 安装完成: {version} @ {}", exe_path.display());
    Ok(InstallOutcome {
        action: InstallAction::Installed,
        version,
        dir: install_dir,
    })
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn 防穿越_root内放行_同级与上级拒绝() {
        let root = Path::new(r"D:\sandbox\env");
        assert!(is_safe_under_root(root, Path::new(r"D:\sandbox\env\jq")));
        assert!(
            is_safe_under_root(root, Path::new(r"d:\SANDBOX\env\jq")),
            "大小写不敏感"
        );
        assert!(!is_safe_under_root(root, Path::new(r"D:\sandbox\evil")));
        assert!(!is_safe_under_root(root, Path::new(r"D:\sandbox")));
        // root 自身不算「之下」（root 补尾斜杠后自身不匹配）
        assert!(!is_safe_under_root(root, root));
        // 兄弟前缀不能误判（env2 不是 env 之下）
        assert!(!is_safe_under_root(root, Path::new(r"D:\sandbox\env2\jq")));
    }

    #[test]
    fn 防穿越_相对路径按当前目录展开() {
        // std::path::absolute 不做符号链接解析，与 GetFullPath 语义一致
        let root = Path::new(".");
        let abs_root = std::path::absolute(root).expect("应可取绝对路径");
        let inside = abs_root.join("sub");
        assert!(is_safe_under_root(root, &inside));
    }
}

/// POSIX 用户 bin 直链用例（M016 纪律：cfg 下沉到用例所在模块，Windows 上整段不参与编译；
/// 早退通道与幂等分支的接线只在真机/WSL 验证，落点语义在这里锁）。
#[cfg(all(test, not(windows)))]
mod user_bin_link_tests {
    use super::*;

    #[test]
    fn 直链_首建幂等重指与真文件不动() {
        let dir = tempfile::tempdir().expect("临时目录");
        let user_bin = dir.path().join("bin");
        let tool_dir = dir.path().join("share/codex/bin");
        std::fs::create_dir_all(&tool_dir).expect("建工具目录");
        let exe = tool_dir.join("codex");
        std::fs::write(&exe, b"new").expect("写新 exe");
        let dst = user_bin.join("codex");

        // 1) 首建：落点目录不存在也自动建
        assert!(
            link_into_user_bin(&user_bin, "codex", &exe).expect("首建应成功"),
            "首建应报告有动作"
        );
        assert_eq!(std::fs::read_link(&dst).expect("应为链接"), exe);
        // 2) 幂等：再跑无动作
        assert!(
            !link_into_user_bin(&user_bin, "codex", &exe).expect("幂等应成功"),
            "已指对不应重复动作"
        );
        // 3) 悬空链接（target 已卸）：先删再建
        std::fs::remove_file(&dst).expect("拆链接");
        std::os::unix::fs::symlink(tool_dir.join("gone"), &dst).expect("造悬空链接");
        assert!(link_into_user_bin(&user_bin, "codex", &exe).expect("悬空重指应成功"));
        assert_eq!(std::fs::read_link(&dst).expect("应为链接"), exe);
        // 4) 有效但指向旧 target（版本目录型布局升级）：重指，防陈旧遮蔽
        let old = dir.path().join("share/codex-old/bin/codex");
        std::fs::create_dir_all(old.parent().expect("父目录")).expect("建旧目录");
        std::fs::write(&old, b"old").expect("写旧 exe");
        std::fs::remove_file(&dst).expect("拆链接");
        std::os::unix::fs::symlink(&old, &dst).expect("造旧链接");
        assert!(link_into_user_bin(&user_bin, "codex", &exe).expect("旧 target 应重指"));
        assert_eq!(std::fs::read_link(&dst).expect("应为链接"), exe);
        // 5) 真文件（用户自装）不动：仅名字相同
        std::fs::remove_file(&dst).expect("拆链接");
        std::fs::write(&dst, b"user-owned").expect("写用户文件");
        assert!(!link_into_user_bin(&user_bin, "codex", &exe).expect("真文件应跳过"));
        assert_eq!(std::fs::read(&dst).expect("读文件"), b"user-owned");
        // 6) exe 本就落该目录（多数 POSIX 绿色工具）：不建自指链接
        let same = user_bin.join("jq");
        std::fs::write(&same, b"jq").expect("写真 exe");
        assert!(!link_into_user_bin(&user_bin, "jq", &same).expect("同路径应跳过"));
        assert!(
            std::fs::symlink_metadata(&same)
                .expect("元数据")
                .file_type()
                .is_file(),
            "同路径落点不应被换成链接"
        );
    }

    #[test]
    fn fnm节点目录_别名优先且无别名取数值段最大() {
        let dir = tempfile::tempdir().expect("临时目录");
        let base = dir.path().join("fnm");
        let mk = |v: &str, with_npm: bool| {
            let bin = base
                .join("node-versions")
                .join(v)
                .join("installation")
                .join("bin");
            std::fs::create_dir_all(&bin).expect("建版本目录");
            if with_npm {
                std::fs::write(bin.join("npm"), b"#!/bin/sh\n").expect("写 npm");
            }
            bin
        };
        mk("v9.9.9", true);
        mk("v24.20.0", true);
        mk("v99.0.0", false); // 版本最大但无 npm：必须跳过
        let picked = fnm_node_bin_in(&base).expect("应解析出 node bin");
        assert!(
            picked.ends_with("node-versions/v24.20.0/installation/bin"),
            "应取含 npm 的数值最大版本（字典序会错取 v9）: {}",
            picked.display()
        );
        // aliases/default 优先，且支持相对符号链接（fnm 实际写的就是相对路径）
        std::fs::create_dir_all(base.join("aliases")).expect("建 aliases");
        std::os::unix::fs::symlink(
            "../node-versions/v9.9.9/installation",
            base.join("aliases").join("default"),
        )
        .expect("建 default 别名");
        let via_alias = fnm_node_bin_in(&base).expect("别名应解析");
        assert!(
            via_alias.ends_with("node-versions/v9.9.9/installation/bin"),
            "默认别名应优先于版本扫描: {}",
            via_alias.display()
        );
        // 别名目录不存在/无 npm 的版本全部排除后应返回 None
        let empty = dir.path().join("empty-fnm");
        std::fs::create_dir_all(
            empty
                .join("node-versions")
                .join("v1.0.0")
                .join("installation")
                .join("bin"),
        )
        .expect("建空版本目录");
        assert!(fnm_node_bin_in(&empty).is_none(), "无 npm 的版本不应被选中");
        // 别名指向无 npm 的残缺版本时，落回版本扫描（而不是就断在那儿）
        let broken_alias = dir.path().join("fnm-broken-alias");
        let with_npm = broken_alias
            .join("node-versions")
            .join("v20.0.0")
            .join("installation")
            .join("bin");
        std::fs::create_dir_all(&with_npm).expect("建可用版本");
        std::fs::write(with_npm.join("npm"), b"#!/bin/sh\n").expect("写 npm");
        let partial = broken_alias
            .join("node-versions")
            .join("v99.0.0")
            .join("installation")
            .join("bin");
        std::fs::create_dir_all(&partial).expect("建残缺版本");
        std::fs::create_dir_all(broken_alias.join("aliases")).expect("建 aliases");
        std::os::unix::fs::symlink(
            "../node-versions/v99.0.0/installation",
            broken_alias.join("aliases").join("default"),
        )
        .expect("建残缺别名");
        let fell_back = fnm_node_bin_in(&broken_alias).expect("应落回版本扫描");
        assert!(
            fell_back.ends_with("node-versions/v20.0.0/installation/bin"),
            "别名无 npm 时应落回含 npm 的版本: {}",
            fell_back.display()
        );
    }

    #[test]
    fn 会话级shim判据_只认fnm_multishells段() {
        // O6：锚定只排除会话级目录；真安装路径（系统/homebrew）不受影响
        assert!(is_session_shim(Path::new(
            "/run/user/1000/fnm_multishells/12345_1789117621809/bin/npm"
        )));
        assert!(is_session_shim(Path::new(
            r"C:\Users\ray\AppData\Local\fnm_multishells\13508_1789117621809\npm.cmd"
        )));
        assert!(!is_session_shim(Path::new(
            "/home/ray/.local/share/fnm/node-versions/v24.20.0/installation/bin/npm"
        )));
        assert!(!is_session_shim(Path::new("/opt/homebrew/bin/npm")));
        assert!(!is_session_shim(Path::new("/usr/local/bin/npm")));
    }
}

#[cfg(test)]
mod ark_managed_tests {
    use super::mark_ark_managed;

    /// REQ-0012：落痕内容 = ark 版本号，幂等同内容零重写，旧版本值覆盖为新值。
    #[test]
    fn 落痕_内容为版本号且幂等() {
        let dir = tempfile::tempdir().expect("临时目录");
        mark_ark_managed(dir.path()).expect("首次落痕");
        let marker = dir.path().join("ark-managed");
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap().trim(),
            env!("CARGO_PKG_VERSION"),
            "内容应为 ark 版本号"
        );
        mark_ark_managed(dir.path()).expect("二次落痕幂等应过");
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap().trim(),
            env!("CARGO_PKG_VERSION")
        );
        // 版本变（跨版本升级）覆盖写新值
        std::fs::write(&marker, "0.0.1\n").unwrap();
        mark_ark_managed(dir.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap().trim(),
            env!("CARGO_PKG_VERSION"),
            "旧版本落痕应被新版本覆盖"
        );
    }
}
