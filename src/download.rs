//! download：资产下载与缓存复用，语义对齐 helpers.ps1 的 Save-ReleaseAsset。
//! 缓存目录 <EnvRoot>\cache\<asset>：
//! - 命中且 sha256 一致则复用；不符删除重下；无 sha 基准且文件非空则复用。
//! - 下载先写 `<asset>.part` 再 rename，失败不留半截 dest。
//! - 下载走 ureq（3 次指数退避），失败回退系统 curl.exe（--retry 5）。
//! - sha256 计算用 sha2，比较统一大写。

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use sha2::{Digest, Sha256};

const MAX_ATTEMPTS: u32 = 3;

/// 缓存路径：<EnvRoot>\cache\<asset>。
pub fn cache_path(env_root: &Path, asset_name: &str) -> PathBuf {
    env_root.join("cache").join(asset_name)
}

/// 计算文件 sha256，返回大写 hex（比较基准统一大写）。
///
/// # Errors
/// 返回 Err（人读原因串）当：sha256 校验失败: {}\n期望 {}\n实际 {} 等（完整失败面见函数体错误构造）。
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let f = File::open(path).map_err(|e| format!("打开文件失败: {}: {e}", path.display()))?;
    let mut reader = BufReader::new(f);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("读取文件失败: {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:X}", hasher.finalize()))
}

/// 下载资产到缓存并复用：对齐 Save-ReleaseAsset 的缓存三分支（cache_reuse 提取共用）。
/// expected_sha256 为 None 时无校验基准，已有缓存直接复用；force 跳过复用直接重下。
///
/// # Errors
/// 返回 Err（人读原因串）当：sha256 校验失败: {}\n期望 {}\n实际 {} 等（完整失败面见函数体错误构造）。
pub fn download_asset(
    env_root: &Path,
    asset_name: &str,
    url: &str,
    expected_sha256: Option<&str>,
    force: bool,
) -> Result<PathBuf, String> {
    let dest = cache_path(env_root, asset_name);
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("创建缓存目录失败: {}: {e}", dir.display()))?;
    }
    if let Some(hit) = cache_reuse(&dest, expected_sha256, force)? {
        return Ok(hit);
    }

    download_url(url, &dest)?;
    if let Some(exp) = expected_sha256 {
        let actual = sha256_file(&dest)?;
        if !actual.eq_ignore_ascii_case(exp) {
            return Err(format!(
                "sha256 校验失败: {}\n期望 {}\n实际 {}",
                dest.display(),
                exp.to_uppercase(),
                actual
            ));
        }
    }
    eprintln!("[OK] 已下载: {}", dest.display());
    Ok(dest)
}

/// 强制重下（删旧再下）：校验清单类资产每次取新，不复用缓存。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn download_fresh(env_root: &Path, asset_name: &str, url: &str) -> Result<PathBuf, String> {
    let dest = cache_path(env_root, asset_name);
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("创建缓存目录失败: {}: {e}", dir.display()))?;
    }
    if dest.exists() {
        fs::remove_file(&dest).map_err(|e| format!("删除旧文件失败: {}: {e}", dest.display()))?;
    }
    download_url(url, &dest)?;
    Ok(dest)
}

// ===================== D08/D44 镜像优先下载链（D08 兜底 2026-09-07；D44 反转为主通道 2026-09-13，官方成兜底）=====================

/// 自建分发镜像基址（种子终态 69/69，ohmycloud#2）。
pub const MIRROR_BASE: &str = "https://env.ohmygh.com";

/// 镜像段 URL：`{MIRROR_BASE}/{tool}/{version}/{asset}`。
/// evergreen 引导器走 latest 段（`rust/latest/rustup-init.exe` 同构，version 传 "latest"）。
pub fn mirror_url(tool: &str, version: &str, asset: &str) -> String {
    format!("{MIRROR_BASE}/{tool}/{version}/{asset}")
}

/// 镜像 latest 段边车 URL：`{MIRROR_BASE}/{tool}/latest/{asset}.sha256`。
pub fn mirror_sidecar_url(tool: &str, asset: &str) -> String {
    format!("{MIRROR_BASE}/{tool}/latest/{asset}.sha256")
}

/// URL 追加 query 参数（已含 query 用 `&` 连接）。
/// 镜像段缓存击穿用（ohmycloud#9 五端验收发现：CF 边缘缓存对 R2 覆写不失效，
/// 同 path 陈旧对象会被锚校验拦下；CF 缓存键含 query 而 R2 取对象只看 path）：
/// 边车带时间戳每次回源取新，资产带锚值（锚变缓存键变，锚同则缓存对象必与锚一致）。
pub fn with_query(url: &str, kv: &str) -> String {
    let sep = if url.contains('?') { "&" } else { "?" };
    format!("{url}{sep}{kv}")
}

/// D51 官方优先逃逸阀判定纯核（测试面）：值域仅 `0` 触发逃逸；`1` 为兼容 no-op
/// （镜像默认通道转正后 =1 已是默认行为）、未设与其余值均走默认镜像优先。
pub(crate) fn is_mirror_off(val: Option<&str>) -> bool {
    val == Some("0")
}

/// D51 官方优先逃逸阀（`ARK_MIRROR=0`）：镜像故障时跳过镜像首试
/// 直走官方链（下载层两条链与 selfupdate 元数据读序同阀）；解析面无需此阀——pin 驱动
/// 本就零 API 且主通道之外的兜底本就是官方直链。
pub(crate) fn mirror_off() -> bool {
    is_mirror_off(crate::platform::env_var("ARK_MIRROR").as_deref())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 带镜像优先的资产下载（D44 反转，2026-09-13 用户裁定「安装默认走 ohmygh，官方是兜底」）：
/// - 缓存三分支照常（命中即复用，与渠道无关）；
/// - 镜像段**单次快速首试**（不退避不 curl：镜像未播该版本是常态而非异常，
///   404 须秒级回落官方，不能拖满重试链）；
/// - 镜像失败（未命中 / 网络错 / 锚不符）回落官方完整链（ureq 三次退避加 curl 兜底）；
/// - 校验锚语义不变：expected_sha256 在位则镜像段同锚校验（锚不符视同镜像失败回落，
///   CF 陈旧对象被锚拦下）；双链全败才报错，错误信息带两段。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn download_asset_with_mirror(
    env_root: &Path,
    asset_name: &str,
    url: &str,
    expected_sha256: Option<&str>,
    force: bool,
    tool: &str,
    version: &str,
) -> Result<PathBuf, String> {
    download_asset_with_mirror_urls(
        env_root,
        asset_name,
        url,
        &mirror_url(tool, version, asset_name),
        expected_sha256,
        force,
    )
}

/// 上一函数的显式 URL 形态（测试注入不可达地址用，不拼镜像段）。
fn download_asset_with_mirror_urls(
    env_root: &Path,
    asset_name: &str,
    official_url: &str,
    mirror_dl_url: &str,
    expected_sha256: Option<&str>,
    force: bool,
) -> Result<PathBuf, String> {
    let dest = cache_path(env_root, asset_name);
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("创建缓存目录失败: {}: {e}", dir.display()))?;
    }
    if let Some(hit) = cache_reuse(&dest, expected_sha256, force)? {
        return Ok(hit);
    }
    // D51 逃逸阀：官方优先时跳过镜像首试（含边车锚解析），直走官方完整链
    if mirror_off() {
        return download_asset(env_root, asset_name, official_url, expected_sha256, true);
    }
    // 锚解析（对线 F1）：调用方锚优先；无锚时取镜像**版本段边车** `{url}.sha256` 作该段锚
    // （R015「无 sha 不入镜」，版本段边车在位是常态；与 latest 段口径统一）；
    // 边车取不到即回落官方——镜像段不产生无校验下载（原 D08「有锚才回落」门的镜像侧等价物）。
    let anchor = match expected_sha256 {
        Some(s) => s.to_string(),
        None => match mirror_sidecar_anchor_fast(&format!("{mirror_dl_url}.sha256")) {
            Ok(sha) => sha,
            Err(sidecar_err) => {
                eprintln!(
                    "[INFO] 镜像版本段无锚（边车取不到），回落官方: {official_url}（{sidecar_err}）"
                );
                return download_asset(env_root, asset_name, official_url, expected_sha256, true);
            }
        },
    };
    // CF 缓存击穿：锚值进 query（锚变缓存键变；锚校验兜底语义不变）
    let murl_busted = with_query(mirror_dl_url, &format!("v={anchor}"));
    match mirror_fetch_once(&dest, &murl_busted, Some(&anchor)) {
        Ok(()) => {
            eprintln!("[OK] 已下载（镜像优先）: {}", dest.display());
            Ok(dest)
        }
        Err(mirror_err) => {
            eprintln!("[INFO] 镜像未命中或失败，回落官方: {official_url}（{mirror_err}）");
            download_asset(env_root, asset_name, official_url, expected_sha256, true).map_err(
                |official_err| {
                    format!(
                        "镜像与官方双链失败\n镜像({murl_busted}): {mirror_err}\n官方({official_url}): {official_err}"
                    )
                },
            )
        }
    }
}

/// 镜像边车锚单次快取（对线 F1/F3）：单次短超时取文本（不退避不 curl——镜像未播属常态，
/// 须秒级判明），解析首 token 64-hex 大写；`mirror_sidecar_sha` 的快速版。
fn mirror_sidecar_anchor_fast(sidecar_url: &str) -> Result<String, String> {
    // 错误串印实际请求 URL（对线修正批）：入参是裸 URL、实际带 t 击穿 query，此前错误串
    // 只印入参会造成「此跳没穿」的误判（REQ-0004 原 RCA 即源于此）。
    let url = with_query(sidecar_url, &format!("t={}", now_secs()));
    let text = fetch_text_short(&url, Duration::from_secs(20))?;
    parse_sidecar_sha(&text, &url)
}

/// 缓存三分支（原 download_asset 前半，提取共用）：命中返回 Some(dest)。
/// sha 基准在位且不符即删缓存（返回 None 走下载）；无基准且非空复用；空缓存删除。
fn cache_reuse(
    dest: &Path,
    expected_sha256: Option<&str>,
    force: bool,
) -> Result<Option<PathBuf>, String> {
    if !dest.exists() || force {
        return Ok(None);
    }
    if let Some(exp) = expected_sha256 {
        let actual = sha256_file(dest)?;
        if actual.eq_ignore_ascii_case(exp) {
            eprintln!("[OK] 命中缓存（sha256 一致）: {}", dest.display());
            return Ok(Some(dest.to_path_buf()));
        }
        eprintln!(
            "[WARN] 缓存 sha256 不匹配，删除后重新下载: {}",
            dest.display()
        );
        fs::remove_file(dest).map_err(|e| format!("删除旧缓存失败: {}: {e}", dest.display()))?;
    } else {
        let nonempty = fs::metadata(dest).map(|m| m.len() > 0).unwrap_or(false);
        if nonempty {
            eprintln!("[INFO] 已有缓存但无 sha256 基准，复用: {}", dest.display());
            return Ok(Some(dest.to_path_buf()));
        }
        eprintln!(
            "[WARN] 缓存为空（视为未完成），删除后重新下载: {}",
            dest.display()
        );
        fs::remove_file(dest).map_err(|e| format!("删除空缓存失败: {}: {e}", dest.display()))?;
    }
    Ok(None)
}

/// 镜像段单次快速下载（D44）：单次 ureq 不退避不 curl，先 .part 再提交；
/// 锚在位则校验（不符即 Err，视同镜像失败由调用方回落官方）；失败清理不落半截。
fn mirror_fetch_once(dest: &Path, url: &str, expected_sha256: Option<&str>) -> Result<(), String> {
    let part = part_path(dest);
    let _ = fs::remove_file(&part);
    match download_once(url, &part) {
        Ok(()) => commit_part(&part, dest)?,
        Err(e) => {
            let _ = fs::remove_file(&part);
            return Err(e);
        }
    }
    if let Some(exp) = expected_sha256 {
        let actual = sha256_file(dest)?;
        if !actual.eq_ignore_ascii_case(exp) {
            let _ = fs::remove_file(dest);
            return Err(format!(
                "镜像对象 sha256 与锚不符（已删，回落官方）: 期望 {}，实际 {actual}",
                exp.to_uppercase()
            ));
        }
    }
    Ok(())
}

/// 边车文本解析 sha：标准清单行 `<sha>  <filename>`，取首 token 大写化（纯函数可测）。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn parse_sidecar_sha(text: &str, sidecar_url: &str) -> Result<String, String> {
    text.split_whitespace()
        .next()
        .filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|s| s.to_uppercase())
        .ok_or_else(|| format!("边车无有效 sha256: {sidecar_url}"))
}

/// 镜像 .sha256 边车取锚（digest 替代源）。单次短超时快取（对线 F3：不退避不 curl，
/// 镜像未播或不可达须秒级回落官方，完整重试链会让 evergreen 常态路径先赔数十秒）；
/// 时间戳 query 每次回源（latest 段沙滚，边车必须取新）。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn mirror_sidecar_sha(_env_root: &Path, sidecar_url: &str) -> Result<String, String> {
    mirror_sidecar_anchor_fast(sidecar_url)
}

/// 带镜像优先的 latest 段资产下载（D08 第二批，evergreen 引导器：rust / vsbuild；D44 反转）：
/// - 缓存三分支照常；
/// - 镜像 latest 段**首试**：校验锚取同目录 `.sha256` 边车（先边车后资产，边车即当段
///   唯一信任锚，沙滚语义；资产单次快速下载），任一步失败回落官方；
/// - 官方段完整链兜底（evergreen 无 pin 锚，官方段无锚裸下与反转前镜像段口径一致）；
/// - 双链全败才报错，错误信息带两段。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn download_latest_with_sidecar(
    env_root: &Path,
    asset_name: &str,
    official_url: &str,
    tool: &str,
) -> Result<PathBuf, String> {
    download_latest_with_sidecar_urls(
        env_root,
        asset_name,
        official_url,
        &mirror_sidecar_url(tool, asset_name),
        &mirror_url(tool, "latest", asset_name),
    )
}

/// 上一函数的显式 URL 形态（单测注入不可达地址用，不触真网）。
fn download_latest_with_sidecar_urls(
    env_root: &Path,
    asset_name: &str,
    official_url: &str,
    sidecar_url: &str,
    mirror_dl_url: &str,
) -> Result<PathBuf, String> {
    let dest = cache_path(env_root, asset_name);
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("创建缓存目录失败: {}: {e}", dir.display()))?;
    }
    if let Some(hit) = cache_reuse(&dest, None, false)? {
        return Ok(hit);
    }
    // D51 逃逸阀：官方优先时跳过镜像 latest 段首试，直走官方完整链（无锚裸下，与镜像段口径一致）
    if mirror_off() {
        return download_asset(env_root, asset_name, official_url, None, true);
    }
    // 镜像首试：边车锚 → 资产（CF 缓存击穿：锚值进 query）
    let mirror_step = mirror_sidecar_sha(env_root, sidecar_url).and_then(|anchor| {
        let mirror_busted = with_query(mirror_dl_url, &format!("v={anchor}"));
        mirror_fetch_once(&dest, &mirror_busted, Some(&anchor))
    });
    match mirror_step {
        Ok(()) => {
            eprintln!("[OK] 已下载（镜像优先，边车锚校验）: {}", dest.display());
            Ok(dest)
        }
        Err(mirror_err) => {
            eprintln!("[INFO] 镜像未命中或失败，回落官方: {official_url}（{mirror_err}）");
            download_asset(env_root, asset_name, official_url, None, true).map_err(
                |official_err| {
                    format!(
                        "镜像与官方双链失败\n镜像({mirror_dl_url}): {mirror_err}\n官方({official_url}): {official_err}"
                    )
                },
            )
        }
    }
}

fn part_path(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    dest.with_file_name(name)
}

fn commit_part(part: &Path, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        fs::remove_file(dest).map_err(|e| format!("替换缓存失败: {}: {e}", dest.display()))?;
    }
    match fs::rename(part, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(part, dest).map_err(|e| format!("提交缓存失败: {}: {e}", dest.display()))?;
            let _ = fs::remove_file(part);
            Ok(())
        }
    }
}

/// ureq 下载（3 次指数退避），失败回退系统 curl.exe（-L --fail --retry 5）。
/// 先写 `.part` 再提交为 dest，失败删除 part，不留下半截 dest。
fn download_url(url: &str, dest: &Path) -> Result<(), String> {
    let part = part_path(dest);
    let _ = fs::remove_file(&part);
    let mut last_err = String::new();
    for attempt in 1..=MAX_ATTEMPTS {
        match download_once(url, &part) {
            Ok(()) => return commit_part(&part, dest),
            Err(e) => {
                last_err = e;
                let _ = fs::remove_file(&part);
                if attempt < MAX_ATTEMPTS {
                    let wait = 2u64.pow(attempt);
                    eprintln!(
                        "[WARN] 下载失败，{wait}s 后重试（{attempt}/{MAX_ATTEMPTS}）: {last_err}"
                    );
                    std::thread::sleep(Duration::from_secs(wait));
                }
            }
        }
    }

    // 回退系统 curl.exe（对齐 pwsh：ureq/IWR 之外的独立网络栈兜底）
    let curl = which::which("curl")
        .map_err(|_| format!("下载失败且 curl.exe 不可用: {url}\n{last_err}"))?;
    eprintln!("[WARN] ureq 下载失败，改用 curl.exe: {last_err}");
    let status = Command::new(curl)
        .args([
            "-L",
            "--fail",
            "--retry",
            "5",
            "--retry-delay",
            "3",
            "--connect-timeout",
            "20",
            "--max-time",
            "120",
            "-sS",
            "-o",
        ])
        .arg(&part)
        .arg(url)
        .status()
        .map_err(|e| format!("curl.exe 执行失败: {e}"))?;
    if !status.success() || !part.exists() {
        let _ = fs::remove_file(&part);
        return Err(format!("curl.exe 下载失败（{:?}）: {url}", status.code()));
    }
    commit_part(&part, dest)
}

/// 单次短超时文本取回（自动刷新探活用，D33）：不重试、不走 curl 兜底，失败即 Err。
/// 与 download_url 的重试链分离：自动路径要在网络异常时快速退化，不拖慢用户命令。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn fetch_text_short(url: &str, timeout: Duration) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(timeout)
        .timeout(timeout)
        .build();
    let resp = agent
        .get(url)
        .set("User-Agent", "ark-catalog")
        .call()
        .map_err(|e| format!("HTTP 请求失败: {url}: {e}"))?;
    resp.into_string()
        .map_err(|e| format!("读响应失败: {url}: {e}"))
}

/// 单次 ureq 下载：30s 连接超时、120s 总超时，流式写盘到 dest（调用方传入 .part）。
fn download_once(url: &str, dest: &Path) -> Result<(), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(30))
        .timeout(Duration::from_secs(120))
        .build();
    let resp = agent
        .get(url)
        .set("User-Agent", "ark-bootstrap")
        .call()
        .map_err(|e| format!("HTTP 请求失败: {url}: {e}"))?;
    let mut reader = resp.into_reader();
    let f = File::create(dest).map_err(|e| format!("创建文件失败: {}: {e}", dest.display()))?;
    let mut writer = BufWriter::new(f);
    io::copy(&mut reader, &mut writer)
        .map_err(|e| format!("写入文件失败: {}: {e}", dest.display()))?;
    writer
        .flush()
        .map_err(|e| format!("写入文件失败: {}: {e}", dest.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "abc" 的 sha256 是公开常数（FIPS 180-4 示例值），作独立期望值来源。
    #[test]
    fn sha256_计算_大写hex() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let path = dir.path().join("abc.txt");
        fs::write(&path, b"abc").map_err(|e| e.to_string())?;
        assert_eq!(
            sha256_file(&path)?,
            "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
        );
        Ok(())
    }

    #[test]
    fn 缓存_无sha基准_直接复用不触网() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dest = cache_path(dir.path(), "demo.zip");
        fs::create_dir_all(dest.parent().ok_or("无父目录")?).map_err(|e| e.to_string())?;
        fs::write(&dest, b"cached").map_err(|e| e.to_string())?;

        // 无 sha 基准：即使 URL 不可达也应直接复用（不发起网络请求）
        let got = download_asset(
            dir.path(),
            "demo.zip",
            "https://example.invalid/x",
            None,
            false,
        )?;
        assert_eq!(got, dest);
        assert_eq!(fs::read(&got).map_err(|e| e.to_string())?, b"cached");
        Ok(())
    }

    #[test]
    fn 缓存_sha一致_复用不触网() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dest = cache_path(dir.path(), "demo.zip");
        fs::create_dir_all(dest.parent().ok_or("无父目录")?).map_err(|e| e.to_string())?;
        fs::write(&dest, b"abc").map_err(|e| e.to_string())?;

        let sha = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
        let got = download_asset(
            dir.path(),
            "demo.zip",
            "https://example.invalid/x",
            Some(sha),
            false,
        )?;
        assert_eq!(got, dest, "sha 一致应复用缓存");
        Ok(())
    }

    #[test]
    fn dies_缓存_sha不符_删除后重下失败() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dest = cache_path(dir.path(), "demo.zip");
        fs::create_dir_all(dest.parent().ok_or("无父目录")?).map_err(|e| e.to_string())?;
        fs::write(&dest, b"stale").map_err(|e| e.to_string())?;

        // sha 不符：应删除旧缓存并尝试重下；本机不可达端口（连接即刻拒绝）最终报错
        let bogus = "0000000000000000000000000000000000000000000000000000000000000000";
        let err = download_asset(
            dir.path(),
            "demo.zip",
            "http://127.0.0.1:1/x",
            Some(bogus),
            false,
        )
        .expect_err("不可达 URL 应报错");
        assert!(!dest.exists(), "旧缓存应已被删除");
        assert!(!err.is_empty());
        Ok(())
    }

    #[test]
    fn dies_空缓存无sha_不复用() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dest = cache_path(dir.path(), "demo.zip");
        fs::create_dir_all(dest.parent().ok_or("无父目录")?).map_err(|e| e.to_string())?;
        fs::write(&dest, b"").map_err(|e| e.to_string())?;
        let err = download_asset(dir.path(), "demo.zip", "http://127.0.0.1:1/x", None, false)
            .expect_err("空缓存应视为未完成并重下失败");
        assert!(!err.is_empty());
        let part = dest.with_file_name("demo.zip.part");
        assert!(!part.exists(), "失败不应留下 .part");
        Ok(())
    }

    /// 边车标准清单行 `<sha>  <filename>`：首 token 64-hex 取出并大写化；短值与非 hex 拒绝。
    #[test]
    fn 边车解析_标准清单行取首token大写() {
        const URL: &str = "https://mirror.example/tool/latest/a.exe.sha256";
        let sha = "ab7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(
            parse_sidecar_sha(&format!("{sha}  a.exe\n"), URL).unwrap(),
            "AB7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
        );
        assert!(parse_sidecar_sha(&format!("{}  a.exe", &sha[..63]), URL).is_err());
        assert!(parse_sidecar_sha(
            "zz7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  a.exe",
            URL
        )
        .is_err());
        assert!(parse_sidecar_sha("", URL).is_err());
    }

    /// 缓存击穿 query：无 query 用 `?`、已有用 `&` 连接（CF 缓存键含 query 而 R2 只看 path）。
    #[test]
    fn query追加_无有各用对连接符() {
        assert_eq!(
            with_query("https://env.ohmygh.com/a/b.zip", "v=ABC"),
            "https://env.ohmygh.com/a/b.zip?v=ABC"
        );
        assert_eq!(
            with_query("https://env.ohmygh.com/a/b.zip?t=1", "v=ABC"),
            "https://env.ohmygh.com/a/b.zip?t=1&v=ABC"
        );
    }

    /// D51 逃逸阀判定值域：仅 `0` 触发官方优先；`1` 兼容 no-op、未设与其余值走默认镜像。
    #[test]
    fn 逃逸阀判定_值域仅零触发() {
        assert!(!is_mirror_off(None), "未设走默认镜像优先");
        assert!(!is_mirror_off(Some("1")), "=1 已是默认（no-op）");
        assert!(!is_mirror_off(Some("")), "空串不触发");
        assert!(!is_mirror_off(Some("true")), "非零值不触发");
        assert!(is_mirror_off(Some("0")), "仅 0 触发官方优先逃逸");
    }

    /// D44 反转后双断：镜像段（边车）断→回落官方段断→双链报错（镜像在前官方在后），不落资产。
    #[test]
    fn dies_镜像边车断且官方断_双链报错不落资产() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let err = download_latest_with_sidecar_urls(
            dir.path(),
            "demo-init.exe",
            "http://127.0.0.1:1/official",
            "http://127.0.0.1:1/sidecar.sha256",
            "http://127.0.0.1:1/mirror",
        )
        .expect_err("双断应报错");
        assert!(err.contains("镜像与官方双链失败"), "错误应说明双链: {err}");
        // 镜像在前官方在后（反转后链序）
        let mi = err.find("镜像(").expect("应含镜像段");
        let oi = err.find("官方(").expect("应含官方段");
        assert!(mi < oi, "镜像段应在前: {err}");
        assert!(
            !cache_path(dir.path(), "demo-init.exe").exists(),
            "不应留下未校验资产"
        );
        Ok(())
    }

    /// D44 对线 F1/G3：无锚且镜像版本段边车取不到 → 只走官方（不走镜像资产段）。
    /// 判据：错误串不含「镜像(」资产段、含官方 URL、不落资产文件。
    #[test]
    fn dies_无锚且边车断_只走官方不落资产() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let err = download_asset_with_mirror_urls(
            dir.path(),
            "noanchor.zip",
            "http://127.0.0.1:1/official",
            "http://127.0.0.1:1/mirror",
            None,
            false,
        )
        .expect_err("双断应报错");
        assert!(
            !err.contains("镜像("),
            "无锚且边车断不得走镜像资产段（无校验下载门）: {err}"
        );
        assert!(
            err.contains("http://127.0.0.1:1/official"),
            "应含官方段: {err}"
        );
        assert!(
            !cache_path(dir.path(), "noanchor.zip").exists(),
            "不应留下未校验资产"
        );
        Ok(())
    }

    /// D44 反转：主链镜像段单次失败回落官方段（官方也断→双链报错，镜像在前）。
    #[test]
    fn dies_镜像断回落官方也断_双链报错() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let err = download_asset_with_mirror_urls(
            dir.path(),
            "demo.zip",
            "http://127.0.0.1:1/official",
            "http://127.0.0.1:1/mirror",
            Some("0000000000000000000000000000000000000000000000000000000000000000"),
            false,
        )
        .expect_err("双断应报错");
        assert!(err.contains("镜像与官方双链失败"), "{err}");
        let mi = err.find("镜像(").expect("应含镜像段");
        let oi = err.find("官方(").expect("应含官方段");
        assert!(mi < oi, "镜像段应在前: {err}");
        Ok(())
    }
}
