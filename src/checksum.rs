//! checksum：sha256 校验源解析，语义对齐 helpers.ps1 的 Get-OfficialSha256。
//! 优先级（R001 四、2）：pin 的 sha256 > 官方校验源三型——
//! 1) cdn_index_url 的 HashiCorp SHA256SUMS 清单（shasums_url 来自解析结果）；
//! 2) sums_asset 统一校验清单（{version}/{tag} 占位，按 sums_pattern 取行）；
//! 3) asset_sha_suffix 逐资产后缀文件（如 <asset>.sha256）。
//!
//! 校验清单类资产每次强制重下（删旧再下），结果统一大写。

use std::fs;
use std::path::Path;

use regex::Regex;

use crate::catalog::Tool;
use crate::download;
use crate::resolve::Resolution;

const HEX64: &str = "[0-9a-fA-F]{64}";

/// 读取可能为 UTF-8 / UTF-16LE / UTF-16BE 的校验清单文本（PowerShell 发布包常见 UTF-16）。
fn read_checksum_text(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("读取校验清单失败: {}: {e}", path.display()))?;
    // 先按 UTF-8 尝试
    if let Ok(s) = String::from_utf8(bytes.clone()) {
        return Ok(s);
    }
    // UTF-16 LE BOM
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return decode_utf16(&bytes[2..], u16::from_le_bytes);
    }
    // UTF-16 BE BOM
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return decode_utf16(&bytes[2..], u16::from_be_bytes);
    }
    // 无 BOM 时按 LE 尝试（Windows/PowerShell 默认）
    if bytes.len() % 2 == 0 {
        if let Ok(s) = decode_utf16(&bytes, u16::from_le_bytes) {
            return Ok(s);
        }
    }
    Err(format!("校验清单编码无法识别: {}", path.display()))
}

fn decode_utf16<F>(bytes: &[u8], mut read_u16: F) -> Result<String, String>
where
    F: FnMut([u8; 2]) -> u16,
{
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| read_u16(*c))
        .collect();
    String::from_utf16(&units).map_err(|e| format!("UTF-16 解码失败: {e}"))
}

/// 本次下载应遵循的 sha256 基准：pin 的 sha256 优先，但必须 **同 tag 且同 asset**
/// （资产名跨版本不变的工具——uv/jq/bun/fnm/agent——否则会用旧锚校验新包）。
/// 否则查官方校验源，都没有则 None。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn expected_sha256(
    tool: &Tool,
    res: &Resolution,
    env_root: &Path,
) -> Result<Option<String>, String> {
    if let Some(sha) = tool.pin_sha256() {
        if !sha.trim().is_empty() {
            let same_tag = tool.pin_tag() == Some(res.tag.as_str());
            let pinned_asset = tool.pin_asset().unwrap_or("");
            let same_asset = pinned_asset.is_empty() || pinned_asset == res.asset_name;
            if same_tag && same_asset {
                return Ok(Some(sha.to_uppercase()));
            }
        }
    }
    // D43：官方 sha 直值锚（ziglang index per-target shasum）优先于清单与 digest 通道
    if let Some(sha) = &res.official_sha256 {
        return Ok(Some(sha.to_uppercase()));
    }
    official_sha256(tool, res, env_root)
}

/// 官方校验源三型（对齐 Get-OfficialSha256 的分支顺序）。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn official_sha256(
    tool: &Tool,
    res: &Resolution,
    env_root: &Path,
) -> Result<Option<String>, String> {
    // 1) HashiCorp 来源：SHA256SUMS 清单，行内按资产名匹配取 64-hex
    if tool.cdn_index_url.is_some() {
        if let Some(sums_url) = &res.shasums_url {
            let sums_name = sums_url
                .rsplit('/')
                .next()
                .ok_or_else(|| format!("shasums_url 无法取文件名: {sums_url}"))?;
            let path = download::download_fresh(env_root, sums_name, sums_url)?;
            let text = read_checksum_text(&path)?;
            let needle = Regex::new(&regex::escape(&res.asset_name))
                .map_err(|e| format!("资产名正则构造失败: {e}"))?;
            let line = text.lines().find(|l| needle.is_match(l)).ok_or_else(|| {
                format!(
                    "{} 官方 SHA256SUMS 未找到匹配资产: {}",
                    res.tool, res.asset_name
                )
            })?;
            return extract_hex64(line)
                .map(Some)
                .ok_or_else(|| format!("{} 官方 SHA256SUMS 行内无 64-hex: {line}", res.tool));
        }
    }

    // 2) 统一校验清单资产：sums_asset 名带 {version}/{tag} 占位，按 sums_pattern 取行
    if let Some(sums_asset) = &tool.sums_asset {
        let sums_name = sums_asset
            .replace("{version}", &res.version)
            .replace("{tag}", &res.tag);
        let repo = tool
            .repo
            .as_deref()
            .ok_or_else(|| format!("{} 使用 sums_asset 但缺少 repo 字段", res.tool))?;
        let sums_url = format!(
            "https://github.com/{repo}/releases/download/{}/{sums_name}",
            res.tag
        );
        let path = download::download_fresh(env_root, &sums_name, &sums_url)?;
        let text = read_checksum_text(&path)?;
        let pattern = tool
            .sums_pattern()
            .ok_or_else(|| format!("{} 使用 sums_asset 但缺少 sums_pattern", res.tool))?;
        let re = Regex::new(pattern).map_err(|e| format!("{} sums_pattern 非法: {e}", res.tool))?;
        let line = text
            .lines()
            .find(|l| re.is_match(l))
            .ok_or_else(|| format!("{} 官方校验清单中未找到匹配资产: {pattern}", res.tool))?;
        return extract_hex64(line)
            .map(Some)
            .ok_or_else(|| format!("{} 官方校验清单行内无 64-hex: {line}", res.tool));
    }

    // 3) 逐资产后缀文件：<asset>.sha256，全文取第一个 64-hex
    if let Some(suffix) = tool.asset_sha_suffix() {
        let sha_name = format!("{}{suffix}", res.asset_name);
        let sha_url = format!("{}{suffix}", res.asset_url);
        let path = download::download_fresh(env_root, &sha_name, &sha_url)?;
        let text = fs::read_to_string(&path)
            .map_err(|e| format!("读取校验文件失败: {}: {e}", path.display()))?;
        return extract_hex64(text.trim())
            .map(Some)
            .ok_or_else(|| format!("{} 官方 .sha256 解析失败: {sha_url}", res.tool));
    }

    Ok(None)
}

/// 从一行文本提取第一个 64 位 hex 并大写（对齐 pwsh 的 ([0-9a-fA-F]{64}) + ToUpperInvariant）。
pub fn extract_hex64(text: &str) -> Option<String> {
    let re = Regex::new(HEX64).ok()?;
    let m = re.find(text)?;
    Some(m.as_str().to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex64_提取_统一大写() {
        let line = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  abc.zip";
        assert_eq!(
            extract_hex64(line).as_deref(),
            Some("BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD")
        );
    }

    #[test]
    fn hex64_不足64位返回none() {
        assert_eq!(extract_hex64("abc123"), None);
        assert_eq!(extract_hex64(""), None);
    }

    #[test]
    fn 优先级_pin的sha256高于官方源() -> Result<(), String> {
        // pin 有 sha256 时不应触发任何官方源下载（此处 vault 无网也应直接返回）；
        // 三平台 pin 键同值构造，断言在任一平台都命中本平台 pin
        let tool = Tool {
            tag: Some("v1".to_string()),
            linux_tag: Some("v1".to_string()),
            mac_tag: Some("v1".to_string()),
            sha256: Some("a4a9a398".to_string()),
            linux_sha256: Some("a4a9a398".to_string()),
            mac_sha256: Some("a4a9a398".to_string()),
            ..Tool::default()
        };
        let res = Resolution {
            tool: "demo".to_string(),
            tag: "v1".to_string(),
            version: "1".to_string(),
            asset_name: "demo.zip".to_string(),
            asset_size: 0,
            asset_url: "https://example.invalid/demo.zip".to_string(),
            shasums_url: None,
            official_sha256: None,
            fallback_url: None,
        };
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let got = expected_sha256(&tool, &res, dir.path())?;
        assert_eq!(got.as_deref(), Some("A4A9A398"), "pin sha 应直接返回并大写");
        Ok(())
    }

    #[test]
    fn 优先级_同资产不同tag_不用旧pin_sha() -> Result<(), String> {
        // 资产名跨版本不变时，旧 pin sha 不能当新包锚（否则 update 必失败）
        let tool = Tool {
            tag: Some("v1".to_string()),
            linux_tag: Some("v1".to_string()),
            mac_tag: Some("v1".to_string()),
            asset: Some("demo.zip".to_string()),
            linux_asset: Some("demo.zip".to_string()),
            mac_asset: Some("demo.zip".to_string()),
            sha256: Some("A4A9A398".to_string()),
            linux_sha256: Some("A4A9A398".to_string()),
            mac_sha256: Some("A4A9A398".to_string()),
            ..Tool::default()
        };
        let res = Resolution {
            tool: "demo".to_string(),
            tag: "v2".to_string(),
            version: "2".to_string(),
            asset_name: "demo.zip".to_string(),
            asset_size: 0,
            asset_url: "https://example.invalid/demo.zip".to_string(),
            shasums_url: None,
            official_sha256: None,
            fallback_url: None,
        };
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        assert_eq!(
            expected_sha256(&tool, &res, dir.path())?,
            None,
            "tag 变了即使资产名相同也不用旧 pin sha"
        );
        Ok(())
    }

    #[test]
    fn 优先级_无pin且无官方源_返回none() -> Result<(), String> {
        let tool = Tool::default();
        let res = Resolution {
            tool: "demo".to_string(),
            tag: "v1".to_string(),
            version: "1".to_string(),
            asset_name: "demo.zip".to_string(),
            asset_size: 0,
            asset_url: "https://example.invalid/demo.zip".to_string(),
            shasums_url: None,
            official_sha256: None,
            fallback_url: None,
        };
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        assert_eq!(expected_sha256(&tool, &res, dir.path())?, None);
        Ok(())
    }
}
