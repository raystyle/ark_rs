//! catalog：清单主功能。数据面（tools.toml 读写与路径解析）加两个子功能：
//! `ark catalog status`（看解析面与云端同步态）与 `ark catalog sync`（从云端刷新用户数据副本，D33）。
//!
//! 数据契约见 `docs/references/R001`：读用 serde（字段同 R001），
//! 写（pin 回写）用 toml_edit DocumentMut 直接改文档树，保住字段顺序与注释。
//! 路径解析优先级：
//! - EnvRoot：`--env-root` 参数 > `ARK_ROOT` 环境变量 > 存在 D:\ 则 D:\ohmyenv 否则 C:\ohmyenv
//! - catalog：`ARK_CATALOG` 环境变量 > exe 上级的 catalog\tools.toml > cwd\catalog\tools.toml
//!   > 用户数据目录；四级全 miss 时自举拉取（镜像边车锚，键读序 `ark/catalog` 主先、`ome/catalog` 兼容回落，#10）
//!   > 用户数据目录 catalog\tools.toml（自部署布局）

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use toml_edit::{DocumentMut, Formatted, Item, Value};

use crate::resolve::Resolution;

/// 工具条目：字段与 R001 一一对应；可选字段为空时整行省略，故全部 Option。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Tool {
    // —— 静态元数据（ome 不回写）——
    /// 七类 taxonomy 分类（agent/runtime/cli 等分组与排序依据）
    pub category: Option<String>,
    /// 部署形态：envroot 泊位或 installer 系统装
    pub deploy: Option<String>,
    /// EnvRoot 下相对目录（Windows 名录布局）
    pub dir: Option<String>,
    /// PATH 注册目录（支持 `~` 与 `$VAR` 展开）
    pub bin: Option<String>,
    /// 本机可执行文件相对路径（探测与验证目标）
    pub exe: Option<String>,
    /// 解压九分派类型（zip/targz-bin/msi/copy/ark-self 等）
    pub extract: Option<String>,
    /// GitHub 仓坐标 owner/name（resolve 与自更新源）
    pub repo: Option<String>,
    /// release tag 前缀（缺省 v）
    pub tag_prefix: Option<String>,
    /// 资产名匹配正则（release 资产检索）
    pub asset_pattern: Option<String>,
    /// 版本号提取正则（自资产名或 tag）
    pub version_pattern: Option<String>,
    // —— 已装版本探测（D28 入册清单化：探测参数与正则自 toolver 源码表迁字段；
    //    probe_args 缺省 ["--version"]，oscdimg 无参为 []；probe_pattern 取第 1 捕获组）——
    /// 已装版本探测实参（缺省 --version；oscdimg 无参为空）
    pub probe_args: Option<Vec<String>>,
    /// 版本探测正则（取第 1 捕获组）
    pub probe_pattern: Option<String>,
    // —— manifest 引用（R016 D39：声明该工具的安装逻辑在 manifest.toml 同名节；缺省同名语义）——
    /// manifest 面状态（两件分离各自诊断）
    pub manifest: Option<String>,
    // —— guide 字段（D25 入册；D50 起数据面保留：真源云端 ohmycloud，ark 侧无渲染消费者）——
    /// 一句话描述（D25 入册的数据面元数据）
    pub desc: Option<String>,
    /// 引导提示的环境变量键清单
    pub guide_env: Option<Vec<String>>,
    /// 引导提示的目录清单
    pub guide_dirs: Option<Vec<String>>,
    /// 引导注意事项多行文本（D25）
    pub guide_notes: Option<String>,
    /// cdn 模板下载直链（`{version}` 占位）
    pub cdn_url: Option<String>,
    /// index 型清单 URL（resolve 分支 c）
    pub cdn_index_url: Option<String>,
    /// cdn 资产名正则（target 键形）
    pub cdn_asset_pattern: Option<String>,
    /// 版本文件 URL（latest 滚动源）
    pub cdn_version_url: Option<String>,
    /// Linux cdn 直链（回退通用）
    pub linux_cdn_url: Option<String>,
    /// Linux cdn 资产名正则（回退通用）
    pub linux_cdn_asset_pattern: Option<String>,
    /// Linux 额外多二进制（逗号分隔，同目录落位）
    pub linux_extra_bins: Option<String>,
    /// 校验清单资产名（SHA256SUMS 类）
    pub sums_asset: Option<String>,
    /// 校验清单行匹配正则
    pub sums_pattern: Option<String>,
    /// 边车后缀（`.sha256` 形态的锚边车）
    pub asset_sha_suffix: Option<String>,
    /// 引导器资产名（evergreen 型安装引导）
    pub bootstrap_asset: Option<String>,
    // —— Linux / macOS 平台专属字段（缺失时回退到通用字段）——
    /// Linux 平台 repo（缺失回退通用）
    pub linux_repo: Option<String>,
    /// Linux 资产名正则（回退通用）
    pub linux_asset_pattern: Option<String>,
    /// Linux 目录布局（回退通用）
    pub linux_dir: Option<String>,
    /// Linux PATH 目录（回退通用）
    pub linux_bin: Option<String>,
    /// Linux 可执行路径（回退通用）
    pub linux_exe: Option<String>,
    /// Linux 解压类型（回退通用）
    pub linux_extract: Option<String>,
    /// Linux 校验行正则（回退通用）
    pub linux_sums_pattern: Option<String>,
    /// Linux 边车后缀（回退通用）
    pub linux_asset_sha_suffix: Option<String>,
    /// Linux 引导器资产（回退通用）
    pub linux_bootstrap_asset: Option<String>,
    // —— macOS 专属字段（缺失时回退 linux_*，再回退通用；仅 darwin 构建生效）——
    /// macOS 平台 repo（缺失回退 linux 再回退通用）
    pub mac_repo: Option<String>,
    /// macOS 资产名正则（回退链同上）
    pub mac_asset_pattern: Option<String>,
    /// macOS 目录布局（回退链同上）
    pub mac_dir: Option<String>,
    /// macOS PATH 目录（回退链同上）
    pub mac_bin: Option<String>,
    /// macOS 可执行路径（回退链同上）
    pub mac_exe: Option<String>,
    /// macOS 解压类型（回退链同上）
    pub mac_extract: Option<String>,
    /// macOS 校验行正则（回退链同上）
    pub mac_sums_pattern: Option<String>,
    /// macOS 边车后缀（回退链同上）
    pub mac_asset_sha_suffix: Option<String>,
    /// macOS 引导器资产（回退链同上）
    pub mac_bootstrap_asset: Option<String>,
    /// macOS cdn 直链（回退链同上）
    pub mac_cdn_url: Option<String>,
    /// macOS cdn 资产正则（回退链同上）
    pub mac_cdn_asset_pattern: Option<String>,
    /// macOS 额外多二进制（回退链同上）
    pub mac_extra_bins: Option<String>,
    // —— pin 字段（ark pin/update 回写；按平台分列，通用四键即 Windows pin）——
    /// pin 锁定的 release tag（Windows 通用位；ark pin/update 回写）
    pub tag: Option<String>,
    /// pin 锁定的版本号（Windows 通用位）
    pub version: Option<String>,
    /// pin 锁定的资产名（Windows 通用位）
    pub asset: Option<String>,
    /// pin 锁定的资产 sha256（Windows 通用位）
    pub sha256: Option<String>,
    /// Linux 平台 pin tag（分列不跨平台回退）
    pub linux_tag: Option<String>,
    /// Linux 平台 pin 版本
    pub linux_version: Option<String>,
    /// Linux 平台 pin 资产名
    pub linux_asset: Option<String>,
    /// Linux 平台 pin sha256
    pub linux_sha256: Option<String>,
    /// macOS 平台 pin tag（分列不跨平台回退）
    pub mac_tag: Option<String>,
    /// macOS 平台 pin 版本
    pub mac_version: Option<String>,
    /// macOS 平台 pin 资产名
    pub mac_asset: Option<String>,
    /// macOS 平台 pin sha256
    pub mac_sha256: Option<String>,
    /// 版本锁定开关（静态元数据，跨平台生效）：true 时 update/pin/带版本选项的 install 全部跳过，
    /// 用于钉死特定版本（如 bun 1.3.14——最后一个完全用 Zig 编写核心的版本，2026-09-01 用户裁决）。
    pub hold: Option<bool>,
}

impl Tool {
    /// 是否版本锁定（hold = true）。
    pub fn is_held(&self) -> bool {
        self.hold.unwrap_or(false)
    }

    /// 一行用途说明（D25 入册；D50 起数据面保留，ark 侧暂无渲染消费者）。
    pub fn desc(&self) -> &str {
        self.desc.as_deref().unwrap_or("")
    }

    /// 引导提示的环境变量键清单（凭据类键只显在否不显值；数据面保留）。
    pub fn guide_env(&self) -> &[String] {
        self.guide_env.as_deref().unwrap_or(&[])
    }

    /// 引导提示的安装与数据目录清单（支持 `~` 与环境变量展开；数据面保留）。
    pub fn guide_dirs(&self) -> &[String] {
        self.guide_dirs.as_deref().unwrap_or(&[])
    }

    /// 使用注意事项（版本语义、升级例外、PATH 特性等静态知识）。
    pub fn guide_notes(&self) -> &str {
        self.guide_notes.as_deref().unwrap_or("")
    }

    /// 当前平台适用的 repo（mac 依次回退 `mac_repo`/`linux_repo`/`repo`；Linux 依次 `linux_repo`/`repo`；Windows 取 `repo`）。
    pub fn repo(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        if let Some(v) = self.mac_repo.as_deref() {
            return Some(v);
        }
        #[cfg(not(windows))]
        if let Some(v) = self.linux_repo.as_deref() {
            return Some(v);
        }
        self.repo.as_deref()
    }

    /// 当前平台适用的 asset_pattern（回退链同 `repo()`）。
    pub fn asset_pattern(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        if let Some(v) = self.mac_asset_pattern.as_deref() {
            return Some(v);
        }
        #[cfg(not(windows))]
        if let Some(v) = self.linux_asset_pattern.as_deref() {
            return Some(v);
        }
        self.asset_pattern.as_deref()
    }

    /// 当前平台适用的安装目录字段（回退链同 `repo()`）。
    pub fn dir(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        if let Some(v) = self.mac_dir.as_deref() {
            return Some(v);
        }
        #[cfg(not(windows))]
        if let Some(v) = self.linux_dir.as_deref() {
            return Some(v);
        }
        self.dir.as_deref()
    }

    /// 当前平台适用的 PATH 目录字段（回退链同 `repo()`）。
    pub fn bin(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        if let Some(v) = self.mac_bin.as_deref() {
            return Some(v);
        }
        #[cfg(not(windows))]
        if let Some(v) = self.linux_bin.as_deref() {
            return Some(v);
        }
        self.bin.as_deref()
    }

    /// 当前平台适用的 exe 字段（回退链同 `repo()`）。
    pub fn exe(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        if let Some(v) = self.mac_exe.as_deref() {
            return Some(v);
        }
        #[cfg(not(windows))]
        if let Some(v) = self.linux_exe.as_deref() {
            return Some(v);
        }
        self.exe.as_deref()
    }

    /// 平台专属 exe 字段原始值（`mac_exe`/`linux_exe`，无通用回退）。
    /// 有值时 exe 相对 install_dir；为 None 时 `exe()` 必来自通用字段（Windows 风格，自带 dir 段、相对 EnvRoot）。
    pub fn platform_exe(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        if let Some(v) = self.mac_exe.as_deref() {
            return Some(v);
        }
        #[cfg(not(windows))]
        if let Some(v) = self.linux_exe.as_deref() {
            return Some(v);
        }
        None
    }

    /// 当前平台适用的 extract 字段（回退链同 `repo()`）。
    pub fn extract(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        if let Some(v) = self.mac_extract.as_deref() {
            return Some(v);
        }
        #[cfg(not(windows))]
        if let Some(v) = self.linux_extract.as_deref() {
            return Some(v);
        }
        self.extract.as_deref()
    }

    /// 当前平台适用的 sums_pattern（回退链同 `repo()`）。
    pub fn sums_pattern(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        if let Some(v) = self.mac_sums_pattern.as_deref() {
            return Some(v);
        }
        #[cfg(not(windows))]
        if let Some(v) = self.linux_sums_pattern.as_deref() {
            return Some(v);
        }
        self.sums_pattern.as_deref()
    }

    /// 当前平台适用的 asset_sha_suffix（回退链同 `repo()`）。
    pub fn asset_sha_suffix(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        if let Some(v) = self.mac_asset_sha_suffix.as_deref() {
            return Some(v);
        }
        #[cfg(not(windows))]
        if let Some(v) = self.linux_asset_sha_suffix.as_deref() {
            return Some(v);
        }
        self.asset_sha_suffix.as_deref()
    }

    /// 当前平台适用的 bootstrap_asset（回退链同 `repo()`）。
    pub fn bootstrap_asset(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        if let Some(v) = self.mac_bootstrap_asset.as_deref() {
            return Some(v);
        }
        #[cfg(not(windows))]
        if let Some(v) = self.linux_bootstrap_asset.as_deref() {
            return Some(v);
        }
        self.bootstrap_asset.as_deref()
    }

    /// 当前平台适用的 cdn_url（回退链同 `repo()`）。
    /// 直链模板（平台严格）：Windows 取通用 `cdn_url`；Linux 只取 `linux_cdn_url`；
    /// macOS 取 `mac_cdn_url` 回退 `linux_cdn_url`。非 Windows **不回退通用 cdn_url**——
    /// 通用 cdn_url 是 Windows 资产直链（go/zig/dotnet/oscdimg），回退会让 Linux 侧
    /// 解析到 Windows 包（2026-09-01 WSL install all 实证踩坑）。
    pub fn cdn_url(&self) -> Option<&str> {
        #[cfg(windows)]
        {
            self.cdn_url.as_deref()
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            self.linux_cdn_url.as_deref()
        }
        #[cfg(target_os = "macos")]
        {
            self.mac_cdn_url
                .as_deref()
                .or(self.linux_cdn_url.as_deref())
        }
    }

    /// 当前平台适用的 cdn_asset_pattern（回退链同 `repo()`；vault 类 cdn 工具各平台资产名不同）。
    pub fn cdn_asset_pattern(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        if let Some(v) = self.mac_cdn_asset_pattern.as_deref() {
            return Some(v);
        }
        #[cfg(not(windows))]
        if let Some(v) = self.linux_cdn_asset_pattern.as_deref() {
            return Some(v);
        }
        self.cdn_asset_pattern.as_deref()
    }

    /// 当前平台的补充二进制叶子名列表（空格/逗号分隔）：`*-bin` 提取与 zip/targz 的 chmod
    /// 除 exe 主二进制外还覆盖这些成员（如 age 的 age-keygen、ast-grep 的 sg）。
    pub fn extra_bins(&self) -> Vec<&str> {
        #[cfg(target_os = "macos")]
        let raw = self
            .mac_extra_bins
            .as_deref()
            .or(self.linux_extra_bins.as_deref());
        #[cfg(all(not(windows), not(target_os = "macos")))]
        let raw = self.linux_extra_bins.as_deref();
        #[cfg(windows)]
        let raw: Option<&str> = None;
        raw.map(|s| s.split([',', ' ']).filter(|x| !x.is_empty()).collect())
            .unwrap_or_default()
    }

    /// 当前平台锁定的 tag（pin 按平台分列、无跨平台回退：Windows 通用、Linux `linux_*`、mac `mac_*`）。
    pub fn pin_tag(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        return self.mac_tag.as_deref();
        #[cfg(all(not(windows), not(target_os = "macos")))]
        return self.linux_tag.as_deref();
        #[cfg(windows)]
        self.tag.as_deref()
    }

    /// 当前平台锁定的 version。
    pub fn pin_version(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        return self.mac_version.as_deref();
        #[cfg(all(not(windows), not(target_os = "macos")))]
        return self.linux_version.as_deref();
        #[cfg(windows)]
        self.version.as_deref()
    }

    /// 当前平台锁定的 asset。
    pub fn pin_asset(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        return self.mac_asset.as_deref();
        #[cfg(all(not(windows), not(target_os = "macos")))]
        return self.linux_asset.as_deref();
        #[cfg(windows)]
        self.asset.as_deref()
    }

    /// 当前平台锁定的 sha256。
    pub fn pin_sha256(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        return self.mac_sha256.as_deref();
        #[cfg(all(not(windows), not(target_os = "macos")))]
        return self.linux_sha256.as_deref();
        #[cfg(windows)]
        self.sha256.as_deref()
    }
}

/// 当前平台 pin 字段的 TOML 键名（Windows 无前缀，Linux/mac 加平台前缀）。
pub fn pin_key(base: &str) -> String {
    #[cfg(target_os = "macos")]
    {
        format!("mac_{base}")
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        format!("linux_{base}")
    }
    #[cfg(windows)]
    {
        base.to_string()
    }
}

/// 已加载的 catalog：order 保工具书写顺序（即安装/更新顺序），tools 按键查值。
pub struct Catalog {
    /// 清单文件路径（回写与状态面定位）
    pub path: PathBuf,
    /// 工具书写顺序（安装与更新遍历序）
    pub order: Vec<String>,
    /// 工具条目按键查值表
    pub tools: HashMap<String, Tool>,
}

#[derive(Deserialize)]
struct RawCatalog {
    tools: HashMap<String, Tool>,
}

impl Catalog {
    /// 从磁盘加载 catalog。
    ///
    /// # Errors
    /// 返回 Err（人读原因串）当：读取 catalog 失败 等（完整失败面见函数体错误构造）。
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("读取 catalog 失败: {}: {e}", path.display()))?;
        Self::parse(&text, path.to_path_buf())
    }

    /// 解析 catalog 文本（顺序取文档树、取值走 serde）。
    ///
    /// # Errors
    /// 返回 Err（人读原因串）当：未知工具: {seg}（catalog: {}） 等（完整失败面见函数体错误构造）。
    pub fn parse(text: &str, path: PathBuf) -> Result<Self, String> {
        // toml 0.8 风格 serde 序列化会丢字段顺序，故顺序另从 DocumentMut 取
        let doc: DocumentMut = text
            .parse()
            .map_err(|e| format!("catalog TOML 语法错误: {}: {e}", path.display()))?;
        let mut order = Vec::new();
        let tools_item = doc
            .get("tools")
            .ok_or_else(|| format!("catalog 缺少 [tools] 节: {}", path.display()))?;
        let tools_table = tools_item
            .as_table_like()
            .ok_or_else(|| format!("catalog [tools] 不是表: {}", path.display()))?;
        for (name, _) in tools_table.iter() {
            order.push(name.to_string());
        }
        let raw: RawCatalog = toml_edit::de::from_str(text)
            .map_err(|e| format!("catalog 字段解析失败: {}: {e}", path.display()))?;
        Ok(Catalog {
            path,
            order,
            tools: raw.tools,
        })
    }

    /// 子命令的工具选择：`all` 展开为全部（保序）；单工具校验存在性；逗号串
    /// 逐段拆分校验（D49 尾修：status HINT 教的「ark update a,b,c」此前整串被当
    /// 单名拒收，自产提示不可执行）。重复段保序去重，空段滤除。
    ///
    /// # Errors
    /// 返回 Err（人读原因串）当：未知工具: {seg}（catalog: {}） 等（完整失败面见函数体错误构造）。
    pub fn select(&self, name: &str) -> Result<Vec<String>, String> {
        if name == "all" {
            return Ok(self.order.clone());
        }
        let mut out: Vec<String> = Vec::new();
        for seg in name.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if !self.tools.contains_key(seg) {
                return Err(format!(
                    "未知工具: {seg}（catalog: {}）",
                    self.path.display()
                ));
            }
            if !out.iter().any(|n| n == seg) {
                out.push(seg.to_string());
            }
        }
        Ok(out)
    }

    /// 按名取工具条目。
    ///
    /// # Errors
    /// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
    pub fn tool(&self, name: &str) -> Result<&Tool, String> {
        self.tools
            .get(name)
            .ok_or_else(|| format!("catalog 中无工具: {name}"))
    }
}

/// EnvRoot 解析：显式参数 > ARK_ROOT > 平台默认。
/// 对齐 helpers.ps1 Get-DefaultEnvRoot：参数与环境变量都会裁掉尾部斜杠。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn resolve_env_root(cli: Option<&str>) -> Result<PathBuf, String> {
    if let Some(v) = cli {
        let v = v.trim().trim_end_matches(['/', '\\']);
        if !v.is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    if let Some(v) = crate::platform::env_var("ARK_ROOT") {
        let v = v.trim_end_matches(['/', '\\']);
        if !v.is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    Ok(crate::platform::default_env_root())
}

/// catalog 路径解析：`ARK_CATALOG` > exe 上级的 catalog\tools.toml（仓库与旧自部署布局）
/// > cwd\catalog\tools.toml > 用户数据目录 catalog\tools.toml（新自部署布局，self-deploy 时同步）。
/// > 四级全 miss（裸二进制端，ohmyenv-rs#10 缺口 3）时自举拉取到用户数据目录。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn resolve_catalog_path() -> Result<PathBuf, String> {
    if let Some(v) = crate::platform::env_var("ARK_CATALOG") {
        return Ok(PathBuf::from(v));
    }
    match catalog_candidates(&catalog_search_roots())
        .into_iter()
        .find(|p| p.exists())
    {
        Some(p) => Ok(p),
        None => bootstrap_catalog().map_err(|e| {
            format!("未找到 catalog\\tools.toml 且自举失败（可设 ARK_CATALOG 指定路径）: {e}")
        }),
    }
}

/// catalog 自举（ohmyenv-rs#10 缺口 3；D37 终态改为 fail-closed）：
/// 一律走云端 `ome/catalog/tools.toml` 三重门（边车 sha 锚、`Catalog::load` 解析、内嵌公钥验签），
/// 落位清单与签名件到用户数据目录并写检查标记。仓库件已退役，故不再有官方 raw 兜底路径；
/// 镜像不可达即如实报错（提示网络与 `ARK_CATALOG` 出路），不静默放行未验签内容。
fn bootstrap_catalog() -> Result<PathBuf, String> {
    let env_root = crate::platform::default_env_root();
    let dst_dir = crate::platform::metadata_dir().join("catalog");
    std::fs::create_dir_all(&dst_dir).map_err(|e| format!("建数据目录失败: {e}"))?;
    let dst = dst_dir.join("tools.toml");
    let cloud = fetch_cloud(&env_root).map_err(|e| {
        format!(
            "catalog 自举失败（云端 {} 不可达或未过三重门）: {e}\n检查网络后重试；或用 ARK_CATALOG 指定本地清单",
            crate::download::MIRROR_BASE
        )
    })?;
    place(&cloud.path, &dst)?;
    place(&cloud.sig_path, &signature_path(&dst))?;
    write_marker(&dst, now_secs(), &cloud.sha);
    // D40：自举无基线可比（本就没有已见件），但拉到手即把 seq 记成基线，
    // 免得后续镜像回滚在「已见仍为 0」的窗口里被放行。
    record_seen_seq_from_local(&dst);
    eprintln!("[OK] catalog 已自举（云端验签通过）: {}", dst.display());
    Ok(dst)
}

/// 候选根目录（按优先级）：exe 上两级（仓库 target\ 布局与旧自部署 `<ome>\bin\ome.exe` 布局）、
/// cwd（仓库根运行）、用户数据目录（新自部署布局）。cwd 先于用户数据目录，
/// 保证仓库内开发/测试永远命中仓库 catalog，不被部署副本遮蔽。
fn catalog_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            if let Some(up) = exe_dir.parent() {
                roots.push(up.to_path_buf());
            }
        }
    }
    roots.push(std::env::current_dir().unwrap_or_default());
    roots.push(crate::platform::metadata_dir());
    roots
}

/// 由根目录列表生成 catalog 候选路径（保持入参顺序）。
fn catalog_candidates(roots: &[PathBuf]) -> Vec<PathBuf> {
    roots
        .iter()
        .map(|r| r.join("catalog").join("tools.toml"))
        .collect()
}

/// pin 回写：用 toml_edit 直接改文档树，只动当前平台的 tag/version/asset/sha256 四个键
/// （Windows 通用、Linux `linux_*`、mac `mac_*`，见 R001），保住字段顺序、缩进与注释。
/// 版本变化时删除 sha256 行（等 install 回填），同版本 re-pin 保留。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn write_pin(path: &Path, tool: &str, res: &Resolution) -> Result<bool, String> {
    let mut version_changed = false;
    let (k_tag, k_version, k_asset, k_sha) = (
        pin_key("tag"),
        pin_key("version"),
        pin_key("asset"),
        pin_key("sha256"),
    );
    update_tool_table(path, tool, |table| {
        let old_version = table
            .get(&k_version)
            .and_then(|i| i.as_str())
            .map(str::to_string);
        version_changed = old_version.as_deref() != Some(res.version.as_str());
        set_string(table, &k_tag, &res.tag);
        set_string(table, &k_version, &res.version);
        set_string(table, &k_asset, &res.asset_name);
        if version_changed {
            // 版本真变才清 sha（同版本 re-pin 保留旧 sha）
            table.remove(&k_sha);
        }
    })?;
    Ok(version_changed)
}

/// sha256 回填：只写当前平台的 sha256 一个键（install 成功后回填空 sha；统一大写）。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn write_sha256(path: &Path, tool: &str, sha: &str) -> Result<(), String> {
    let k_sha = pin_key("sha256");
    update_tool_table(path, tool, |table| {
        set_string(table, &k_sha, &sha.to_uppercase())
    })
}

/// 共享回写助手：解析 DocumentMut、定位 [tools.<名>] 表交给 edit 闭包、写回时保持原行尾风格。
fn update_tool_table(
    path: &Path,
    tool: &str,
    edit: impl FnOnce(&mut dyn toml_edit::TableLike),
) -> Result<(), String> {
    // D34：内容一改，随件签名即失效；先撤签名件，避免留一份"签过但内容已变"的假凭证
    let _ = fs::remove_file(signature_path(path));
    let text = fs::read_to_string(path)
        .map_err(|e| format!("读取 catalog 失败: {}: {e}", path.display()))?;
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| format!("catalog TOML 语法错误: {}: {e}", path.display()))?;

    let tools = doc
        .get_mut("tools")
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| format!("catalog 缺少 [tools] 节: {}", path.display()))?;
    let table = tools
        .get_mut(tool)
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| format!("catalog 中无工具节 [tools.{tool}]"))?;
    edit(table);

    // 保持原文件行尾风格：toml_edit 序列化统一出 LF，原文件为 CRLF 时整体换回，避免全文件换行噪音
    let mut out = doc.to_string();
    if text.contains("\r\n") {
        out = out.replace("\r\n", "\n").replace('\n', "\r\n");
    }
    fs::write(path, out).map_err(|e| format!("写回 catalog 失败: {}: {e}", path.display()))?;
    Ok(())
}

/// 设置字符串键：已存在则只换值本体、保留 decor（缩进与行内注释）；不存在则追加到节尾。
fn set_string(table: &mut dyn toml_edit::TableLike, key: &str, v: &str) {
    match table.get_mut(key) {
        Some(Item::Value(Value::String(s))) => {
            let decor = s.decor().clone();
            let mut f = Formatted::new(v.to_string());
            *f.decor_mut() = decor;
            *s = f;
        }
        Some(item) => {
            *item = toml_edit::value(v);
        }
        None => {
            table.insert(key, toml_edit::value(v));
        }
    }
}

// ══════════════ 云端清单（主功能 catalog 的两个子功能：status 看、sync 刷，D33） ══════════════
//
// 口径（2026-09-10 用户三项定稿）：
// - **权威落位**：仓库副本是开发与离线兜底源；云端 `env.ohmygh.com/ome/catalog/tools.toml`
//   是运行态权威，部署机按 TTL 刷新用户数据副本。
// - **子功能 status**（`ark catalog status`）：看解析面来源、本地与云端锚、检查年龄、TTL 与同步态。
// - **子功能 sync**（`ark catalog sync`）：立即从云端刷新；命令前的自动刷新走同一实现。
// - **信任锚**：镜像 `.sha256` 边车自算即锚（先边车后资产，锚不符或解析不过一律拒收），签名留后。
//
// 边界：只刷用户数据副本（仓库 cwd、二进制同级、`ARK_CATALOG` 指定面不读不改，开发态零干扰）；
// 自动路径网络异常快速退化（单次 5s 探活、不重试），失败记退避标记；部署位 pin 回写仍是临时态
// （四.7）：云端刷新会以云端权威覆盖本地副本。

/// 云端清单在镜像里的键（D41 C：主键 `ark/catalog/`；兼容键 `ome/catalog/` 为 omc 铺段前
/// 的过渡窗回落，读序主键先、404 落兼容键；停 ome/ 键判据为存量机水位清零）。
pub const CLOUD_CATALOG_KEY: &str = "ark/catalog/tools.toml";
const CLOUD_CATALOG_KEY_COMPAT: &str = "ome/catalog/tools.toml";
/// 云端 manifest 键（R016 两件分离：与 tools.toml 同批同签；双键读序同上）。
pub const CLOUD_MANIFEST_KEY: &str = "ark/catalog/manifest.toml";
const CLOUD_MANIFEST_KEY_COMPAT: &str = "ome/catalog/manifest.toml";

/// 云端 catalog 键读序（主键先、兼容键回落）。
fn cloud_catalog_keys() -> [&'static str; 2] {
    [CLOUD_CATALOG_KEY, CLOUD_CATALOG_KEY_COMPAT]
}

/// 云端 manifest 键读序。
fn cloud_manifest_keys() -> [&'static str; 2] {
    [CLOUD_MANIFEST_KEY, CLOUD_MANIFEST_KEY_COMPAT]
}
/// 内嵌的云端清单签名公钥（D34，minisign 与 Ed25519 的 base64 公钥行；key id 见下）。
/// 私钥只在本机 `~/.config/ome/catalog-signing.key` 与 CI 密钥库出现；其他机器只要二进制带此公钥即可校验。
/// 轮换：先发版同时内嵌新旧两把公钥（任一验过即通过），再换私钥重签云端件，机器更新完后摘掉旧钥。
const CLOUD_CATALOG_PUBKEYS: [&str; 1] =
    ["RWQWI4x407+T+51a9TZWS487QCZbhzehIoD2+e/4quSr3hpsu9nmDR4o"];
/// 内嵌公钥的 key id（人读标注，来自 `catalog-sign pubkey` 输出）。
pub const CLOUD_CATALOG_PUBKEY_ID: &str = "FB93BFD3788C2316";
/// 自动刷新默认 TTL（秒）：一天一次锚比对。
pub const DEFAULT_TTL_SECS: u64 = 24 * 60 * 60;
/// 自动路径探活超时（秒）：网络异常时快速退化，不拖慢用户命令。
const PROBE_TIMEOUT_SECS: u64 = 5;

/// 刷新结果。
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// 未联网：`off` 显式关闭（TTL 0 或离线）、`fresh` 未过期、`unreachable` 探活失败（已退避）。
    Skipped(&'static str),
    /// 本地与云端同锚（只更新检查标记）。
    InSync {
        /// 对齐的锚
        sha: String,
    },
    /// 已从云端拉取并落位。
    Updated {
        /// 拉取的锚
        sha: String,
    },
}

impl Outcome {
    /// 命令面动作词（数据块 `action` 字段值）。
    pub fn action(&self) -> &'static str {
        match self {
            Outcome::Skipped(_) => "skipped",
            Outcome::InSync { .. } => "current",
            Outcome::Updated { .. } => "updated",
        }
    }

    /// 跳过原因（非跳过为空串）；`unreachable` 供自动路径退避后如实标注。
    pub fn reason(&self) -> &'static str {
        match self {
            Outcome::Skipped(r) => r,
            _ => "",
        }
    }

    /// 关联的清单 sha（跳过时为空）。
    pub fn sha(&self) -> Option<&str> {
        match self {
            Outcome::Skipped(_) => None,
            Outcome::InSync { sha } | Outcome::Updated { sha } => Some(sha),
        }
    }
}

/// 用户数据副本路径：`<metadata>\catalog\tools.toml`（self-deploy 同步位，运行态权威的落点）。
pub fn user_data_catalog_path() -> PathBuf {
    crate::platform::metadata_dir()
        .join("catalog")
        .join("tools.toml")
}

/// 检查标记路径：与目标同目录的 `.last-sync`（记上次检查时刻与锚，不碰 catalog 文件 mtime）。
fn marker_path(target: &Path) -> Option<PathBuf> {
    target.parent().map(|d| d.join(".last-sync"))
}

/// 云端清单 URL（带锚击穿 query：锚变缓存键变，锚同则缓存对象必与锚一致）。
fn cloud_catalog_url(key: &str, sha: &str) -> String {
    crate::download::with_query(
        &format!("{}/{key}", crate::download::MIRROR_BASE),
        &format!("v={sha}"),
    )
}

/// 云端边车锚 URL（每次回源的时间戳击穿由调用方补 query）。
fn cloud_sidecar_url(key: &str) -> String {
    format!("{}/{key}.sha256", crate::download::MIRROR_BASE)
}

/// 云端签名件 URL（minisign 惯例：`<清单>.minisig`）。
fn cloud_signature_url(key: &str) -> String {
    format!("{}/{key}.minisig", crate::download::MIRROR_BASE)
}

/// 云端 manifest 资产 URL（R016 两件分离：与 catalog 同批同签；锚击穿 query 由调用方加）。
/// 注意 `MIRROR_BASE` 自带 scheme，一律 `{}/{key}` 形态拼，别再加前缀（M019 双 scheme 教训）。
fn cloud_manifest_url(key: &str) -> String {
    format!("{}/{key}", crate::download::MIRROR_BASE)
}

/// 云端 manifest 边车锚 URL。
fn cloud_manifest_sidecar_url(key: &str) -> String {
    format!("{}/{key}.sha256", crate::download::MIRROR_BASE)
}

/// 云端 manifest 签名件 URL。
fn cloud_manifest_signature_url(key: &str) -> String {
    format!("{}/{key}.minisig", crate::download::MIRROR_BASE)
}

/// 清单的分离签名路径（`<清单>.minisig`）。
pub fn signature_path(catalog: &Path) -> PathBuf {
    let mut p = catalog.as_os_str().to_os_string();
    p.push(".minisig");
    PathBuf::from(p)
}

/// 清单签名状态（D34）：valid 通过内嵌公钥验签；invalid 有签名但验不过；missing 无签名件。
#[derive(Debug, PartialEq, Eq)]
pub enum SignatureState {
    /// 验签通过
    Valid,
    /// 有签名但验不过（含原因）
    Invalid(String),
    /// 无签名件
    Missing,
}

impl SignatureState {
    /// 命令面词（数据块 `signature` 字段值）。
    pub fn label(&self) -> &'static str {
        match self {
            SignatureState::Valid => "valid",
            SignatureState::Invalid(_) => "invalid",
            SignatureState::Missing => "missing",
        }
    }
}

/// 用内嵌公钥集合验签（任一公钥通过即可，供密钥轮换过渡期使用；纯函数可测）。
///
/// # Errors
/// 返回 Err（人读原因串）当：清单签名校验不过: {last} 等（完整失败面见函数体错误构造）。
pub fn verify_with_embedded_keys(data: &[u8], sig_text: &str) -> Result<(), String> {
    let sig = minisign_verify::Signature::decode(sig_text)
        .map_err(|e| format!("签名件格式不合法: {e}"))?;
    let mut last = String::new();
    for pk_b64 in CLOUD_CATALOG_PUBKEYS {
        let pk = minisign_verify::PublicKey::from_base64(pk_b64)
            .map_err(|e| format!("内嵌公钥不合法: {e}"))?;
        match pk.verify(data, &sig, false) {
            Ok(()) => return Ok(()),
            Err(e) => last = e.to_string(),
        }
    }
    Err(format!("清单签名校验不过: {last}"))
}

/// 校验磁盘清单与其分离签名（`<清单>.minisig`）。
pub fn check_signature(catalog: &Path) -> SignatureState {
    let sig_path = signature_path(catalog);
    let (Ok(data), Ok(sig_text)) = (std::fs::read(catalog), std::fs::read_to_string(&sig_path))
    else {
        return SignatureState::Missing;
    };
    match verify_with_embedded_keys(&data, &sig_text) {
        Ok(()) => SignatureState::Valid,
        Err(e) => SignatureState::Invalid(e),
    }
}

/// 该路径是否为用户数据副本（运行态权威落点；签名巡检只对它告警）。
pub fn is_user_data_catalog(path: &Path) -> bool {
    normalize_path(path) == normalize_path(&user_data_catalog_path())
}

/// 取云端签名件到缓存（时间戳击穿，避免 CF 陈旧对象；键随命中的段）。
fn fetch_signature(env_root: &Path, key: &str) -> Result<PathBuf, String> {
    let url = crate::download::with_query(&cloud_signature_url(key), &format!("t={}", now_secs()));
    crate::download::download_fresh(env_root, "cloud-tools.toml.minisig", &url)
}

/// TTL 解析（纯函数）：离线优先，其次显式秒数（0 关），非法值回落默认。
pub fn resolve_ttl(ttl_env: Option<&str>, offline_env: Option<&str>) -> u64 {
    if offline_env.map(|v| v.trim() == "1").unwrap_or(false) {
        return 0;
    }
    match ttl_env.map(str::trim).filter(|v| !v.is_empty()) {
        None => DEFAULT_TTL_SECS,
        Some(v) => v.parse::<u64>().unwrap_or(DEFAULT_TTL_SECS),
    }
}

/// 当前 TTL（读 `ARK_CATALOG_TTL` / `ARK_OFFLINE`）。
pub fn auto_ttl() -> u64 {
    let ttl = crate::platform::env_var("ARK_CATALOG_TTL");
    let offline = crate::platform::env_var("ARK_OFFLINE");
    resolve_ttl(ttl.as_deref(), offline.as_deref())
}

/// 标记文件解析（纯函数）：`<unix 秒>\n<sha256>\n`。
pub fn parse_marker(text: &str) -> Option<(u64, String)> {
    let mut lines = text.lines();
    let at = lines.next()?.trim().parse::<u64>().ok()?;
    let sha = lines.next()?.trim();
    if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some((at, sha.to_uppercase()))
}

/// 标记文件文本（纯函数，与 parse_marker 对称；小写写入，读出一律大写）。
pub fn marker_text(at: u64, sha: &str) -> String {
    format!("{at}\n{}\n", sha.to_lowercase())
}

/// 是否需要联网比对（纯函数）：目标缺失、无标记、标记过期、或本地已被改写（标记锚与本地不符）。
pub fn needs_check(
    target_exists: bool,
    local_sha: Option<&str>,
    marker: Option<&(u64, String)>,
    now: u64,
    ttl: u64,
) -> bool {
    if !target_exists {
        return true;
    }
    let Some((at, sha)) = marker else {
        return true;
    };
    if now.saturating_sub(*at) >= ttl {
        return true;
    }
    match local_sha {
        Some(local) => !local.eq_ignore_ascii_case(sha),
        None => true,
    }
}

/// 解析面来源分类（纯函数，供 status 子功能报告）：userdata / repo / env / other。
pub fn classify_origin(
    path: &Path,
    user_data: &Path,
    cwd_catalog: Option<&Path>,
    env_catalog: Option<&Path>,
) -> &'static str {
    let same = |a: &Path, b: &Path| normalize_path(a) == normalize_path(b);
    if env_catalog.is_some_and(|e| same(path, e)) {
        return "env";
    }
    if same(path, user_data) {
        return "userdata";
    }
    if cwd_catalog.is_some_and(|c| same(path, c)) {
        return "repo";
    }
    "other"
}

/// 路径归一化比较（大小写不敏感、去尾分隔符）。
fn normalize_path(p: &Path) -> String {
    p.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_lowercase()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 文件 sha（大写）；不存在或读失败为 None。
fn file_sha(path: &Path) -> Option<String> {
    crate::download::sha256_file(path)
        .ok()
        .map(|s| s.to_uppercase())
}

fn read_marker(target: &Path) -> Option<(u64, String)> {
    let path = marker_path(target)?;
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| parse_marker(&t))
}

fn write_marker(target: &Path, at: u64, sha: &str) {
    if let Some(path) = marker_path(target) {
        let _ = std::fs::write(path, marker_text(at, sha));
    }
}

/// 单次短探键读序（404 短路不吃重试链；D41 C：过渡窗主键缺省时防探活拖慢命令）。
/// 返回命中键与锚（t 击穿单次取值即新鲜）。
fn probe_keys_single_shot(
    keys: [&'static str; 2],
    sidecar_of: impl Fn(&str) -> String,
) -> Result<(&'static str, String), String> {
    let mut last = String::new();
    for key in keys {
        let url = crate::download::with_query(&sidecar_of(key), &format!("t={}", now_secs()));
        match crate::download::fetch_text_short(&url, Duration::from_secs(PROBE_TIMEOUT_SECS))
            .and_then(|text| crate::download::parse_sidecar_sha(&text, &url))
        {
            Ok(sha) => return Ok((key, sha)),
            Err(e) => last = format!("{key}: {e}"),
        }
    }
    Err(format!("云端边车探活双键读序全败: {last}"))
}

/// 云端清单锚解析（键读序：先单次短探定键（404 短路），命中键再走完整下载链取权威锚
/// （重试加 curl 兜底）；命令面用）。时间戳击穿在 fetch 层（mirror_sidecar_anchor_fast，
/// 996d342 起）统一做，call-site 不双写（对线裁定：REQ-0004 原 RCA 不成立）。
fn resolve_cloud_sha(env_root: &Path) -> Result<(&'static str, String), String> {
    let (key, _) = probe_cloud_sha()?;
    let sha = crate::download::mirror_sidecar_sha(env_root, &cloud_sidecar_url(key))?;
    Ok((key, sha))
}

/// 云端清单锚（边车首 token，大写；走下载链重试与 curl 兜底，命令面用）。
///
/// # Errors
/// 返回 Err（人读原因串）当：{what} 回滚重放拒收: 拉到 seq {pulled} 低于已见 {seen}（镜像回滚或重放旧签名件；如确认回退请手动删 .{what}.seq 后重试） 等（完整失败面见函数体错误构造）。
pub fn cloud_sha(env_root: &Path) -> Result<String, String> {
    resolve_cloud_sha(env_root).map(|(_, sha)| sha)
}

/// 短超时探活取锚与命中键（自动路径用）：单次请求、无重试、不拖慢用户命令；键读序同上。
fn probe_cloud_sha() -> Result<(&'static str, String), String> {
    probe_keys_single_shot(cloud_catalog_keys(), cloud_sidecar_url)
}

/// 云端 manifest 边车锚探活（短超时、只读不落缓存；与 sync 前置同口径，键读序同上）。
fn probe_manifest_sha() -> Result<(&'static str, String), String> {
    probe_keys_single_shot(cloud_manifest_keys(), cloud_manifest_sidecar_url)
}

/// 顶层单调序号（回滚重放防护，S006 候选 B / D40）：签发侧每批 +1，
/// 端上记已见 seq，收到更低即拒收（防「重放旧但签名有效的清单对」与镜像桶回滚）。
/// 键位契约（omc 2026-09-11 落地）：tools.toml 与 manifest.toml 顶层 `seq = <int>`；
/// `generated_at`（ISO8601Z）为信息性非安全边界。缺 seq 视为 0（兼容首发前的件）。
///
/// # Errors
/// 返回 Err（人读原因串）当：{what} 回滚重放拒收: 拉到 seq {pulled} 低于已见 {seen}（镜像回滚或重放旧签名件；如确认回退请手动删 .{what}.seq 后重试） 等（完整失败面见函数体错误构造）。
pub fn toplevel_seq(path: &Path) -> Result<u64, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读清单失败: {e}"))?;
    let doc: toml_edit::DocumentMut = text
        .parse()
        .map_err(|e| format!("清单 TOML 解析失败（seq 读取）: {e}"))?;
    match doc.get("seq").and_then(|i| i.as_integer()) {
        Some(v) if v >= 0 => Ok(v as u64),
        _ => Ok(0),
    }
}

/// 已见 seq 记录（与各清单同目录 `.last-seq`，tools 与 manifest 各一；一行整数）。
fn seen_seq_path(target: &Path) -> Option<PathBuf> {
    let stem = target.file_name()?.to_string_lossy().to_string();
    target.parent().map(|d| d.join(format!(".{stem}.seq")))
}

fn read_seen_seq(target: &Path) -> u64 {
    seen_seq_path(target)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(0)
}

fn write_seen_seq(target: &Path, seq: u64) {
    if let Some(p) = seen_seq_path(target) {
        let _ = std::fs::write(p, seq.to_string());
    }
}

/// 已见 seq 地板（D41 C 回滚门基线复核）：目标记录与旧 ohmyenv 目录同名记录取 max
/// ——旧二进制并行期仍在旧位刷新水位，只看新位会留「已见仍低」的放行窗口；
/// 旧目录冻结判据与停段同源（存量机水位清零后旧位记录恒定）。纯函数便于测。
fn seen_seq_floor_in(target: &Path, legacy_counterpart: Option<&Path>) -> u64 {
    let own = read_seen_seq(target);
    match legacy_counterpart {
        Some(c) if c != target => std::cmp::max(own, read_seen_seq(c)),
        _ => own,
    }
}

/// 生产入口：旧目录同名记录参与地板。
fn seen_seq_floor(target: &Path) -> u64 {
    let counterpart = target
        .file_name()
        .and_then(|_| crate::platform::legacy_metadata_dir())
        .map(|old| {
            old.join("catalog")
                .join(target.file_name().unwrap_or_default())
        });
    seen_seq_floor_in(target, counterpart.as_deref())
}

/// 把**在位件**的 seq 记成已见基线（升级到 seq 感知引擎后，首次自动刷新/自举补记；
/// 无 seq 的旧件不建记录，避免留下恒 0 的空记录）。
fn record_seen_seq_from_local(target: &Path) {
    if let Ok(seq) = toplevel_seq(target) {
        if seq > 0 {
            write_seen_seq(target, seq);
        }
    }
}

/// seq 门（纯函数可测）：拉到 seq 低于已见即拒收（报错不降级）；等于幂等重放；
/// 大于即收（调用方落位后 write_seen_seq）。缺 seq 视 0。
///
/// # Errors
/// 返回 Err（人读原因串）当：{what} 回滚重放拒收: 拉到 seq {pulled} 低于已见 {seen}（镜像回滚或重放旧签名件；如确认回退请手动删 .{what}.seq 后重试） 等（完整失败面见函数体错误构造）。
pub fn seq_gate(pulled: u64, seen: u64, what: &str) -> Result<u64, String> {
    if pulled < seen {
        return Err(format!(
            "{what} 回滚重放拒收: 拉到 seq {pulled} 低于已见 {seen}（镜像回滚或重放旧签名件；如确认回退请手动删 .{what}.seq 后重试）"
        ));
    }
    Ok(pulled)
}

/// 已拉到缓存的云端清单（含分离签名件路径）。
pub struct CloudCatalog {
    /// 清单文件路径（回写与状态面定位）
    pub path: PathBuf,
    /// 分离签名件路径（minisig）
    pub sig_path: PathBuf,
    /// 清单内容 sha256 锚
    pub sha: String,
    /// 顶层 seq（回滚重放防护；缺省 0）
    pub seq: u64,
}

/// 按给定锚拉取云端清单到缓存（键读序：主键先、兼容键回落；任一键全链通过即返回），
/// 过 sha、解析、内嵌公钥验签三重验证（任一不过即拒收）。
///
/// # Errors
/// 返回 Err（人读原因串）当：云端清单双键读序全败（锚 {sha}）: {last} 等（完整失败面见函数体错误构造）。
pub fn fetch_with_anchor(env_root: &Path, sha: &str) -> Result<CloudCatalog, String> {
    // 短探定键先（404 短路不吃重试链，D41 C：过渡窗主键缺省防拖慢）；命中键单链拉取，
    // 失败落另一键自愈（锚不配即 sha 不符，同段成对不破）
    let ordered: Vec<&'static str> = match probe_cloud_sha() {
        Ok((hit, _)) => {
            let other = cloud_catalog_keys().into_iter().find(|k| *k != hit);
            match other {
                Some(o) => vec![hit, o],
                None => vec![hit],
            }
        }
        Err(_) => cloud_catalog_keys().to_vec(),
    };
    let mut last = String::new();
    for key in ordered {
        match fetch_with_anchor_keyed(env_root, key, sha) {
            Ok(cc) => return Ok(cc),
            Err(e) => last = format!("{key}: {e}"),
        }
    }
    Err(format!("云端清单双键读序全败（锚 {sha}）: {last}"))
}

/// 单键拉取（三重验证：sha、解析、验签）。
fn fetch_with_anchor_keyed(env_root: &Path, key: &str, sha: &str) -> Result<CloudCatalog, String> {
    let path = crate::download::download_fresh(
        env_root,
        "cloud-tools.toml",
        &cloud_catalog_url(key, sha),
    )?;
    let got = crate::download::sha256_file(&path)?;
    if !got.eq_ignore_ascii_case(sha) {
        return Err(format!("云端清单锚不符: 边车 {sha} 实拉 {got}"));
    }
    Catalog::load(&path)?;
    let sig_path = fetch_signature(env_root, key)?;
    let data = std::fs::read(&path).map_err(|e| format!("读云端清单失败: {e}"))?;
    let sig_text = std::fs::read_to_string(&sig_path)
        .map_err(|e| format!("读云端签名失败: {}: {e}", sig_path.display()))?;
    verify_with_embedded_keys(&data, &sig_text)
        .map_err(|e| format!("云端清单签名校验不过，拒绝落位: {e}"))?;
    let seq = toplevel_seq(&path)?;
    Ok(CloudCatalog {
        path,
        sig_path,
        sha: sha.to_uppercase(),
        seq,
    })
}

/// 取锚后拉取（sync 子功能用；锚与键同源解析）。
///
/// # Errors
/// 返回 Err（人读原因串）当：云端 manifest 锚不符: 边车 {sha} 实拉 {got} 等（完整失败面见函数体错误构造）。
pub fn fetch_cloud(env_root: &Path) -> Result<CloudCatalog, String> {
    let (_, sha) = resolve_cloud_sha(env_root)?;
    fetch_with_anchor(env_root, &sha)
}

/// 落位（先写同目录临时文件再替换，避免半截文件成为运行态）。
/// 临时名随目标名派生（catalog 与签名件各用各的 tmp，避免并发刷新时两种内容互串）。
fn tmp_path(target: &Path) -> PathBuf {
    match target.file_name() {
        Some(name) => target.with_file_name(format!("{}.tmp", name.to_string_lossy())),
        None => target.with_extension("tmp"),
    }
}

fn place(src: &Path, target: &Path) -> Result<(), String> {
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("建目录失败: {}: {e}", dir.display()))?;
    }
    let tmp = tmp_path(target);
    std::fs::copy(src, &tmp).map_err(|e| format!("写临时文件失败: {}: {e}", tmp.display()))?;
    if std::fs::rename(&tmp, target).is_err() {
        // 目标被占用或跨卷时退回复制覆盖：非原子，半截风险由签名巡检与解析校验兜底（会阻断并可由 sync 自愈）
        std::fs::copy(&tmp, target).map_err(|e| format!("落位失败: {}: {e}", target.display()))?;
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(())
}

/// current 判定（纯函数，D45）：sha 同锚**且**云端 seq 不高于已见才 current。
/// 锚探测（sha 边车）与 toml 本体拉取是两次网络交互，CF 残影下可能边车旧本体新：
/// 此时 seq 高于已见，按 current 走会丢弃已拉到的新档（本轮 zig 数据复验实锤的缺口）。
fn is_current(local_sha: Option<&str>, cloud_sha: &str, cloud_seq: u64, seen: u64) -> bool {
    local_sha.is_some_and(|l| l.eq_ignore_ascii_case(cloud_sha)) && cloud_seq <= seen
}

/// 子功能 sync 的核心：刷新到指定目标（命令面与自动路径共用；target 独立解析，便于沙盒测试）。
///
/// # Errors
/// 返回 Err（人读原因串）当：云端 manifest 锚不符: 边车 {sha} 实拉 {got} 等（完整失败面见函数体错误构造）。
pub fn sync_to(env_root: &Path, target: &Path, force: bool, ttl: u64) -> Result<Outcome, String> {
    if ttl == 0 && !force {
        return Ok(Outcome::Skipped("off"));
    }
    let now = now_secs();
    let local_sha = file_sha(target);
    if !force
        && !needs_check(
            target.exists(),
            local_sha.as_deref(),
            read_marker(target).as_ref(),
            now,
            ttl,
        )
    {
        return Ok(Outcome::Skipped("fresh"));
    }
    let cloud = fetch_cloud(env_root)?;
    // 回滚重放门（D40 加 D41 C 地板）：先过 seq 再谈内容（防重放旧签名件与镜像桶回滚；
    // 地板含旧 ohmyenv 目录水位，防旧二进制并行期窗口）
    seq_gate(cloud.seq, seen_seq_floor(target), "tools.toml")?;
    // CF 边缘残影第二道（D45，对岸三犯根治的端上消费面）：锚探测与 toml 拉取异源
    // （sha 边车旧、本体新时 sha 说 current），已拉到手的 seq 高于已见不得按 current 丢弃新档
    let in_sync = is_current(
        local_sha.as_deref(),
        &cloud.sha,
        cloud.seq,
        seen_seq_floor(target),
    );
    if in_sync {
        // 内容同锚：补签名件（本地可能缺，比如首次带签名上线或本地被改写后签名被封存）
        place(&cloud.sig_path, &signature_path(target))?;
    } else {
        place(&cloud.path, target)?;
        place(&cloud.sig_path, &signature_path(target))?;
    }
    write_marker(target, now, &cloud.sha);
    write_seen_seq(target, cloud.seq);
    // manifest 与 catalog 独立演进（R016 两件分离）：catalog 锚未变（含 --force 同步）
    // 也必须拉 manifest，否则 omc 单方面上线 manifest 后台端永远拿不到（走内建回退漂移）。
    // 失败只告警不拦目录同步（双轨供给不断），但绝不静默吞错（撤内建前补新鲜度门，R016 六）。
    if let Err(e) = sync_manifest_if_present(env_root, target) {
        eprintln!("[WARN] 云端 manifest 未同步，沿用本地版本（走内建回退）: {e}");
    }
    if in_sync {
        return Ok(Outcome::InSync { sha: cloud.sha });
    }
    Ok(Outcome::Updated { sha: cloud.sha })
}

/// 拉取云端 manifest 三件套（R016 D39）：边车锚、sha 比对、解析、minisign 验签全过才落位；
/// 云端无 manifest（404，未上线过渡期）静默跳过——双轨不破供给。失败只告警不拦 catalog 同步。
fn sync_manifest_if_present(env_root: &Path, tools_target: &Path) -> Result<(), String> {
    use crate::download::{download_fresh, sha256_file, with_query};
    // 键读序（D41 C）：单次短探定键（404 短路不吃重试链），命中键拉三件套
    let (mkey, sha) = match probe_manifest_sha() {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("[INFO] 云端 manifest 不可得（未上线或网络未通），跳过（R016 双轨）: {e}");
            return Ok(());
        }
    };
    let url = with_query(&cloud_manifest_url(mkey), &format!("v={sha}"));
    let path = download_fresh(env_root, "cloud-manifest.toml", &url)?;
    let got = sha256_file(&path)?;
    if !got.eq_ignore_ascii_case(&sha) {
        return Err(format!("云端 manifest 锚不符: 边车 {sha} 实拉 {got}"));
    }
    crate::manifest::parse(
        &std::fs::read_to_string(&path).map_err(|e| format!("读 manifest 失败: {e}"))?,
    )?;
    let sig = download_fresh(
        env_root,
        "cloud-manifest.toml.minisig",
        &with_query(
            &cloud_manifest_signature_url(mkey),
            &format!("t={}", now_secs()),
        ),
    )?;
    let sig_text =
        std::fs::read_to_string(&sig).map_err(|e| format!("读 manifest 签名失败: {e}"))?;
    let data = std::fs::read(&path).map_err(|e| format!("读 manifest 失败: {e}"))?;
    verify_with_embedded_keys(&data, &sig_text)
        .map_err(|e| format!("云端 manifest 签名校验不过，拒绝落位: {e}"))?;
    let target = crate::manifest::path_for(tools_target);
    // 回滚重放门（D40 加 D41 C 地板）：manifest 独立记已见 seq（地板含旧目录水位）
    let mseq = toplevel_seq(&path)?;
    seq_gate(mseq, seen_seq_floor(&target), "manifest.toml")?;
    place(&path, &target)?;
    place(&sig, &signature_path(&target))?;
    write_seen_seq(&target, mseq);
    eprintln!("[OK] manifest 已同步（schema 校验、验签与 seq {mseq} 通过）");
    Ok(())
}

/// 自动刷新（仅用户数据副本路径）：TTL 判定在联网之前；网络异常单次探活即退化，
/// 失败记退避标记（锚取本地值），TTL 内不再重试，保证不拖慢用户命令。
///
/// # Errors
/// 返回 Err（人读原因串）当：操作失败（见错误串） 等（完整失败面见函数体错误构造）。
pub fn auto_refresh(env_root: &Path) -> Result<Outcome, String> {
    let ttl = auto_ttl();
    if ttl == 0 {
        return Ok(Outcome::Skipped("off"));
    }
    let target = user_data_catalog_path();
    let now = now_secs();
    let local_sha = file_sha(&target);
    if !needs_check(
        target.exists(),
        local_sha.as_deref(),
        read_marker(&target).as_ref(),
        now,
        ttl,
    ) {
        // manifest 有独立时效：tools 锚不变时 manifest 可能已换（D39 共识①）。
        // fresh 判据用文件 mtime（无第二标记文件）；过期则只补拉 manifest。
        if manifest_stale(&target, now, ttl) {
            let _ = sync_manifest_if_present(env_root, &target);
        }
        return Ok(Outcome::Skipped("fresh"));
    }
    let cloud = match probe_cloud_sha() {
        Ok((_, sha)) => sha,
        Err(_) => {
            if let Some(local) = &local_sha {
                write_marker(&target, now, local);
            }
            return Ok(Outcome::Skipped("unreachable"));
        }
    };
    if local_sha
        .as_deref()
        .is_some_and(|l| l.eq_ignore_ascii_case(&cloud))
    {
        write_marker(&target, now, &cloud);
        // D40：在位件即已见基线，首次自动刷新即补记（否则记录停在 0，回滚会被放行）
        record_seen_seq_from_local(&target);
        let _ = sync_manifest_if_present(env_root, &target);
        return Ok(Outcome::InSync { sha: cloud });
    }
    let fetched = fetch_with_anchor(env_root, &cloud)?;
    // D40：自动路径同样先过门再落位——否则镜像回滚走默认路径就进来了（显式 sync 有门，
    // 自动刷新是最常走的路径）。拒收后打退避标记，避免每命令重探重报。
    if let Err(e) = seq_gate(fetched.seq, seen_seq_floor(&target), "tools.toml") {
        eprintln!("[WARN] {e}");
        if let Some(local) = &local_sha {
            write_marker(&target, now, local);
        }
        return Ok(Outcome::Skipped("rollback"));
    }
    place(&fetched.path, &target)?;
    place(&fetched.sig_path, &signature_path(&target))?;
    write_marker(&target, now, &fetched.sha);
    write_seen_seq(&target, fetched.seq);
    let _ = sync_manifest_if_present(env_root, &target);
    Ok(Outcome::Updated { sha: fetched.sha })
}

/// manifest 是否过期（D39 共识①）：无第二标记文件，以 mtime 对 TTL 判；
/// 文件缺失视为过期（首拉）；mtime 取不到视为不过期（保守少拉，四门兜底内容安全）。
fn manifest_stale(tools_target: &Path, now: u64, ttl: u64) -> bool {
    if ttl == 0 {
        return false;
    }
    let Some(dir) = tools_target.parent() else {
        return false;
    };
    let path = dir.join("manifest.toml");
    let Ok(meta) = std::fs::metadata(&path) else {
        return true;
    };
    let Ok(mt) = meta.modified() else {
        return false;
    };
    let age = mt
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(age) >= ttl
}

/// 命令入口接线：解析面**就是**用户数据副本时按 TTL 刷新；跳过与失败都不拦命令。
pub fn auto_refresh_if_user_data(env_root: &Path, resolved: &Path) {
    if normalize_path(resolved) != normalize_path(&user_data_catalog_path()) {
        return;
    }
    if let Ok(Outcome::Updated { sha }) = auto_refresh(env_root) {
        let short = &sha[..sha.len().min(8)];
        eprintln!(
            "[OK] catalog 已刷新: {}（云端 {short}）",
            resolved.display()
        );
    }
}

/// 子功能 status 的数据面：解析面路径与来源、本地与云端锚、检查年龄、TTL 与离线态。
pub struct CatalogState {
    /// 清单文件路径（回写与状态面定位）
    pub path: PathBuf,
    /// 解析面来源标签（repo/userdata/cloud 等）
    pub origin: &'static str,
    /// 本地副本 sha256 锚
    pub local_sha: Option<String>,
    /// 云端 sha256 锚（边车探测）
    pub cloud_sha: Option<String>,
    /// 云端探测失败原因（含则未达）
    pub cloud_error: Option<String>,
    /// 距上次同步秒数（None 未同步过）
    pub age_secs: Option<u64>,
    /// 自动刷新 TTL 秒数
    pub ttl_secs: u64,
    /// 离线开关态（ARK_OFFLINE）
    pub offline: bool,
    /// 同步新鲜标记（.last-sync）
    pub synced: bool,
    /// 解析面清单的独立签名状态（D34，本地校验）。
    pub signature: SignatureState,
    /// manifest 面（R016 六节新鲜度门：撤内建后配置真空面靠它可见）。
    pub manifest: ManifestState,
}

/// manifest 面状态：在位与本地锚、年龄、云端锚对比与签名态（两件各自可诊断）。
pub struct ManifestState {
    /// 清单文件路径（回写与状态面定位）
    pub path: PathBuf,
    /// manifest 在位性
    pub present: bool,
    /// 本地副本 sha256 锚
    pub local_sha: Option<String>,
    /// 云端 sha256 锚（边车探测）
    pub cloud_sha: Option<String>,
    /// 云端探测失败原因（含则未达）
    pub cloud_error: Option<String>,
    /// 距上次同步秒数（None 未同步过）
    pub age_secs: Option<u64>,
    /// 同步新鲜标记（.last-sync）
    pub synced: bool,
    /// manifest 独立签名状态（sync 只落验签过的件，故 invalid 意味着本地被改过）。
    pub signature: SignatureState,
}

/// manifest 面采集（纯函数可测：云端锚探活结果由调用方注入）。
fn manifest_state_from(path: &Path, cloud: Result<String, String>) -> ManifestState {
    let now = now_secs();
    let local_sha = file_sha(path);
    let age_secs = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| now.saturating_sub(d.as_secs()));
    let (cloud_sha, cloud_error) = match cloud {
        Ok(s) => (Some(s), None),
        Err(e) => (None, Some(e)),
    };
    let synced = match (&local_sha, &cloud_sha) {
        (Some(l), Some(c)) => l.eq_ignore_ascii_case(c),
        _ => false,
    };
    ManifestState {
        path: path.to_path_buf(),
        present: path.exists(),
        local_sha,
        cloud_sha,
        cloud_error,
        age_secs,
        synced,
        signature: check_signature(path),
    }
}

/// 子功能 status 采集（云端不可达时如实标注 error 字段，不报错退出）。
pub fn catalog_state(env_root: &Path, resolved: &Path) -> CatalogState {
    let user_data = user_data_catalog_path();
    let cwd_catalog = std::env::current_dir()
        .ok()
        .map(|c| c.join("catalog").join("tools.toml"));
    let env_catalog = crate::platform::env_var("ARK_CATALOG").map(PathBuf::from);
    let local_sha = file_sha(resolved);
    let ttl_secs = auto_ttl();
    let now = now_secs();
    let age_secs = read_marker(&user_data)
        .map(|(at, _)| now.saturating_sub(at))
        .or_else(|| {
            std::fs::metadata(&user_data)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| now.saturating_sub(d.as_secs()))
        });
    let (cloud_sha, cloud_error) = match cloud_sha(env_root) {
        Ok(s) => (Some(s), None),
        Err(e) => (None, Some(e)),
    };
    let synced = match (&local_sha, &cloud_sha) {
        (Some(l), Some(c)) => l.eq_ignore_ascii_case(c),
        _ => false,
    };
    // manifest 面同源采集：catalog 探活已失败（云端整体不可达）就不重复探 manifest，省一次超时
    let manifest_cloud = match &cloud_error {
        Some(e) => Err(format!("云端不可达（随 catalog 探活）: {e}")),
        None => probe_manifest_sha().map(|(_, sha)| sha),
    };
    CatalogState {
        path: resolved.to_path_buf(),
        origin: classify_origin(
            resolved,
            &user_data,
            cwd_catalog.as_deref(),
            env_catalog.as_deref(),
        ),
        local_sha,
        cloud_sha,
        cloud_error,
        age_secs,
        ttl_secs,
        offline: ttl_secs == 0,
        synced,
        signature: check_signature(resolved),
        manifest: manifest_state_from(&crate::manifest::path_for(resolved), manifest_cloud),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 夹具原文即期望值来源（R001 字段契约的实例化），避免重言式断言。
    const FIXTURE: &str = include_str!("../tests/fixtures/tools.toml");

    fn fixture_catalog() -> Catalog {
        Catalog::parse(FIXTURE, PathBuf::from("tests/fixtures/tools.toml")).expect("夹具应能解析")
    }

    /// D49 尾修：逗号串逐段校验（status HINT 文案 join(",") 复制即跑）、保序去重、空段滤除。
    #[test]
    fn 选择_逗号串拆分与去重() {
        let cat = fixture_catalog();
        let got = cat.select("python,python, vault ,,").expect("逗号串应拆分");
        assert_eq!(
            got,
            vec!["python".to_string(), "vault".to_string()],
            "去重保序滤空"
        );
        assert!(
            cat.select("python,not-exist").is_err(),
            "任一段未知整体拒绝"
        );
    }

    /// D45：current 判定的 seq 消费——同锚且 seq 不高于已见才 current（CF 残影下
    /// 边车旧本体新时 seq 高于已见，不得丢弃新档）。
    #[test]
    fn current判定_sha同锚还要seq不高于已见() {
        let sha = "AA";
        assert!(is_current(Some("aa"), sha, 5, 5), "同锚同 seq：current");
        assert!(
            is_current(Some("aa"), sha, 4, 5),
            "同锚 seq 低（回放已被门拦，双保险）：current"
        );
        assert!(
            !is_current(Some("aa"), sha, 6, 5),
            "同锚但 seq 高：边车旧本体新，不得按 current 丢新档"
        );
        assert!(!is_current(Some("BB"), sha, 5, 5), "异锚：不 current");
        assert!(!is_current(None, sha, 5, 5), "本地无件：不 current");
    }

    #[test]
    fn catalog_candidates_按根目录顺序生成() {
        let roots = vec![
            PathBuf::from("repo-root"),
            PathBuf::from("cwd"),
            PathBuf::from("user-data"),
        ];
        let got = catalog_candidates(&roots);
        assert_eq!(
            got,
            vec![
                PathBuf::from("repo-root")
                    .join("catalog")
                    .join("tools.toml"),
                PathBuf::from("cwd").join("catalog").join("tools.toml"),
                PathBuf::from("user-data")
                    .join("catalog")
                    .join("tools.toml"),
            ]
        );
    }

    #[test]
    fn hold字段_解析与判定() -> Result<(), String> {
        let toml = "[tools.x]\nversion = \"1.0\"\nhold = true\n\n[tools.y]\nversion = \"2.0\"\n";
        let cat = Catalog::parse(toml, PathBuf::from("synthetic.toml"))?;
        assert!(cat.tool("x")?.is_held(), "hold=true 应判定锁定");
        assert!(!cat.tool("y")?.is_held(), "缺省不锁定");
        Ok(())
    }

    #[test]
    fn catalog_解析_取第一个存在的候选() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let miss = dir.path().join("miss");
        let hit = dir.path().join("hit");
        std::fs::create_dir_all(hit.join("catalog")).map_err(|e| e.to_string())?;
        std::fs::write(hit.join("catalog").join("tools.toml"), "").map_err(|e| e.to_string())?;
        let got = catalog_candidates(&[miss.clone(), hit.clone(), miss.clone()])
            .into_iter()
            .find(|p| p.exists());
        assert_eq!(got, Some(hit.join("catalog").join("tools.toml")));
        Ok(())
    }

    #[test]
    fn catalog_夹具解析_三工具保序且字段齐全() {
        let cat = fixture_catalog();
        // 节序即七类类序：python(runtime) → vault(service) → age(cli)
        assert_eq!(cat.order, vec!["python", "vault", "age"]);

        let age = cat.tool("age").expect("age 应存在");
        assert_eq!(age.repo.as_deref(), Some("FiloSottile/age"));
        assert_eq!(age.tag_prefix.as_deref(), Some("v"));
        assert_eq!(age.tag.as_deref(), Some("v1.3.1"));
        assert_eq!(age.version.as_deref(), Some("1.3.1"));
        assert_eq!(age.asset.as_deref(), Some("age-v1.3.1-windows-amd64.zip"));
        assert_eq!(
            age.sha256.as_deref(),
            Some("C56E8CE22F7E80CB85AD946CC82D198767B056366201D3E1A2B93D865BE38154")
        );
        // 平台分列 pin：三套并存、互不覆盖（R001 平台边界）
        assert_eq!(
            age.linux_asset.as_deref(),
            Some("age-v1.3.1-linux-amd64.tar.gz")
        );
        assert_eq!(
            age.mac_sha256.as_deref(),
            Some("01120EA2CBF0463D4C6BD767F99F3271BBED1CDC8A9AA718A76BA1FE4F01998B")
        );

        let vault = cat.tool("vault").expect("vault 应存在");
        assert_eq!(
            vault.cdn_index_url.as_deref(),
            Some("https://releases.hashicorp.com/vault/index.json")
        );
        assert!(vault.repo.is_none(), "纯 cdn 工具无 repo 字段");

        let python = cat.tool("python").expect("python 应存在");
        assert_eq!(
            python.version_pattern.as_deref(),
            Some("cpython-([0-9.]+)\\+")
        );
    }

    /// 三字段同设时各平台取值（期望值即 R001 平台边界契约：mac 专属 > linux > 通用）。
    fn chain_tool() -> Tool {
        Tool {
            repo: Some("o/win".to_string()),
            linux_repo: Some("o/lin".to_string()),
            mac_repo: Some("o/mac".to_string()),
            ..Tool::default()
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn mac_字段族_优先于linux与通用_缺失逐级回退() {
        let t = chain_tool();
        assert_eq!(t.repo(), Some("o/mac"), "mac 三字段同设应取 mac 专属");
        let no_mac = Tool {
            mac_repo: None,
            ..chain_tool()
        };
        assert_eq!(no_mac.repo(), Some("o/lin"), "mac 缺失应回退 linux");
        assert_eq!(
            chain_tool().platform_exe(),
            None,
            "无专属 exe 字段时 platform_exe 为 None"
        );
    }

    #[test]
    #[cfg(all(not(windows), not(target_os = "macos")))]
    fn linux_字段族_优先于通用_mac字段不生效() {
        let t = chain_tool();
        assert_eq!(t.repo(), Some("o/lin"), "linux 下 linux 字段优先于通用");
        assert_eq!(t.platform_exe(), None);
    }

    #[test]
    #[cfg(windows)]
    fn windows_仅通用字段_平台专属族不生效() {
        let t = chain_tool();
        assert_eq!(t.repo(), Some("o/win"), "Windows 下只认通用字段");
        assert_eq!(t.platform_exe(), None);
    }

    #[test]
    fn select_all_展开为全部工具_按书写顺序() {
        let cat = fixture_catalog();
        assert_eq!(cat.select("all").expect("all 应可选"), cat.order);
    }

    #[test]
    fn dies_select_未知工具() {
        let cat = fixture_catalog();
        let err = cat.select("nonexistent").expect_err("未知工具应报错");
        assert!(err.contains("未知工具"), "错误应含提示: {err}");
    }

    #[test]
    fn env_root_显式参数优先_并裁尾斜杠() {
        let root = resolve_env_root(Some(r"E:\env\")).expect("显式参数应生效");
        assert_eq!(root, PathBuf::from(r"E:\env"));
    }

    #[test]
    fn env_root_ark_root覆盖与撤销() {
        // ARK_ROOT 在位即覆盖；撤销后回落平台默认（旧名读回已撤，2026-09-18）
        std::env::set_var("ARK_ROOT", "/tmp/ark-root-primary");
        assert_eq!(
            resolve_env_root(None).expect("应可解析"),
            PathBuf::from("/tmp/ark-root-primary"),
            "ARK_ROOT 在位即覆盖"
        );
        std::env::remove_var("ARK_ROOT");
    }

    /// 回写后重读文档树：字段顺序应与原文件一致（sha256 被版本变更删除除外）。
    fn key_order(text: &str, tool: &str) -> Vec<String> {
        let doc: DocumentMut = text.parse().expect("TOML 应能解析");
        let table = doc["tools"][tool].as_table_like().expect("工具节应为表");
        table.iter().map(|(k, _)| k.to_string()).collect()
    }

    /// 夹具 age 的当前平台 pin sha 键与旧值（期望值取自夹具文件，三平台各取其一）。
    fn age_pin_sha_old() -> (&'static str, &'static str) {
        #[cfg(windows)]
        {
            (
                "sha256",
                "C56E8CE22F7E80CB85AD946CC82D198767B056366201D3E1A2B93D865BE38154",
            )
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            (
                "linux_sha256",
                "BDC69C09CBDD6CF8B1F333D372A1F58247B3A33146406333E30C0F26E8F51377",
            )
        }
        #[cfg(target_os = "macos")]
        {
            (
                "mac_sha256",
                "01120EA2CBF0463D4C6BD767F99F3271BBED1CDC8A9AA718A76BA1FE4F01998B",
            )
        }
    }

    #[test]
    fn pin_回写保序保注释_版本变更清sha() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let path = dir.path().join("tools.toml");
        fs::write(&path, FIXTURE).map_err(|e| e.to_string())?;

        let res = Resolution {
            tool: "age".to_string(),
            tag: "v1.4.0".to_string(),
            version: "1.4.0".to_string(),
            asset_name: "age-v1.4.0-windows-amd64.zip".to_string(),
            asset_size: 0,
            asset_url: "https://example.invalid/age.zip".to_string(),
            shasums_url: None,
            official_sha256: None,
            fallback_url: None,
        };
        let changed = write_pin(&path, "age", &res)?;
        assert!(changed, "1.3.1 到 1.4.0 应判定为版本变更");

        let (sha_key, sha_old) = age_pin_sha_old();
        let out = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        // 注释（文件头与 sha256 上方的行注释）应保留
        assert!(out.contains("# 测试夹具"), "文件头注释应保留");
        assert!(out.contains("# age    = GitHub release"), "节上注释应保留");
        assert!(!out.contains(sha_old), "版本变更后本平台旧 sha256 应被删除");
        assert!(out.contains("version = \"1.4.0\""), "version 应被更新");
        assert!(out.contains("tag = \"v1.4.0\""), "tag 应被更新");

        // 字段顺序：本平台 sha256 被删除，其余键顺序不变
        let before: Vec<String> = key_order(FIXTURE, "age")
            .into_iter()
            .filter(|k| k != sha_key)
            .collect();
        assert_eq!(key_order(&out, "age"), before, "回写不应打乱字段顺序");
        // 其它工具节完全不动
        assert!(out.contains("version = \"3.12.11\""), "python 节不应受影响");
        Ok(())
    }

    #[test]
    fn pin_同版本repin_保留sha256() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let path = dir.path().join("tools.toml");
        fs::write(&path, FIXTURE).map_err(|e| e.to_string())?;

        let res = Resolution {
            tool: "age".to_string(),
            tag: "v1.3.1".to_string(),
            version: "1.3.1".to_string(),
            asset_name: "age-v1.3.1-windows-amd64.zip".to_string(),
            asset_size: 0,
            asset_url: "https://example.invalid/age.zip".to_string(),
            shasums_url: None,
            official_sha256: None,
            fallback_url: None,
        };
        let changed = write_pin(&path, "age", &res)?;
        assert!(!changed, "同版本 re-pin 不算变更");

        let (_, sha_old) = age_pin_sha_old();
        let out = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        assert!(out.contains(sha_old), "同版本 re-pin 应保留本平台 sha256");
        Ok(())
    }

    #[test]
    fn pin_回写保持原文件crlf行尾() -> Result<(), String> {
        // 夹具检出行尾随平台 autocrlf 变化，先归一 LF 再转 CRLF（同 write_pin 内 314 行惯用法）
        let src = FIXTURE.replace("\r\n", "\n").replace('\n', "\r\n");
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let path = dir.path().join("tools.toml");
        fs::write(&path, &src).map_err(|e| e.to_string())?;

        let res = Resolution {
            tool: "age".to_string(),
            tag: "v1.4.0".to_string(),
            version: "1.4.0".to_string(),
            asset_name: "age-v1.4.0-windows-amd64.zip".to_string(),
            asset_size: 0,
            asset_url: "https://example.invalid/age.zip".to_string(),
            shasums_url: None,
            official_sha256: None,
            fallback_url: None,
        };
        write_pin(&path, "age", &res)?;

        let out = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let bare_lf = out.replace("\r\n", "").contains('\n');
        assert!(!bare_lf, "CRLF 文件回写后不应混入裸 LF");
        assert!(out.contains("version = \"1.4.0\"\r\n"), "内容应正常更新");
        Ok(())
    }

    #[test]
    fn pin_未pin工具_追加pin字段到节尾() -> Result<(), String> {
        // 未 pin 的工具没有 tag/version/asset 行（R001：可选字段为空整行省略）
        let src = "[tools.demo]\nrepo = \"owner/demo\"\ntag_prefix = \"v\"\n";
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let path = dir.path().join("tools.toml");
        fs::write(&path, src).map_err(|e| e.to_string())?;

        let res = Resolution {
            tool: "demo".to_string(),
            tag: "v2.0.0".to_string(),
            version: "2.0.0".to_string(),
            asset_name: "demo.zip".to_string(),
            asset_size: 0,
            asset_url: "https://example.invalid/demo.zip".to_string(),
            shasums_url: None,
            official_sha256: None,
            fallback_url: None,
        };
        write_pin(&path, "demo", &res)?;

        let out = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        assert_eq!(
            key_order(&out, "demo"),
            vec![
                "repo".to_string(),
                "tag_prefix".to_string(),
                pin_key("tag"),
                pin_key("version"),
                pin_key("asset")
            ],
            "本平台 pin 字段应追加在静态元数据之后"
        );
        Ok(())
    }
}

/// 云端清单两子功能的单测（D33）：TTL 解析、标记读写、联网判据、来源分类、动作词。
#[cfg(test)]
mod refresh_tests {
    use super::*;

    const SHA_A: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const SHA_B: &str = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";

    #[test]
    fn ttl解析_默认与关闭与离线() {
        assert_eq!(resolve_ttl(None, None), DEFAULT_TTL_SECS);
        assert_eq!(resolve_ttl(Some("0"), None), 0, "0 表示关闭自动刷新");
        assert_eq!(resolve_ttl(Some("600"), None), 600);
        assert_eq!(
            resolve_ttl(Some("abc"), None),
            DEFAULT_TTL_SECS,
            "非法值回落默认"
        );
        assert_eq!(resolve_ttl(Some("600"), Some("1")), 0, "offline=1 优先关闭");
        assert_eq!(resolve_ttl(None, Some("0")), DEFAULT_TTL_SECS, "离线只认 1");
    }

    #[test]
    fn 标记读写_对称且坏内容不采纳() {
        assert_eq!(
            parse_marker(&marker_text(1_700_000_000, SHA_A)),
            Some((1_700_000_000, SHA_A.to_string())),
            "小写写入大写读出"
        );
        assert_eq!(parse_marker(""), None);
        assert_eq!(parse_marker("123\n"), None, "缺 sha 行");
        assert_eq!(parse_marker("x\nabcd\n"), None, "时刻非法");
        assert_eq!(parse_marker("1\nzz\n"), None, "sha 非法");
    }

    #[test]
    fn 需要联网_四种判据() {
        let marker = (1000u64, SHA_A.to_string());
        assert!(needs_check(false, None, None, 1000, 60), "目标缺失即需检查");
        assert!(needs_check(true, None, None, 1000, 60), "无标记即需检查");
        assert!(
            !needs_check(true, Some(SHA_A), Some(&marker), 1030, 60),
            "标记新鲜且本地未改写则跳过"
        );
        assert!(
            needs_check(true, Some(SHA_A), Some(&marker), 1060, 60),
            "超过 TTL 需检查"
        );
        assert!(
            needs_check(true, Some(SHA_B), Some(&marker), 1030, 60),
            "本地被改写（self-deploy 或人为）即需检查"
        );
    }

    #[test]
    fn 来源分类_四态且大小写与尾分隔符不敏感() {
        let user_data = PathBuf::from(r"C:\data\ohmyenv\catalog\tools.toml");
        let cwd = PathBuf::from(r"D:\ohmyenv-rs\catalog\tools.toml");
        let env = PathBuf::from(r"C:\tmp\custom.toml");
        assert_eq!(
            classify_origin(
                Path::new(r"C:\DATA\ohmyenv\catalog\tools.toml\"),
                &user_data,
                Some(&cwd),
                None
            ),
            "userdata"
        );
        assert_eq!(classify_origin(&cwd, &user_data, Some(&cwd), None), "repo");
        assert_eq!(
            classify_origin(&env, &user_data, Some(&cwd), Some(&env)),
            "env"
        );
        assert_eq!(
            classify_origin(Path::new("/x/tools.toml"), &user_data, None, None),
            "other"
        );
    }

    #[test]
    fn 动作词与原因_命令面稳定() {
        assert_eq!(Outcome::Skipped("fresh").action(), "skipped");
        assert_eq!(Outcome::Skipped("fresh").reason(), "fresh");
        assert_eq!(Outcome::Skipped("unreachable").reason(), "unreachable");
        assert_eq!(Outcome::Updated { sha: SHA_A.into() }.action(), "updated");
        assert_eq!(Outcome::InSync { sha: SHA_A.into() }.action(), "current");
        assert_eq!(Outcome::InSync { sha: SHA_A.into() }.sha(), Some(SHA_A));
        assert_eq!(Outcome::Skipped("off").sha(), None);
    }

    #[test]
    fn manifest面_在位缺失与云端锚对比() {
        // R016 六节新鲜度门：撤内建后 manifest 面必须可诊断（在位、本地锚、年龄、云端锚、签名）
        let dir = tempfile::tempdir().expect("临时目录");
        let mpath = dir.path().join("manifest.toml");
        let missing = manifest_state_from(&mpath, Ok(SHA_A.to_string()));
        assert!(!missing.present, "缺文件应判不在位");
        assert!(missing.local_sha.is_none() && missing.age_secs.is_none());
        assert!(!missing.synced, "本地缺件不应判同锚");
        assert_eq!(missing.signature, SignatureState::Missing);
        // 在位且与云端锚一致
        std::fs::write(&mpath, "schema_version = 1\n").expect("写 manifest");
        let sha = file_sha(&mpath).expect("算 sha");
        let present = manifest_state_from(&mpath, Ok(sha.clone()));
        assert!(present.present && present.synced, "在位同锚应判同步");
        assert!(present.age_secs.is_some(), "在位应有年龄");
        assert_eq!(present.local_sha.as_deref(), Some(sha.as_str()));
        // 云端锚不同（omc 已发新版或本地被改）
        assert!(
            !manifest_state_from(&mpath, Ok(SHA_B.to_string())).synced,
            "锚不同不应判同步"
        );
        // 云端不可达：如实标 error，不 panic
        let unreachable = manifest_state_from(&mpath, Err("HTTP 请求失败".to_string()));
        assert!(unreachable.cloud_sha.is_none() && unreachable.cloud_error.is_some());
    }

    #[test]
    fn 云端键读序_主键ark先兼容ome后() {
        // D41 C：防软漂移机检——主键必须 ark/ 先、兼容键 ome/ 后
        let ck = cloud_catalog_keys();
        assert!(
            ck[0].starts_with("ark/catalog/") && ck[1].starts_with("ome/catalog/"),
            "catalog 键读序: {ck:?}"
        );
        let mk = cloud_manifest_keys();
        assert!(
            mk[0].starts_with("ark/catalog/") && mk[1].starts_with("ome/catalog/"),
            "manifest 键读序: {mk:?}"
        );
    }

    #[test]
    fn seq地板_旧目录水位参与且防御自指() {
        // D41 C：新旧目录并行期地板取 max（旧二进制仍在旧位写水位）；自指防御；无旧位取自身
        let dir = tempfile::tempdir().expect("临时目录");
        let new_cat = dir.path().join("ark").join("catalog").join("tools.toml");
        let old_cat = dir
            .path()
            .join("ohmyenv")
            .join("catalog")
            .join("tools.toml");
        std::fs::create_dir_all(new_cat.parent().expect("新目录")).expect("建新目录");
        std::fs::create_dir_all(old_cat.parent().expect("旧目录")).expect("建旧目录");
        std::fs::write(&new_cat, "x").expect("写新位");
        std::fs::write(&old_cat, "x").expect("写旧位");
        write_seen_seq(&new_cat, 3);
        write_seen_seq(&old_cat, 7);
        assert_eq!(
            seen_seq_floor_in(&new_cat, Some(&old_cat)),
            7,
            "旧水位高取 max（防并行期放行窗口）"
        );
        write_seen_seq(&new_cat, 9);
        assert_eq!(
            seen_seq_floor_in(&new_cat, Some(&old_cat)),
            9,
            "新水位高取自身"
        );
        assert_eq!(seen_seq_floor_in(&old_cat, Some(&old_cat)), 7, "自指不叠加");
        assert_eq!(seen_seq_floor_in(&new_cat, None), 9, "无旧位取自身");
    }

    #[test]
    fn seq门_缺省兼容与拒降级() {
        let dir = tempfile::tempdir().expect("临时目录");
        let cat = dir.path().join("tools.toml");
        std::fs::write(&cat, "seq = 7\n[tools.jq]\n").expect("写带 seq 清单");
        assert_eq!(toplevel_seq(&cat).expect("应读到 seq"), 7);
        // 缺 seq / 负值 / 非整数一律视 0（兼容首发前件与坏数据，不因读不到就报错）
        std::fs::write(&cat, "[tools.jq]\n").expect("写无 seq 清单");
        assert_eq!(toplevel_seq(&cat).expect("缺 seq 视 0"), 0);
        std::fs::write(&cat, "seq = -3\n").expect("写负 seq");
        assert_eq!(toplevel_seq(&cat).expect("负 seq 视 0"), 0);
        std::fs::write(&cat, "seq = \"7\"\n").expect("写字符串 seq");
        assert_eq!(toplevel_seq(&cat).expect("非整数 seq 视 0"), 0);
        assert!(
            toplevel_seq(&dir.path().join("nope.toml")).is_err(),
            "文件读不到应如实报错"
        );
        // 门：低拒（报错带出路提示）、等过（幂等重放）、高过
        let low = seq_gate(2, 7, "tools.toml").expect_err("回滚应拒收");
        assert!(
            low.contains("低于已见 7") && low.contains(".tools.toml.seq"),
            "报错应含已见值与出路: {low}"
        );
        assert_eq!(seq_gate(7, 7, "tools.toml").expect("同 seq 应放行"), 7);
        assert_eq!(seq_gate(9, 7, "tools.toml").expect("更高 seq 应放行"), 9);
        // 已见记录读写往返；坏记录按 0（fail-open，签名与锚仍在兜底，出路是不砖）
        write_seen_seq(&cat, 42);
        assert_eq!(read_seen_seq(&cat), 42);
        std::fs::write(seen_seq_path(&cat).expect("记录路径"), "not-a-number").expect("写坏记录");
        assert_eq!(read_seen_seq(&cat), 0, "坏记录按 0");
        // 在位件补记基线（升级到 seq 感知引擎后首刷补记的机制）
        std::fs::write(&cat, "seq = 7\n").expect("写带 seq 清单");
        let rec = seen_seq_path(&cat).expect("记录路径");
        std::fs::remove_file(&rec).expect("先删记录");
        record_seen_seq_from_local(&cat);
        assert_eq!(read_seen_seq(&cat), 7, "在位件 seq 应被记成基线");
        // 无 seq 的旧件不留下恒 0 记录
        std::fs::write(&cat, "[tools.jq]\n").expect("写无 seq 清单");
        std::fs::remove_file(&rec).expect("先删记录");
        record_seen_seq_from_local(&cat);
        assert!(!rec.exists(), "无 seq 旧件不应留下恒 0 记录");
    }

    /// 自检签名：内容 `ome-catalog-signature-selftest\n` 的 minisign 签名（2026-09-10 签署）。
    /// 消息与签名是被签内容对（私钥在 ohmycloud 侧），剔除批不改：改串须持钥重签。
    /// 用途：锁住「内嵌公钥加签名格式」这一对不漂移；换钥时本常量须同步更新（属预期红灯）。
    const SELFTEST_MSG: &str = "ome-catalog-signature-selftest\n";
    /// 注意 trusted comment 属被签内容（改它等于改签名），故此处逐字保留签署当时文本。
    const SELFTEST_SIG: &str = "\
untrusted comment: ome catalog signature: selftest.txt
RUQWI4x407+T+15MR2QUPmLELWU02ipckyrZjLgfDEYkHI41DYoEqT4VADuEQT2fiWvF9YoXvfMYDwU3oOdbJ7hfkRhB8pifRAs=
trusted comment: ome catalog signature: C:\\Users\\ray\\AppData\\Local\\Temp\\catalog-sign-test\\selftest.txt
FeN3CEmfyojZlc/nYDHD/JGL8Z+H9HoUj3KAq2lbtYuxMBqsTUiuenVbaqyyFM4L433njWZO45arpsiAAwzzAQ==";

    #[test]
    fn 验签_自检签名通过且篡改即失败() {
        assert!(
            verify_with_embedded_keys(SELFTEST_MSG.as_bytes(), SELFTEST_SIG).is_ok(),
            "内嵌公钥应能验过本仓签名"
        );
        let tampered = format!("{SELFTEST_MSG}# tamper\n");
        assert!(
            verify_with_embedded_keys(tampered.as_bytes(), SELFTEST_SIG).is_err(),
            "内容一改即验不过"
        );
        assert!(
            verify_with_embedded_keys(SELFTEST_MSG.as_bytes(), "not-a-signature").is_err(),
            "坏签名件即拒收"
        );
    }

    #[test]
    fn 内嵌公钥与仓库公钥文件一致() {
        let repo_pub = include_str!("../.tools/catalog-sign/catalog-signing.pub");
        assert!(
            repo_pub.contains(CLOUD_CATALOG_PUBKEYS[0]),
            "仓库公钥文件与内嵌公钥必须一致（换钥需两处同步）"
        );
        assert!(
            repo_pub.contains(CLOUD_CATALOG_PUBKEY_ID),
            "key id 标注需与仓库公钥文件一致"
        );
    }

    #[test]
    fn 签名路径与状态_命名与缺件() {
        let dir = tempfile::tempdir().expect("建沙盒失败");
        let cat = dir.path().join("tools.toml");
        std::fs::write(&cat, "x").expect("写文件失败");
        assert_eq!(
            signature_path(&cat).file_name().unwrap().to_string_lossy(),
            "tools.toml.minisig"
        );
        assert_eq!(
            check_signature(&cat),
            SignatureState::Missing,
            "无签名件即 missing"
        );
        assert_eq!(SignatureState::Missing.label(), "missing");
        assert_eq!(SignatureState::Valid.label(), "valid");
    }

    #[test]
    fn 落位临时名_随目标派生且落位不留残件() {
        let dir = tempfile::tempdir().expect("建沙盒失败");
        let cat = dir.path().join("tools.toml");
        let sig = signature_path(&cat);
        assert_ne!(
            tmp_path(&cat),
            tmp_path(&sig),
            "清单与签名件的临时名必须不同，避免并发刷新时两种内容互串"
        );
        assert_eq!(
            tmp_path(&cat).file_name().unwrap().to_string_lossy(),
            "tools.toml.tmp"
        );
        assert_eq!(
            tmp_path(&sig).file_name().unwrap().to_string_lossy(),
            "tools.toml.minisig.tmp"
        );
        let src = dir.path().join("src.bin");
        std::fs::write(&src, b"payload").expect("写源失败");
        place(&src, &cat).expect("落位清单失败");
        place(&src, &sig).expect("落位签名件失败");
        assert_eq!(std::fs::read(&cat).unwrap(), b"payload");
        assert_eq!(std::fs::read(&sig).unwrap(), b"payload");
        let leftovers: Vec<String> = std::fs::read_dir(dir.path())
            .expect("读目录失败")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "落位后不应留临时件: {leftovers:?}");
    }
}
