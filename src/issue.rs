//! issue 域（REQ-0009，对齐 ohmycloud REQ-057 契约）：自研命令仓统一 issue 入口
//! issues.ohmygh.com。new 一键提交自动带 tool=ark 与版本/平台/host；list/show 读面。
//! 契约文档真源 = ohmycloud docs/requirements/REQ-057 契约节（omc 客户端与 Worker
//! 服务端共 src/lib/issuecontract.ts 单源；本模块为其 Rust 面等价实现，校验语义对齐）。
//!
//! 网络面统一 ureq（仓内既有依赖，零新增 crate）：提交 POST、列表与详情 GET；
//! 基址 env `ARK_ISSUES_API` 可覆盖（测与灰度，对齐 omc 的 OMC_ISSUES_API）。

use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;
use serde::{Deserialize, Serialize};

/// 默认 HTTP 超时毫秒（对齐 omc 参考实现 20s）。
pub const TIMEOUT_MS: u64 = 20_000;
/// 契约上限：title trim 后至多 200（REQ-057）。
pub const TITLE_MAX: usize = 200;
/// 契约上限：body 至多 20000（REQ-057）。
pub const BODY_MAX: usize = 20_000;
/// 统一入口域（fleet 共用 issue 入口，REQ-057）。
pub const ISSUES_DOMAIN: &str = "issues.ohmygh.com";

/// API 基址：env `ARK_ISSUES_API` 覆盖供测与灰度，缺省统一入口域。
pub fn issues_api_base() -> String {
    std::env::var("ARK_ISSUES_API").unwrap_or_else(|_| format!("https://{ISSUES_DOMAIN}"))
}

/// 提交体六字段（POST /api/issues 的 JSON 体；回执 201 形 `{ok,id,url}`）。
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct IssueFields {
    /// 工具名（形 `^[a-z][a-z0-9_-]{0,31}$`；缺省 ark）
    pub tool: String,
    /// 标题（trim 后 1 至 200）
    pub title: String,
    /// 正文（至多 20000）
    pub body: String,
    /// 版本（客户端截 40）
    pub version: String,
    /// 平台串（形 `linux/x86_64`，截 64）
    pub platform: String,
    /// 主机名（截 64）
    pub host: String,
}

/// tool 形 `^[a-z][a-z0-9_-]{0,31}$`（契约共形；OnceLock 预编译一次）。
fn tool_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-z][a-z0-9_-]{0,31}$").expect("tool 形正则恒合法"))
}

/// UTF-16 单元计数（与服务端 JS `.length` 同语义；星平面字符（emoji）计 2，
/// 客户端先行拒绝防「放行后服务端 400」，codex 评审 G2）。
fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// 按 UTF-16 单元截断（与服务端 JS `.slice` 同语义；与 utf16_len 计数口径配套，
/// codex 二轮残余收尾，避免计数与截断混用两套语义）。
fn clip_utf16(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut units = 0usize;
    for ch in s.chars() {
        let u = ch.len_utf16();
        if units + u > max {
            break;
        }
        out.push(ch);
        units += u;
    }
    out
}

/// 提交体校验（tool 与 title 规则拒绝；version/platform/host 客户端先截断）。
/// 与 REQ-057 的 validateIssue 语义对齐：拒绝项带人读原因，截断项静默收敛。
///
/// # Errors
/// 返回 Err（人读原因串）当：tool 形不符、title 空或超 200、body 超 20000。
pub fn validate_issue(
    tool: &str,
    title: &str,
    body: &str,
    version: &str,
    platform: &str,
    host: &str,
) -> Result<IssueFields, String> {
    let tool = tool.trim();
    if !tool_re().is_match(tool) {
        return Err(format!(
            "tool 形如 ^[a-z][a-z0-9_-]{{0,31}}$（收到 {tool:?}）"
        ));
    }
    let title = title.trim();
    if title.is_empty() {
        return Err("title 不能为空".to_string());
    }
    if utf16_len(title) > TITLE_MAX {
        return Err(format!("title 超长（至多 {TITLE_MAX}）"));
    }
    if utf16_len(body) > BODY_MAX {
        return Err(format!("body 超长（至多 {BODY_MAX}）"));
    }
    let clip = |s: &str, max: usize| clip_utf16(s.trim(), max);
    Ok(IssueFields {
        tool: tool.to_string(),
        title: title.to_string(),
        body: body.to_string(),
        version: clip(version, 40),
        platform: clip(platform, 64),
        host: clip(host, 64),
    })
}

/// 自身版本（Cargo 包版本单一来源）。
pub fn self_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 平台串形 `linux/x86_64`（os/arch 取编译目标常量，运行态恒定）。
pub fn self_platform() -> String {
    format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// 主机名：env `HOSTNAME`（交互 shell 常在但多不导出）与 `COMPUTERNAME`（win）
/// 之后兜底 `hostname` 子命令（agent 非交互场景实测 env 双缺，2026-09-17 实弹 #7
/// host=unknown 教训），仍缺才 `unknown`（截断归 validate_issue 客户端收敛）。
pub fn self_host() -> String {
    if let Ok(h) = std::env::var("HOSTNAME") {
        if !h.trim().is_empty() {
            return h;
        }
    }
    if let Ok(h) = std::env::var("COMPUTERNAME") {
        if !h.trim().is_empty() {
            return h;
        }
    }
    if let Ok(out) = std::process::Command::new("hostname").output() {
        if out.status.success() {
            let h = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !h.is_empty() {
                return h;
            }
        }
    }
    "unknown".to_string()
}

/// HTTP 客户端（超时毫秒；ureq Agent 复用连接池）。`timeout_ms == 0` 不设整体
/// 超时（ureq 缺省形，即「不限时」；注意 ureq 2 的 `Duration::ZERO` 是即超时而非
/// 不限时，codex 二轮 F 实测 0.003 秒红）。
pub fn http_client(timeout_ms: u64) -> ureq::Agent {
    let builder = ureq::AgentBuilder::new();
    if timeout_ms == 0 {
        return builder.build();
    }
    builder.timeout(Duration::from_millis(timeout_ms)).build()
}

/// 列表行（list 与 show 共用面；created_at 为服务端 UTC ISO 形）。
#[derive(Deserialize, Debug)]
pub struct IssueRow {
    /// issue 编号（详情页路径 `/i/<id>`）
    pub id: i64,
    /// 工具名（服务端老行缺字段容错空串，codex 评审 G4）
    #[serde(default)]
    pub tool: String,
    /// 标题
    #[serde(default)]
    pub title: String,
    /// 状态（open|closed）
    #[serde(default)]
    pub status: String,
    /// 提交时的版本
    #[serde(default)]
    pub version: String,
    /// 提交时的平台串
    #[serde(default)]
    pub platform: String,
    /// 提交时的主机名
    #[serde(default)]
    pub host: String,
    /// 创建时间（UTC ISO 形）
    #[serde(default)]
    pub created_at: String,
}

/// 详情（列表行加正文）。
#[derive(Deserialize, Debug)]
pub struct IssueFull {
    /// 列表行字段（服务端 issue 对象扁平共形，serde flatten 收敛）
    #[serde(flatten)]
    pub row: IssueRow,
    /// 正文全文
    pub body: String,
}

/// ureq 2 缺省把 4xx/5xx 当 `Err(Error::Status(code, resp))` 且无旗标可关（error.rs
/// 文档形）：此处把 Status 还原为响应交 read_json 分诊（保住服务端 error 文案与
/// HTTP 码，codex 评审 F1——否则 429 限速会被误报「服务不可达」），其余 Err 才算
/// 传输面不可达。
fn unwrap_http(
    r: Result<ureq::Response, ureq::Error>,
    what: &str,
) -> Result<ureq::Response, String> {
    match r {
        Ok(resp) => Ok(resp),
        Err(ureq::Error::Status(_, resp)) => Ok(resp),
        Err(e) => Err(format!("{what}：issues 服务不可达（{e}）")),
    }
}

/// 读响应体并按服务端错误形（`{error}`）转人读原因；HTTP 非 2xx 一律拒绝。
fn read_json(resp: ureq::Response, what: &str) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let text = resp
        .into_string()
        .map_err(|e| format!("{what}：读回执失败：{e}"))?;
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    if (200..300).contains(&status) {
        return Ok(v);
    }
    let err = v
        .get("error")
        .and_then(|e| e.as_str())
        .unwrap_or("回执不识别");
    Err(format!("{what}（HTTP {status}）：{err}"))
}

/// 提交：POST /api/issues；201 取 `{ok,id,url}`（429 限速与 400 校验不过带服务端文案）。
///
/// # Errors
/// 返回 Err 当：服务不可达、序列化失败、HTTP 非 2xx（带服务端 error 文案）、回执不识别。
pub fn post_issue(
    agent: &ureq::Agent,
    base: &str,
    f: &IssueFields,
) -> Result<(i64, String), String> {
    let body = serde_json::to_string(f).map_err(|e| format!("序列化提交体失败：{e}"))?;
    let resp = unwrap_http(
        agent
            .post(&format!("{base}/api/issues"))
            .set("content-type", "application/json")
            .send_string(&body),
        "提交失败",
    )?;
    let v = read_json(resp, "提交失败")?;
    let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
    match (
        ok,
        v.get("id").and_then(|x| x.as_i64()),
        v.get("url").and_then(|x| x.as_str()),
    ) {
        (true, Some(id), Some(url)) => Ok((id, url.to_string())),
        _ => Err("提交失败：回执不识别（期望 201 {ok,id,url}）".to_string()),
    }
}

/// 列表 URL 拼接（纯函数）：limit 夹取 1 至 100（与服务端 Math.min(100, Math.max(1, …))
/// 同口径，codex 评审 G4）；before 在位追加 keyset 游标参数（家族统一标准 #53）。
pub fn build_list_url(
    base: &str,
    tool: Option<&str>,
    status: Option<&str>,
    limit: u32,
    before: Option<i64>,
) -> String {
    let clamped = limit.clamp(1, 100);
    let mut url = format!("{base}/api/issues?limit={clamped}");
    if let Some(t) = tool {
        url.push_str(&format!("&tool={}", urlencode(t)));
    }
    if let Some(s) = status {
        url.push_str(&format!("&status={}", urlencode(s)));
    }
    if let Some(b) = before {
        url.push_str(&format!("&before={b}"));
    }
    url
}

/// 饱和判定（纯函数，家族统一标准 #52）：返回条数恰打满夹取后 limit 即饱和
///（旧条目可能仍被截断；count 只报本次返回数非在册总数）。
pub fn list_saturated(returned: usize, limit: u32) -> bool {
    returned == limit.clamp(1, 100) as usize
}

/// 列表：GET /api/issues?tool=&status=&limit=&before=（新到旧；limit 1 至 100 由
/// 服务端封顶；`before` 是 keyset 游标（#53 家族统一标准），取该 id 之前更旧的一页，
/// 非法值服务端回 400；不带 before 的旧形请求回执不变）。
///
/// # Errors
/// 返回 Err 当：服务不可达、HTTP 非 2xx（含 before 非法 400）、回执形不符或行解析失败。
pub fn list_issues(
    agent: &ureq::Agent,
    base: &str,
    tool: Option<&str>,
    status: Option<&str>,
    limit: u32,
    before: Option<i64>,
) -> Result<Vec<IssueRow>, String> {
    let url = build_list_url(base, tool, status, limit, before);
    let resp = unwrap_http(agent.get(&url).call(), "列表失败")?;
    let v = read_json(resp, "列表失败")?;
    let rows = v
        .get("issues")
        .and_then(|x| x.as_array())
        .ok_or_else(|| "列表失败：回执不识别（期望 {ok,issues[]}）".to_string())?;
    serde_json::from_value(serde_json::Value::Array(rows.clone()))
        .map_err(|e| format!("列表失败：行解析失败：{e}"))
}

/// 详情：GET /api/issues/<id>（含正文）。
///
/// # Errors
/// 返回 Err 当：服务不可达、HTTP 非 2xx（含 id 不存在的 404）、回执形不符或解析失败。
pub fn show_issue(agent: &ureq::Agent, base: &str, id: i64) -> Result<IssueFull, String> {
    let resp = unwrap_http(
        agent.get(&format!("{base}/api/issues/{id}")).call(),
        "详情失败",
    )?;
    let v = read_json(resp, "详情失败")?;
    let issue = v
        .get("issue")
        .ok_or_else(|| "详情失败：回执不识别（期望 {ok,issue}）".to_string())?;
    serde_json::from_value(issue.clone()).map_err(|e| format!("详情失败：解析失败：{e}"))
}

/// 查询串百分号编码（tool/status 过滤值；仅保留字母数字与 `-_.~`）。
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 家族统一标准 #52/#53：URL 拼接 clamp 与 before 游标形、饱和判定边界。
    #[test]
    fn 列表url_clamp与before游标形() {
        let base = "https://issues.ohmygh.com";
        assert_eq!(
            build_list_url(base, None, None, 20, None),
            format!("{base}/api/issues?limit=20"),
            "旧形无 before 回执面不变"
        );
        assert_eq!(
            build_list_url(base, None, None, 500, None),
            format!("{base}/api/issues?limit=100"),
            "超界夹到服务端上限"
        );
        assert_eq!(
            build_list_url(base, None, None, 0, None),
            format!("{base}/api/issues?limit=1"),
            "零值夹到下界"
        );
        assert_eq!(
            build_list_url(base, Some("ark"), Some("open"), 3, Some(51)),
            format!("{base}/api/issues?limit=3&tool=ark&status=open&before=51"),
            "keyset 游标参数在位"
        );
    }

    #[test]
    fn 饱和判定_恰打满夹取界() {
        assert!(list_saturated(3, 3), "恰打满即饱和");
        assert!(list_saturated(100, 500), "打满夹取后上限即饱和");
        assert!(!list_saturated(2, 3), "未打满零提示");
        assert!(!list_saturated(0, 3), "空集不饱和");
    }

    #[test]
    fn 校验_tool形接受与拒绝() {
        assert!(validate_issue("ark", "t", "", "", "", "").is_ok());
        assert!(validate_issue("office-cli_2", "t", "", "", "", "").is_ok());
        for bad in ["Ark", "1ark", "ark x", "-ark", ""] {
            assert!(
                validate_issue(bad, "t", "", "", "", "").is_err(),
                "{bad} 应拒"
            );
        }
        // 33 字符超上限（1 加 32 恰 33）
        let over = format!("a{}", "b".repeat(32));
        assert!(validate_issue(&over, "t", "", "", "", "").is_err());
        // 32 字符恰上限（1 加 31）
        let edge = format!("a{}", "b".repeat(31));
        assert!(validate_issue(&edge, "t", "", "", "", "").is_ok());
    }

    #[test]
    fn 校验_title边界() {
        let t200: String = "字".repeat(TITLE_MAX);
        assert!(validate_issue("ark", &t200, "", "", "", "").is_ok());
        let t201: String = "字".repeat(TITLE_MAX + 1);
        assert!(validate_issue("ark", &t201, "", "", "", "").is_err());
        // UTF-16 语义（与服务端 JS .length 对齐）：emoji 星平面字符计 2
        let e100: String = "😀".repeat(100);
        assert!(validate_issue("ark", &e100, "", "", "", "").is_ok());
        let e101: String = "😀".repeat(101);
        assert!(validate_issue("ark", &e101, "", "", "", "").is_err());
        // trim 后空拒
        assert!(validate_issue("ark", "   ", "", "", "", "").is_err());
        // trim 收敛后过
        assert!(validate_issue("ark", "  标题  ", "", "", "", "").is_ok());
    }

    #[test]
    fn 截断_utf16单元语义() {
        // 星平面字符计 2 单元：40 个 emoji 截 64 即 32 个整字符，不劈代理对
        let clipped = clip_utf16(&"😀".repeat(40), 64);
        assert_eq!(utf16_len(&clipped), 64);
        assert_eq!(clipped.chars().count(), 32);
        assert!(utf16_len(&clip_utf16(&"v".repeat(60), 40)) <= 40);
    }

    #[test]
    fn 校验_body边界与截断() {
        let b: String = "x".repeat(BODY_MAX);
        assert!(validate_issue("ark", "t", &b, "", "", "").is_ok());
        let over: String = "x".repeat(BODY_MAX + 1);
        assert!(validate_issue("ark", "t", &over, "", "", "").is_err());
        // version 40 / platform 与 host 64 客户端截断（非拒绝）
        let f = validate_issue(
            "ark",
            "t",
            "",
            &"v".repeat(60),
            &"p".repeat(100),
            &"h".repeat(100),
        )
        .unwrap();
        assert_eq!(f.version.len(), 40);
        assert_eq!(f.platform.len(), 64);
        assert_eq!(f.host.len(), 64);
    }

    #[test]
    fn 平台串与版本非空() {
        assert!(self_platform().contains('/'));
        assert!(!self_version().is_empty());
    }

    #[test]
    fn 查询编码_保留面与转义面() {
        assert_eq!(urlencode("ark"), "ark");
        assert_eq!(urlencode("a b&c"), "a%20b%26c");
        assert_eq!(urlencode("中"), "%E4%B8%AD");
    }

    #[test]
    fn 基址_环境覆盖与缺省() {
        // 无 env 时缺省统一入口域
        std::env::remove_var("ARK_ISSUES_API");
        assert_eq!(issues_api_base(), format!("https://{ISSUES_DOMAIN}"));
    }
}
