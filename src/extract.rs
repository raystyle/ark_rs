//! extract：解压/安装分派，对齐 helpers.ps1 Install-ToolVersion 的 switch（1148-1242 行）。
//! zip/targz 展平顶层单包裹目录；copy/single 单 exe 落目录（资产实为归档时剥层到二进制，
//! yq 病灶 2026-10-10）；gsudo 只取 x64；
//! 7z-extra 用 bootstrap 7zr.exe 解压并 shim 7za.exe 成 7z.exe 后清空目录其余文件；
//! 7zsfx 同步等待退出码；msi 走 msiexec /qn；rmux 跑资产内官方 install.ps1；

use std::fs::{self, File};
use std::io;
use std::io::Read;
use std::path::Path;
#[cfg(any(not(windows), test))]
use std::path::PathBuf;
#[cfg(windows)]
use std::process::Command;

use crate::catalog::Tool;

/// 解压/安装主分派（对齐 pwsh switch ($d.Extract)）。
/// cache_path 为已下载资产，install_dir 为目标目录（msi/official 类由调用方另行处理）。
///
/// # Errors
/// 返回 Err（人读原因串）当：{tool} targz-bin 提取类型仅在 Linux / macOS 可用 等（完整失败面见函数体错误构造）。
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
        // copy/single：单 exe 落目录（文件名取 exe 字段的叶子名，对齐 pwsh copy 分支）；
        // 资产实为归档时（yq 形 gzip 套 tar / zip）剥层到二进制再落，不拿归档本体冒充可执行
        "copy" | "single" => {
            let leaf = def
                .exe()
                .and_then(|e| e.rsplit(['\\', '/']).next())
                .ok_or_else(|| format!("{tool} 缺少 exe 字段，无法确定落地文件名"))?;
            let dst = install_dir.join(leaf);
            copy_asset_as_binary(tool, leaf, cache_path, &dst)?;
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
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
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
///
/// # Errors
/// 返回 Err（人读原因串）当：{tool} 缺少 BootstrapAsset: {bootstrap} 等（完整失败面见函数体错误构造）。
pub fn extract_targz(archive: &Path, dest: &Path) -> Result<(), String> {
    let f =
        File::open(archive).map_err(|e| format!("打开 tar.gz 失败: {}: {e}", archive.display()))?;
    let gz = flate2::read::GzDecoder::new(f);
    let mut ar = tar::Archive::new(gz);
    ar.unpack(dest)
        .map_err(|e| format!("tar.gz 解压失败: {}: {e}", archive.display()))
}

/// tar.xz 解压（xz2 + tar crate）。
///
/// # Errors
/// 返回 Err（人读原因串）当：{tool} 缺少 BootstrapAsset: {bootstrap} 等（完整失败面见函数体错误构造）。
pub fn extract_tarxz(archive: &Path, dest: &Path) -> Result<(), String> {
    let f =
        File::open(archive).map_err(|e| format!("打开 tar.xz 失败: {}: {e}", archive.display()))?;
    let xz = xz2::read::XzDecoder::new(f);
    let mut ar = tar::Archive::new(xz);
    ar.unpack(dest)
        .map_err(|e| format!("tar.xz 解压失败: {}: {e}", archive.display()))
}

/// 裸 tar 解压（gzip/xz 剥压缩层后的内层归档，tar crate）。
///
/// # Errors
/// 返回 Err（人读原因串）当：打开/解压失败（见错误串）等（完整失败面见函数体错误构造）。
pub fn extract_tar(archive: &Path, dest: &Path) -> Result<(), String> {
    let f =
        File::open(archive).map_err(|e| format!("打开 tar 失败: {}: {e}", archive.display()))?;
    let mut ar = tar::Archive::new(f);
    ar.unpack(dest)
        .map_err(|e| format!("tar 解压失败: {}: {e}", archive.display()))
}

/// copy 型资产的落盘形态（双机升级轮 yq 病灶，2026-10-10 对齐单件一）：
/// copy 语义是「资产本体即单个可执行文件」，但上游会把单二进制打成归档发放
/// （yq v4.54.1 的 yq_linux_amd64.tar.gz 是 gzip 套 tar 内含 ./yq_linux_amd64、
/// yq_windows_amd64.zip 内含 yq.exe）——原样落盘会把归档本体冒充可执行，
/// 装后验版本报「未找到可执行文件或无法读取版本」。落盘前按魔数嗅探剥层。
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum AssetKind {
    /// gzip 压缩层（内层再嗅探：tar 归档或裸二进制）
    Gzip,
    /// xz 压缩层（同上）
    Xz,
    /// zip 归档
    Zip,
    /// 裸 tar 归档
    Tar,
    /// 非归档（copy 既有语义，原样复制）
    Plain,
}

/// 按魔数嗅探资产形态：gzip `1f 8b`、xz `fd 37 7a 58 5a 00`、zip `PK\x03\x04`、
/// tar 在 257 偏移的 `ustar`（GNU 与 POSIX 两形 tar 都含此前缀）。
fn sniff_asset_kind(path: &Path) -> Result<AssetKind, String> {
    let mut f = File::open(path).map_err(|e| format!("打开资产失败: {}: {e}", path.display()))?;
    let mut head = Vec::with_capacity(262);
    f.by_ref()
        .take(262)
        .read_to_end(&mut head)
        .map_err(|e| format!("读取资产失败: {}: {e}", path.display()))?;
    Ok(if head.starts_with(&[0x1f, 0x8b]) {
        AssetKind::Gzip
    } else if head.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
        AssetKind::Xz
    } else if head.starts_with(b"PK\x03\x04") {
        AssetKind::Zip
    } else if head.len() > 261 && &head[257..262] == b"ustar" {
        AssetKind::Tar
    } else {
        AssetKind::Plain
    })
}

/// copy 落盘：非归档原样复制（既有语义零变化）；归档把解包链补完到二进制层——
/// 压缩层（gzip/xz）先剥，内层是 tar 则展开成员，再按 exe 叶子名挑真身
/// （`pick_payload_member`：精确名或平台变体名）落盘。
fn copy_asset_as_binary(
    tool: &str,
    leaf: &str,
    cache_path: &Path,
    dst: &Path,
) -> Result<(), String> {
    let kind = sniff_asset_kind(cache_path)?;
    if kind == AssetKind::Plain {
        fs::copy(cache_path, dst).map_err(|e| format!("{tool} 复制资产失败: {e}"))?;
        return Ok(());
    }
    // 目录名带纳秒时戳：同进程并行 install（集成测试多工具并行）下仅 pid 会互踩
    let uniq = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp =
        std::env::temp_dir().join(format!("ark-copy-bin-{tool}-{}-{uniq}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let result = (|| {
        fs::create_dir_all(&tmp).map_err(|e| format!("{tool} 创建临时目录失败: {e}"))?;
        let members = tmp.join("members");
        fs::create_dir_all(&members).map_err(|e| format!("{tool} 创建临时目录失败: {e}"))?;
        match kind {
            AssetKind::Zip => extract_zip(cache_path, &members)?,
            AssetKind::Tar => extract_tar(cache_path, &members)?,
            AssetKind::Gzip | AssetKind::Xz => {
                let dec = tmp.join("decoded");
                decode_compressed_to_file(tool, kind, cache_path, &dec)?;
                if sniff_asset_kind(&dec)? == AssetKind::Tar {
                    extract_tar(&dec, &members)?;
                } else {
                    // 压缩内容即裸二进制（gzip 单层发放），无需挑成员
                    fs::copy(&dec, dst).map_err(|e| format!("{tool} 复制资产失败: {e}"))?;
                    return Ok(());
                }
            }
            AssetKind::Plain => unreachable!("Plain 已在上游原样复制"),
        }
        let src = pick_payload_member(&members, leaf).ok_or_else(|| {
            let mut files = Vec::new();
            collect_files(&members, &mut files);
            let names: Vec<String> = files.iter().map(|p| p.display().to_string()).collect();
            format!(
                "{tool} 资产是归档但未找到可执行成员: {leaf}（或其平台变体）; members={names:?}"
            )
        })?;
        fs::copy(&src, dst).map_err(|e| format!("{tool} 复制资产失败: {e}"))?;
        Ok(())
    })();
    let _ = fs::remove_dir_all(&tmp);
    result
}

/// 剥压缩层到单文件（gzip 走 flate2、xz 走 xz2，R005 组合现成库）。
fn decode_compressed_to_file(
    tool: &str,
    kind: AssetKind,
    archive: &Path,
    dest: &Path,
) -> Result<(), String> {
    let f = File::open(archive).map_err(|e| format!("{tool} 打开资产失败: {e}"))?;
    let mut reader: Box<dyn io::Read> = match kind {
        AssetKind::Gzip => Box::new(flate2::read::GzDecoder::new(f)),
        AssetKind::Xz => Box::new(xz2::read::XzDecoder::new(f)),
        _ => return Err(format!("{tool} 非 gzip/xz 不走压缩层解码")),
    };
    let mut w = File::create(dest).map_err(|e| format!("{tool} 创建解压文件失败: {e}"))?;
    io::copy(&mut reader, &mut w).map_err(|e| format!("{tool} 解压失败: {e}"))?;
    Ok(())
}

/// 从归档成员目录挑真身二进制：先按 exe 叶子名精确匹配（任意包裹层深度）；
/// 未命中再按平台变体名——成员 stem 等于 exe stem（yq.exe 对叶名 yq）或以
/// 「exe stem + `_`/`-`」开头（yq → yq_linux_amd64），多候选取最大者
/// （确定性口径：变体通常唯一，多命中时大文件优先近似主二进制）。
fn pick_payload_member(dir: &Path, leaf: &str) -> Option<PathBuf> {
    if let Some(p) = walkdir_find_file(dir, leaf) {
        return Some(p);
    }
    let stem = leaf.strip_suffix(".exe").unwrap_or(leaf);
    let mut files = Vec::new();
    collect_files(dir, &mut files);
    files
        .into_iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| {
                    let s = n.strip_suffix(".exe").unwrap_or(n);
                    s == stem
                        || s.starts_with(&format!("{stem}_"))
                        || s.starts_with(&format!("{stem}-"))
                })
                .unwrap_or(false)
        })
        .max_by_key(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
}

/// 递归收集 dir 下全部普通文件路径。
fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_files(&p, out);
        } else {
            out.push(p);
        }
    }
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
///
/// # Errors
/// 返回 Err（人读原因串）当：{tool} 缺少 BootstrapAsset: {bootstrap} 等（完整失败面见函数体错误构造）。
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
                // 平台变体名回退（yq 形：归档成员是 yq_linux_amd64 而 exe 叶名是 yq）
                .or_else(|| pick_payload_member(&tmp, leaf))
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

    /// 造 yq 形资产夹具：gzip( tar( ./<member> + 干扰成员 ) )。
    /// member 用平台变体名（./yq_linux_amd64）对齐上游 v4.54.1 真实布局。
    fn build_targz_with_member(
        dir: &Path,
        asset_name: &str,
        member: &str,
        payload: &[u8],
    ) -> Result<PathBuf, String> {
        let tar_path = dir.join("inner.tar");
        {
            let f = File::create(&tar_path).map_err(|e| e.to_string())?;
            let mut tb = tar::Builder::new(f);
            let mut h = tar::Header::new_gnu();
            h.set_size(payload.len() as u64);
            h.set_mode(0o755);
            h.set_cksum();
            tb.append_data(&mut h, member, payload)
                .map_err(|e| e.to_string())?;
            let mut h2 = tar::Header::new_gnu();
            h2.set_size(9);
            h2.set_cksum();
            tb.append_data(&mut h2, "yq.1", b"man page".as_slice())
                .map_err(|e| e.to_string())?;
            tb.finish().map_err(|e| e.to_string())?;
        }
        let gz_path = dir.join(asset_name);
        {
            let f = File::create(&gz_path).map_err(|e| e.to_string())?;
            let mut enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
            io::Write::write_all(&mut enc, &fs::read(&tar_path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            enc.finish().map_err(|e| e.to_string())?;
        }
        Ok(gz_path)
    }

    /// 造 yq 形 copy 工具夹具（三平台 extract/exe 同设 copy/yq，测试平台无关）。
    fn copy_tool_def() -> Tool {
        Tool {
            extract: Some("copy".into()),
            linux_extract: Some("copy".into()),
            mac_extract: Some("copy".into()),
            exe: Some("yq\\yq.exe".into()),
            linux_exe: Some("yq".into()),
            mac_exe: Some("yq".into()),
            ..Tool::default()
        }
    }

    /// yq 病灶回归（2026-10-10 双机升级轮对齐单件一）：copy 型资产是 gzip 套 tar
    /// 双层归档（内含 ./yq_linux_amd64 平台变体名）时，解包链须完整到二进制层——
    /// 装出物是真身字节（ELF 头）而非 tar/gzip 归档本体，且带执行位。
    #[test]
    fn copy解包_gzip套tar_抽真身elf非归档本体() -> Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let mut payload = b"\x7fELF\x02\x01\x01\x00".to_vec();
        payload.extend_from_slice(b"fake-yq-binary-payload");
        let asset = build_targz_with_member(
            tmp.path(),
            "yq_linux_amd64.tar.gz",
            "./yq_linux_amd64",
            &payload,
        )?;

        let install_dir = tmp.path().join("bin");
        fs::create_dir_all(&install_dir).map_err(|e| e.to_string())?;
        extract_asset("yq", &copy_tool_def(), &asset, &install_dir, tmp.path())?;

        let dst = install_dir.join("yq");
        let got = fs::read(&dst).map_err(|e| e.to_string())?;
        assert_eq!(got, payload, "装出物应是真身二进制而非 tar/gzip 本体");
        assert!(got.starts_with(b"\x7fELF"), "装出物应是可执行 ELF");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&dst)
                .map_err(|e| e.to_string())?
                .permissions()
                .mode();
            assert!(mode & 0o111 != 0, "装出物应带执行位");
        }
        Ok(())
    }

    /// win yq 形：copy 资产是 zip 归档（内含 yq.exe 与干扰文件），按 exe 叶名（或
    /// stem 变体）挑真身；落地文件名仍取本平台 exe 叶子名。
    #[test]
    fn copy解包_zip归档_按叶名挑真身() -> Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let payload = b"MZ\x90\x00fake-yq-exe-payload".to_vec();
        let zip_path = tmp.path().join("yq_windows_amd64.zip");
        {
            let f = File::create(&zip_path).map_err(|e| e.to_string())?;
            let mut zw = zip::ZipWriter::new(f);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("yq.exe", opts).map_err(|e| e.to_string())?;
            io::Write::write_all(&mut zw, &payload).map_err(|e| e.to_string())?;
            zw.start_file("README", opts).map_err(|e| e.to_string())?;
            io::Write::write_all(&mut zw, b"readme").map_err(|e| e.to_string())?;
            zw.finish().map_err(|e| e.to_string())?;
        }

        let install_dir = tmp.path().join("yq");
        fs::create_dir_all(&install_dir).map_err(|e| e.to_string())?;
        extract_asset("yq", &copy_tool_def(), &zip_path, &install_dir, tmp.path())?;

        // 落地名取本平台 exe 叶子名：Windows 叶 yq.exe（精确命中），POSIX 叶 yq（stem 变体命中）
        let dst = install_dir.join(if cfg!(windows) { "yq.exe" } else { "yq" });
        let got = fs::read(&dst).map_err(|e| e.to_string())?;
        assert_eq!(got, payload, "zip 内应按叶名挑出 yq.exe 真身");
        assert!(got.starts_with(b"MZ"), "win 形装出物应是 PE 可执行");
        Ok(())
    }

    /// gzip 单层（内容即裸二进制、无 tar 内层）也剥层落盘。
    #[test]
    fn copy解包_gzip裸二进制_剥压缩层() -> Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let payload = b"\x7fELF\x02plain-gz-binary".to_vec();
        let gz_path = tmp.path().join("tool.gz");
        {
            let f = File::create(&gz_path).map_err(|e| e.to_string())?;
            let mut enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
            io::Write::write_all(&mut enc, &payload).map_err(|e| e.to_string())?;
            enc.finish().map_err(|e| e.to_string())?;
        }

        let install_dir = tmp.path().join("bin");
        fs::create_dir_all(&install_dir).map_err(|e| e.to_string())?;
        extract_asset("yq", &copy_tool_def(), &gz_path, &install_dir, tmp.path())?;
        let got = fs::read(install_dir.join("yq")).map_err(|e| e.to_string())?;
        assert_eq!(got, payload, "gzip 单层应剥出裸二进制本体");
        Ok(())
    }

    /// 非归档资产保持既有语义：原样复制（既有 copy 工具零行为变化）。
    #[test]
    fn copy解包_非归档资产_原样复制() -> Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let payload = b"\x7fELF\x02raw-single-binary".to_vec();
        let asset = tmp.path().join("tool.bin");
        fs::write(&asset, &payload).map_err(|e| e.to_string())?;

        let install_dir = tmp.path().join("bin");
        fs::create_dir_all(&install_dir).map_err(|e| e.to_string())?;
        extract_asset("yq", &copy_tool_def(), &asset, &install_dir, tmp.path())?;
        let got = fs::read(install_dir.join("yq")).map_err(|e| e.to_string())?;
        assert_eq!(got, payload, "非归档资产应原样复制");
        Ok(())
    }

    /// 归档内无匹配成员（也无平台变体）如实报错，不落半截产物。
    #[test]
    fn dies_copy解包_归档无匹配成员_如实报错() -> Result<(), String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let asset = build_targz_with_member(
            tmp.path(),
            "docs.tar.gz",
            "./docs/readme.txt",
            b"no binary here",
        )?;

        let install_dir = tmp.path().join("bin");
        fs::create_dir_all(&install_dir).map_err(|e| e.to_string())?;
        let err = extract_asset("yq", &copy_tool_def(), &asset, &install_dir, tmp.path())
            .expect_err("归档无匹配成员应报错");
        assert!(err.contains("未找到可执行成员"), "错误应说明缺成员: {err}");
        assert!(!install_dir.join("yq").exists(), "不应落半截产物");
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
