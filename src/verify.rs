//! verify：部署域验收维度（P0026 M3 第一批，数据驱动自 catalog 三态）。
//! 维度注册表按平台生效，判定三层：catalog 工具组（locked==installed，强于 ps1 存在性断言）、
//! 文件全在组（存在性，对齐 ps1 弱断言）、任一存在组（aria2 系统位）。
//! 输出 `dim=PASS/FAIL/NA` 收割行（NA 不参与收割）。
//! 密钥、secret-guard、mesh、compileMatrix 等非部署域不在本命令范围。

use std::path::Path;

use crate::catalog::Catalog;
use crate::status;
use crate::toolver;

/// 单个验收维度定义（静态注册表条目）。
struct DimDef {
    name: &'static str,
    /// Windows 端生效
    windows: bool,
    /// Linux / macOS 端生效
    posix: bool,
    /// catalog 工具组：组内全部本平台在管且 locked==installed 才 PASS；组内有平台不适用工具则 NA
    tools: &'static [&'static str],
    /// 文件全在组（存在性断言；支持 `~` 与 `{envroot}` 占位）
    files_all: &'static [&'static str],
    /// 任一存在组（如 aria2 的 apt 位与 brew 位）
    files_any: &'static [&'static str],
}

/// 部署域维度注册表。
static DIMS: &[DimDef] = &[
    // ── Windows 端 ──
    DimDef {
        name: "toolRoot",
        windows: true,
        posix: false,
        tools: &["sops", "herdr", "gh", "mq"],
        files_all: &[],
        files_any: &[],
    },
    DimDef {
        name: "bun",
        windows: true,
        posix: true,
        tools: &["bun"],
        files_all: &[],
        files_any: &[],
    },
    DimDef {
        name: "dev-go",
        windows: true,
        posix: false,
        tools: &["go"],
        files_all: &[],
        files_any: &[],
    },
    DimDef {
        name: "dev-zig",
        windows: true,
        posix: false,
        tools: &["zig"],
        files_all: &[],
        files_any: &[],
    },
    DimDef {
        name: "dev-fnm",
        windows: true,
        posix: false,
        tools: &["fnm"],
        files_all: &["{envroot}\\fnm\\node-versions"],
        files_any: &[],
    },
    DimDef {
        name: "dev-rust",
        windows: true,
        posix: false,
        tools: &[],
        files_all: &[
            "{envroot}\\cargo\\bin\\rustc.exe",
            "{envroot}\\rustup\\toolchains",
        ],
        files_any: &[],
    },
    DimDef {
        name: "vsbuild",
        windows: true,
        posix: false,
        tools: &[],
        files_all: &[
            "{envroot}\\vsbuild\\MSBuild\\Current\\Bin\\MSBuild.exe",
            "{envroot}\\vsbuild\\VC\\Tools\\MSVC",
        ],
        files_any: &[],
    },
    DimDef {
        name: "aria2",
        windows: true,
        posix: false,
        tools: &[],
        files_all: &[],
        files_any: &["{envroot}\\aria2\\aria2c.exe"],
    },
    DimDef {
        name: "rmux",
        windows: true,
        posix: false,
        tools: &["rmux"],
        files_all: &[],
        files_any: &[],
    },
    // ── POSIX 端 ──
    DimDef {
        name: "toolRoot",
        windows: false,
        posix: true,
        tools: &["sops", "herdr", "gh"],
        files_all: &[],
        files_any: &[],
    },
    DimDef {
        name: "go",
        windows: false,
        posix: true,
        tools: &["go"],
        files_all: &[],
        files_any: &[],
    },
    DimDef {
        name: "zig",
        windows: false,
        posix: true,
        tools: &["zig"],
        files_all: &[],
        files_any: &[],
    },
    DimDef {
        name: "dev-rust",
        windows: false,
        posix: true,
        tools: &[],
        files_all: &["~/.cargo/bin/rustc", "~/.rustup/toolchains"],
        files_any: &[],
    },
    DimDef {
        name: "localbin16",
        windows: false,
        posix: true,
        tools: &[],
        files_all: &[
            "~/.local/bin/age",
            "~/.local/bin/age-keygen",
            "~/.local/bin/sops",
            "~/.local/bin/gh",
            "~/.local/bin/uv",
            "~/.local/bin/uvx",
            "~/.local/bin/rg",
            "~/.local/bin/jq",
            "~/.local/bin/mq",
            "~/.local/bin/yq",
            "~/.local/bin/just",
            "~/.local/bin/herdr",
            "~/.local/bin/ast-grep",
        ],
        files_any: &[],
    },
    DimDef {
        name: "rmux",
        windows: false,
        posix: true,
        tools: &["rmux"],
        files_all: &[
            "~/.local/bin/rmux",
            "~/.local/bin/rmux-daemon",
            "~/.local/libexec/rmux/rmux",
        ],
        files_any: &[],
    },
    DimDef {
        name: "aria2",
        windows: false,
        posix: true,
        tools: &[],
        files_all: &[],
        files_any: &["/usr/bin/aria2c", "/opt/homebrew/bin/aria2c"],
    },
];

/// 全部维度名（含跨平台分列重复；heal 注册表对齐校验用）。
pub fn dim_names() -> Vec<&'static str> {
    DIMS.iter().map(|d| d.name).collect()
}

/// 维度判定结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// 维度通过
    Pass,
    /// 维度失败
    Fail,
    /// 本平台不适用
    Na,
}

impl Verdict {
    /// 判定短名（dim=PASS/FAIL/NA 收割行）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Pass => "PASS",
            Verdict::Fail => "FAIL",
            Verdict::Na => "NA",
        }
    }
}

/// 展开断言路径：`~` 前缀与 `{envroot}` 占位。
fn expand_probe(raw: &str, env_root: &Path) -> std::path::PathBuf {
    let s = raw.replace("{envroot}", &env_root.display().to_string());
    crate::platform::expand_install_path(&s)
}

/// 跑部署域验收（流式）：维度所需工具探完即经 emit 回调输出（保持注册表顺序中的可出即出），
/// 返回全部 (维度名, 判定)。filter 为空跑全部，否则只跑指定维度（未知维度报错）。
/// 探测逐工具拉起 --version 子进程，批量探完才输出会被感知为卡顿——与 status 同理走流式。
pub fn run_verify_with<F: FnMut(&str, Verdict) -> Result<(), String>>(
    cat: &Catalog,
    env_root: &Path,
    filter: &[String],
    mut emit: F,
) -> Result<Vec<(String, Verdict)>, String> {
    let on_windows = cfg!(windows);
    let active: Vec<&DimDef> = DIMS
        .iter()
        .filter(|d| {
            let applicable = if on_windows { d.windows } else { d.posix };
            applicable && (filter.is_empty() || filter.iter().any(|f| f == d.name))
        })
        .collect();
    // 行缓存：维度在其工具全部到齐后判定（保持注册表顺序，可出即出）
    let mut rows: Vec<status::StatusRow> = Vec::new();
    let mut out: Vec<(String, Verdict)> = Vec::new();
    let mut flush_ready =
        |rows: &[status::StatusRow], out: &mut Vec<(String, Verdict)>| -> Result<(), String> {
            // 多轮扫描：一个维度出列后，仅依赖文件组的更早维度可能随之可出
            loop {
                let mut progressed = false;
                for def in &active {
                    if out.iter().any(|(n, _)| n == def.name) {
                        continue;
                    }
                    let ready = def.tools.iter().all(|t| rows.iter().any(|r| r.name == *t));
                    if ready {
                        let v = judge(def, rows, cat, env_root);
                        emit(def.name, v)?;
                        out.push((def.name.to_string(), v));
                        progressed = true;
                    }
                }
                if !progressed {
                    break;
                }
            }
            Ok(())
        };
    status::collect_status_with(cat, env_root, |row: &status::StatusRow| {
        rows.push(row.clone());
        flush_ready(&rows, &mut out)
    })?;
    flush_ready(&rows, &mut out)?; // 收尾：文件组维度兜底出列
                                   // 兜底：维度含 catalog 缺失工具（row 永不到齐）时按 judge 判 NA 出列，不静默漏维度
    for def in &active {
        if !out.iter().any(|(n, _)| n == def.name) {
            let v = judge(def, &rows, cat, env_root);
            emit(def.name, v)?;
            out.push((def.name.to_string(), v));
        }
    }
    for want in filter {
        if !out.iter().any(|(n, _)| n == want) {
            return Err(format!("未知验收维度: {want}（当前平台注册表内不存在）"));
        }
    }
    Ok(out)
}

/// 跑部署域验收（非流式兼容口）：内部走空回调。
pub fn run_verify(
    cat: &Catalog,
    env_root: &Path,
    filter: &[String],
) -> Result<Vec<(String, Verdict)>, String> {
    run_verify_with(cat, env_root, filter, |_, _| Ok(()))
}

/// 单维度判定：工具组三态（组内有平台不适用工具则 NA）加文件组存在性。
fn judge(def: &DimDef, rows: &[status::StatusRow], cat: &Catalog, env_root: &Path) -> Verdict {
    for t in def.tools {
        let tool = match cat.tool(t) {
            Ok(td) => td,
            Err(_) => return Verdict::Na, // catalog 缺工具：数据漂移，不误报 FAIL
        };
        if !toolver::platform_managed(tool) {
            return Verdict::Na;
        }
        let Some(row) = rows.iter().find(|r| r.name == *t) else {
            return Verdict::Na;
        };
        let Some(installed) = &row.installed else {
            return Verdict::Fail;
        };
        if row.locked.as_deref() != Some(installed.as_str()) {
            return Verdict::Fail;
        }
    }
    for f in def.files_all {
        if !expand_probe(f, env_root).exists() {
            return Verdict::Fail;
        }
    }
    if !def.files_any.is_empty()
        && !def
            .files_any
            .iter()
            .any(|f| expand_probe(f, env_root).exists())
    {
        return Verdict::Fail;
    }
    Verdict::Pass
}

/// 汇总（--json 用）。
pub fn summarize(rows: &[(String, Verdict)]) -> (usize, Vec<String>) {
    let fails = rows
        .iter()
        .filter(|(_, v)| *v == Verdict::Fail)
        .map(|(n, _)| n.clone())
        .collect::<Vec<_>>();
    (rows.len(), fails)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 注册表完整性：toolRoot 在两平台各有定义；所有 files 路径带占位或绝对/~ 前缀。
    #[test]
    fn 维度注册表_平台覆盖完整() {
        let win_toolroot = DIMS
            .iter()
            .filter(|d| d.name == "toolRoot" && d.windows)
            .count();
        let posix_toolroot = DIMS
            .iter()
            .filter(|d| d.name == "toolRoot" && d.posix)
            .count();
        assert_eq!(win_toolroot, 1, "Windows toolRoot 应有定义");
        assert_eq!(posix_toolroot, 1, "POSIX toolRoot 应有定义");
        for d in DIMS {
            assert!(d.windows || d.posix, "{} 至少覆盖一个平台", d.name);
            for f in d.files_all.iter().chain(d.files_any.iter()) {
                assert!(
                    f.starts_with('~') || f.starts_with('/') || f.contains("{envroot}"),
                    "{} 的路径 {} 应为 ~/、绝对或 {{envroot}} 占位形态",
                    d.name,
                    f
                );
            }
        }
    }

    /// 文件组判定：全在 PASS、缺一 FAIL、任一组命中 PASS、全缺 FAIL。
    /// 路径按平台分隔符构造（unix 上反斜杠是合法文件名字符，Windows 风格样例会误报缺失——CI 首跑实证）。
    #[test]
    fn 文件组判定_全在与任一() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let root = dir.path();
        std::fs::create_dir_all(root.join("a").join("b")).map_err(|e| e.to_string())?;
        std::fs::write(root.join("a").join("b").join("x.exe"), b"1").map_err(|e| e.to_string())?;

        let sep = std::path::MAIN_SEPARATOR;
        let probe = format!("{{envroot}}{sep}a{sep}b{sep}x.exe");
        let missing = format!("{{envroot}}{sep}nope");
        assert!(expand_probe(&probe, root).exists());
        assert!(!expand_probe(&missing, root).exists());

        let any = ["/nonexistent-1", probe.as_str()];
        assert!(any.iter().any(|f| expand_probe(f, root).exists()));
        let any_none = ["/nonexistent-1", "/nonexistent-2"];
        assert!(!any_none.iter().any(|f| expand_probe(f, root).exists()));
        Ok(())
    }

    /// 汇总：FAIL 维度计数与名单。
    #[test]
    fn 汇总_fail名单() {
        let rows = vec![
            ("toolRoot".to_string(), Verdict::Pass),
            ("bun".to_string(), Verdict::Fail),
            ("ghost".to_string(), Verdict::Na),
        ];
        let (total, fails) = summarize(&rows);
        assert_eq!(total, 3);
        assert_eq!(fails, vec!["bun".to_string()]);
    }
}
