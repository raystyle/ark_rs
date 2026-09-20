//! ledger：仓级公共账本命令面接线（REQ-0015；总台修正令 2026-09-20 收口为
//! ledger-rs crate 唯一实现——签名道加只增面全在 crate，本仓不再自研客户端副本）。
//! 真源 https://ledger.ohmygh.com（issues.ohmygh.com 已转只读保役）。
//! 权限收口：只增不关不删——issue new/list/show 加 artifact publish/attest
//!（attest_dev|attest_prod|verification_failed）加 list；close/status 推进与
//! promote/demote/supersede 及删除唯一道 = 开发工作台 herdr 委托 omc 工位执行。
//! 私钥密档 `~/.config/ark/ledger.key`（32 字节 hex；`ARK_LEDGER_KEY` 环境或
//! `ARK_LEDGER_KEY_FILE` 可覆盖）——不进仓不进 argv。

use std::path::PathBuf;

use ledger_client::{KeyPair, Ledger};
use serde_json::Value;

/// 本仓 repo_id（规范化 remote）。
pub const REPO_ID: &str = "github.com/raystyle/ark_rs";

/// 列表夹取界（服务端 clampLimit 同口径；家族标准默认 100 即上限）。
pub const LIST_LIMIT_MAX: u32 = 100;

/// digest 形校验：`sha256:<64hex>`（小写；一律正文或记录哈希为身份，库不收二进制实体）。
pub fn valid_digest(digest: &str) -> bool {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// 正文或记录哈希（digest 即身份；crate content_digest 同源）。
pub fn content_digest(text: &str) -> String {
    ledger_client::content_digest(text)
}

/// 饱和判定（家族标准 #52 同口径，纯函数）：返回条数恰打满夹取后 limit。
pub fn list_saturated(returned: usize, limit: u32) -> bool {
    returned == limit.clamp(1, LIST_LIMIT_MAX) as usize
}

/// 装配账本客户端：私钥读环境 `ARK_LEDGER_KEY`（32 字节 hex）优先，次
/// `ARK_LEDGER_KEY_FILE` 指定密档，缺省 `~/.config/ark/ledger.key`。
///
/// # Errors
/// 返回 Err（人读原因串）当：密档缺位或 crate 密钥装载失败。
pub fn client() -> Result<Ledger, String> {
    let secret = match std::env::var("ARK_LEDGER_KEY") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => {
            let path = std::env::var("ARK_LEDGER_KEY_FILE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| default_key_file());
            std::fs::read_to_string(&path)
                .map_err(|e| format!("读私钥密档失败: {}: {e}", path.display()))?
                .trim()
                .to_string()
        }
    };
    let key = KeyPair::load_secret_hex(&secret).map_err(|e| format!("私钥装载失败: {e}"))?;
    Ok(Ledger::new(REPO_ID, key))
}

fn default_key_file() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("ark")
        .join("ledger.key")
}

/// issue 开单（crate 只增面）：回执 issue 号。
///
/// # Errors
/// 返回 Err（人读原因串）当：标题/kind 校验不过（本地早拦）或服务端 4xx/5xx 透传。
pub fn issue_new(
    title: &str,
    kind: &str,
    acceptance: &str,
    body: Option<&str>,
) -> Result<u64, String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err("title 必填且至多 200 字符".to_string());
    }
    if !["bug", "improvement"].contains(&kind) {
        return Err("kind 仅 bug(BUG 错误任务)或 improvement(改进优化任务)".to_string());
    }
    client()?
        .issue_new(title, kind, acceptance, body)
        .map_err(|e| format!("ledger 写入失败: {e}"))
}

/// 只读客户端（crate read_only 构造，GET 面免签免私钥）。
fn reader() -> Ledger {
    Ledger::read_only(REPO_ID)
}

/// issue 列表（crate 读面，家族翻页 more=1 恒带；免私钥）：回执含 issues 与 has_more。
///
/// # Errors
/// 返回 Err（人读原因串）当：网络失败或回执形不符。
pub fn issue_list(limit: u32, before: Option<u64>) -> Result<(Vec<Value>, Option<bool>), String> {
    let v = reader()
        .issue_list(limit.clamp(1, LIST_LIMIT_MAX), before)
        .map_err(|e| format!("ledger 请求失败: {e}"))?;
    let issues = v
        .get("issues")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "列表回执不识别（期望 {ok,count,issues[]}）".to_string())?;
    Ok((issues, v.get("has_more").and_then(Value::as_bool)))
}

/// issue 详情（crate 读面）：projection 加 timeline。
///
/// # Errors
/// 返回 Err（人读原因串）当：网络失败或 404（issue 未找到）。
pub fn issue_show(n: u64) -> Result<Value, String> {
    reader()
        .issue_show(n)
        .map_err(|e| format!("ledger 请求失败: {e}"))
}

/// artifact 发布（crate 只增面）：回执 artifact_id。
///
/// # Errors
/// 返回 Err（人读原因串）当：name/kind/digest 校验不过（本地早拦）或服务端 4xx 透传。
#[allow(clippy::too_many_arguments)] // crate _full 形直传（收口薄层，拆结构体反增面）
pub fn artifact_publish(
    name: &str,
    kind: &str,
    digest: &str,
    version: Option<&str>,
    git_range: Option<&str>,
    deps: &[String],
    note: Option<&str>,
    summary: Option<&str>,
    outcome: Option<&str>,
) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 200 {
        return Err("name 必填且至多 200 字符".to_string());
    }
    if !ledger_client::ARTIFACT_KINDS.contains(&kind) {
        return Err(format!(
            "kind 仅 {}",
            ledger_client::ARTIFACT_KINDS.join("|")
        ));
    }
    if !valid_digest(digest) {
        return Err(
            "digest 须 sha256:<64hex>（一律正文或记录哈希为身份；库不收二进制实体）".to_string(),
        );
    }
    client()?
        .artifact_publish_full(
            name, kind, digest, version, git_range, deps, note, summary, outcome,
        )
        .map_err(|e| format!("ledger 写入失败: {e}"))
}

/// artifact 证明（crate 只增面，收口后仅三型；promote/demote/supersede 归 omc 工作台）。
///
/// # Errors
/// 返回 Err（人读原因串）当：attest_type 校验不过或服务端 4xx 透传。
pub fn artifact_attest(
    artifact_id: &str,
    attest_type: &str,
    checks: Value,
    note: Option<&str>,
) -> Result<Value, String> {
    if !ledger_client::ATTEST_TYPES.contains(&attest_type) {
        return Err(format!("type 仅 {}", ledger_client::ATTEST_TYPES.join("|")));
    }
    client()?
        .artifact_attest(artifact_id, attest_type, checks, note)
        .map_err(|e| format!("ledger 写入失败: {e}"))
}

/// artifact 列表（crate 读面：current/env 过滤；kind/name 过滤归网页面）。
///
/// # Errors
/// 返回 Err（人读原因串）当：网络失败或回执形不符。
pub fn artifact_list(current: bool, env_filter: Option<&str>) -> Result<Vec<Value>, String> {
    let v = reader()
        .artifact_list(current, env_filter)
        .map_err(|e| format!("ledger 请求失败: {e}"))?;
    v.get("artifacts")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "artifact 列表回执不识别（期望 {ok,count,artifacts[]}）".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 收口后本地纯函数面：digest 边界与正文哈希同源（签名道与 kid 测试随 crate）。
    #[test]
    fn digest校验_边界() {
        assert!(valid_digest(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
        assert!(
            !valid_digest(
                "sha256:0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"
            ),
            "大写拒"
        );
        assert!(!valid_digest("sha256:short"));
        assert!(!valid_digest(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
        assert_eq!(
            content_digest("x"),
            "sha256:2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881"
        );
    }

    /// 家族标准饱和判定同口径（#52）。
    #[test]
    fn 饱和判定_同口径() {
        assert!(list_saturated(100, 100));
        assert!(list_saturated(100, 500));
        assert!(!list_saturated(3, 100));
    }
}
