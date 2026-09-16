//! heal：部署域幂等自愈（P0026 M4，heal-map.psd1 的 42 键迁嵌入注册表）。
//! 语义：verify 判 FAIL 的维度按本表得到幂等修复动作，`ark heal <dim|all> [--dry-run]` 执行；
//! 所有动作幂等、可无脑重跑、不破坏性（对齐 heal.ps1 原则）。
//! 42 键四类归宿（2026-09-01/02 裁决）：
//! - install 类 16 行（toolRoot/aria2 按平台分列，dev-rust 已建 rustup 模型）→ ome 原生安装
//!   （catalog pin 驱动，与 verify 断言一致；rust 为 evergreen 引导器稳定滚动）；
//! - 密钥载体 dsKey/akKey 与镜像源 bunfig/goproxy → heal-keys.py / heal-mirror.py 原生移植；
//! - agent 域 12 键休眠（四件套配置归 ohmyagents）；
//! - 非 ome 域路由（secret-guard 密钥防护、POSIX 残留清零、compileMatrix 编译验收编排、
//!   POSIX aria2 系统位——不在 ome 自愈范围，只提示不越界）。
//!
//! mac-* 四键为 ps1 远端路由时代的专列；ome 在 mac 本机原生运行，归一为别名指向普通键。

use std::path::Path;

use crate::catalog::Catalog;
use crate::install::{install_tool, InstallAction, InstallOptions};
use crate::resolve::{resolve_tool, ResolveOptions};

/// heal 动作类型（嵌入注册表条目）。
enum HealAction {
    /// ome 原生安装（catalog 工具组；`all` 为全量安装，对齐 ohmywsl install all）
    Install(&'static [&'static str]),
    /// 密钥载体/端点补齐（heal-keys.py 移植；明文密钥不落盘，只写现场解密指令）
    Keys,
    /// bunfig npmmirror 镜像（heal-mirror.py 移植）
    MirrorBunfig,
    /// go goproxy.cn 镜像（GOENV 文件 upsert，D44 委托 manifest 单一权威）
    MirrorGoproxy,
    /// 平台专列别名（mac-* 归一普通键）
    Alias(&'static str),
    /// 休眠（agent 域 2026-09-01 裁决）
    Dormant(&'static str),
    /// 非 ome 自愈域（只提示不越界）
    Routed(&'static str),
}

/// heal 注册表条目（名称可跨平台分列多行，对齐 verify DIMS 形态）。
struct HealDef {
    name: &'static str,
    /// Windows 端生效
    windows: bool,
    /// Linux / macOS 端生效
    posix: bool,
    action: HealAction,
}

/// heal-map 42 键注册表（动作换 ome 原生）。
static HEALS: &[HealDef] = &[
    // ── 工具缺失（install 类；verify 部署维度与 heal 一一对应）──
    HealDef {
        name: "toolRoot",
        windows: true,
        posix: false,
        action: HealAction::Install(&["sops", "herdr", "gh", "mq"]),
    },
    HealDef {
        name: "toolRoot",
        windows: false,
        posix: true,
        action: HealAction::Install(&["sops", "herdr", "gh"]),
    },
    HealDef {
        name: "bun",
        windows: true,
        posix: true,
        action: HealAction::Install(&["bun"]),
    },
    HealDef {
        name: "dev-go",
        windows: true,
        posix: false,
        action: HealAction::Install(&["go"]),
    },
    HealDef {
        name: "dev-zig",
        windows: true,
        posix: false,
        action: HealAction::Install(&["zig"]),
    },
    HealDef {
        name: "dev-fnm",
        windows: true,
        posix: false,
        action: HealAction::Install(&["fnm"]),
    },
    HealDef {
        name: "dev-rust",
        windows: true,
        posix: true,
        action: HealAction::Install(&["rust"]),
    },
    HealDef {
        name: "vsbuild",
        windows: true,
        posix: false,
        action: HealAction::Install(&["vsbuild"]),
    },
    HealDef {
        name: "aria2",
        windows: true,
        posix: false,
        action: HealAction::Install(&["aria2"]),
    },
    HealDef {
        name: "aria2",
        windows: false,
        posix: true,
        action: HealAction::Routed(
            "POSIX aria2 为系统位（apt /usr/bin、brew /opt/homebrew），非 ome 绿装域",
        ),
    },
    HealDef {
        name: "go",
        windows: false,
        posix: true,
        action: HealAction::Install(&["go"]),
    },
    HealDef {
        name: "zig",
        windows: false,
        posix: true,
        action: HealAction::Install(&["zig"]),
    },
    HealDef {
        name: "localbin16",
        windows: false,
        posix: true,
        action: HealAction::Install(&[
            "age", "sops", "gh", "uv", "rg", "jq", "mq", "yq", "just", "herdr", "ast-grep",
        ]),
    },
    HealDef {
        name: "rmux",
        windows: true,
        posix: true,
        action: HealAction::Install(&["rmux"]),
    },
    HealDef {
        name: "uv",
        windows: false,
        posix: true,
        action: HealAction::Install(&["uv"]),
    },
    HealDef {
        name: "astgrep",
        windows: true,
        posix: true,
        action: HealAction::Install(&["ast-grep"]),
    },
    HealDef {
        name: "yq",
        windows: true,
        posix: true,
        action: HealAction::Install(&["yq"]),
    },
    // ── 密钥载体/端点（heal-keys.py 原生移植）──
    HealDef {
        name: "dsKey",
        windows: true,
        posix: true,
        action: HealAction::Keys,
    },
    HealDef {
        name: "akKey",
        windows: true,
        posix: true,
        action: HealAction::Keys,
    },
    // ── 镜像源（heal-mirror.py 原生移植）──
    HealDef {
        name: "bunfig",
        windows: true,
        posix: true,
        action: HealAction::MirrorBunfig,
    },
    HealDef {
        name: "goproxy",
        windows: true,
        posix: true,
        action: HealAction::MirrorGoproxy,
    },
    // ── mac 专列别名（ome 本机原生跑普通键）──
    HealDef {
        name: "mac-go",
        windows: false,
        posix: true,
        action: HealAction::Alias("go"),
    },
    HealDef {
        name: "mac-zig",
        windows: false,
        posix: true,
        action: HealAction::Alias("zig"),
    },
    HealDef {
        name: "mac-bun",
        windows: false,
        posix: true,
        action: HealAction::Alias("bun"),
    },
    HealDef {
        name: "mac-localbin16",
        windows: false,
        posix: true,
        action: HealAction::Alias("localbin16"),
    },
    // ── 休眠（agent 域 2026-09-01 裁决：四件套安装与配置移出，归 ohmyagents）──
    HealDef {
        name: "claude",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 版本域（A 族）已按 2026-09-01 裁决休眠"),
    },
    HealDef {
        name: "codex",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 版本域（A 族）已按 2026-09-01 裁决休眠"),
    },
    HealDef {
        name: "kimi",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 版本域（A 族）已按 2026-09-01 裁决休眠"),
    },
    HealDef {
        name: "grok",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 版本域（A 族）已按 2026-09-01 裁决休眠"),
    },
    HealDef {
        name: "claudeSkip",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 参数化信任域（H 族）已按 2026-09-01 裁决休眠"),
    },
    HealDef {
        name: "codexClean",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 参数化信任域（H 族）已按 2026-09-01 裁决休眠"),
    },
    HealDef {
        name: "grokFT",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 参数化信任域（H 族）已按 2026-09-01 裁决休眠"),
    },
    HealDef {
        name: "grokMarket",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 配置域（B 族）已按 2026-09-01 裁决休眠"),
    },
    HealDef {
        name: "zhipu",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 配置域（B 族）已按 2026-09-01 裁决休眠"),
    },
    HealDef {
        name: "deepseek",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 配置域（B 族）已按 2026-09-01 裁决休眠"),
    },
    HealDef {
        name: "kimiK3",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 配置域（B 族）已按 2026-09-01 裁决休眠"),
    },
    HealDef {
        name: "residOldsg",
        windows: true,
        posix: true,
        action: HealAction::Dormant("agent 残留清理域已按 2026-09-01 裁决休眠"),
    },
    // ── 非 ome 自愈域（只提示不越界）──
    HealDef {
        name: "sgPy",
        windows: false,
        posix: true,
        action: HealAction::Routed("secret-guard 属密钥防护域，不在 ome 自愈范围"),
    },
    HealDef {
        name: "sgRef",
        windows: false,
        posix: true,
        action: HealAction::Routed("secret-guard 属密钥防护域，不在 ome 自愈范围"),
    },
    HealDef {
        name: "win-sgPy",
        windows: true,
        posix: false,
        action: HealAction::Routed("secret-guard 属密钥防护域，不在 ome 自愈范围"),
    },
    HealDef {
        name: "win-sgRef",
        windows: true,
        posix: false,
        action: HealAction::Routed("secret-guard 属密钥防护域，不在 ome 自愈范围"),
    },
    HealDef {
        name: "residOldbin",
        windows: false,
        posix: true,
        action: HealAction::Routed("POSIX 旧 EnvRoot 残留清零不在 ome 自愈范围"),
    },
    HealDef {
        name: "residOldzig",
        windows: false,
        posix: true,
        action: HealAction::Routed("POSIX 旧 EnvRoot 残留清零不在 ome 自愈范围"),
    },
    HealDef {
        name: "compileMatrix",
        windows: true,
        posix: true,
        action: HealAction::Routed("编译验收编排不在 ome 自愈范围（部署验收走 ark verify）"),
    },
];

/// 单次自愈执行结果行。
pub struct HealRow {
    /// 维度名（verify 维度注册表的键）
    pub dim: String,
    /// 动作类：install / keys / bunfig / goproxy / alias / dormant / routed / inapplicable
    pub action: &'static str,
    /// 参数（install 的工具组、alias 的目标键；其余空）
    pub params: String,
    /// 结果：healed / ok / partial / fail / dry-run / skip
    pub result: String,
    /// 明细（kv 走 stderr 提示，结构化入 detail 字段）
    pub detail: Vec<String>,
}

/// 当前平台是否生效。
fn applicable(def: &HealDef) -> bool {
    if cfg!(windows) {
        def.windows
    } else {
        def.posix
    }
}

/// 跑自愈（流式）：维度动作完成即经 emit 回调输出。
/// dim 为 `all` 或单维度名；dry_run 只打印计划不执行。
/// 返回全部结果行；fail/partial 行由调用方汇总退出码。
pub fn run_heal_with<F: FnMut(&HealRow) -> Result<(), String>>(
    cat: &Catalog,
    env_root: &Path,
    dim: &str,
    dry_run: bool,
    mut emit: F,
) -> Result<Vec<HealRow>, String> {
    let home = dirs::home_dir().ok_or("无法确定用户主目录")?;
    let mut rows: Vec<HealRow> = Vec::new();

    if dim == "all" {
        let mut dormant = 0usize;
        let mut routed = 0usize;
        for def in HEALS {
            match &def.action {
                HealAction::Dormant(_) => dormant += 1,
                HealAction::Routed(_) => routed += 1,
                // 别名的规范键已在本轮执行，不再重复
                HealAction::Alias(_) => {}
                _ => {
                    if applicable(def) {
                        let row = run_def(cat, env_root, &home, def, dry_run)?;
                        emit(&row)?;
                        rows.push(row);
                    }
                }
            }
        }
        eprintln!(
            "[INFO] all 跳过休眠键 {dormant} 个（agent 域）与外域键 {routed} 个（不在 ome 自愈范围）"
        );
        return Ok(rows);
    }

    // 单维度：按名取当前平台行；无当前平台行但键存在 → 平台不适用提示
    let named: Vec<&HealDef> = HEALS.iter().filter(|d| d.name == dim).collect();
    let Some(def) = named.iter().find(|d| applicable(d)) else {
        if named.is_empty() {
            let avail: Vec<&str> = HEALS
                .iter()
                .filter(|d| applicable(d))
                .map(|d| d.name)
                .collect();
            return Err(format!(
                "未知自愈维度: {dim}（当前平台可用: {}；heal all 跑全部）",
                avail.join(", ")
            ));
        }
        let row = HealRow {
            dim: dim.to_string(),
            action: "inapplicable",
            params: String::new(),
            result: "skip".to_string(),
            detail: vec!["键存在但当前平台不适用（他平台专列）".to_string()],
        };
        eprintln!("[INFO] {}", row.detail[0]);
        emit(&row)?;
        return Ok(vec![row]);
    };
    let row = run_named(cat, env_root, &home, def, dry_run)?;
    emit(&row)?;
    rows.push(row);
    Ok(rows)
}

/// 执行单条注册表行（all 模式与单维度共用）。
fn run_def(
    cat: &Catalog,
    env_root: &Path,
    home: &Path,
    def: &HealDef,
    dry_run: bool,
) -> Result<HealRow, String> {
    match &def.action {
        HealAction::Install(tools) => run_install_dim(cat, env_root, def.name, tools, dry_run),
        HealAction::Keys => {
            let mut row = HealRow {
                dim: def.name.to_string(),
                action: "keys",
                params: String::new(),
                result: "ok".to_string(),
                detail: Vec::new(),
            };
            if dry_run {
                row.result = "dry-run".to_string();
                row.detail
                    .push("补齐密钥载体 env.sh / 端点（明文密钥不落盘）".to_string());
            } else {
                let (changed, detail) = heal_keys_carrier(home)?;
                if changed {
                    row.result = "healed".to_string();
                }
                row.detail = detail;
            }
            Ok(row)
        }
        HealAction::MirrorBunfig => {
            let mut row = HealRow {
                dim: def.name.to_string(),
                action: "bunfig",
                params: String::new(),
                result: "ok".to_string(),
                detail: Vec::new(),
            };
            if dry_run {
                row.result = "dry-run".to_string();
                row.detail
                    .push("写 ~/.bunfig.toml npmmirror 镜像".to_string());
            } else {
                let changed = heal_bunfig(home)?;
                row.result = if changed { "healed" } else { "ok" }.to_string();
                row.detail.push(format!("~/.bunfig.toml: {}", row.result));
            }
            Ok(row)
        }
        HealAction::MirrorGoproxy => {
            let mut row = HealRow {
                dim: def.name.to_string(),
                action: "goproxy",
                params: String::new(),
                result: "ok".to_string(),
                detail: Vec::new(),
            };
            if dry_run {
                row.result = "dry-run".to_string();
                row.detail
                    .push("GOENV 文件 upsert GOPROXY/GOSUMDB（goproxy.cn）".to_string());
            } else {
                // D44 对线 F2：写语义单一权威在 manifest mirror 面（heal 委托，win 同改
                // 文件直写不依赖 go 在位；整写毁用户键的旧语义退役）
                let changed = crate::manifest::ensure_go_env("https://goproxy.cn,direct")?;
                row.result = if changed { "healed" } else { "ok" }.to_string();
                row.detail.push(format!("goproxy: {}", row.result));
            }
            Ok(row)
        }
        // all 模式已滤除；此处仅为单维度路径完备
        HealAction::Alias(_) | HealAction::Dormant(_) | HealAction::Routed(_) => unreachable!(),
    }
}

/// 单维度入口（可命中别名/休眠/外域行；别名解析到当前平台的规范行后执行）。
fn run_named(
    cat: &Catalog,
    env_root: &Path,
    home: &Path,
    def: &HealDef,
    dry_run: bool,
) -> Result<HealRow, String> {
    match &def.action {
        HealAction::Alias(target) => {
            let canonical = HEALS
                .iter()
                .find(|d| d.name == *target && applicable(d))
                .ok_or_else(|| format!("别名目标键不存在或当前平台不适用: {target}"))?;
            let mut row = run_def(cat, env_root, home, canonical, dry_run)?;
            row.dim = def.name.to_string();
            row.action = "alias";
            row.params = (*target).to_string();
            Ok(row)
        }
        HealAction::Dormant(reason) => {
            eprintln!("[INFO] {name} 键休眠: {reason}", name = def.name);
            Ok(HealRow {
                dim: def.name.to_string(),
                action: "dormant",
                params: String::new(),
                result: "skip".to_string(),
                detail: vec![reason.to_string()],
            })
        }
        HealAction::Routed(reason) => {
            eprintln!("[INFO] {name} 非 ome 自愈域: {reason}", name = def.name);
            Ok(HealRow {
                dim: def.name.to_string(),
                action: "routed",
                params: String::new(),
                result: "skip".to_string(),
                detail: vec![reason.to_string()],
            })
        }
        _ => run_def(cat, env_root, home, def, dry_run),
    }
}

/// install 类维度：catalog pin 驱动幂等安装（不注册 PATH、不回写锁定——heal 只修到 verify 可过，
/// 对齐 heal-map 的 ohmyenv install 语义）。单工具失败收集进明细不中断（all 循环容错同源）。
fn run_install_dim(
    cat: &Catalog,
    env_root: &Path,
    dim: &str,
    tools: &'static [&'static str],
    dry_run: bool,
) -> Result<HealRow, String> {
    let mut row = HealRow {
        dim: dim.to_string(),
        action: "install",
        params: tools.join(","),
        result: "ok".to_string(),
        detail: Vec::new(),
    };
    if dry_run {
        row.result = "dry-run".to_string();
        row.detail.push(format!("ark install {}", tools.join(" ")));
        return Ok(row);
    }
    let names: Vec<String> = if tools == ["all"] {
        cat.select("all")?
    } else {
        tools.iter().map(|s| s.to_string()).collect()
    };
    let mut installed = 0usize;
    let mut errors = 0usize;
    for name in &names {
        match install_one(cat, env_root, name) {
            Ok(InstallAction::Installed) => installed += 1,
            Ok(InstallAction::Skipped) => {}
            Err(e) => {
                errors += 1;
                eprintln!("[WARN] {name}: {e}");
                row.detail.push(format!("{name}: {e}"));
            }
        }
    }
    row.result = match (installed, errors) {
        (_, e) if e > 0 && installed > 0 => "partial".to_string(),
        (_, e) if e > 0 => "fail".to_string(),
        (i, _) if i > 0 => "healed".to_string(),
        _ => "ok".to_string(),
    };
    Ok(row)
}

/// heal 内安装单工具（对齐 cmd_install 的特型分派：平台不适用跳过、vsbuild/docker 专用模块、
/// 其余 pin 解析 + 幂等安装）。
fn install_one(cat: &Catalog, env_root: &Path, name: &str) -> Result<InstallAction, String> {
    let def = cat.tool(name)?;
    if !crate::toolver::platform_managed(def) {
        eprintln!("[INFO] {name} 当前平台不适用（无本平台 exe 字段），跳过");
        return Ok(InstallAction::Skipped);
    }
    if crate::vsbuild::is_vsbuild(def) {
        return crate::vsbuild::install(def, env_root, true).map(|o| o.action);
    }
    if crate::rustup::is_rustup(def) {
        return crate::rustup::install(def, env_root, true).map(|o| o.action);
    }
    // ome 自管条目：升级走 self update 三通道，heal 的 install all 不碰
    if crate::selfupdate::is_ome_self(def) {
        eprintln!("[INFO] {name} 自管理：升级走 `ark self update`（dev/stable/git 三通道）");
        return Ok(InstallAction::Skipped);
    }
    let opts = InstallOptions {
        configure: true,
        update_lock: false,
        force: false,
    };
    if crate::docker::is_docker(def) {
        let res = resolve_tool(name, def, &ResolveOptions::default())?;
        return crate::docker::install(def, env_root, &res, true).map(|o| o.action);
    }
    let res = resolve_tool(name, def, &ResolveOptions::default())?;
    install_tool(cat, env_root, name, &res, &opts).map(|o| o.action)
}

// ── heal-keys.py 原生移植（密钥载体/端点；明文密钥绝不落盘）──

/// ohmyenv-secrets 统一标准位载体内容（与 heal-keys.py 逐字节一致；现场解密指令，不含明文 key）。
#[cfg(not(windows))]
const SECRETS_ENV_SH: &str = r#"# --- ohmyenv-secrets（统一标准位载体；幂等，可反复生成）---
export SOPS_AGE_KEY_FILE="$HOME/.config/sops/age/keys.txt"
[ ! -f "$SOPS_AGE_KEY_FILE" ] && export SOPS_AGE_KEY_FILE="$HOME/.config/age/keys.txt"
export ANTHROPIC_BASE_URL="https://open.bigmodel.cn/api/anthropic"
if [ -x "$HOME/.local/bin/sops" ]; then
  SOPS="$HOME/.local/bin/sops"
elif command -v sops >/dev/null 2>&1; then
  SOPS="$(command -v sops)"
else
  SOPS=""
fi
if [ -n "$SOPS" ]; then
  for _cfg in deepseek-key.yaml:DEEPSEEK_API_KEY zhipu-key.yaml:ANTHROPIC_AUTH_TOKEN gh-token.yaml:GH_TOKEN; do
    _f="${_cfg%%:*}"; _env="${_cfg##*:}"
    _v="$("${SOPS}" --input-type yaml --output-type yaml -d "$HOME/.config/ohmyenv-secrets/$_f" 2>/dev/null | sed -n "s/^${_env}:[[:space:]]*//p")"
    [ -n "$_v" ] && export "${_env}"="$(printf '%s' "$_v" | base64 -d 2>/dev/null)"
  done
  unset _cfg _f _env _v
fi
# --- end ---
"#;

/// rc 挂钩行（.profile / .bashrc / .zshrc）。
#[cfg(not(windows))]
const SECRETS_RC_LINE: &str =
    r#"[ -f "$HOME/.config/ohmyenv-secrets/env.sh" ] && . "$HOME/.config/ohmyenv-secrets/env.sh""#;

/// env.sh 内容时效标志（2026-08-27 坑：只判存在会漏「文件在但内容旧」——键名改版后 sed 匹配旧键名解不出 key）。
#[cfg(not(windows))]
const SECRETS_REQUIRED_MARKERS: &[&str] = &[
    "DEEPSEEK_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "GH_TOKEN",
    "ANTHROPIC_BASE_URL",
];

/// 密钥载体/端点补齐（heal-keys.py 移植）。
/// POSIX：确保 ~/.config/ohmyenv-secrets/env.sh 存在且内容时效，并挂钩三个 rc；
/// Windows：确保用户级 ANTHROPIC_BASE_URL 端点（密钥由 profile 惰性注入，本动作只补端点）。
/// 返回 (是否有变更, 明细行)。
pub fn heal_keys_carrier(home: &Path) -> Result<(bool, Vec<String>), String> {
    #[cfg(windows)]
    {
        let _ = home; // Windows 端点走注册表，不用 home
        const WANT: &str = "https://open.bigmodel.cn/api/anthropic";
        let cur = crate::platform::get_user_env_var("ANTHROPIC_BASE_URL")?;
        if cur.as_deref() == Some(WANT) {
            Ok((false, vec!["ANTHROPIC_BASE_URL: ok(already)".to_string()]))
        } else {
            crate::platform::set_user_env_var("ANTHROPIC_BASE_URL", WANT)?;
            Ok((true, vec!["ANTHROPIC_BASE_URL: healed".to_string()]))
        }
    }
    #[cfg(not(windows))]
    {
        let mut detail = Vec::new();
        let mut changed = false;
        let cfg_dir = home.join(".config").join("ohmyenv-secrets");
        std::fs::create_dir_all(&cfg_dir)
            .map_err(|e| format!("创建目录失败: {}: {e}", cfg_dir.display()))?;
        let env_sh = cfg_dir.join("env.sh");
        let content = std::fs::read_to_string(&env_sh).unwrap_or_default();
        let missing: Vec<&str> = SECRETS_REQUIRED_MARKERS
            .iter()
            .filter(|k| !content.contains(*k))
            .copied()
            .collect();
        if !env_sh.exists() {
            std::fs::write(&env_sh, SECRETS_ENV_SH)
                .map_err(|e| format!("写 env.sh 失败: {}: {e}", env_sh.display()))?;
            detail.push("env.sh: CREATED".to_string());
            changed = true;
        } else if !missing.is_empty() {
            std::fs::write(&env_sh, SECRETS_ENV_SH)
                .map_err(|e| format!("写 env.sh 失败: {}: {e}", env_sh.display()))?;
            detail.push(format!("env.sh: REWRITTEN (缺 {})", missing.join(", ")));
            changed = true;
        } else {
            detail.push("env.sh: OK(exists + content current)".to_string());
        }
        for rc_name in [".profile", ".bashrc", ".zshrc"] {
            let rc = home.join(rc_name);
            if !rc.exists() {
                std::fs::File::create(&rc).map_err(|e| format!("创建 {rc_name} 失败: {e}"))?;
            }
            let text = std::fs::read_to_string(&rc).unwrap_or_default();
            if !text.contains("ohmyenv-secrets/env.sh") {
                let mut out = text;
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push('\n');
                out.push_str(SECRETS_RC_LINE);
                out.push('\n');
                std::fs::write(&rc, out).map_err(|e| format!("写 {rc_name} 失败: {e}"))?;
                detail.push(format!("{rc_name}: HOOKED"));
                changed = true;
            } else {
                detail.push(format!("{rc_name}: OK(hooked)"));
            }
        }
        Ok((changed, detail))
    }
}

// ── heal-mirror.py 原生移植（镜像源）──

/// bunfig npmmirror：写语义单一权威在 manifest mirror 节（D42 收编，防双份漂移）。
pub fn heal_bunfig(home: &Path) -> Result<bool, String> {
    crate::manifest::ensure_bunfig(home, "https://registry.npmmirror.com/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// heal-map.psd1 的 42 键名 oracle（迁移对账源）。
    const PSD1_KEYS: &[&str] = &[
        "claude",
        "codex",
        "kimi",
        "grok",
        "claudeSkip",
        "codexClean",
        "grokFT",
        "grokMarket",
        "dsKey",
        "akKey",
        "zhipu",
        "localbin16",
        "rmux",
        "uv",
        "sgPy",
        "sgRef",
        "win-sgPy",
        "win-sgRef",
        "dev-go",
        "dev-zig",
        "dev-rust",
        "dev-fnm",
        "go",
        "zig",
        "bun",
        "mac-go",
        "mac-zig",
        "mac-bun",
        "mac-localbin16",
        "deepseek",
        "kimiK3",
        "toolRoot",
        "astgrep",
        "yq",
        "bunfig",
        "goproxy",
        "residOldsg",
        "residOldbin",
        "residOldzig",
        "aria2",
        "vsbuild",
        "compileMatrix",
    ];

    /// 注册表与 psd1 对账：42 键名一致（toolRoot/aria2 平台分列共 44 行）。
    #[test]
    fn 注册表_四十二键名与psd1对齐() {
        let names: BTreeSet<&str> = HEALS.iter().map(|d| d.name).collect();
        let oracle: BTreeSet<&str> = PSD1_KEYS.iter().copied().collect();
        assert_eq!(names, oracle, "heal 键名应与 heal-map.psd1 完全一致");
        assert_eq!(HEALS.len(), 44, "toolRoot 与 aria2 按平台分列应共 44 行");
        // 别名目标必须在注册表内
        for def in HEALS {
            if let HealAction::Alias(target) = &def.action {
                assert!(
                    HEALS.iter().any(|d| d.name == *target),
                    "别名目标不存在: {target}"
                );
            }
        }
    }

    /// 注册表与 verify 对账：verify 的每个维度都有 heal 键（FAIL 可自愈，缺口即注册表漂移）。
    #[test]
    fn 注册表_verify维度全覆盖() {
        let names: BTreeSet<&str> = HEALS.iter().map(|d| d.name).collect();
        for dim in crate::verify::dim_names() {
            assert!(names.contains(dim), "verify 维度 {dim} 缺 heal 键");
        }
    }

    /// bunfig：缺文件写入、含镜像标记跳过、无标记旧文件重写。
    #[test]
    fn bunfig_缺文件写入与幂等() -> Result<(), String> {
        let home = tempfile::tempdir().map_err(|e| e.to_string())?;
        assert!(heal_bunfig(home.path())?, "首次应写入");
        let content =
            std::fs::read_to_string(home.path().join(".bunfig.toml")).map_err(|e| e.to_string())?;
        assert!(content.contains("npmmirror"), "内容应为 npmmirror 镜像");
        assert!(!heal_bunfig(home.path())?, "已含标记应跳过");
        std::fs::write(home.path().join(".bunfig.toml"), "# 用户自定义\n")
            .map_err(|e| e.to_string())?;
        assert!(heal_bunfig(home.path())?, "无镜像标记的旧文件应重写");
        Ok(())
    }

    // goproxy 写语义测试随实现收编 manifest.rs（D44 对线 F2：heal 纯委托）。

    /// 密钥载体（POSIX）：env.sh 创建、内容旧重写、rc 挂钩与幂等。
    #[cfg(not(windows))]
    #[test]
    fn 密钥载体_envsh与rc挂钩幂等() -> Result<(), String> {
        let home = tempfile::tempdir().map_err(|e| e.to_string())?;
        let (changed, detail) = heal_keys_carrier(home.path())?;
        assert!(changed, "首次应创建载体与挂钩");
        assert!(detail.iter().any(|d| d.contains("env.sh: CREATED")));
        assert!(detail.iter().any(|d| d.contains(".bashrc: HOOKED")));
        let env_sh = home
            .path()
            .join(".config")
            .join("ohmyenv-secrets")
            .join("env.sh");
        let content = std::fs::read_to_string(&env_sh).map_err(|e| e.to_string())?;
        assert!(
            content.contains("ANTHROPIC_AUTH_TOKEN"),
            "载体应含现场解密指令"
        );
        let (changed2, detail2) = heal_keys_carrier(home.path())?;
        assert!(!changed2, "第二次应全 OK");
        assert!(detail2
            .iter()
            .any(|d| d.contains("OK(exists + content current)")));
        // 内容旧（缺键名标志）：应重写
        std::fs::write(&env_sh, "# stale\nexport OLD=1\n").map_err(|e| e.to_string())?;
        let (changed3, detail3) = heal_keys_carrier(home.path())?;
        assert!(changed3, "内容旧应重写");
        assert!(detail3.iter().any(|d| d.contains("REWRITTEN")));
        Ok(())
    }
}
