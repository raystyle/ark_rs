//! status：三态对照（locked/installed/path）。
//! 语义：
//! msi 与 official（exe 含 %）走环境展开，其余 Join EnvRoot；installed 实跑 exe 探测；
//! path 查用户 PATH 原始值（不展开比较，大小写不敏感）。

use std::path::{Path, PathBuf};

use crate::catalog::Catalog;
use crate::envpath;
use crate::toolver;

/// status 三态行：locked=pin version，installed=实跑探测，path=用户 PATH 是否含 bin。
#[derive(Clone)]
pub struct StatusRow {
    /// 工具名
    pub name: String,
    /// 分类（分组渲染与排序）
    pub category: String,
    /// 锁定版本（catalog pin；None 未锁）
    pub locked: Option<String>,
    /// 已装版本（探针实测；None 未探到）
    pub installed: Option<String>,
    /// PATH 在位性
    pub path: bool,
    /// 本平台无 effective exe（平台不适用，如 shellcheck 在 Windows）时为 None，渲染为 -。
    pub exe: Option<PathBuf>,
}

/// 收集全部工具三态（按 catalog 书写顺序）。
pub fn collect_status(cat: &Catalog, env_root: &Path) -> Result<Vec<StatusRow>, String> {
    collect_status_with(cat, env_root, |_| Ok(()))
}

/// 流式收集：每探完一个工具立即回调 on_row（status 命令逐行输出的关键——
/// 探测要逐工具拉起 `--version` 子进程，整批探完才打印会被感知为卡顿）；
/// verify 等纯数据消费方传空回调走 collect_status。
pub fn collect_status_with<F: FnMut(&StatusRow) -> Result<(), String>>(
    cat: &Catalog,
    env_root: &Path,
    mut on_row: F,
) -> Result<Vec<StatusRow>, String> {
    let mut rows = Vec::new();
    for name in &cat.order {
        let def = cat.tool(name)?;
        // 平台不适用（无本平台 exe，如 shellcheck 在 Windows 只有 linux 字段）：如实空态行，不探测
        if !toolver::platform_managed(def) {
            let row = StatusRow {
                name: name.clone(),
                category: def.category.clone().unwrap_or_default(),
                locked: def.pin_version().map(str::to_string),
                installed: None,
                path: false,
                exe: None,
            };
            on_row(&row)?;
            rows.push(row);
            continue;
        }
        let exe = toolver::exe_path(def, env_root)?;
        // agent 类存量纳管（D07）与 npm-tgz 型（bin 落 npm 全局 bin，装在用户位不在
        // EnvRoot）：探测位换 PATH 首个命中，在位即 installed 探版本、path=true、exe 指
        // 真实位；未命中回落 EnvRoot 位如实空态
        if def.category.as_deref() == Some("agent") || def.extract() == Some("npm-tgz") {
            // npm-tgz 的 bin 名可与工具名不同（browser-harness 的 bin 是 bh）：
            // 探测用 bin 字段；agent 类 bin 名即工具名
            let probe = def.bin().unwrap_or(name.as_str());
            if let Some(found) = toolver::find_on_path(probe) {
                let installed = toolver::installed_version(&found, def);
                let row = StatusRow {
                    name: name.clone(),
                    category: def.category.clone().unwrap_or_default(),
                    locked: def.pin_version().map(str::to_string),
                    installed,
                    path: true,
                    exe: Some(found),
                };
                on_row(&row)?;
                rows.push(row);
                continue;
            }
        }
        let installed = toolver::installed_version(&exe, def);
        // vsbuild 特判：PATH 在机器级（HKLM）而非用户 PATH，按 MSBuild 与 cl 目录全在判定
        if crate::vsbuild::is_vsbuild(def) {
            let dirs = crate::vsbuild::machine_path_dirs(env_root);
            let in_path = !dirs.is_empty()
                && dirs
                    .iter()
                    .all(|d| crate::platform::machine_path_contains(d).unwrap_or(false));
            let row = StatusRow {
                name: name.clone(),
                category: def.category.clone().unwrap_or_default(),
                locked: def.pin_version().map(str::to_string),
                installed,
                path: in_path,
                exe: Some(exe),
            };
            on_row(&row)?;
            rows.push(row);
            continue;
        }
        let is_official = toolver::is_official(def);
        // bin 目录：official 取 exe 上一级（展开后），其余 EnvRoot\bin 字段；无 bin 则 path=false
        let in_path = match (def.bin(), is_official) {
            (Some(_), true) => {
                let bin = exe
                    .parent()
                    .ok_or_else(|| format!("{name} official exe 无法取上级目录"))?;
                envpath::user_path_contains(bin)?
            }
            (Some(b), false) => {
                let raw = crate::platform::expand_install_path(b);
                // 相对路径拼到 EnvRoot 下（Windows 名录是相对 bin；Linux 多为 ~/ 绝对）
                let bin = if raw.is_absolute() {
                    raw
                } else {
                    env_root.join(raw)
                };
                envpath::user_path_contains(&bin)?
            }
            (None, _) => exe
                .parent()
                .map(envpath::user_path_contains)
                .transpose()?
                .unwrap_or(false),
        };
        let row = StatusRow {
            name: name.clone(),
            category: def.category.clone().unwrap_or_default(),
            locked: def.pin_version().map(str::to_string),
            installed,
            path: in_path,
            exe: Some(exe),
        };
        on_row(&row)?;
        rows.push(row);
    }
    Ok(rows)
}

/// 七类 taxonomy（catalog category 值 → 中文组名，展示序即数组序）。
/// 2026-09-02 用户裁决归类定稿（统一「依赖」后缀）：操作编排依赖（ome/oma/herdr 编排类 CLI，
/// oma 待 ohmyagents 集成）、运行时依赖（语言与子系统运行时及其管理器）、编译器依赖（语言工具链）、
/// 运行时衍生依赖（经运行时包管理器安装，如 uv tool 的 browser-harness；omcf 待 ohmycloud 集成）、
/// 多路复用依赖（rmux）、远程服务依赖（openssh/vault 客户端加服务端常驻）、命令工具依赖。
pub static GROUPS: &[(&str, &str)] = &[
    ("agent", "智能体依赖"),
    ("base", "操作编排依赖"),
    ("runtime", "运行时依赖"),
    ("runtime-manager", "运行时管理器依赖"),
    ("compiler", "编译器依赖"),
    ("derived", "运行时衍生依赖"),
    ("mux", "多路复用依赖"),
    ("service", "远程服务依赖"),
    ("security", "密钥安全管理"),
    ("cli", "命令工具依赖"),
];

/// 分类中文组名（九类为主，D07 起 agent 与运行时管理器入册；旧值兜底转换期防炸，未知值为空串）。
pub fn category_label(category: &str) -> &'static str {
    if let Some((_, label)) = GROUPS.iter().find(|(c, _)| *c == category) {
        return label;
    }
    match category {
        "key" => "密钥",
        "agent" => "智能体环境",
        "project" => "项目管理",
        "extras" => "扩展工具",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 十类分组_标签与展示序() {
        // 期望值来源：2026-09-02 七类定稿，2026-09-05 D07 加 agent 与 runtime-manager，
        // 2026-09-07 用户裁定 age/sops 独立密钥安全管理类、vault 移除（十类）
        assert_eq!(
            GROUPS,
            &[
                ("agent", "智能体依赖"),
                ("base", "操作编排依赖"),
                ("runtime", "运行时依赖"),
                ("runtime-manager", "运行时管理器依赖"),
                ("compiler", "编译器依赖"),
                ("derived", "运行时衍生依赖"),
                ("mux", "多路复用依赖"),
                ("service", "远程服务依赖"),
                ("security", "密钥安全管理"),
                ("cli", "命令工具依赖"),
            ]
        );
        assert_eq!(category_label("runtime"), "运行时依赖");
        assert_eq!(category_label("agent"), "智能体依赖");
        assert_eq!(category_label("runtime-manager"), "运行时管理器依赖");
        assert_eq!(category_label("security"), "密钥安全管理");
        assert_eq!(category_label("mux"), "多路复用依赖");
        // 旧值兜底与未知值空串（转换期防炸）
        assert_eq!(category_label("key"), "密钥");
        assert_eq!(category_label("ghost"), "");
    }

    /// 平台不适用工具（effective exe 缺失）出空态行不报错。
    /// 现实对应 shellcheck（仅 linux 字段）在 Windows 触发；用全平台无 exe 的 ghost 做可移植验证。
    #[test]
    fn status_平台不适用工具出空态行() -> Result<(), String> {
        let toml = "[tools.ghost]\ncategory = \"extras\"\nlinux_dir = \"~/.local/bin\"\n";
        let cat = Catalog::parse(toml, PathBuf::from("synthetic.toml"))?;
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let rows = collect_status(&cat, dir.path())?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "ghost");
        assert!(rows[0].installed.is_none(), "空态不探测");
        assert!(!rows[0].path);
        assert!(rows[0].exe.is_none(), "本平台无 exe 渲染为 -");
        assert!(!toolver::platform_managed(cat.tool("ghost")?));
        Ok(())
    }
}
