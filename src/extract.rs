//! extract：解压/安装分派，对齐 helpers.ps1 Install-ToolVersion 的 switch（1148-1242 行）。
//! zip/targz 展平顶层单包裹目录；copy/single 单 exe 落目录；gsudo 只取 x64；
//! 7z-extra 用 bootstrap 7zr.exe 解压并 shim 7za.exe 成 7z.exe 后清空目录其余文件；
//! 7zsfx 同步等待退出码；msi 走 msiexec /qn；rmux 跑资产内官方 install.ps1；

use std::fs::{self, File};
use std::io;
use std::path::Path;
#[cfg(any(not(windows), test))]
use std::path::PathBuf;
#[cfg(windows)]
use std::process::Command;

use crate::catalog::Tool;

/// 解压/安装主分派（对齐 pwsh switch ($d.Extract)）。
/// cache_path 为已下载资产，install_dir 为目标目录（msi/official 类由调用方另行处理）。
pub fn extract_asset(
    tool: &str,
    def: &Tool,
    cache_path: &Path,
    install_dir: &Path,
    env_root: &Path,
) -> Result<(), String> {
    let kind = def
        .extract()
        .ok_or_else(|| format!("{tool} 缺少 extract 字段"))?;
    match kind {
        "zip" => {
            extract_zip(cache_path, install_dir)?;
            flatten_single_wrapper(install_dir)?;
            #[cfg(not(windows))]
            set_executable_for_tool(def, install_dir)?;
            Ok(())
        }
        "targz" => {
            extract_targz(cache_path, install_dir)?;
            flatten_single_wrapper(install_dir)?;
            #[cfg(not(windows))]
            set_executable_for_tool(def, install_dir)?;
            Ok(())
        }
        "targz-bin" => {
            #[cfg(windows)]
            {
                let _ = (tool, def, cache_path, install_dir, env_root);
                Err(format!("{tool} targz-bin 提取类型仅在 Linux / macOS 可用"))
            }
            #[cfg(not(windows))]
            {
                extract_targz_single_binary(tool, def, cache_path, install_dir)?;
                Ok(())
            }
        }
        "tarxz-bin" => {
            #[cfg(windows)]
            {
                let _ = (tool, def, cache_path, install_dir, env_root);
                Err(format!("{tool} tarxz-bin 提取类型仅在 Linux / macOS 可用"))
            }
            #[cfg(not(windows))]
            {
                extract_tarxz_single_binary(tool, def, cache_path, install_dir)?;
                Ok(())
            }
        }
        "zip-bin" => {
            #[cfg(windows)]
            {
                let _ = (tool, def, cache_path, install_dir, env_root);
                Err(format!("{tool} zip-bin 提取类型仅在 Linux / macOS 可用"))
            }
            #[cfg(not(windows))]
            {
                extract_zip_bin(tool, def, cache_path, install_dir)?;
                Ok(())
            }
        }
        // zip 全量解压不展平：Windows 目录型布局（zig 的版本目录树）用，与 targz-dir/tarxz-dir 对称
        "zip-dir" => {
            #[cfg(not(windows))]
            {
                let _ = (tool, def, cache_path, install_dir, env_root);
                Err(format!("{tool} zip-dir 提取类型仅在 Windows 可用"))
            }
            #[cfg(windows)]
            {
                extract_zip(cache_path, install_dir)?;
                Ok(())
            }
        }
        // tar.gz / tar.xz 全量解压不展平：目录型运行时（zig 版本目录、go 的 go/ 树）用
        "targz-dir" => {
            #[cfg(windows)]
            {
                let _ = (tool, def, cache_path, install_dir, env_root);
                Err(format!("{tool} targz-dir 提取类型仅在 Linux / macOS 可用"))
            }
            #[cfg(not(windows))]
            {
                extract_targz(cache_path, install_dir)?;
                Ok(())
            }
        }
        "tarxz-dir" => {
            #[cfg(windows)]
            {
                let _ = (tool, def, cache_path, install_dir, env_root);
                Err(format!("{tool} tarxz-dir 提取类型仅在 Linux / macOS 可用"))
            }
            #[cfg(not(windows))]
            {
                extract_tarxz(cache_path, install_dir)?;
                Ok(())
            }
        }
        // copy/single：单 exe 落目录（文件名取 exe 字段的叶子名，对齐 pwsh copy 分支）
        "copy" | "single" => {
            let leaf = def
                .exe()
                .and_then(|e| e.rsplit(['\\', '/']).next())
                .ok_or_else(|| format!("{tool} 缺少 exe 字段，无法确定落地文件名"))?;
            let dst = install_dir.join(leaf);
            fs::copy(cache_path, &dst).map_err(|e| format!("{tool} 复制资产失败: {e}"))?;
            #[cfg(not(windows))]
            set_executable(&dst)?;
            Ok(())
        }
        "gsudo" => {
            #[cfg(not(windows))]
            {
                let _ = (tool, def, cache_path, install_dir, env_root);
                Err(format!("{tool} gsudo 提取类型仅在 Windows 可用"))
            }
            #[cfg(windows)]
            {
                // gsudo.portable.zip 多架构（x64/x86/arm64/net46-AnyCpu）；只取 x64 展平，其余架构删除
                extract_zip(cache_path, install_dir)?;
                let x64 = install_dir.join("x64");
                if !x64.exists() {
                    return Err(format!("{tool} 资产缺少 x64 目录"));
                }
                move_children(&x64, install_dir)?;
                for arch in ["x64", "x86", "arm64", "net46-AnyCpu"] {
                    let p = install_dir.join(arch);
                    if p.exists() {
                        let _ = fs::remove_dir_all(&p);
                    }
                }
                Ok(())
            }
        }
        "7zsfx" => {
            #[cfg(not(windows))]
            {
                let _ = (tool, cache_path, install_dir);
                Err(format!("{tool} 7zsfx 提取类型仅在 Windows 可用"))
            }
            #[cfg(windows)]
            {
                // SFX 自解压 exe 是 GUI 子系统：必须同步等待拿真实退出码（对齐 pwsh Start-Process -Wait）
                let status = Command::new(cache_path)
                    .arg("-y")
                    .arg(format!("-o{}", install_dir.display()))
                    .status()
                    .map_err(|e| format!("{tool} 自解压启动失败: {e}"))?;
                if !status.success() {
                    return Err(format!("{tool} 自解压失败（exit={:?}）", status.code()));
                }
                Ok(())
            }
        }
        "7z-archive" => {
            #[cfg(not(windows))]
            {
                let _ = (cache_path, install_dir);
                Err(format!("{tool} 7z-archive 提取类型仅在 Windows 可用"))
            }
            #[cfg(windows)]
            {
                // 7zXXX-x64.exe 是 7z 归档；Windows 自带 tar(bsdtar) 可直接解包
                run_tar(&[
                    "-xf",
                    &cache_path.to_string_lossy(),
                    "-C",
                    &install_dir.to_string_lossy(),
                ])
                .map_err(|e| format!("{tool} 7z 归档解包失败: {e}"))
            }
        }
        "7z-extra" => {
            #[cfg(not(windows))]
            {
                let _ = (tool, def, cache_path, install_dir, env_root);
                Err(format!("{tool} 7z-extra 提取类型仅在 Windows 可用"))
            }
            #[cfg(windows)]
            {
                extract_7z_extra(tool, def, cache_path, install_dir, env_root)
            }
        }
        "msi" => {
            #[cfg(not(windows))]
            {
                let _ = (tool, cache_path);
                Err(format!("{tool} msi 提取类型仅在 Windows 可用"))
            }
            #[cfg(windows)]
            {
                // per-machine 静默安装需管理员：未提权且 gsudo 可用时经 gsudo msiexec（对齐 vsbuild 提权模式）；
                // MSI 自行注册 PATH；0/3010（需重启）均视为成功
                let mut cmd = if crate::platform::is_elevated() {
                    let mut c = Command::new("msiexec.exe");
                    c.arg("/i");
                    c
                } else {
                    let gsudo = which::which("gsudo").map_err(|_| {
                        format!(
                            "{tool} MSI 安装需管理员：以管理员终端重跑，或先 ark install gsudo 后自动提权"
                        )
                    })?;
                    eprintln!("[INFO] {tool} MSI 需要管理员，经 gsudo msiexec 提权");
                    let mut c = Command::new(gsudo);
                    c.arg("msiexec.exe").arg("/i");
                    c
                };
                let status = cmd
                    .arg(cache_path)
                    .args(["/qn", "/norestart", "DISABLE_TELEMETRY=1"])
                    .status()
                    .map_err(|e| format!("{tool} msiexec 启动失败: {e}"))?;
                match status.code() {
                    Some(0) | Some(3010) => Ok(()),
                    code => Err(format!("{tool} MSI 安装失败（exit={code:?}）")),
                }
            }
        }
        "rmux" => {
            #[cfg(not(windows))]
            {
                let _ = (tool, cache_path);
                Err(format!("{tool} rmux 提取类型仅在 Windows 可用"))
            }
            #[cfg(windows)]
            {
                extract_rmux(tool, cache_path)
            }
        }
        other => Err(format!("未知解压类型: {other}")),
    }
}

/// zip 解压（zip crate，防路径穿越：拒绝越出目标的条目）。
pub fn extract_zip(archive: &Path, dest: &Path) -> Result<(), String> {
    let f =
        File::open(archive).map_err(|e| format!("打开 zip 失败: {}: {e}", archive.display()))?;
    let mut zip = zip::ZipArchive::new(f)
        .map_err(|e| format!("读取 zip 失败: {}: {e}", archive.display()))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("读取 zip 条目失败: {e}"))?;
        let Some(name) = entry.enclosed_name() else {
            continue; // 非法/越界名直接跳过（zip crate 已过滤 .. 穿越）
        };
        let out = dest.join(&name);
        if entry.is_dir() {
            fs::create_dir_all(&out)
                .map_err(|e| format!("创建目录失败: {}: {e}", out.display()))?;
        } else {
            if let Some(p) = out.parent() {
                fs::create_dir_all(p).map_err(|e| format!("创建目录失败: {}: {e}", p.display()))?;
            }
            let mut w =
                File::create(&out).map_err(|e| format!("创建文件失败: {}: {e}", out.display()))?;
            io::copy(&mut entry, &mut w)
                .map_err(|e| format!("写出文件失败: {}: {e}", out.display()))?;
        }
    }
    Ok(())
}

/// tar.gz 解压（flate2 + tar crate）。
pub fn extract_targz(archive: &Path, dest: &Path) -> Result<(), String> {
    let f =
        File::open(archive).map_err(|e| format!("打开 tar.gz 失败: {}: {e}", archive.display()))?;
    let gz = flate2::read::GzDecoder::new(f);
    let mut ar = tar::Archive::new(gz);
    ar.unpack(dest)
        .map_err(|e| format!("tar.gz 解压失败: {}: {e}", archive.display()))
}

/// tar.xz 解压（xz2 + tar crate）。
pub fn extract_tarxz(archive: &Path, dest: &Path) -> Result<(), String> {
    let f =
        File::open(archive).map_err(|e| format!("打开 tar.xz 失败: {}: {e}", archive.display()))?;
    let xz = xz2::read::XzDecoder::new(f);
    let mut ar = tar::Archive::new(xz);
    ar.unpack(dest)
        .map_err(|e| format!("tar.xz 解压失败: {}: {e}", archive.display()))
}

/// 展平顶层单包裹目录（如 age zip 的 age/、python 的 python/ 包裹层）：
/// 只有一个子目录时，把子目录内容上提一层，并递归继续展平。
/// `allow_root_files`：为 true 时允许根目录已有其他文件（Linux ~/.local/bin 多工具共享场景）；
/// 递归调用传 false，避免把 legitimate 子目录也展平。
/// 先把子目录重命名为临时目录再移动，避免子目录名与其中文件同名时产生自覆盖冲突（如 age/age）。
///
/// 平台口径（2026-09-01，gh 2.98.0 布局变更踩坑，M102 M004）：
/// Windows 安装目录均为专属目录（EnvRoot\<dir>，安装前已清空），一律严格判定——
/// 「唯一子目录且零文件」才算包裹层；gh 2.98.0 起 zip 无包裹层、顶层即 bin/ 与 LICENSE，
/// 宽松模式会把业务目录 bin/ 误当包裹层上提。宽松模式仅供 Linux/mac 共享目录场景。
pub fn flatten_single_wrapper(dir: &Path) -> Result<bool, String> {
    #[cfg(windows)]
    {
        flatten_single_wrapper_impl(dir, false)
    }
    #[cfg(not(windows))]
    {
        flatten_single_wrapper_impl(dir, true)
    }
}

fn flatten_single_wrapper_impl(dir: &Path, allow_root_files: bool) -> Result<bool, String> {
    let entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("读取目录失败: {}: {e}", dir.display()))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("读取目录失败: {}: {e}", dir.display()))?;
    let mut dirs = entries.iter().filter(|e| e.path().is_dir());
    let file_count = entries.iter().filter(|e| e.path().is_file()).count();
    let (Some(inner), None) = (dirs.next(), dirs.next()) else {
        return Ok(false);
    };
    if !allow_root_files && file_count > 0 {
        return Ok(false);
    }
    let inner = inner.path();
    let tmp = dir.join(format!(".ark-flatten-{}", std::process::id()));
    fs::rename(&inner, &tmp).map_err(|e| {
        format!(
            "重命名包裹目录失败: {} -> {}: {e}",
            inner.display(),
            tmp.display()
        )
    })?;
    let result = (|| {
        move_children(&tmp, dir)?;
        Ok(true)
    })();
    let _ = fs::remove_dir(&tmp);
    if result.is_ok() {
        // 递归展平多层包裹（如 gh_2.98.0_linux_amd64/bin/gh）；
        // 递归时要求子目录下无其他文件，避免误 flatten legitimate 子目录。
        flatten_single_wrapper_impl(dir, false)?;
    }
    result
}

/// 把 src 下全部条目移动到 dst（对齐 pwsh 的 Move-Item -Force）。
fn move_children(src: &Path, dst: &Path) -> Result<(), String> {
    for entry in fs::read_dir(src).map_err(|e| format!("读取目录失败: {}: {e}", src.display()))?
    {
        let entry = entry.map_err(|e| format!("读取目录失败: {e}"))?;
        let target = dst.join(entry.file_name());
        if target.exists() {
            if target.is_dir() {
                fs::remove_dir_all(&target)
                    .map_err(|e| format!("覆盖清理失败: {}: {e}", target.display()))?;
            } else {
                fs::remove_file(&target)
                    .map_err(|e| format!("覆盖清理失败: {}: {e}", target.display()))?;
            }
        }
        fs::rename(entry.path(), &target).map_err(|e| {
            format!(
                "移动失败: {} -> {}: {e}",
                entry.path().display(),
                target.display()
            )
        })?;
    }
    Ok(())
}

/// 7z-extra：用 bootstrap 7zr.exe 解压 extra.7z，取 x64/7za.exe（或顶层 7za.exe）shim 成 7z.exe，
/// 只保留 7z.exe、清理解压出的其余文件保持目录干净（对齐 pwsh 7z-extra 分支）。
#[cfg(windows)]
fn extract_7z_extra(
    tool: &str,
    def: &Tool,
    cache_path: &Path,
    install_dir: &Path,
    env_root: &Path,
) -> Result<(), String> {
    let bootstrap = def
        .bootstrap_asset
        .as_deref()
        .ok_or_else(|| format!("{tool} 缺少 bootstrap_asset 字段"))?;
    let boot = env_root.join("cache").join(bootstrap);
    if !boot.exists() {
        return Err(format!("{tool} 缺少 BootstrapAsset: {bootstrap}"));
    }
    let status = Command::new(&boot)
        .arg("x")
        .arg(cache_path)
        .arg(format!("-o{}", install_dir.display()))
        .arg("-y")
        .status()
        .map_err(|e| format!("{tool} 7zr 启动失败: {e}"))?;
    if !status.success() {
        return Err(format!(
            "{tool} extra.7z 解压失败（exit={:?}）",
            status.code()
        ));
    }
    let mut src7za = install_dir.join("x64").join("7za.exe");
    if !src7za.exists() {
        src7za = install_dir.join("7za.exe");
    }
    if !src7za.exists() {
        return Err(format!("{tool} extra.7z 内未找到 7za.exe"));
    }
    fs::copy(&src7za, install_dir.join("7z.exe"))
        .map_err(|e| format!("{tool} shim 7z.exe 失败: {e}"))?;
    // 只保留 7z.exe（7za 单文件 standalone），删除其余文件与空目录
    remove_all_except(install_dir, "7z.exe")?;
    Ok(())
}

/// Linux / macOS：设置文件可执行权限（copy/targz/zip 解压后二进制默认可能无执行位）。
#[cfg(not(windows))]
fn set_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(path)
        .map_err(|e| format!("读取权限失败: {}: {e}", path.display()))?
        .permissions();
    perm.set_mode(perm.mode() | 0o755);
    std::fs::set_permissions(path, perm)
        .map_err(|e| format!("设置可执行权限失败: {}: {e}", path.display()))
}

/// Linux / macOS：从 tar.gz 中提取单个二进制（按 exe 字段叶子名查找，无视包裹层深度）。
#[cfg(not(windows))]
fn extract_targz_single_binary(
    tool: &str,
    def: &Tool,
    cache_path: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    extract_tar_single_binary(tool, def, cache_path, install_dir, "tar.gz", extract_targz)
}

/// Linux / macOS：从 tar.xz 中提取单个二进制（按 exe 字段叶子名查找，无视包裹层深度）。
#[cfg(not(windows))]
fn extract_tarxz_single_binary(
    tool: &str,
    def: &Tool,
    cache_path: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    extract_tar_single_binary(tool, def, cache_path, install_dir, "tar.xz", extract_tarxz)
}

/// Linux / macOS：从 tar 归档中提取单个二进制的通用实现（exe 主二进制 + extra_bins 补充成员）。
#[cfg(not(windows))]
fn extract_tar_single_binary(
    tool: &str,
    def: &Tool,
    cache_path: &Path,
    install_dir: &Path,
    kind: &str,
    extract: fn(&Path, &Path) -> Result<(), String>,
) -> Result<(), String> {
    let exe_leaf = def
        .exe()
        .and_then(|e| e.rsplit(['\\', '/']).next())
        .ok_or_else(|| format!("{tool} 缺少 exe 字段，无法确定提取目标"))?;
    let mut leaves = vec![exe_leaf.to_string()];
    leaves.extend(def.extra_bins().iter().map(|s| s.to_string()));
    let tmp = std::env::temp_dir().join(format!("ark-{kind}-bin-{}-{}", tool, std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| format!("创建临时目录失败: {}: {e}", tmp.display()))?;
    let result = (|| {
        extract(cache_path, &tmp)?;
        fs::create_dir_all(install_dir)
            .map_err(|e| format!("创建安装目录失败: {}: {e}", install_dir.display()))?;
        for leaf in &leaves {
            let src = walkdir_find_file(&tmp, leaf)
                .ok_or_else(|| format!("{tool} 在 {kind} 中未找到可执行文件: {leaf}"))?;
            let dst = install_dir.join(leaf);
            fs::copy(&src, &dst).map_err(|e| {
                format!(
                    "{tool} 复制二进制失败: {} -> {}: {e}",
                    src.display(),
                    dst.display()
                )
            })?;
            set_executable(&dst)?;
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&tmp);
    result
}

/// Linux / macOS：从 zip 中按叶子名提取二进制成员（exe 主二进制 + extra_bins），无视包裹层深度。
#[cfg(not(windows))]
fn extract_zip_bin(
    tool: &str,
    def: &Tool,
    cache_path: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    let exe_leaf = def
        .exe()
        .and_then(|e| e.rsplit(['\\', '/']).next())
        .ok_or_else(|| format!("{tool} 缺少 exe 字段，无法确定提取目标"))?;
    let mut leaves = vec![exe_leaf.to_string()];
    leaves.extend(def.extra_bins().iter().map(|s| s.to_string()));
    let f = File::open(cache_path)
        .map_err(|e| format!("{tool} 打开 zip 失败: {}: {e}", cache_path.display()))?;
    let mut zip = zip::ZipArchive::new(f)
        .map_err(|e| format!("{tool} 读取 zip 失败: {}: {e}", cache_path.display()))?;
    fs::create_dir_all(install_dir)
        .map_err(|e| format!("创建安装目录失败: {}: {e}", install_dir.display()))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("{tool} 读取 zip 条目失败: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let Some(leaf) = name.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !leaves.iter().any(|l| l == leaf) {
            continue;
        }
        let dst = install_dir.join(leaf);
        let mut w = File::create(&dst)
            .map_err(|e| format!("{tool} 创建文件失败: {}: {e}", dst.display()))?;
        io::copy(&mut entry, &mut w)
            .map_err(|e| format!("{tool} 写出文件失败: {}: {e}", dst.display()))?;
        set_executable(&dst)?;
    }
    // 主二进制必须命中（extras 缺失同样报错：数据契约明示的成员不允许静默丢）
    for leaf in &leaves {
        let dst = install_dir.join(leaf);
        if !dst.exists() {
            return Err(format!("{tool} 在 zip 中未找到可执行文件: {leaf}"));
        }
    }
    Ok(())
}

/// 在 dir 下递归查找名为 name 的普通文件。
#[cfg(not(windows))]
fn walkdir_find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = walkdir_find_file(&path, name) {
                return Some(found);
            }
        }
    }
    None
}

/// Linux / macOS：根据 exe 字段叶子名与 extra_bins 在安装目录中设置可执行权限。
#[cfg(not(windows))]
fn set_executable_for_tool(def: &Tool, install_dir: &Path) -> Result<(), String> {
    let Some(exe_rel) = def.exe() else {
        return Ok(());
    };
    let leaf = exe_rel.rsplit(['\\', '/']).next().unwrap_or(exe_rel);
    let target = install_dir.join(leaf);
    if target.exists() {
        set_executable(&target)?;
    }
    for extra in def.extra_bins() {
        let t = install_dir.join(extra);
        if t.exists() {
            set_executable(&t)?;
        }
    }
    Ok(())
}

/// 递归删除 dir 下除指定文件名外的全部文件与目录（7z-extra 清场用）。
#[cfg(windows)]
fn remove_all_except(dir: &Path, keep_name: &str) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("读取目录失败: {}: {e}", dir.display()))?
    {
        let entry = entry.map_err(|e| format!("读取目录失败: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            remove_all_except(&path, keep_name)?;
            // 清完内容后目录若已空则删除（7z.exe 只留在顶层，子目录必空）
            let _ = fs::remove_dir(&path);
        } else if entry.file_name() != keep_name {
            fs::remove_file(&path).map_err(|e| format!("清理文件失败: {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// rmux 官方布局：zip 内附官方 install.ps1，跑它装到 LOCALAPPDATA 官方目录并自校验。
#[cfg(windows)]
fn extract_rmux(tool: &str, cache_path: &Path) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("rmux-install-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| format!("创建临时目录失败: {}: {e}", tmp.display()))?;
    let result = (|| {
        extract_zip(cache_path, &tmp)?;
        // 包根：第一个含 install.ps1 的子目录
        let pkg_root = fs::read_dir(&tmp)
            .map_err(|e| format!("读取临时目录失败: {e}"))?
            .filter_map(|e| e.ok().map(|x| x.path()))
            .find(|p| p.is_dir() && p.join("install.ps1").exists())
            .ok_or_else(|| "rmux 资产内未找到 install.ps1".to_string())?;
        let status = Command::new("pwsh")
            .args(["-NoProfile", "-File"])
            .arg(pkg_root.join("install.ps1"))
            .status()
            .map_err(|e| format!("rmux install.ps1 启动失败（pwsh 不可用？）: {e}"))?;
        if !status.success() {
            return Err(format!(
                "{tool} 官方 install.ps1 失败（exit={:?}）",
                status.code()
            ));
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&tmp);
    result
}

/// 调系统 tar.exe（bsdtar）。
#[cfg(windows)]
fn run_tar(args: &[&str]) -> Result<(), String> {
    let status = Command::new("tar")
        .args(args)
        .status()
        .map_err(|e| format!("tar.exe 不可用: {e}"))?;
    if !status.success() {
        return Err(format!("tar 失败（exit={:?}）", status.code()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一个目录树：dir/inner/{a.txt, sub/b.txt}，用于展平测试。
    fn make_wrapper(dir: &Path, inner: &str) -> Result<PathBuf, String> {
        let inner_dir = dir.join(inner);
        fs::create_dir_all(inner_dir.join("sub")).map_err(|e| e.to_string())?;
        fs::write(inner_dir.join("a.txt"), b"a").map_err(|e| e.to_string())?;
        fs::write(inner_dir.join("sub").join("b.txt"), b"b").map_err(|e| e.to_string())?;
        Ok(inner_dir)
    }

    #[test]
    fn 展平_顶层单包裹目录_内容上提() -> Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let root = tmp.path().join("install");
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        make_wrapper(&root, "age")?;

        assert!(flatten_single_wrapper(&root)?, "单包裹目录应展平");
        assert!(root.join("a.txt").exists(), "文件应上提到顶层");
        assert!(root.join("sub").join("b.txt").exists(), "子目录应整体上提");
        assert!(!root.join("age").exists(), "包裹层应被删除");
        Ok(())
    }

    #[test]
    fn 展平_多目录不动_顶层文件的平台口径() -> Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;

        // 场景一：顶层有文件 + 一个目录。Linux/mac 共享目录（~/.local/bin）场景应展平；
        // Windows 专属目录场景不得展平（gh 2.98.0 布局：顶层即 bin/ 与 LICENSE，bin/ 是业务目录）
        let root1 = tmp.path().join("case1");
        fs::create_dir_all(root1.join("sub")).map_err(|e| e.to_string())?;
        fs::write(root1.join("sub").join("a.txt"), b"a").map_err(|e| e.to_string())?;
        fs::write(root1.join("top.txt"), b"t").map_err(|e| e.to_string())?;
        #[cfg(not(windows))]
        {
            assert!(
                flatten_single_wrapper(&root1)?,
                "顶层有文件也应展平单包裹目录"
            );
            assert!(root1.join("a.txt").exists(), "包裹层内容应上提");
            assert!(root1.join("top.txt").exists(), "原有顶层文件应保留");
            assert!(!root1.join("sub").exists(), "包裹层应被删除");
        }
        #[cfg(windows)]
        {
            assert!(
                !flatten_single_wrapper(&root1)?,
                "Windows 专属目录：顶层有文件时不展平（防业务目录误判）"
            );
            assert!(root1.join("sub").join("a.txt").exists(), "布局应原样保留");
        }

        // 场景二：两个顶层目录
        let root2 = tmp.path().join("case2");
        fs::create_dir_all(root2.join("a")).map_err(|e| e.to_string())?;
        fs::create_dir_all(root2.join("b")).map_err(|e| e.to_string())?;
        assert!(!flatten_single_wrapper(&root2)?, "多目录不应展平");
        Ok(())
    }

    /// gh 2.98.0 起 windows zip 无包裹层（顶层即 bin/gh.exe 与 LICENSE）：
    /// Windows 不得把 bin/ 误当包裹层上提（M102 M004 回归锚）。
    #[test]
    #[cfg(windows)]
    fn 展平_gh新布局_业务bin目录不上提() -> Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let root = tmp.path().join("gh");
        fs::create_dir_all(root.join("bin")).map_err(|e| e.to_string())?;
        fs::write(root.join("bin").join("gh.exe"), b"stub").map_err(|e| e.to_string())?;
        fs::write(root.join("LICENSE"), b"lic").map_err(|e| e.to_string())?;
        assert!(!flatten_single_wrapper(&root)?, "gh 新布局不应展平");
        assert!(
            root.join("bin").join("gh.exe").exists(),
            "bin/gh.exe 应原样保留"
        );
        Ok(())
    }

    #[test]
    fn zip解压_含目录与嵌套文件() -> Result<(), String> {
        // 盲删 ensure_bunx_shim 时被连带删掉后补回（extract_zip 无别的用例覆盖）
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let zip_path = tmp.path().join("demo.zip");
        {
            let f = File::create(&zip_path).map_err(|e| e.to_string())?;
            let mut zw = zip::ZipWriter::new(f);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("demo/a.txt", opts)
                .map_err(|e| e.to_string())?;
            io::Write::write_all(&mut zw, b"hello").map_err(|e| e.to_string())?;
            zw.add_directory("demo/sub/", opts)
                .map_err(|e| e.to_string())?;
            zw.start_file("demo/sub/b.txt", opts)
                .map_err(|e| e.to_string())?;
            io::Write::write_all(&mut zw, b"world").map_err(|e| e.to_string())?;
            zw.finish().map_err(|e| e.to_string())?;
        }
        let dest = tmp.path().join("out");
        fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
        extract_zip(&zip_path, &dest)?;
        assert_eq!(
            fs::read(dest.join("demo").join("a.txt")).map_err(|e| e.to_string())?,
            b"hello"
        );
        assert_eq!(
            fs::read(dest.join("demo").join("sub").join("b.txt")).map_err(|e| e.to_string())?,
            b"world"
        );
        // 展平后应只剩 a.txt 与 sub/
        assert!(flatten_single_wrapper(&dest)?);
        assert!(dest.join("a.txt").exists());
        Ok(())
    }
}
