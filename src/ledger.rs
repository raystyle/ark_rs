//! ledger：仓级公共账本客户端面（REQ-0015，对齐 ohmycloud REQ-063 Phase 3 CLI 原生集成）。
//! 真源 https://ledger.ohmygh.com（issues.ohmygh.com 过渡期保役，本 CLI 面已切新真源）。
//! issue 流 = 立项修复或改进的义务（open 至 done，关单须先有 result 引 digest）；
//! artifact 流 = 产物共享库本体（publish 至 attest 至 promote/demote/supersede）。
//! 写入五头签名道：Idempotency-Key 加 X-Key-Id 加 X-Timestamp（正负 60 秒窗）加
//! X-Nonce（10 分钟不重）加 X-Signature（Ed25519，签名基 v1/POST/路径/时间戳/nonce/
//! 幂等键/body sha256 各行换行连，base64url 编码）；幂等同键同内容回放、同键异内容 409。
//! 私钥运行时读环境 `ARK_LEDGER_KEY` 或密档（`ARK_LEDGER_KEY_FILE`，缺省
//! `~/.config/ark/ledger.key`，base64url 32 字节 seed）——不进仓不进 argv；
//! 公钥 JWK 以常量内置（身份分发面），kid = sha256hex(规范化 JSON {crv,kty,x})。

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64URL;
use base64::Engine;
use ed25519_dalek::Signer;
use serde_json::{json, Value};

use crate::download::sha256_file;
use sha2::{Digest, Sha256};

/// HTTP 超时缺省（毫秒；0 = 不限时）。
pub const TIMEOUT_MS: u64 = 20000;

/// HTTP 客户端（timeout 毫秒；0 = 不限时；ureq 2.x 形，原 issue 域同款迁移）。
pub fn http_client(timeout_ms: u64) -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new().timeout_connect(std::time::Duration::from_millis(
        timeout_ms.min(10_000).max(1),
    ));
    if timeout_ms > 0 {
        builder = builder.timeout(std::time::Duration::from_millis(timeout_ms));
    }
    builder.build()
}
/// 账本服务基址（REQ-063；issues.ohmygh.com 过渡期保役，CLI 面已切此真源）。
pub const LEDGER_BASE: &str = "https://ledger.ohmygh.com";

/// 本仓 repo_id（规范化 remote）。
pub const REPO_ID: &str = "github.com/raystyle/ark_rs";

/// 本仓公钥 JWK（身份分发面，CLI 内置；对应私钥在提交侧密档，永不进仓）。
pub const PUBLIC_JWK: &str =
    r#"{"crv":"Ed25519","kty":"OKP","x":"pb0-690h-Hbavq7bgJQ9hdtbACIM9ndkgS74GzbxNe4"}"#;

/// key id = sha256hex(规范化 JSON {crv,kty,x}，键序字母)：注册与 X-Key-Id 即此值。
pub const KEY_ID: &str = "7f3658485ffd07e2e5712374c2a635959c23a4149405d19873a4174acf6aba18";

/// issue kind 面（服务端 ISSUE_KINDS 同口径）。
pub const ISSUE_KINDS: &[&str] = &["bug", "improvement"];

/// issue 事件面。
pub const ISSUE_EVENT_TYPES: &[&str] = &["claim", "release", "status", "result", "blocker"];

/// issue 状态面（status 事件 to 值）。
pub const ISSUE_STATUSES: &[&str] = &["open", "claimed", "in_progress", "blocked", "done"];

/// artifact kind 面（服务端 ARTIFACT_KINDS 同口径，15 值）。
pub const ARTIFACT_KINDS: &[&str] = &[
    "binary",
    "image",
    "wasm",
    "sbom",
    "schema",
    "openapi",
    "eval-set",
    "benchmark",
    "runbook",
    "decision",
    "attested-report",
    "experience",
    "lesson",
    "research",
    "prototype",
];

/// artifact 证明面。
pub const ATTEST_TYPES: &[&str] = &[
    "attest_dev",
    "attest_prod",
    "verification_failed",
    "promote",
    "demote",
    "supersede",
];

/// 列表夹取界（服务端 clampLimit 同口径；家族标准默认 100 即上限）。
pub const LIST_LIMIT_MAX: u32 = 100;

/// kid 派生（纯函数）：sha256hex(规范化 JSON，键序字母、无空白）。
pub fn kid_of_jwk(jwk: &str) -> String {
    let v: Value = serde_json::from_str(jwk).unwrap_or(Value::Null);
    let mut keys: Vec<&String> = v
        .as_object()
        .map(|m| m.keys().collect())
        .unwrap_or_default();
    keys.sort();
    let canonical: Vec<String> = keys.iter().map(|k| format!("\"{}\":{}", k, v[k])).collect();
    let canonical = format!("{{{}}}", canonical.join(","));
    hex_upper(&Sha256::digest(canonical.as_bytes()))
        .to_lowercase()
        .to_string()
}

/// 签名基构造（纯函数，服务端 signatureBase 同构）：
/// `v1\nPOST\n<pathname>\n<ts>\n<nonce>\n<idem>\n<sha256hex(body)>` 各行换行连。
pub fn signature_base(
    method: &str,
    pathname: &str,
    ts: &str,
    nonce: &str,
    idem: &str,
    body_sha256_hex: &str,
) -> String {
    format!("v1\n{method}\n{pathname}\n{ts}\n{nonce}\n{idem}\n{body_sha256_hex}")
}

/// body sha256（小写 hex）。
pub fn body_sha256_hex(body: &str) -> String {
    hex_lower(&Sha256::digest(body.as_bytes()))
}

/// 幂等键生成（uuid v4；同键同内容回放、同键异内容 409 须换键——每次写入必新键）。
pub fn new_idempotency_key() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// nonce 生成（10 分钟窗不重；uuid v4 形）。
pub fn new_nonce() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// 小写 hex 的 sha256（kid 用小写；本仓统一大写仅限资产校验面，账本面循服务端小写）
fn hex_upper(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

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

/// 文件 digest（本地算锚：artifact publish 的正文或记录哈希即此值）。
///
/// # Errors
/// 返回 Err（人读原因串）当：读文件或哈希计算失败。
pub fn file_digest(path: &std::path::Path) -> Result<String, String> {
    let sha = sha256_file(path)?;
    Ok(format!("sha256:{}", sha.to_lowercase()))
}

/// 私钥装载：`ARK_LEDGER_KEY`（base64url 32 字节 seed）优先，次 `ARK_LEDGER_KEY_FILE`
/// 指定密档，缺省 `~/.config/ark/ledger.key`；不进仓不进 argv。
///
/// # Errors
/// 返回 Err（人读原因串）当：密钥缺位、解码失败或长度非 32 字节。
pub fn load_signing_key() -> Result<ed25519_dalek::SigningKey, String> {
    let seed_b64 = match std::env::var("ARK_LEDGER_KEY") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => {
            let path = std::env::var("ARK_LEDGER_KEY_FILE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| default_key_file());
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("读私钥密档失败: {}: {e}", path.display()))?;
            text.trim().to_string()
        }
    };
    let seed = B64URL
        .decode(seed_b64.as_bytes())
        .map_err(|e| format!("私钥 base64url 解码失败: {e}"))?;
    let seed: [u8; 32] = seed
        .try_into()
        .map_err(|_| "私钥须为 32 字节 seed 的 base64url 形".to_string())?;
    Ok(ed25519_dalek::SigningKey::from_bytes(&seed))
}

fn default_key_file() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("ark")
        .join("ledger.key")
}

/// 签名一次写入（五头齐出）：返回 (headers, body_text)。
///
/// # Errors
/// 返回 Err（人读原因串）当：私钥装载或签名失败。
pub fn signed_post_parts(
    pathname: &str,
    body: &Value,
) -> Result<(Vec<(String, String)>, String), String> {
    let key = load_signing_key()?;
    let body_text = serde_json::to_string(body).map_err(|e| format!("body 序列化失败: {e}"))?;
    let ts = now_secs().to_string();
    let nonce = new_nonce();
    let idem = new_idempotency_key();
    let base = signature_base(
        "POST",
        pathname,
        &ts,
        &nonce,
        &idem,
        &body_sha256_hex(&body_text),
    );
    let sig = key.sign(base.as_bytes()).to_bytes();
    let headers = vec![
        ("Idempotency-Key".to_string(), idem),
        ("X-Key-Id".to_string(), KEY_ID.to_string()),
        ("X-Timestamp".to_string(), ts),
        ("X-Nonce".to_string(), nonce),
        ("X-Signature".to_string(), B64URL.encode(sig)),
    ];
    Ok((headers, body_text))
}

/// 读面 GET（无签名无配额）：回执 JSON。
///
/// # Errors
/// 返回 Err（人读原因串）当：网络失败或非 2xx。
pub fn ledger_get(agent: &ureq::Agent, url: &str) -> Result<Value, String> {
    let resp = agent
        .get(url)
        .call()
        .map_err(|e| format!("ledger 请求失败: {url}: {e}"))?;
    let status = resp.status();
    let text = resp
        .into_string()
        .map_err(|e| format!("读 ledger 响应失败: {e}"))?;
    if !(200..300).contains(&status) {
        return Err(format!("ledger {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("ledger 回执解析失败: {e}"))
}

/// 写面 POST（五头签名道）：回执 JSON 与状态码（201 为写入，409 同键异内容，429 配额）。
///
/// # Errors
/// 返回 Err（人读原因串）当：网络失败或非 2xx。
pub fn ledger_post(agent: &ureq::Agent, pathname: &str, body: &Value) -> Result<Value, String> {
    let url = format!("{LEDGER_BASE}{pathname}");
    let (headers, body_text) = signed_post_parts(pathname, body)?;
    let mut req = agent.post(&url).set("content-type", "application/json");
    for (k, v) in &headers {
        req = req.set(k, v);
    }
    let resp = req
        .send_string(&body_text)
        .map_err(|e| format!("ledger 写入失败: {url}: {e}"))?;
    let status = resp.status();
    let text = resp
        .into_string()
        .map_err(|e| format!("读 ledger 回执失败: {e}"))?;
    if status == 409 {
        return Err(format!(
            "ledger 409 同幂等键携不同内容（须换键重发）: {text}"
        ));
    }
    if status == 429 {
        return Err(format!(
            "ledger 429 写入配额已满（per-key 50/UTC 日）: {text}"
        ));
    }
    if !(200..300).contains(&status) {
        return Err(format!("ledger {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("ledger 回执解析失败: {e}"))
}

/// issue 开单：POST /repos/<repo>/issues {title, kind, acceptance, body?}。
///
/// # Errors
/// 返回 Err（人读原因串）当：校验不过或写入失败（服务端错误串透传归因）。
pub fn issue_new(
    agent: &ureq::Agent,
    title: &str,
    kind: &str,
    acceptance: &str,
    body: Option<&str>,
) -> Result<(i64, Value), String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err("title 必填且至多 200 字符".to_string());
    }
    if !ISSUE_KINDS.contains(&kind) {
        return Err("kind 仅 bug(BUG 错误任务)或 improvement(改进优化任务)".to_string());
    }
    let mut payload = json!({ "title": title, "kind": kind, "acceptance": acceptance });
    if let Some(b) = body {
        payload["body"] = json!(b);
    }
    let resp = ledger_post(agent, &format!("/repos/{REPO_ID}/issues"), &payload)?;
    let n = resp
        .get("issue")
        .and_then(Value::as_i64)
        .ok_or_else(|| "开单回执不识别（期望 201 {ok,issue,event}）".to_string())?;
    Ok((n, resp))
}

/// issue 事件：POST /repos/<repo>/issues/<n>/events {type, payload{...}, body?}。
///
/// # Errors
/// 返回 Err（人读原因串）当：校验不过或写入失败（关单 done 须先有 result，服务端 400 透传）。
pub fn issue_event(
    agent: &ureq::Agent,
    n: i64,
    event_type: &str,
    payload: Value,
    body: Option<&str>,
) -> Result<Value, String> {
    if !ISSUE_EVENT_TYPES.contains(&event_type) {
        return Err(format!("type 仅 {}", ISSUE_EVENT_TYPES.join("|")));
    }
    if event_type == "status" {
        let to = payload.get("to").and_then(Value::as_str).unwrap_or("");
        if !ISSUE_STATUSES.contains(&to) {
            return Err(format!("to 仅 {}", ISSUE_STATUSES.join("|")));
        }
    }
    let mut req = json!({ "type": event_type, "payload": payload });
    if let Some(b) = body {
        req["body"] = json!(b);
    }
    ledger_post(agent, &format!("/repos/{REPO_ID}/issues/{n}/events"), &req)
}

/// issue 关单链：先 result（引用 digest 或 artifact_id）再 status done（两写两幂等键；
/// 服务端关单校验 result 在先，400 透传）。
///
/// # Errors
/// 返回 Err（人读原因串）当：任一写入失败（第二写失败时第一写已入账，如实报半链态）。
pub fn issue_close(
    agent: &ureq::Agent,
    n: i64,
    digest: &str,
    note: Option<&str>,
) -> Result<(Value, Value), String> {
    if !valid_digest(digest) {
        return Err("digest 须 sha256:<64hex>（小写；关单 result 引产物或正文哈希）".to_string());
    }
    let mut payload = json!({ "digest": digest });
    if let Some(a) = digest.strip_prefix("sha256:") {
        payload["artifact_hint"] = json!(format!("sha256:{a}"));
    }
    let result = issue_event(agent, n, "result", payload.clone(), note)?;
    let status = match issue_event(agent, n, "status", json!({ "to": "done" }), note) {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "result 已入账但关单 status 失败（半链态：补跑 status done 或管理面直关）: {e}"
            ))
        }
    };
    Ok((result, status))
}

/// issue 列表（家族翻页形）：GET /repos/<repo>/issues?limit=&before=；
/// 回执 {ok,count,issues[{issue_n,title,status,kind,assignee,hasResult}],has_more?}。
///
/// # Errors
/// 返回 Err（人读原因串）当：网络失败或回执形不符。
pub fn issue_list(
    agent: &ureq::Agent,
    limit: u32,
    before: Option<i64>,
) -> Result<(Vec<Value>, Option<bool>), String> {
    let clamped = limit.clamp(1, LIST_LIMIT_MAX);
    // 恒带 more=1 索取 has_more（服务端 before 或 more=1 才回该字段；首页也要翻页信号）
    let mut url = format!("{LEDGER_BASE}/repos/{REPO_ID}/issues?limit={clamped}&more=1");
    if let Some(b) = before {
        url.push_str(&format!("&before={b}"));
    }
    let v = ledger_get(agent, &url)?;
    let issues = v
        .get("issues")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "列表回执不识别（期望 {ok,count,issues[]}）".to_string())?;
    let has_more = v.get("has_more").and_then(Value::as_bool);
    Ok((issues, has_more))
}

/// 饱和判定（家族标准 #52 同口径，纯函数）：返回条数恰打满夹取后 limit。
pub fn list_saturated(returned: usize, limit: u32) -> bool {
    returned == limit.clamp(1, LIST_LIMIT_MAX) as usize
}

/// issue 详情：GET /repos/<repo>/issues/<n>（projection 加 timeline）。
///
/// # Errors
/// 返回 Err（人读原因串）当：网络失败或 404（issue 未找到）。
pub fn issue_show(agent: &ureq::Agent, n: i64) -> Result<Value, String> {
    ledger_get(agent, &format!("{LEDGER_BASE}/repos/{REPO_ID}/issues/{n}"))
}

/// artifact 发布：POST /repos/<repo>/artifacts {name,kind,digest,...}。
///
/// # Errors
/// 返回 Err（人读原因串）当：kind/digest/name 校验不过或写入失败（同 digest 409 透传）。
pub fn artifact_publish(
    agent: &ureq::Agent,
    name: &str,
    kind: &str,
    digest: &str,
    extras: &Value,
) -> Result<(String, Value), String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 200 {
        return Err("name 必填且至多 200 字符".to_string());
    }
    if !ARTIFACT_KINDS.contains(&kind) {
        return Err(format!("kind 仅 {}", ARTIFACT_KINDS.join("|")));
    }
    if !valid_digest(digest) {
        return Err(
            "digest 须 sha256:<64hex>（一律正文或记录哈希为身份；库不收二进制实体）".to_string(),
        );
    }
    let mut payload = json!({ "name": name, "kind": kind, "digest": digest });
    if let (Some(k), Some(v)) = (extras.as_object(), payload.as_object_mut()) {
        for (key, val) in k {
            v.insert(key.clone(), val.clone());
        }
    }
    let resp = ledger_post(agent, &format!("/repos/{REPO_ID}/artifacts"), &payload)?;
    let id = resp
        .get("artifact_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "发布回执不识别（期望 201 {ok,artifact_id,digest,event}）".to_string())?
        .to_string();
    Ok((id, resp))
}

/// artifact 证明：POST /repos/<repo>/artifacts/<id>/attestations {type,...}。
///
/// # Errors
/// 返回 Err（人读原因串）当：type 校验不过或写入失败。
pub fn artifact_attest(
    agent: &ureq::Agent,
    artifact_id: &str,
    attest_type: &str,
    extras: &Value,
) -> Result<Value, String> {
    if !ATTEST_TYPES.contains(&attest_type) {
        return Err(format!("type 仅 {}", ATTEST_TYPES.join("|")));
    }
    let mut payload = json!({ "type": attest_type });
    if let Some(obj) = extras.as_object() {
        for (k, v) in obj {
            payload[k] = v.clone();
        }
    }
    ledger_post(
        agent,
        &format!("/repos/{REPO_ID}/artifacts/{artifact_id}/attestations"),
        &payload,
    )
}

/// artifact 列表：GET /repos/<repo>/artifacts?current=&env=&kind=&name=。
///
/// # Errors
/// 返回 Err（人读原因串）当：网络失败或回执形不符。
pub fn artifact_list(
    agent: &ureq::Agent,
    current: bool,
    env_filter: Option<&str>,
    kind: Option<&str>,
    name: Option<&str>,
) -> Result<Vec<Value>, String> {
    let mut url = format!("{LEDGER_BASE}/repos/{REPO_ID}/artifacts?");
    let mut sep = "";
    if current {
        url.push_str("current=1");
        sep = "&";
    }
    if let Some(e) = env_filter {
        url.push_str(sep);
        url.push_str(&format!("env={e}"));
        sep = "&";
    }
    if let Some(k) = kind {
        url.push_str(sep);
        url.push_str(&format!("kind={k}"));
        sep = "&";
    }
    if let Some(n) = name {
        url.push_str(sep);
        url.push_str(&format!("name={n}"));
    }
    let v = ledger_get(agent, &url)?;
    v.get("artifacts")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "artifact 列表回执不识别（期望 {ok,count,artifacts[]}）".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 签名基构造（服务端 signatureBase 同构，逐字段逐换行）。
    #[test]
    fn 签名基_逐字段构造() {
        let base = signature_base(
            "POST",
            "/repos/github.com/raystyle/ark_rs/issues",
            "1700000000",
            "nonce-abc",
            "idem-xyz",
            "aa11",
        );
        assert_eq!(
            base,
            "v1\nPOST\n/repos/github.com/raystyle/ark_rs/issues\n1700000000\nnonce-abc\nidem-xyz\naa11"
        );
    }

    /// kid 派生：内置 JWK 派生值与常量 KEY_ID 逐字等（注册面单一真源自证）。
    #[test]
    fn kid派生_与内置常量等() {
        assert_eq!(kid_of_jwk(PUBLIC_JWK), KEY_ID);
        // 键序无关（规范化）：乱序 JSON 同 kid
        let shuffled =
            r#"{"x":"pb0-690h-Hbavq7bgJQ9hdtbACIM9ndkgS74GzbxNe4","kty":"OKP","crv":"Ed25519"}"#;
        assert_eq!(kid_of_jwk(shuffled), KEY_ID);
    }

    /// digest 形校验边界：形对、大写拒、短 hex 拒、无前缀拒。
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
        assert!(!valid_digest(""));
    }

    /// 幂等键语义：每次写入必新键（uuid v4 唯一性）。
    #[test]
    fn 幂等键_每次新键() {
        let a = new_idempotency_key();
        let b = new_idempotency_key();
        assert_ne!(a, b, "同键异内容会 409，每次写入必换键");
        assert!(uuid::Uuid::parse_str(&a).is_ok(), "uuid v4 形");
    }

    /// kind 面校验：issue 两值、artifact 十六值、证明六值（与服务端 Set 同口径）。
    #[test]
    fn kind面_与服务端同口径() {
        assert!(ISSUE_KINDS.contains(&"bug"));
        assert!(ISSUE_KINDS.contains(&"improvement"));
        assert!(!ISSUE_KINDS.contains(&"task"));
        assert_eq!(ARTIFACT_KINDS.len(), 15);
        for k in [
            "lesson",
            "experience",
            "research",
            "prototype",
            "binary",
            "sbom",
        ] {
            assert!(ARTIFACT_KINDS.contains(&k), "{k} 应在册");
        }
        assert!(!ARTIFACT_KINDS.contains(&"doc"));
        assert_eq!(ATTEST_TYPES.len(), 6);
        assert!(ATTEST_TYPES.contains(&"attest_dev"));
        assert!(!ATTEST_TYPES.contains(&"attest"));
    }

    /// 家族标准饱和判定同口径（#52）：恰打满夹取界即饱和。
    #[test]
    fn 饱和判定_同口径() {
        assert!(list_saturated(100, 100));
        assert!(list_saturated(100, 500));
        assert!(!list_saturated(3, 100));
    }

    /// body sha256 小写 hex（签名基第七行材料）。
    #[test]
    fn body哈希_小写hex() {
        assert_eq!(
            body_sha256_hex("{}"),
            "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
    }
}
