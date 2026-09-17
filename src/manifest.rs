//! manifest.toml：安装配置部署逻辑的数据面（R016 B 层，D39 第一波引擎）。
//!
//! 结构：`schema_version` 加每工具一节 `[manifest.<tool>]`，含 L1 声明原语
//! （`env_set` 用户级键值表、`shims` 别名表、`mirror` 镜像源节 D42）与 L2 受控命令
//! （`post_install` 分平台 argv 数组——每条是参数数组非 shell 字符串，无元字符解释）。
//!
//! 生命周期：与 tools.toml 同目录（catalog sync 顺带拉取三件套，同锚同签）；
//! 文件或工具节缺失时零原语、零动作（内建双轨已于 2026-09-11 撤除，omc 数据面为唯一来源）。
//! 高 `schema_version` 拒载并提示升级 ome（R016 前进兼容红线）。
//! mirror 节不升 schema（serde 容忍未知字段，旧引擎静默忽略零动作，前向兼容同红线）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// 引擎支持的 manifest schema 大版本（R016 v0.2 定稿）。
pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

/// manifest.toml 根结构。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ManifestFile {
    /// manifest schema 版本（拒载面）
    pub schema_version: Option<u32>,
    /// 工具名到安装逻辑节的映射
    pub manifest: HashMap<String, ToolManifest>,
}

/// 单工具节：L1 声明原语加 L2 受控命令（未识别字段由 serde default 容忍，lint 白名单把关）。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ToolManifest {
    /// L1：用户级环境变量键值表（win=注册表 HKCU、POSIX=profile 标记块，platform 通道）。
    pub env_set: Option<HashMap<String, String>>,
    /// L1：别名表（键=别名名、值=同 bin 目录的源名；win=硬链接加 .cmd 兜底、POSIX=符号链接）。
    pub shims: Option<HashMap<String, String>>,
    /// L1：镜像源节（D42）。值全数据面声明，落点与合并语义引擎按类型实现（见 `Mirror`）。
    pub mirror: Option<Mirror>,
    /// L2：受控命令（分平台 argv 数组）。
    pub post_install: Option<PostInstall>,
}

/// L1 镜像源节（D42，全集对齐 ohmypwsh set-mirror.ps1 / P0017 五端统一口径）。
/// 键全可选、按需声明；幂等：各键内容一致零重写。
/// - `env`：用户级镜像变量（通道同 env_set：win=注册表、POSIX=profile env 块即 shell rc）；
/// - `env_unset`：旧通道变量撤除（防旧 env 值盖过文件配置，如 UV_INDEX_URL 盖 uv.toml）；
/// - `npm_registry`：`~/.npmrc` 行级 upsert `registry=<url>`（认证行等其他行原样保留）；
/// - `bunfig_registry`：`~/.bunfig.toml` 整文件（含本 URL 即不重写，heal_bunfig 同语义）；
/// - `uv_index`：uv.toml 整文件 `[[index]]` default（POSIX `~/.config/uv/`、win `%APPDATA%\uv\`）；
/// - `pip_index`：pip.conf（POSIX `~/.config/pip/pip.conf`、win `%APPDATA%\pip\pip.ini`）；
/// - `cargo_config`：CARGO_HOME 解析位 config.toml 整文件（rsproxy 全量形态等内容比对）；
/// - `goproxy`：go 代理语义键（D44 补位，值如 `https://goproxy.cn,direct`）：GOENV 文件
///   行级 upsert（win `%APPDATA%\go\env`、POSIX `~/.config/go/env`，即 `go env -w` 持久位，
///   直写不依赖 go 在位），配套 GOSUMDB=sum.golang.google.cn。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Mirror {
    /// 落 shell rc 的环境变量键值
    pub env: Option<HashMap<String, String>>,
    /// 需撤除的环境变量键
    pub env_unset: Option<Vec<String>>,
    /// npm registry 镜像源
    pub npm_registry: Option<String>,
    /// bunfig.toml registry 镜像源
    pub bunfig_registry: Option<String>,
    /// uv index 镜像源（uv.toml）
    pub uv_index: Option<String>,
    /// pip index-url 镜像源（pip.conf）
    pub pip_index: Option<String>,
    /// cargo sparse 镜像配置内容
    pub cargo_config: Option<String>,
    /// go 代理语义键（值如 `https://goproxy.cn,direct`）：行级 upsert 落 GOENV 文件
    /// （win `%APPDATA%\go\env`、POSIX `~/.config/go/env`，go 二进制无须在位），
    /// 配套 GOSUMDB=sum.golang.google.cn（goproxy.cn 生态的 sumdb 镜像，heal 先例同款）。
    pub goproxy: Option<String>,
}

impl Mirror {
    /// 全键空判定（lint：空节应省略）。
    pub fn is_empty_conf(&self) -> bool {
        self.env.as_ref().is_none_or(|m| m.is_empty())
            && self.env_unset.as_ref().is_none_or(|v| v.is_empty())
            && self.npm_registry.is_none()
            && self.bunfig_registry.is_none()
            && self.uv_index.is_none()
            && self.pip_index.is_none()
            && self.cargo_config.is_none()
            && self.goproxy.is_none()
    }
}

/// L2 受控命令：每平台一组 argv 数组；`skip` 显式声明「该平台无命令」（三键齐备 lint 依据）。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PostInstall {
    /// Windows 受控命令表
    pub win: Option<Vec<Vec<String>>>,
    /// Linux 受控命令表
    pub linux: Option<Vec<Vec<String>>>,
    /// macOS 受控命令表
    pub mac: Option<Vec<Vec<String>>>,
    /// 跳过命令名清单
    pub skip: Option<Vec<String>>,
}

/// L2 单条命令执行超时（R016 三节：300s 默认）。
const POST_INSTALL_TIMEOUT_SECS: u64 = 300;

/// 输出尾窗：滚动只留最近这么些字节（既抽干管道又不随输出无限吃内存；3 行足矣）。
const PIPE_TAIL_KEEP: usize = 64 * 1024;

/// 抽干线程回传尾窗的等待上限。子进程遗留的后台孙进程会一直持着写端不关，
/// 不能无限等（等不到就放弃尾行，不影响退出码与超时判定）。
const PIPE_DRAIN_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// 并发抽干一个管道并只回传尾窗：子进程输出超过管道缓冲（约 64KB）时写端会阻塞，
/// 若等它退出后再读，则子进程永不退出、轮询窗口耗尽而误判超时（M017 实证）。
fn drain_pipe<R: std::io::Read + Send + 'static>(mut r: R) -> std::sync::mpsc::Receiver<Vec<u8>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut keep: Vec<u8> = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match r.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    keep.extend_from_slice(&buf[..n]);
                    if keep.len() > PIPE_TAIL_KEEP {
                        let cut = keep.len() - PIPE_TAIL_KEEP;
                        keep.drain(..cut);
                    }
                }
            }
        }
        let _ = tx.send(keep);
    });
    rx
}

/// 尾窗取尾 3 行拼一行（失败报告用；非 UTF-8 按损耗转换）。
fn tail_lines(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" | ")
}

/// manifest.toml 路径：与 catalog（tools.toml）同目录（R016 两件分离同批落位）。
/// 唯一推导，sync 落位、status 诊断与 install 消费共用，避免三处各拼一次漂移。
pub fn path_for(catalog: &Path) -> PathBuf {
    match catalog.parent() {
        Some(dir) => dir.join("manifest.toml"),
        None => PathBuf::from("manifest.toml"),
    }
}

/// 载入 manifest.toml（传 catalog 路径，落位与消费同一推导）；缺文件返回空（零原语）；
/// 高 schema 版本拒载（报错由调用方传导）。
///
/// # Errors
/// 返回 Err（人读原因串）当：manifest schema_version {v} 高于引擎支持 {SUPPORTED_SCHEMA_VERSION}：请 ark self update 后重试 等（完整失败面见函数体错误构造）。
pub fn load(catalog: &Path) -> Result<ManifestFile, String> {
    let path = path_for(catalog);
    if !path.exists() {
        return Ok(ManifestFile::default());
    }
    parse(
        &std::fs::read_to_string(&path)
            .map_err(|e| format!("读 manifest 失败: {}: {e}", path.display()))?,
    )
}

/// 解析（纯函数可测；反序列化走 toml_edit serde feature，零新增依赖）。
///
/// # Errors
/// 返回 Err（人读原因串）当：manifest schema_version {v} 高于引擎支持 {SUPPORTED_SCHEMA_VERSION}：请 ark self update 后重试 等（完整失败面见函数体错误构造）。
pub fn parse(text: &str) -> Result<ManifestFile, String> {
    let f: ManifestFile =
        toml_edit::de::from_str(text).map_err(|e| format!("manifest.toml 解析失败: {e}"))?;
    match f.schema_version.unwrap_or(1) {
        v if v <= SUPPORTED_SCHEMA_VERSION => Ok(f),
        v => Err(format!(
            "manifest schema_version {v} 高于引擎支持 {SUPPORTED_SCHEMA_VERSION}：请 ark self update 后重试"
        )),
    }
}

/// 当前平台键选 L2 命令（纯函数可测）。
pub fn platform_commands(pi: &PostInstall) -> impl Iterator<Item = &Vec<String>> {
    let list = if cfg!(windows) {
        pi.win.as_ref()
    } else if cfg!(target_os = "macos") {
        pi.mac.as_ref()
    } else {
        pi.linux.as_ref()
    };
    list.into_iter().flatten()
}

/// 平台是否被显式跳过（三键齐备语义：有命令或进 skip 即齐备）。
pub fn platform_covered(pi: &PostInstall) -> bool {
    let (has, name) = if cfg!(windows) {
        (pi.win.is_some(), "win")
    } else if cfg!(target_os = "macos") {
        (pi.mac.is_some(), "mac")
    } else {
        (pi.linux.is_some(), "linux")
    };
    has || pi
        .skip
        .as_ref()
        .is_some_and(|s| s.iter().any(|p| p == name))
}

/// L1：应用用户级环境变量（逐键幂等）。键值先过形态校验（与 mirror 同一红线：
/// 本地面无签名门，换行/双引号值拼进 export 行即注入面，对线 R5 确认轮收口 env_set）。
///
/// # Errors
/// 返回 Err（人读原因串）当：env_set 键形态非法: {k} 等（完整失败面见函数体错误构造）。
pub fn apply_env_set(m: &ToolManifest) -> Result<(), String> {
    let Some(kv) = &m.env_set else { return Ok(()) };
    for (k, v) in kv {
        if !env_key_sane(k) {
            return Err(format!("env_set 键形态非法: {k}"));
        }
        if !env_value_sane(v) {
            return Err(format!("env_set 值含换行或双引号，拒绝落源: {k}"));
        }
    }
    for (k, v) in kv {
        crate::platform::set_user_env_var(k, v)?;
        eprintln!("[OK] manifest env_set 已设: {k}={v}（新终端生效）");
    }
    Ok(())
}

// ── L1 mirror 节落源（D42）：值数据面声明，落点与合并语义引擎按类型实现 ──

/// 环境写入面值形态校验（对线 R5，注入面；env_set 与 mirror 单值键共用）：值拒绝换行与
/// 双引号——TOML 转义可产出真实换行，拼进 npmrc/bunfig/uv.toml/bashrc/export 行即注入
/// 额外行（rc 注入即命令执行）；本地面（ARK_CATALOG 指目录）无签名门，写入前必须自拒。
/// lint 同规则复用（单一权威）。
pub fn env_value_sane(v: &str) -> bool {
    !v.contains(['\n', '\r', '"'])
}

/// 环境写入面键形态校验：标识符形态（防键里带 `=` 或元字符破坏 export 行）。lint 同规则复用。
pub fn env_key_sane(k: &str) -> bool {
    !k.is_empty()
        && k.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// L1：镜像源节落源（幂等）。env 先行（post_install 子进程继承，FNM_NODE_DIST_MIRROR
/// 对 fnm install 即时生效）；`env_unset` 进程面无条件先清（「只在当前 shell 导出」的旧变量
/// 不在持久面，不清则 post_install 子进程照样继承并盖过文件配置，对线 R2）；
/// 整面受测试隔离闸门（用户环境写入面第五面，M002 同型防漏）。
/// 失败语义同 L1 硬错：写不进就是没配上。
///
/// # Errors
/// 返回 Err（人读原因串）当：mirror env 键形态非法: {k} 等（完整失败面见函数体错误构造）。
pub fn apply_mirror(m: &ToolManifest, tool: &str, home: &Path) -> Result<(), String> {
    let Some(mir) = &m.mirror else { return Ok(()) };
    if crate::platform::user_env_write_blocked() {
        eprintln!("[INFO] ARK_TEST_NO_PATH_REG=1：跳过 {tool} mirror 落源（测试隔离）");
        return Ok(());
    }
    // 值与键形态先全量校验（硬错）：任一键不 sane 即拒整节，不做半套落源
    for (k, v) in mir.env.iter().flatten() {
        if !env_key_sane(k) {
            return Err(format!("mirror env 键形态非法: {k}"));
        }
        if !env_value_sane(v) {
            return Err(format!("mirror env 值含换行或双引号，拒绝落源: {k}"));
        }
    }
    for (what, v) in [
        ("npm_registry", mir.npm_registry.as_deref()),
        ("bunfig_registry", mir.bunfig_registry.as_deref()),
        ("uv_index", mir.uv_index.as_deref()),
        ("pip_index", mir.pip_index.as_deref()),
        ("goproxy", mir.goproxy.as_deref()),
    ] {
        if v.is_some_and(|v| !env_value_sane(v)) {
            return Err(format!("mirror {what} 值含换行或双引号，拒绝落源"));
        }
    }
    for k in mir.env_unset.iter().flatten() {
        if !env_key_sane(k) {
            return Err(format!("mirror env_unset 键形态非法: {k}"));
        }
    }
    for (k, v) in mir.env.iter().flatten() {
        let cur = crate::platform::get_user_env_var(k)?;
        if cur.as_deref() != Some(v.as_str()) {
            crate::platform::set_user_env_var(k, v)?;
            eprintln!("[OK] mirror env 已设: {k}={v}（新终端生效）");
        } else {
            eprintln!("[INFO] mirror env 已是: {k}={v}");
        }
    }
    for k in mir.env_unset.iter().flatten() {
        // 进程面无条件先清（对线 R2）：会话导出的旧变量不在持久面，但子进程会继承
        std::env::remove_var(k);
        if crate::platform::remove_user_env_var(k)? {
            eprintln!("[OK] mirror 旧 env 已撤: {k}");
        }
    }
    if let Some(url) = &mir.npm_registry {
        mirror_report(ensure_npmrc(home, url), "~/.npmrc registry")?;
    }
    if let Some(url) = &mir.bunfig_registry {
        mirror_report(ensure_bunfig(home, url), "~/.bunfig.toml registry")?;
    }
    if let Some(url) = &mir.uv_index {
        mirror_report(ensure_uv_toml(url), "uv.toml index")?;
    }
    if let Some(url) = &mir.pip_index {
        mirror_report(ensure_pip_conf(url), "pip.conf index-url")?;
    }
    if let Some(content) = &mir.cargo_config {
        mirror_report(ensure_cargo_config(content), "cargo config.toml")?;
    }
    if let Some(v) = &mir.goproxy {
        mirror_report(ensure_go_env(v), "go env GOPROXY/GOSUMDB")?;
    }
    Ok(())
}

/// mirror 落源报告（写/已目标态/失败三态统一出口）。
fn mirror_report(r: Result<bool, String>, what: &str) -> Result<(), String> {
    match r {
        Ok(true) => eprintln!("[OK] mirror 已写: {what}"),
        Ok(false) => eprintln!("[INFO] mirror 已是目标态: {what}"),
        Err(e) => return Err(format!("mirror 落源失败（{what}）: {e}")),
    }
    Ok(())
}

/// npmrc 的 registry 行 upsert（纯函数）：有 registry 行原位替换（键名大小写与空白容忍），
/// 无则追加文末；**其余行（认证 token、scope 配置等）逐字保留**——npmrc 常载凭据，整文件重写会毁数据。
pub fn npmrc_upsert(text: &str, url: &str) -> String {
    let line = format!("registry={url}");
    let is_registry = |l: &str| {
        l.split_once('=')
            .is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case("registry"))
    };
    let mut out: Vec<String> = Vec::new();
    let mut replaced = false;
    for l in text.lines() {
        if is_registry(l) {
            out.push(line.clone());
            replaced = true;
        } else {
            out.push(l.to_string());
        }
    }
    if !replaced {
        out.push(line);
    }
    let mut s = out.join("\n");
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// npm registry 落 `~/.npmrc`（UTF-8 无 BOM）。返回是否写入。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn ensure_npmrc(home: &Path, url: &str) -> Result<bool, String> {
    let p = home.join(".npmrc");
    let cur = std::fs::read_to_string(&p).unwrap_or_default();
    let want = npmrc_upsert(&cur, url);
    if want == cur {
        return Ok(false);
    }
    std::fs::write(&p, want).map_err(|e| format!("写 npmrc 失败: {}: {e}", p.display()))?;
    Ok(true)
}

/// bunfig registry 落 `~/.bunfig.toml`（整文件；含本 URL 即不重写，heal-mirror 同语义；
/// URL 比对去尾斜杠——npmmirror 两种写法等价不应来回重写）。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn ensure_bunfig(home: &Path, url: &str) -> Result<bool, String> {
    let p = home.join(".bunfig.toml");
    let want = format!("[install]\nregistry = \"{url}\"\n");
    let content = std::fs::read_to_string(&p).unwrap_or_default();
    let marker = url.trim_end_matches('/');
    if p.exists() && content.contains(marker) {
        return Ok(false);
    }
    std::fs::write(&p, want).map_err(|e| format!("写 bunfig.toml 失败: {}: {e}", p.display()))?;
    Ok(true)
}

/// uv.toml 目标内容（纯函数）：`[[index]]` default 形态（uv 0.4.23+ 数组表）。
pub fn uv_toml_content(url: &str) -> String {
    format!("[[index]]\nurl = \"{url}\"\ndefault = true\n")
}

/// uv index 落用户配置目录下 `uv/uv.toml`（POSIX `~/.config/uv/`、win `%APPDATA%\uv\`，
/// uv 各平台原生发现位；目录注入便于测）。返回是否写入。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn ensure_uv_toml_in(config_dir: &Path, url: &str) -> Result<bool, String> {
    let dir = config_dir.join("uv");
    std::fs::create_dir_all(&dir).map_err(|e| format!("建目录失败: {}: {e}", dir.display()))?;
    let p = dir.join("uv.toml");
    let want = uv_toml_content(url);
    let cur = std::fs::read_to_string(&p).unwrap_or_default();
    if cur == want {
        return Ok(false);
    }
    std::fs::write(&p, want).map_err(|e| format!("写 uv.toml 失败: {}: {e}", p.display()))?;
    Ok(true)
}

/// 生产入口（uv）：用户配置目录解析（dirs 同源，POSIX=XDG ~/.config、win=%APPDATA%）。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn ensure_uv_toml(url: &str) -> Result<bool, String> {
    let base = dirs::config_dir().ok_or("无法确定用户配置目录（uv.toml）")?;
    ensure_uv_toml_in(&base, url)
}

/// pip.conf 目标内容（纯函数）。
pub fn pip_conf_content(url: &str) -> String {
    format!("[global]\nindex-url = {url}\n")
}

/// pip 配置文件名（win=pip.ini、POSIX=pip.conf；pip 各平台原生发现位）。
pub fn pip_conf_name() -> &'static str {
    if cfg!(windows) {
        "pip.ini"
    } else {
        "pip.conf"
    }
}

/// pip index 落用户配置目录下 `pip/`（目录注入便于测）。返回是否写入。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn ensure_pip_conf_in(config_dir: &Path, url: &str) -> Result<bool, String> {
    let dir = config_dir.join("pip");
    std::fs::create_dir_all(&dir).map_err(|e| format!("建目录失败: {}: {e}", dir.display()))?;
    let p = dir.join(pip_conf_name());
    let want = pip_conf_content(url);
    let cur = std::fs::read_to_string(&p).unwrap_or_default();
    if cur == want {
        return Ok(false);
    }
    std::fs::write(&p, want).map_err(|e| format!("写 pip 配置失败: {}: {e}", p.display()))?;
    Ok(true)
}

/// 生产入口（pip）。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn ensure_pip_conf(url: &str) -> Result<bool, String> {
    let base = dirs::config_dir().ok_or("无法确定用户配置目录（pip）")?;
    ensure_pip_conf_in(&base, url)
}

/// CARGO_HOME 解析：进程环境变量 > 用户级变量（win 注册表 / POSIX profile 块，
/// ark Windows 接管模型重定位 EnvRoot 位由此生效）> 默认 `~/.cargo`。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn cargo_home_path() -> Result<PathBuf, String> {
    if let Some(v) = std::env::var_os("CARGO_HOME") {
        return Ok(PathBuf::from(v));
    }
    if let Ok(Some(v)) = crate::platform::get_user_env_var("CARGO_HOME") {
        let expanded = crate::platform::expand_env_vars(&v);
        return Ok(crate::platform::expand_install_path(&expanded));
    }
    dirs::home_dir()
        .map(|h| h.join(".cargo"))
        .ok_or_else(|| "无法确定用户主目录（cargo home）".to_string())
}

/// cargo 镜像配置落 CARGO_HOME/config.toml（整文件内容比对；rsproxy 全量形态等内容即零重写；
/// 目录注入便于测）。返回是否写入。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn ensure_cargo_config_in(cargo_home: &Path, content: &str) -> Result<bool, String> {
    std::fs::create_dir_all(cargo_home)
        .map_err(|e| format!("建目录失败: {}: {e}", cargo_home.display()))?;
    let p = cargo_home.join("config.toml");
    let cur = std::fs::read_to_string(&p).unwrap_or_default();
    if cur == content {
        return Ok(false);
    }
    std::fs::write(&p, content)
        .map_err(|e| format!("写 cargo config 失败: {}: {e}", p.display()))?;
    Ok(true)
}

/// 生产入口（cargo）。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn ensure_cargo_config(content: &str) -> Result<bool, String> {
    ensure_cargo_config_in(&cargo_home_path()?, content)
}

/// GOENV 文件的行级 upsert（纯函数）：GOPROXY 与 GOSUMDB 两键原位替换或追加，
/// 其余行（GOTOOLCHAIN 等用户配置）逐字保留——go env 文件常载用户键，整写会毁。
pub fn go_env_upsert(text: &str, goproxy: &str) -> String {
    let wants = [("GOPROXY", goproxy), ("GOSUMDB", "sum.golang.google.cn")];
    let mut out: Vec<String> = Vec::new();
    let mut seen = [false; 2];
    for l in text.lines() {
        let mut replaced = false;
        for (i, (k, v)) in wants.iter().enumerate() {
            if l.split_once('=').is_some_and(|(key, _)| key.trim() == *k) {
                out.push(format!("{k}={v}"));
                seen[i] = true;
                replaced = true;
                break;
            }
        }
        if !replaced {
            out.push(l.to_string());
        }
    }
    for (i, (k, v)) in wants.iter().enumerate() {
        if !seen[i] {
            out.push(format!("{k}={v}"));
        }
    }
    let mut s = out.join("\n");
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// go 代理落 GOENV 文件（win `%APPDATA%\go\env`、POSIX `~/.config/go/env`，
/// 即 `go env -w` 的持久位，直写文件不依赖 go 二进制在位；目录注入便于测）。
///
/// # Errors
/// 返回 Err（人读原因串）当：{tool} post_install 未覆盖当前平台（无命令且未声明 skip，R016 三键齐备） 等（完整失败面见函数体错误构造）。
pub fn ensure_go_env_in(config_dir: &Path, goproxy: &str) -> Result<bool, String> {
    ensure_go_env_file(&config_dir.join("go").join("env"), goproxy)
}

/// GOENV 文件直写（对线 G1）：path 即目标文件本身（GOENV 自定义值就是文件路径，
/// 不得再拼目录层级）；内容比对幂等。
///
/// # Errors
/// 返回 Err（人读原因串）当：{tool} post_install 未覆盖当前平台（无命令且未声明 skip，R016 三键齐备） 等（完整失败面见函数体错误构造）。
pub fn ensure_go_env_file(path: &Path, goproxy: &str) -> Result<bool, String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("建目录失败: {}: {e}", dir.display()))?;
    }
    let cur = std::fs::read_to_string(path).unwrap_or_default();
    let want = go_env_upsert(&cur, goproxy);
    if want == cur {
        return Ok(false);
    }
    std::fs::write(path, want).map_err(|e| format!("写 go env 失败: {}: {e}", path.display()))?;
    Ok(true)
}

/// 生产入口（go）。GOENV 尊重（对线 F7）：进程/用户级 `GOENV` 设 `off` 时跳过文件面
/// （不落默认位不误报已写）；设自定义路径时写该路径；缺省走 config_dir 平台位。
/// GOSUMDB 配套为引擎统一目标态（会覆盖用户自定义值，与 heal 同款权衡，R016 已注）。
///
/// # Errors
/// 返回 Err（人读原因串）当：{tool} post_install 未覆盖当前平台（无命令且未声明 skip，R016 三键齐备） 等（完整失败面见函数体错误构造）。
pub fn ensure_go_env(goproxy: &str) -> Result<bool, String> {
    let override_path = std::env::var("GOENV")
        .ok()
        .or_else(|| crate::platform::get_user_env_var("GOENV").ok().flatten());
    if let Some(v) = override_path {
        if v.eq_ignore_ascii_case("off") {
            eprintln!("[INFO] GOENV=off：go env 文件面跳过（GOPROXY/GOSUMDB 不写）");
            return Ok(false);
        }
        let p = crate::platform::expand_install_path(&crate::platform::expand_env_vars(&v));
        return ensure_go_env_file(&p, goproxy);
    }
    let base = dirs::config_dir().ok_or("无法确定用户配置目录（go env）")?;
    ensure_go_env_in(&base, goproxy)
}

/// win `.cmd` 兜底内容（纯函数可测）：`%~dp0` 相对定位（
/// 目录含空格时整体加引号即可，路径不落进内容、无转义面），纯 ASCII 无 BOM（cmd 不认 BOM）。
pub fn shim_cmd_content(source: &str) -> String {
    format!("@\"%~dp0{source}.exe\" %*\r\n")
}

/// L1：生成别名（win=硬链接加 `.cmd` 兜底、POSIX=符号链接；目标已存在即跳过，幂等）。
/// `bin_dir` 必须是**目标二进制所在目录**（源与别名同目录；调用方传 exe 父目录而非工具根）。
/// win 硬链接要求同卷同 NTFS：跨卷或 exFAT 等不支持时自动回落到 `.cmd`。
///
/// # Errors
/// 返回 Err（人读原因串）当：{tool} post_install 未覆盖当前平台（无命令且未声明 skip，R016 三键齐备） 等（完整失败面见函数体错误构造）。
pub fn apply_shims(m: &ToolManifest, bin_dir: &Path) -> Result<(), String> {
    let Some(shims) = &m.shims else { return Ok(()) };
    for (alias, source) in shims {
        let (src, dst) = if cfg!(windows) {
            (
                bin_dir.join(format!("{source}.exe")),
                bin_dir.join(format!("{alias}.exe")),
            )
        } else {
            (bin_dir.join(source), bin_dir.join(alias))
        };
        if !src.exists() {
            eprintln!("[WARN] manifest shims 源不存在，跳过: {}", src.display());
            continue;
        }
        if dst.exists() {
            eprintln!("[INFO] manifest shims 已存在，跳过: {}", dst.display());
            continue;
        }
        #[cfg(windows)]
        {
            if std::fs::hard_link(&src, &dst).is_ok() {
                eprintln!("[OK] manifest shim 已创建（硬链接）: {}", dst.display());
                continue;
            }
            let cmd = bin_dir.join(format!("{alias}.cmd"));
            std::fs::write(&cmd, shim_cmd_content(source))
                .map_err(|e| format!("写 {alias}.cmd 失败: {e}"))?;
            eprintln!("[OK] manifest shim 已创建（cmd 兜底）: {}", cmd.display());
        }
        #[cfg(not(windows))]
        {
            std::os::unix::fs::symlink(&src, &dst)
                .map_err(|e| format!("符号链接失败 {} -> {}: {e}", dst.display(), src.display()))?;
            eprintln!("[OK] manifest shim 已创建（符号链接）: {}", dst.display());
        }
    }
    Ok(())
}

/// L2：逐条执行受控命令；超时 300s 杀进程；失败只报不回滚，报告含退出码与输出尾行（R016 三节）。
///
/// # Errors
/// 返回 Err（人读原因串）当：{tool} post_install 未覆盖当前平台（无命令且未声明 skip，R016 三键齐备） 等（完整失败面见函数体错误构造）。
pub fn run_post_install(m: &ToolManifest, tool: &str) -> Result<(), String> {
    run_post_install_with_timeout(
        m,
        tool,
        std::time::Duration::from_secs(POST_INSTALL_TIMEOUT_SECS),
    )
}

/// 超时可注入版（单测跑杀进程路径不必等 300s；语义与公开入口一致）。
fn run_post_install_with_timeout(
    m: &ToolManifest,
    tool: &str,
    timeout: std::time::Duration,
) -> Result<(), String> {
    let Some(pi) = &m.post_install else {
        return Ok(());
    };
    if !platform_covered(pi) {
        return Err(format!(
            "{tool} post_install 未覆盖当前平台（无命令且未声明 skip，R016 三键齐备）"
        ));
    }
    for argv in platform_commands(pi) {
        if argv.is_empty() {
            return Err(format!("{tool} post_install 含空命令"));
        }
        eprintln!("[INFO] manifest post_install: {}", argv.join(" "));
        let mut child = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("{tool} post_install 启动失败（{}）: {e}", argv.join(" ")))?;
        // 两路管道并发抽干：不抽干则输出超管道缓冲的子进程写阻塞、永不退出（M017）
        let stdout_rx = child.stdout.take().map(drain_pipe);
        let stderr_rx = child.stderr.take().map(drain_pipe);
        let deadline = std::time::Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if std::time::Instant::now() >= deadline {
                        // 杀进程树必须在父进程还活着时做：Windows 的 kill 只杀直接子进程，
                        // 而 taskkill /T 靠「父进程在位」找树，父先死则报 not found 且孙进程
                        // 存活（M021 本机实证：kill 后 taskkill /T 无效，反过来则整棵清掉）。
                        #[cfg(windows)]
                        {
                            let pid = child.id().to_string();
                            let _ = std::process::Command::new("taskkill")
                                .args(["/PID", &pid, "/T", "/F"])
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .status();
                        }
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(format!(
                            "{tool} post_install 超时（{}s）已终止: {}",
                            timeout.as_secs(),
                            argv.join(" ")
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => return Err(format!("{tool} post_install 等待失败: {e}")),
            }
        };
        // 失败报告：退出码加输出尾行（R016 三节，omc 评审建议；尾窗在抽干线程里已备好）
        if !status.success() {
            let mut tail_s = String::new();
            if let Some(rx) = &stdout_rx {
                if let Ok(buf) = rx.recv_timeout(PIPE_DRAIN_WAIT) {
                    tail_s.push_str(&tail_lines(&buf));
                }
            }
            if let Some(rx) = &stderr_rx {
                let mut err_tail = String::new();
                if let Ok(buf) = rx.recv_timeout(PIPE_DRAIN_WAIT) {
                    err_tail.push_str(&tail_lines(&buf));
                }
                if !err_tail.is_empty() {
                    if !tail_s.is_empty() {
                        tail_s.push_str(" ; ");
                    }
                    tail_s.push_str(&err_tail);
                }
            }
            return Err(format!(
                "{tool} post_install 失败（{}）: 退出码 {:?}{}（R016：只报不回滚）",
                argv.join(" "),
                status.code(),
                if tail_s.is_empty() {
                    String::new()
                } else {
                    format!("，尾行: {tail_s}")
                }
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)] // 平台自适应用例按需补键，字面量反而更难读
mod tests {
    use super::*;

    #[test]
    fn 解析_工具节与三键() {
        let f = parse(
            "schema_version = 1\n[manifest.pwsh.env_set]\nPOWERSHELL_TELEMETRY_OPTOUT = \"1\"\n[manifest.bun]\n[manifest.bun.shims]\nbunx = \"bun\"\n[manifest.demo.post_install]\nwin = [[\"cmd\", \"/c\", \"echo hi\"]]\nskip = [\"linux\", \"mac\"]\n",
        )
        .expect("应解析");
        assert_eq!(f.manifest.len(), 3);
        assert_eq!(
            f.manifest["pwsh"].env_set.as_ref().unwrap()["POWERSHELL_TELEMETRY_OPTOUT"],
            "1"
        );
        assert_eq!(f.manifest["bun"].shims.as_ref().unwrap()["bunx"], "bun");
        assert!(f.manifest["demo"].post_install.is_some());
    }

    #[test]
    fn 高版本拒载() {
        let e = parse("schema_version = 2\n").expect_err("应拒载");
        assert!(e.contains("高于引擎支持"), "{e}");
    }

    #[test]
    fn mirror节_解析全键与空节判定() {
        let f = parse(
            "schema_version = 1\n[manifest.fnm.mirror]\nnpm_registry = \"https://registry.npmmirror.com\"\n[manifest.fnm.mirror.env]\nFNM_NODE_DIST_MIRROR = \"https://npmmirror.com/mirrors/node/\"\n[manifest.uv.mirror]\nuv_index = \"https://mirrors.tuna.tsinghua.edu.cn/pypi/web/simple/\"\npip_index = \"https://mirrors.tuna.tsinghua.edu.cn/pypi/web/simple/\"\nenv_unset = [\"UV_INDEX_URL\"]\n[manifest.rust.mirror]\n",
        )
        .expect("应解析");
        let fnm = &f.manifest["fnm"];
        let mir = fnm.mirror.as_ref().expect("mirror 节应在");
        assert_eq!(
            mir.npm_registry.as_deref(),
            Some("https://registry.npmmirror.com")
        );
        assert_eq!(
            mir.env.as_ref().unwrap()["FNM_NODE_DIST_MIRROR"],
            "https://npmmirror.com/mirrors/node/"
        );
        assert!(!mir.is_empty_conf(), "有键即非空");
        let uv = &f.manifest["uv"];
        let um = uv.mirror.as_ref().unwrap();
        assert!(um.uv_index.is_some() && um.pip_index.is_some());
        assert_eq!(
            um.env_unset.as_deref(),
            Some(["UV_INDEX_URL".to_string()].as_slice())
        );
        // cargo_config 空表存在但无值：全键空应省略（lint 依据）
        let rust_mir = f.manifest["rust"].mirror.as_ref().unwrap();
        assert!(rust_mir.is_empty_conf(), "仅空表无值应判空节");
        // 无 mirror 节的工具零动作
        assert!(!f.manifest.contains_key("nope"));
    }

    #[test]
    fn npmrc_upsert_行级替换与保留() {
        // 无 registry 行：追加文末，原有行不动
        let t1 = npmrc_upsert(
            "//registry.npmjs.org/:_authToken=secret\n",
            "https://registry.npmmirror.com",
        );
        assert!(t1.contains("registry=https://registry.npmmirror.com"));
        assert!(t1.contains("_authToken=secret"), "认证行必须保留");
        // 有 registry 行（带空白与大小写变体）：原位替换，不追加第二条
        let t2 = npmrc_upsert(
            "registry = https://registry.npmjs.org/\n_authToken=x\n",
            "https://registry.npmmirror.com",
        );
        assert_eq!(
            t2.lines()
                .filter(|l| l
                    .split_once('=')
                    .is_some_and(|(k, _)| k.trim() == "registry"))
                .count(),
            1,
            "只应有一条 registry 赋值行: {t2}"
        );
        assert!(t2.starts_with("registry=https://registry.npmmirror.com\n"));
        assert!(t2.contains("_authToken=x"));
        // 幂等：目标态再跑逐字不变
        let t3 = npmrc_upsert(&t1, "https://registry.npmmirror.com");
        assert_eq!(t3, t1);
        // 空文本：单行成文件
        assert_eq!(
            npmrc_upsert("", "https://registry.npmmirror.com"),
            "registry=https://registry.npmmirror.com\n"
        );
    }

    #[test]
    fn npmrc_落盘幂等() -> Result<(), String> {
        let home = tempfile::tempdir().map_err(|e| e.to_string())?;
        assert!(
            ensure_npmrc(home.path(), "https://registry.npmmirror.com")?,
            "首次应写"
        );
        let c1 = std::fs::read_to_string(home.path().join(".npmrc")).map_err(|e| e.to_string())?;
        assert_eq!(c1, "registry=https://registry.npmmirror.com\n");
        assert!(
            !ensure_npmrc(home.path(), "https://registry.npmmirror.com")?,
            "同值应跳过"
        );
        assert!(
            ensure_npmrc(home.path(), "https://registry.example.com")?,
            "换源应重写"
        );
        Ok(())
    }

    #[test]
    fn bunfig_尾斜杠等价与幂等() -> Result<(), String> {
        let home = tempfile::tempdir().map_err(|e| e.to_string())?;
        assert!(
            ensure_bunfig(home.path(), "https://registry.npmmirror.com/")?,
            "首次应写"
        );
        // 无尾斜杠同源 URL：不应来回重写（npmmirror 两写法等价）
        assert!(
            !ensure_bunfig(home.path(), "https://registry.npmmirror.com")?,
            "尾斜杠等价应跳过"
        );
        // 无镜像标记的旧文件重写（heal-mirror 同语义）
        std::fs::write(home.path().join(".bunfig.toml"), "# 用户自定义\n")
            .map_err(|e| e.to_string())?;
        assert!(ensure_bunfig(
            home.path(),
            "https://registry.npmmirror.com/"
        )?);
        Ok(())
    }

    #[test]
    fn uv与pip_落盘内容比对幂等() -> Result<(), String> {
        let cfg = tempfile::tempdir().map_err(|e| e.to_string())?;
        let tuna = "https://mirrors.tuna.tsinghua.edu.cn/pypi/web/simple/";
        assert!(ensure_uv_toml_in(cfg.path(), tuna)?, "uv 首次应写");
        let uv_toml = std::fs::read_to_string(cfg.path().join("uv").join("uv.toml"))
            .map_err(|e| e.to_string())?;
        assert!(uv_toml.contains("[[index]]"));
        assert!(uv_toml.contains(&format!("url = \"{tuna}\"")));
        assert!(uv_toml.contains("default = true"));
        assert!(!ensure_uv_toml_in(cfg.path(), tuna)?, "内容一致应跳过");
        assert!(ensure_pip_conf_in(cfg.path(), tuna)?, "pip 首次应写");
        let pip = std::fs::read_to_string(cfg.path().join("pip").join(pip_conf_name()))
            .map_err(|e| e.to_string())?;
        assert!(pip.contains(&format!("index-url = {tuna}")));
        assert!(!ensure_pip_conf_in(cfg.path(), tuna)?, "内容一致应跳过");
        Ok(())
    }

    #[test]
    fn cargo配置_落盘内容比对幂等() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let content = "[source.crates-io]\nreplace-with = \"rsproxy\"\n";
        assert!(ensure_cargo_config_in(dir.path(), content)?);
        assert!(
            !ensure_cargo_config_in(dir.path(), content)?,
            "内容一致应跳过"
        );
        assert!(
            ensure_cargo_config_in(dir.path(), "[other]\n")?,
            "内容漂移应重写"
        );
        Ok(())
    }

    /// 对线 R5：值与键形态校验（注入面；TOML 转义可产出真实换行）。
    #[test]
    fn mirror值键形态_校验规则() {
        assert!(env_value_sane(
            "https://mirrors.tuna.tsinghua.edu.cn/pypi/web/simple/"
        ));
        assert!(!env_value_sane("https://x/\nregistry=evil"), "换行拒");
        assert!(!env_value_sane("say \"hi\""), "双引号拒");
        assert!(!env_value_sane("crlf\r\n"), "\\r 拒");
        assert!(env_key_sane("FNM_NODE_DIST_MIRROR"));
        assert!(env_key_sane("_OK"));
        assert!(!env_key_sane("BAD-KEY"));
        assert!(!env_key_sane("K=1"));
        assert!(!env_key_sane(""));
    }

    /// go 代理语义键：行级 upsert 保用户键、GOSUMDB 配套、幂等。
    #[test]
    fn go代理_行级upsert与幂等() -> Result<(), String> {
        // 纯函数：旧官方值原位换、用户键保留、缺键追加
        let t1 = go_env_upsert(
            "GOPROXY=proxy.golang.org,direct
GOTOOLCHAIN=local
",
            "https://goproxy.cn,direct",
        );
        assert!(t1.contains("GOPROXY=https://goproxy.cn,direct"));
        assert!(
            t1.contains("GOSUMDB=sum.golang.google.cn"),
            "配套 sumdb 应追加"
        );
        assert!(t1.contains("GOTOOLCHAIN=local"), "用户键逐字保留");
        assert_eq!(t1.matches("GOPROXY=").count(), 1);
        // 幂等：目标态再跑逐字不变
        assert_eq!(go_env_upsert(&t1, "https://goproxy.cn,direct"), t1);
        // 落盘：注入目录内容比对幂等
        let cfg = tempfile::tempdir().map_err(|e| e.to_string())?;
        assert!(
            ensure_go_env_in(cfg.path(), "https://goproxy.cn,direct")?,
            "首次应写"
        );
        assert!(
            !ensure_go_env_in(cfg.path(), "https://goproxy.cn,direct")?,
            "内容一致应跳过"
        );
        let p = cfg.path().join("go").join("env");
        let c = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
        assert!(c.contains("GOPROXY=https://goproxy.cn,direct"));
        assert!(c.contains("GOSUMDB=sum.golang.google.cn"));
        Ok(())
    }

    /// GOENV 自定义路径即文件本身（对线 G1）：写的就是该文件，不再拼目录层级。
    #[test]
    fn go代理_goenv自定义路径直写该文件() -> Result<(), String> {
        let cfg = tempfile::tempdir().map_err(|e| e.to_string())?;
        let custom = cfg.path().join("dotfiles").join("goenv");
        assert!(
            ensure_go_env_file(&custom, "https://goproxy.cn,direct")?,
            "首写应报 true"
        );
        let c = std::fs::read_to_string(&custom).map_err(|e| e.to_string())?;
        assert!(
            c.contains("GOPROXY=https://goproxy.cn,direct"),
            "写的正是自定义文件本身"
        );
        assert!(
            !cfg.path().join("dotfiles").join("go").exists(),
            "不得多拼 go/env 层级"
        );
        assert!(
            !ensure_go_env_file(&custom, "https://goproxy.cn,direct")?,
            "幂等"
        );
        Ok(())
    }

    #[test]
    fn 平台覆盖与选键() {
        // 当前平台的命令键（平台自适应，避免 win 视角写死）
        let cur = if cfg!(windows) {
            vec![vec![
                "cmd".to_string(),
                "/c".to_string(),
                "echo".to_string(),
            ]]
        } else {
            vec![vec!["echo".to_string()]]
        };
        let mut pi = PostInstall::default();
        if cfg!(windows) {
            pi.win = Some(cur);
        } else if cfg!(target_os = "macos") {
            pi.mac = Some(cur);
        } else {
            pi.linux = Some(cur);
        }
        assert!(platform_covered(&pi), "当前平台有命令即覆盖");
        assert_eq!(platform_commands(&pi).count(), 1);
        let empty = PostInstall::default();
        assert!(!platform_covered(&empty), "全空未覆盖");
        // skip 覆盖路径：当前平台无命令但显式 skip
        let mut skipped = PostInstall::default();
        let cur_name = if cfg!(windows) {
            "win"
        } else if cfg!(target_os = "macos") {
            "mac"
        } else {
            "linux"
        };
        skipped.skip = Some(vec![cur_name.to_string()]);
        assert!(platform_covered(&skipped), "显式 skip 即覆盖");
        assert_eq!(platform_commands(&skipped).count(), 0);
    }

    #[test]
    fn shims_生成与幂等() {
        let dir = tempfile::tempdir().expect("临时目录");
        let src = dir
            .path()
            .join(if cfg!(windows) { "bun.exe" } else { "bun" });
        std::fs::write(&src, b"fake").expect("写源");
        let mut m = ToolManifest::default();
        m.shims = Some([("bunx".to_string(), "bun".to_string())].into());
        apply_shims(&m, dir.path()).expect("应成功");
        let dst = dir
            .path()
            .join(if cfg!(windows) { "bunx.exe" } else { "bunx" });
        assert!(dst.exists(), "别名应生成");
        // 链接而非拷贝：改源即见新内容（硬链接与符号链接同判，M016 平台自适应）
        std::fs::write(&src, b"changed").expect("改源");
        assert_eq!(
            std::fs::read(&dst).expect("读别名"),
            b"changed",
            "别名应与源同体"
        );
        #[cfg(not(windows))]
        assert_eq!(
            std::fs::read_link(&dst).expect("应为符号链接"),
            src,
            "POSIX 别名应指向源"
        );
        apply_shims(&m, dir.path()).expect("幂等二连应成功");
    }

    #[test]
    fn post_install_成功失败与覆盖缺失() {
        // 当前平台自适应成功命令（win=cmd /c echo、POSIX=echo）
        let ok = if cfg!(windows) {
            vec![
                "cmd".to_string(),
                "/c".to_string(),
                "echo".to_string(),
                "ark-ok".to_string(),
            ]
        } else {
            vec!["echo".to_string(), "ark-ok".to_string()]
        };
        let mut m = ToolManifest::default();
        m.post_install = Some(pi_for(ok));
        run_post_install(&m, "t").expect("当前平台命令应成功");
        // 启动失败：一条不存在的命令
        let mut bad = ToolManifest::default();
        bad.post_install = Some(pi_for(vec!["definitely-missing-ark-bin".to_string()]));
        let e = run_post_install(&bad, "t").expect_err("应报失败");
        assert!(e.contains("失败"), "{e}");
        // 未覆盖：当前平台无命令且未 skip
        let mut uncovered = ToolManifest::default();
        let mut upi = PostInstall::default();
        if cfg!(windows) {
            upi.linux = Some(vec![vec!["echo".to_string()]]);
        } else {
            upi.win = Some(vec![vec!["cmd".to_string()]]);
        }
        uncovered.post_install = Some(upi);
        let e = run_post_install(&uncovered, "t").expect_err("未覆盖应报");
        assert!(e.contains("未覆盖"), "{e}");
    }

    #[test]
    fn shim_cmd内容不嵌绝对路径() {
        // 内容用 %~dp0 相对定位：路径含空格靠引号兜住，不落盘绝对路径、无转义面（M017 面）
        let c = shim_cmd_content("bun");
        assert_eq!(c, "@\"%~dp0bun.exe\" %*\r\n");
        assert!(!c.contains('\\'), "内容不应含转义反斜杠: {c}");
        assert!(!c.contains('\u{feff}'), "cmd 不认 BOM");
    }

    #[test]
    fn post_install大输出不阻塞且报告失败尾行() {
        // 输出远超管道缓冲（此处约 1MiB）：不并发抽干则子进程写阻塞被误判超时（M017 实证）。
        // 生成器取「读一个大文件」而非 shell 循环：前者毫秒级且不吃负载（循环版在本机
        // 高负载下从 0.2s 漂到 21s，把有负载的机器变成假红）。
        let dir = tempfile::tempdir().expect("临时目录");
        let big_file = dir.path().join("big.txt");
        std::fs::write(&big_file, ("A".repeat(63) + "\r\n").repeat(16384)).expect("写大文件");
        assert!(
            std::fs::metadata(&big_file).expect("读元数据").len() > 512 * 1024,
            "输出必须远超管道缓冲才有判别力"
        );
        let path = big_file.display().to_string();
        let big = if cfg!(windows) {
            vec![
                "cmd".to_string(),
                "/c".to_string(),
                "type".to_string(),
                path,
            ]
        } else {
            vec!["cat".to_string(), path]
        };
        let mut m = ToolManifest::default();
        m.post_install = Some(pi_for(big));
        // 20s 窗口远小于 300s：真阻塞必超时，抽干后毫秒级成功
        run_post_install_with_timeout(&m, "t", std::time::Duration::from_secs(20))
            .expect("大输出应抽干不阻塞");
        // 失败路径：非零退出码加尾行（退出码与内容都进报告）
        let failing = if cfg!(windows) {
            vec![
                "cmd".to_string(),
                "/c".to_string(),
                "echo boom& exit /b 7".to_string(),
            ]
        } else {
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo boom; exit 7".to_string(),
            ]
        };
        let mut bad = ToolManifest::default();
        bad.post_install = Some(pi_for(failing));
        let e = run_post_install_with_timeout(&bad, "t", std::time::Duration::from_secs(20))
            .expect_err("应报失败");
        assert!(e.contains("退出码 Some(7)"), "{e}");
        assert!(e.contains("boom"), "尾行应带输出: {e}");
    }

    #[test]
    fn post_install超时即杀进程() {
        // 短超时注入跑杀进程路径（真 300s 不可测）：应快速返回超时而非等命令自然结束
        let slow = if cfg!(windows) {
            vec![
                "cmd".to_string(),
                "/c".to_string(),
                "ping -n 20 127.0.0.1".to_string(),
            ]
        } else {
            vec!["sh".to_string(), "-c".to_string(), "sleep 20".to_string()]
        };
        let mut m = ToolManifest::default();
        m.post_install = Some(pi_for(slow));
        let started = std::time::Instant::now();
        let e = run_post_install_with_timeout(&m, "t", std::time::Duration::from_secs(1))
            .expect_err("应超时");
        assert!(e.contains("超时"), "{e}");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "杀进程后应立即返回"
        );
        // Windows 进程树须真被杀净（M021）：kill 只杀 cmd，孙进程 ping 靠 taskkill /T 清
        #[cfg(windows)]
        {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let out = std::process::Command::new("tasklist")
                .args(["/FI", "IMAGENAME eq ping.exe", "/NH"])
                .output()
                .expect("tasklist 应可运行");
            // tasklist 的镜像名是大写（PING.EXE），断言必须大小写不敏感（否则漏报）
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            assert!(
                !text.contains("ping.exe"),
                "超时终止后不应遗留 ping 孙进程: {text}"
            );
        }
    }

    /// 当前平台键的三键齐备构造（当前平台给命令、其余进 skip；M016 平台自适应纪律）。
    fn pi_for(cmd: Vec<String>) -> PostInstall {
        let mut pi = PostInstall::default();
        if cfg!(windows) {
            pi.win = Some(vec![cmd]);
            pi.skip = Some(vec!["linux".into(), "mac".into()]);
        } else if cfg!(target_os = "macos") {
            pi.mac = Some(vec![cmd]);
            pi.skip = Some(vec!["win".into(), "linux".into()]);
        } else {
            pi.linux = Some(vec![cmd]);
            pi.skip = Some(vec!["win".into(), "mac".into()]);
        }
        pi
    }
}
