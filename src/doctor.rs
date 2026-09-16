//! doctor：部署异常诊断（2026-09-02 用户需求：检测部署的错误异常）。
//! 与 verify 的分工：verify 答「部署域维度是否 PASS」（pin 对齐断言），doctor 答
//! 「环境里有哪些部署错误与异味」——版本漂移、探测失败、PATH 死链与重复、
//! pin/sha 缺失、缓存孤儿、EnvRoot 不可写等，输出 check=OK/WARN/FAIL 逐项行，
//! FAIL 即 exit 1（WARN 不拦退出）。

use std::path::{Path, PathBuf};

use crate::catalog::Catalog;
use crate::status::{self, StatusRow};
use crate::toolver;

/// 单项诊断结果。detail 为该项的明细（stderr 人称提示用）。
pub struct DoctorRow {
    /// 诊断行名（检查项标识）
    pub name: &'static str,
    /// 行状态三态
    pub status: &'static str, // "OK" | "WARN" | "FAIL"
    /// 行明细（多行事实陈述）
    pub detail: Vec<String>,
}

// ===================== 诊断两层（D07 三层，D30 收窄 2026-09-10）：系统 / 依赖 =====================
// doctor 升核心命令：先答「本机是什么系统、依赖装了没有」，再出环境错误 check 节（十项
// 加配置健康/部署深诊/网络通连）。两层是事实陈述与缺口计数（缺口走 WARN，不拦退出，
// 检测驱动安装）；FAIL 语义仍专属 check 节的环境错误。
// D30 削减：原 agent 层（四家 binary/version/locked/drift/token 健康块）整层移除——
// agent 装态对账归 omc 舰队面、token/凭据检测归 oma diagnose（用户裁 2026-09-10，
// 重叠功能由 ome 侧减）；依赖层九类分组的智能体依赖组保留（install 域单机装态事实，
// omc 对账的数据源）。agent 单机三态仍可经 `ark status` 查。

/// 系统层事实（非诊断，不占 OK/WARN/FAIL 三态）。
pub struct SysFacts {
    /// 操作系统名（std::env::consts）
    pub os: &'static str,
    /// CPU 架构名
    pub arch: &'static str,
    /// AVX 指令集在位
    pub avx: bool,
    /// AVX2 指令集在位
    pub avx2: bool,
    /// AVX-512F 指令集在位
    pub avx512f: bool,
}

/// 采集系统层事实（OS、架构、AVX 族指令集在位性）。
pub fn system_facts() -> SysFacts {
    SysFacts {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        avx: x86_feature("avx"),
        avx2: x86_feature("avx2"),
        avx512f: x86_feature("avx512f"),
    }
}

#[cfg(target_arch = "x86_64")]
fn x86_feature(f: &str) -> bool {
    match f {
        "avx" => std::arch::is_x86_feature_detected!("avx"),
        "avx2" => std::arch::is_x86_feature_detected!("avx2"),
        _ => std::arch::is_x86_feature_detected!("avx512f"),
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn x86_feature(_f: &str) -> bool {
    false // 非 x86_64（如 darwin-arm64）指令集检测不适用，如实 false（oma caps 同口径）
}

/// 依赖层分组统计（九类 taxonomy 逐组：工具数、缺失数、漂移数）。
pub struct DepGroupStat {
    /// 分类键（taxonomy 组键）
    pub category: String,
    /// 分组显示名
    pub label: &'static str,
    /// 组内工具数
    pub tools: usize,
    /// 组内缺失数
    pub missing: usize,
    /// 组内漂移数
    pub drift: usize,
}

/// 依赖层九类分组统计（逐组工具数、缺失数、漂移数）。
pub fn dep_group_stats(srows: &[StatusRow]) -> Vec<DepGroupStat> {
    status::GROUPS
        .iter()
        .filter_map(|(cat, label)| {
            let group: Vec<&StatusRow> = srows.iter().filter(|r| &r.category == cat).collect();
            if group.is_empty() {
                return None; // 空组不输出（如 mac 侧无 service 组）
            }
            Some(DepGroupStat {
                category: cat.to_string(),
                label,
                tools: group.len(),
                missing: group.iter().filter(|r| r.installed.is_none()).count(),
                drift: group
                    .iter()
                    .filter(|r| {
                        matches!(
                            (&r.installed, &r.locked),
                            (Some(installed), Some(locked)) if installed != locked
                        )
                    })
                    .count(),
            })
        })
        .collect()
}

/// 跑全部诊断项，流式形态：每项算完即经回调输出（三态采集期间先出 envroot 项，
/// 采集完成即连出五个派生项；返回全量行供汇总）。
pub fn run_doctor_with<F>(
    cat: &Catalog,
    env_root: &Path,
    on_row: F,
) -> Result<Vec<DoctorRow>, String>
where
    F: FnMut(&DoctorRow),
{
    let srows = status::collect_status(cat, env_root)?;
    run_doctor_with_status(cat, env_root, &srows, on_row)
}

/// 同 run_doctor_with，但复用调用方已采集的三态行（cmd_doctor 三层诊断共用一次采集，
/// 避免 41 工具版本探针跑两遍）。
pub fn run_doctor_with_status<F>(
    cat: &Catalog,
    env_root: &Path,
    srows: &[StatusRow],
    mut on_row: F,
) -> Result<Vec<DoctorRow>, String>
where
    F: FnMut(&DoctorRow),
{
    let mut rows: Vec<DoctorRow> = Vec::new();
    let mut put = |row: DoctorRow| {
        on_row(&row);
        rows.push(row);
    };

    // 1. EnvRoot 可写（装不进东西是一切部署错误之源）
    put(check_envroot_writable(env_root));

    // 2-7. 三态派生项（复用调用方采集的三态行）
    put(check_version_drift(srows));
    put(check_probe_fail(srows));
    put(check_not_on_path(cat, srows, env_root));
    put(check_pin_missing(cat, srows));
    put(check_sha_missing(cat, srows));

    // 8-9. 用户 PATH 卫生（死链仅报 EnvRoot 域内，系统条目不掺和）
    let entries = crate::platform::user_path_entries().unwrap_or_default();
    put(check_dead_path_entries(&entries, env_root));
    put(check_dup_path_entries(&entries));

    // 10. 缓存孤儿（下载缓存里已无任何 pin 指向的资产）
    put(check_cache_orphans(cat, env_root));

    // 11. 配置健康（D11：运行时与编译器配置，判据与 heal/install 写入动作同源；
    //     密钥域不管；对应工具在装才检查，缺失走 WARN 可 heal 修）
    for row in config_health(srows, env_root) {
        put(row);
    }

    // 12. 特型工具部署深诊（D12：evergreen/安装器型的部署产物核对，
    //     判据从 vsbuild.rs/rustup.rs 写入动作与布局来；在装才查）
    for row in deploy_probes(srows, env_root) {
        put(row);
    }

    // 13. 网络通连（用户 2026-09-07：下载分发与官方渠道的通连诊断）——
    //     官方域（api/download）与镜像域 HEAD 探测，短超时不拖死；不通走 WARN（网络态）
    for row in net_probes() {
        put(row);
    }
    Ok(rows)
}

/// 网络通连探测（D13）：HEAD 各域，5s 连接超时，**并行探测**（串行最坏 5s×n 会拖死 doctor）。
/// 可达即 OK（HTTP 任意状态码都算域通，DNS 失败或超时才 WARN）。官方不通时 install
/// 自动走镜像兜底。域清单 = catalog 实际用到的渠道域（rg 提取）加镜像域。
///
/// 探针目标表（纯函数，自测 7 机检面）：自升级三面与 selfupdate 常量同源——
/// 仓库随 REPO、镜像探针打 ark/stable 主段（D41 B；段缺省 404 也算域通不误报）。
fn net_probe_targets() -> Vec<(&'static str, String, &'static str)> {
    let probe_asset_win = "ark-x86_64-pc-windows-gnu.exe";
    vec![
        (
            "net-github-api",
            format!(
                "https://api.github.com/repos/{}/releases/latest",
                crate::selfupdate::REPO
            ),
            "官方版本解析（GitHub API）",
        ),
        (
            "net-github-dl",
            format!(
                "https://github.com/{}/releases/download/dev/ark-aarch64-apple-darwin",
                crate::selfupdate::REPO
            ),
            "官方资产下载；不通自动回落镜像",
        ),
        (
            "net-mirror",
            crate::download::mirror_url("ark", "stable", &format!("{probe_asset_win}.sha256")),
            "自建分发镜像（兜底通道）",
        ),
        (
            "net-aka-ms",
            "https://aka.ms/vs/17/release/vs_buildtools.exe".to_string(),
            "vsbuild 引导器（aka.ms）",
        ),
        (
            "net-rsproxy",
            "https://rsproxy.cn".to_string(),
            "rust 工具链中国镜像源",
        ),
        (
            "net-goproxy-cn",
            "https://goproxy.cn".to_string(),
            "go 模块中国镜像源（GOPROXY）",
        ),
        (
            "net-npmmirror",
            "https://registry.npmmirror.com".to_string(),
            "bun npm 中国镜像源（bunfig）",
        ),
        (
            "net-go-dev",
            "https://go.dev".to_string(),
            "go 官方下载（go.dev/dl）",
        ),
        ("net-xai", "https://x.ai".to_string(), "grok 官方 CDN"),
        (
            "net-ziglang",
            "https://ziglang.org".to_string(),
            "zig 官方下载",
        ),
        (
            "net-docker",
            "https://download.docker.com".to_string(),
            "docker 官方 static zip",
        ),
        (
            "net-dotnet",
            "https://dotnetcli.azureedge.net".to_string(),
            "dotnet SDK 官方 CDN",
        ),
        (
            "net-msdl",
            "https://msdl.microsoft.com".to_string(),
            "oscdimg 微软符号库",
        ),
    ]
}

fn net_probes() -> Vec<DoctorRow> {
    // 并行 HEAD：每域一线程，join 后按原序出结果
    let targets = net_probe_targets();
    let handles: Vec<_> = targets
        .iter()
        .map(|(_, url, _)| {
            let url = url.to_string();
            std::thread::spawn(move || http_head_ok(&url))
        })
        .collect();
    let results: Vec<bool> = handles
        .into_iter()
        .map(|h| h.join().unwrap_or(false))
        .collect();
    targets
        .iter()
        .zip(results)
        .map(|((name, _, note), ok)| DoctorRow {
            name,
            status: if ok { "OK" } else { "WARN" },
            detail: if ok {
                vec![note.to_string()]
            } else {
                vec![format!("不通：{note}")]
            },
        })
        .collect()
}

/// HEAD 探测：连接与整请求均 5s；**拿到任意 HTTP 状态码即算域通**（含 403/405——不少域拒 HEAD，
/// 但能被 HTTP 层拒绝就说明 DNS 与 TLS 通），仅传输层错（DNS 失败/超时/连接拒）为不通。
fn http_head_ok(url: &str) -> bool {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(5))
        .build();
    match agent.head(url).set("User-Agent", "ark-doctor-net").call() {
        Ok(_) => true,
        Err(ureq::Error::Status(_, _)) => true, // HTTP 层有响应 = 域通
        Err(_) => false,                        // 传输层错 = 不通
    }
}

/// D12 部署深诊：rust 的 toolchain 激活态、vsbuild 的组件实装与机器级 PATH。
/// 判据全部来自安装时写入的产物（rustup-init 装 stable、bootstrapper 装组件三件套、
/// install 写机器 PATH），工具未装则检查不出。
fn deploy_probes(srows: &[StatusRow], env_root: &Path) -> Vec<DoctorRow> {
    let installed = |n: &str| srows.iter().any(|r| r.name == n && r.installed.is_some());
    let mut out = Vec::new();

    if installed("rust") {
        // 走 EnvRoot 重定位后的 rustup，不碰 PATH 上另一套工具链
        #[cfg(windows)]
        let rustup = crate::rustup::cargo_home(env_root)
            .join("bin")
            .join("rustup.exe");
        #[cfg(not(windows))]
        let rustup = crate::rustup::cargo_home(env_root)
            .join("bin")
            .join("rustup");
        let probe = std::process::Command::new(&rustup)
            .args(["show", "active-toolchain"])
            .env("RUSTUP_HOME", crate::rustup::rustup_home(env_root))
            .env("CARGO_HOME", crate::rustup::cargo_home(env_root))
            .output();
        let ok = probe
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("stable"))
            .unwrap_or(false);
        out.push(row_cfg(
            "rust-toolchain",
            ok,
            "rustup active-toolchain 为 stable",
            "ark install rust（rustup update stable）",
        ));
    }
    if installed("vsbuild") && cfg!(windows) {
        // 组件实装：MSBuild.exe 与 VC\Tools\MSVC（VCTools 组件三件套的产物根）
        let msbuild = crate::vsbuild::msbuild_exe(env_root);
        let msvc = crate::vsbuild::install_root(env_root)
            .join("VC")
            .join("Tools")
            .join("MSVC");
        let comp_ok = msbuild.exists() && msvc.is_dir();
        out.push(row_cfg(
            "vsbuild-components",
            comp_ok,
            "MSBuild.exe 与 VC\\Tools\\MSVC 组件实装",
            "ark install vsbuild（补装组件三件套）",
        ));
        // 机器级 PATH 注册态（install 写 HKLM；目录在才要求注册）
        let dirs = crate::vsbuild::machine_path_dirs(env_root);
        let path_ok = !dirs.is_empty()
            && dirs
                .iter()
                .all(|d| crate::platform::machine_path_contains(d).unwrap_or(false));
        out.push(row_cfg(
            "vsbuild-machine-path",
            path_ok,
            "MSBuild 与 cl 目录已注册机器级 PATH",
            "ark install vsbuild（重注册机器 PATH）",
        ));
    }
    out
}

/// D11 配置健康检查：只读比对 ome 各写入动作的目标态（heal-mirror 的 bunfig/goproxy、
/// rustup.rs 的 cargo 镜像与重定位变量、install 的遥测开关）。判据全部有写入动作背书，
/// 不发明新配置项；工具未装则该检查不出（干净简洁）。
fn config_health(srows: &[StatusRow], env_root: &Path) -> Vec<DoctorRow> {
    let installed = |n: &str| srows.iter().any(|r| r.name == n && r.installed.is_some());
    let home = dirs::home_dir();
    let mut out = Vec::new();

    // bunfig npmmirror（heal_bunfig 目标态）
    if installed("bun") {
        let ok = home
            .as_ref()
            .map(|h| {
                std::fs::read_to_string(h.join(".bunfig.toml"))
                    .map(|c| c.contains("npmmirror"))
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        out.push(row_cfg(
            "config-bunfig",
            ok,
            "~/.bunfig.toml npmmirror 镜像",
            "ark heal bunfig",
        ));
    }
    // goproxy.cn（heal_goproxy 目标态：Windows 走 GOENV 文件直写（D44 收编），POSIX 配置文件）
    if installed("go") {
        let ok = if cfg!(windows) {
            std::process::Command::new("go")
                .args(["env", "GOPROXY"])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains("goproxy.cn"))
                .unwrap_or(false)
        } else {
            home.as_ref()
                .map(|h| {
                    std::fs::read_to_string(h.join(".config").join("go").join("env"))
                        .map(|c| c.contains("goproxy.cn"))
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        };
        out.push(row_cfg(
            "config-goproxy",
            ok,
            "GOPROXY=goproxy.cn 镜像",
            "ark heal goproxy",
        ));
    }
    // rust：cargo sparse 镜像与重定位变量（rustup.rs 写入态；CARGO_HOME 重定位 EnvRoot）
    if installed("rust") {
        let cargo_home = crate::platform::get_user_env_var("CARGO_HOME")
            .ok()
            .flatten()
            .map(PathBuf::from)
            .unwrap_or_else(|| env_root.join("cargo"));
        let ok = std::fs::read_to_string(cargo_home.join("config.toml"))
            .map(|c| c.contains("rsproxy"))
            .unwrap_or(false);
        out.push(row_cfg(
            "config-cargo-mirror",
            ok,
            "cargo config.toml rsproxy sparse 镜像",
            "ark install rust（重建配置）",
        ));
        let want = |v: &str| env_root.join(v);
        let (ru_ok, ca_ok) = (
            crate::platform::get_user_env_var("RUSTUP_HOME")
                .ok()
                .flatten()
                .map(|v| Path::new(&v) == want("rustup"))
                .unwrap_or(false),
            crate::platform::get_user_env_var("CARGO_HOME")
                .ok()
                .flatten()
                .map(|v| Path::new(&v) == want("cargo"))
                .unwrap_or(false),
        );
        out.push(row_cfg(
            "config-rust-relocate",
            ru_ok && ca_ok,
            "RUSTUP_HOME/CARGO_HOME 重定位 EnvRoot",
            "ark install rust",
        ));
    }
    // 遥测关闭（ensure_user_env_overrides 写入态）
    let mut tel: Vec<String> = Vec::new();
    if installed("pwsh") {
        tel.push("POWERSHELL_TELEMETRY_OPTOUT".into());
        tel.push("POWERSHELL_UPDATECHECK".into());
    }
    if installed("dotnet") {
        tel.push("DOTNET_CLI_TELEMETRY_OPTOUT".into());
    }
    if !tel.is_empty() {
        let bad: Vec<String> = tel
            .iter()
            .filter(|k| {
                crate::platform::get_user_env_var(k)
                    .ok()
                    .flatten()
                    .is_none()
            })
            .cloned()
            .collect();
        out.push(row_cfg(
            "config-telemetry",
            bad.is_empty(),
            "遥测关闭用户变量（pwsh/dotnet）",
            "ark install pwsh / dotnet（重设开关）",
        ));
        // detail 精确化：缺哪个
        if !bad.is_empty() {
            if let Some(last) = out.last_mut() {
                last.detail = vec![format!("缺: {}", bad.join(", "))];
            }
        }
    }
    out
}

/// 配置行构造：达标 OK，不达标 WARN（heal/install 可修，不拦退出）。
fn row_cfg(name: &'static str, ok: bool, what: &str, fix: &str) -> DoctorRow {
    if ok {
        DoctorRow {
            name,
            status: "OK",
            detail: vec![what.to_string()],
        }
    } else {
        DoctorRow {
            name,
            status: "WARN",
            detail: vec![format!("{what} 未达标，修复: {fix}")],
        }
    }
}

/// 收集全量形态（run_doctor_with 的空回调兼容口）。
pub fn run_doctor(cat: &Catalog, env_root: &Path) -> Result<Vec<DoctorRow>, String> {
    run_doctor_with(cat, env_root, |_| {})
}

/// check 项的人读描述（TTY 报告面用）：老十项 OK 时 detail 为空，名字本身意义不明，
/// 这里给每个检查名一句人话；新项（config/deploy/net）自带 detail 优先。
pub fn check_desc(name: &str) -> &str {
    match name {
        "envroot-writable" => "环境根目录可写",
        "version-drift" => "版本与锁定一致",
        "probe-fail" => "已装工具版本探测正常",
        "not-on-path" => "已装工具 PATH 注册齐全",
        "pin-missing" => "版本锁定（pin）完整",
        "sha-missing" => "sha256 校验锚完整",
        "dead-path-entries" => "用户 PATH 无死链",
        "dup-path-entries" => "用户 PATH 无重复条目",
        "cache-orphans" => "下载缓存无孤儿资产",
        _ => name,
    }
}

/// 汇总：FAIL 与 WARN 计数。
pub fn summarize(rows: &[DoctorRow]) -> (usize, usize, Vec<String>, Vec<String>) {
    let fails = rows
        .iter()
        .filter(|r| r.status == "FAIL")
        .map(|r| r.name.to_string())
        .collect::<Vec<_>>();
    let warns = rows
        .iter()
        .filter(|r| r.status == "WARN")
        .map(|r| r.name.to_string())
        .collect::<Vec<_>>();
    (fails.len(), warns.len(), fails, warns)
}

fn check_envroot_writable(env_root: &Path) -> DoctorRow {
    let probe = env_root.join(".ark-doctor-probe");
    let ok = std::fs::create_dir_all(env_root)
        .ok()
        .and_then(|_| std::fs::write(&probe, b"1").ok())
        .is_some();
    let _ = std::fs::remove_file(&probe);
    DoctorRow {
        name: "envroot-writable",
        status: if ok { "OK" } else { "FAIL" },
        detail: if ok {
            vec![]
        } else {
            vec![format!("EnvRoot 不可写: {}", env_root.display())]
        },
    }
}

/// 版本漂移：locked 已设但 installed 缺失或不等：部署错误的核心形态。
/// agent 类排除（D07 存量原地纳管：PATH 在位即接管不升级；D30 起 agent 漂移对账归 omc
/// 舰队面，ome 不在 doctor 报，单机三态走 `ark status`）。
fn check_version_drift(srows: &[StatusRow]) -> DoctorRow {
    let mut detail = Vec::new();
    for r in srows {
        if r.category == "agent" {
            continue;
        }
        let Some(locked) = &r.locked else { continue };
        match &r.installed {
            None => detail.push(format!("{}: pin {locked} 但未装/探测不到", r.name)),
            Some(inst) if inst != locked => {
                // D37 定案：update 拉云端最新不回写 pin，实装超前云端 pin 多为数据面滞后，
                // 如实 drift 并给反馈去向（锁定单源归数据面）
                detail.push(format!(
                    "{}: pin {locked} 实装 {inst}（实装超前：update 拉新而云端 pin 未升，向数据面反馈）",
                    r.name
                ))
            }
            _ => {}
        }
    }
    DoctorRow {
        name: "version-drift",
        status: if detail.is_empty() { "OK" } else { "FAIL" },
        detail,
    }
}

/// 探测失败：exe **文件在位**但版本读不出（正则缺项或二进制损坏）。
/// `StatusRow.exe` 是期望路径，未装工具也是 Some，不能当「文件在」。
fn check_probe_fail(srows: &[StatusRow]) -> DoctorRow {
    let mut detail = Vec::new();
    for r in srows {
        if r.exe.as_ref().is_some_and(|p| p.is_file()) && r.installed.is_none() {
            detail.push(format!(
                "{}: exe 在位但版本探测失败（检查 toolver 正则或二进制完整性）",
                r.name
            ));
        }
    }
    DoctorRow {
        name: "probe-fail",
        status: if detail.is_empty() { "OK" } else { "WARN" },
        detail,
    }
}

/// 装而未上 PATH：已装且定义了 bin 但用户 PATH 不含（Windows）。
fn check_not_on_path(cat: &Catalog, srows: &[StatusRow], env_root: &Path) -> DoctorRow {
    let mut detail = Vec::new();
    for r in srows {
        if r.installed.is_none() || r.path || r.exe.is_none() {
            continue;
        }
        // 只查有 bin 字段的绿色/官方布局工具；msi 型（pwsh）与机器级（vsbuild）PATH 由安装器管，跳过
        let Ok(def) = cat.tool(&r.name) else { continue };
        if def.bin().is_none() || crate::vsbuild::is_vsbuild(def) {
            continue;
        }
        let _ = env_root;
        detail.push(format!(
            "{}: 已装但不在用户 PATH（ark install {} 可补）",
            r.name, r.name
        ));
    }
    DoctorRow {
        name: "not-on-path",
        status: if detail.is_empty() { "OK" } else { "WARN" },
        detail,
    }
}

/// 本平台在管但未 pin：install 不带选项会失败的前置异味（evergreen 条目如 vsbuild/rust 无 pin 属设计，排除）。
fn check_pin_missing(cat: &Catalog, srows: &[StatusRow]) -> DoctorRow {
    let mut detail = Vec::new();
    for r in srows {
        if r.exe.is_none() {
            continue; // 平台不适用不算
        }
        let Ok(def) = cat.tool(&r.name) else { continue };
        if !toolver::platform_managed(def)
            || r.locked.is_some()
            || crate::vsbuild::is_vsbuild(def)
            || crate::rustup::is_rustup(def)
            || crate::selfupdate::is_ome_self(def)
        {
            continue;
        }
        detail.push(format!(
            "{}: 本平台在管但未 pin（ark pin {} --version <ver>）",
            r.name, r.name
        ));
    }
    DoctorRow {
        name: "pin-missing",
        status: if detail.is_empty() { "OK" } else { "WARN" },
        detail,
    }
}

/// 已 pin 但 sha 缺失（校验降级到官方源；官方源也缺则裸下载）。
/// uv-git 型（uv tool install git 源）无下载资产、sha 不适用，排除。
fn check_sha_missing(cat: &Catalog, srows: &[StatusRow]) -> DoctorRow {
    let mut detail = Vec::new();
    for r in srows {
        if r.locked.is_none() {
            continue;
        }
        let Ok(def) = cat.tool(&r.name) else { continue };
        if sha_missing_for_tool(def) {
            detail.push(format!("{}: pin 无 sha256（重装一次可回填）", r.name));
        }
    }
    DoctorRow {
        name: "sha-missing",
        status: if detail.is_empty() { "OK" } else { "WARN" },
        detail,
    }
}

/// 单工具 sha 缺失判定（纯函数可测）：已 pin 资产型但无 sha；uv-git 无资产语义除外。
fn sha_missing_for_tool(def: &crate::catalog::Tool) -> bool {
    def.pin_sha256().is_none() && def.extract() != Some("uv-git")
}

/// PATH 死链：EnvRoot 域内的用户 PATH 条目指向不存在的目录。
/// 用 Path::starts_with（按路径分量）避免 `D:\ohmyenv` 误伤 `D:\ohmyenv-rs`。
fn check_dead_path_entries(entries: &[String], env_root: &Path) -> DoctorRow {
    let mut detail = Vec::new();
    for e in entries {
        let t = e.trim();
        if t.is_empty() {
            continue;
        }
        let expanded = PathBuf::from(crate::platform::expand_env_vars(t));
        if !expanded.starts_with(env_root) && !Path::new(t).starts_with(env_root) {
            continue;
        }
        if !expanded.is_dir() {
            detail.push(format!("死链: {t}"));
        }
    }
    DoctorRow {
        name: "dead-path-entries",
        status: if detail.is_empty() { "OK" } else { "WARN" },
        detail,
    }
}

/// PATH 重复条目（展开后大小写不敏感比较）。
fn check_dup_path_entries(entries: &[String]) -> DoctorRow {
    let mut seen: Vec<String> = Vec::new();
    let mut detail = Vec::new();
    for e in entries {
        let key = crate::platform::expand_env_vars(e.trim())
            .trim_end_matches(['\\', '/'])
            .to_lowercase();
        if key.is_empty() {
            continue;
        }
        let cur = e.trim().to_string();
        if seen.contains(&key) && !detail.iter().any(|d: &String| d.contains(&cur)) {
            detail.push(format!("重复: {cur}"));
        }
        seen.push(key);
    }
    DoctorRow {
        name: "dup-path-entries",
        status: if detail.is_empty() { "OK" } else { "WARN" },
        detail,
    }
}

/// 缓存孤儿：cache 下已无任何 pin 指向的资产文件。
/// 派生资产（专用模块的引导器/插件与自升级通道资产——被消费但不属任何 pin）放行不算孤儿。
fn check_cache_orphans(cat: &Catalog, env_root: &Path) -> DoctorRow {
    let cache = env_root.join("cache");
    let Ok(rd) = std::fs::read_dir(&cache) else {
        return DoctorRow {
            name: "cache-orphans",
            status: "OK",
            detail: vec![],
        };
    };
    let pinned: Vec<String> = cat
        .order
        .iter()
        .filter_map(|n| {
            cat.tools
                .get(n)
                .map(|t| t.pin_asset().unwrap_or("").to_string())
        })
        .filter(|a| !a.is_empty())
        .collect();
    let bootstraps: Vec<String> = cat
        .order
        .iter()
        .filter_map(|n| cat.tools.get(n).and_then(|t| t.bootstrap_asset()))
        .map(str::to_string)
        .collect();
    let mut detail = Vec::new();
    let mut bytes = 0u64;
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_asset_like = name.ends_with(".zip")
            || name.ends_with(".tar.gz")
            || name.ends_with(".tar.xz")
            || name.ends_with(".exe")
            || name.ends_with(".msi");
        if is_asset_like && !pinned.contains(&name) && !is_derived_asset(&name, &bootstraps) {
            if let Ok(md) = entry.metadata() {
                bytes += md.len();
            }
            detail.push(name);
        }
    }
    let status = if detail.is_empty() { "OK" } else { "WARN" };
    if !detail.is_empty() {
        let mb = bytes as f64 / 1024.0 / 1024.0;
        detail.insert(
            0,
            format!(
                "{} 个孤儿资产共 {mb:.1} MB（无 pin 指向，可清）",
                detail.len()
            ),
        );
    }
    DoctorRow {
        name: "cache-orphans",
        status,
        detail,
    }
}

/// 派生资产判定（纯函数可测）：catalog 声明的 bootstrap 资产（如 7z 的 7zr.exe）、
/// 专用安装模块的引导器（vs_buildtools / rustup-init）、docker compose 插件、
/// ark 自升级通道资产——均被安装链消费但不属任何 pin（D41 双前缀：ome-* 兼容名与 ark-* 主名同认）。
fn is_derived_asset(name: &str, bootstraps: &[String]) -> bool {
    bootstraps.iter().any(|b| b == name)
        || name == crate::vsbuild::BOOTSTRAPPER
        || name == crate::rustup::INIT_EXE
        || name.starts_with("docker-compose-")
        || name.starts_with("ome-")
        || name.starts_with("ark-")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, category: &str, locked: Option<&str>, installed: Option<&str>) -> StatusRow {
        StatusRow {
            name: name.to_string(),
            category: category.to_string(),
            locked: locked.map(str::to_string),
            installed: installed.map(str::to_string),
            path: false,
            exe: None,
        }
    }

    /// 依赖分组统计（D07）：按九类归类计数；空组不出；missing/drift 计数与组内一致。
    #[test]
    fn 依赖分组_计数与空组过滤() {
        let srows = vec![
            row("claude", "agent", Some("2.1.251"), Some("2.1.246")),
            row("kimi", "agent", Some("0.39.1"), None),
            row("jq", "cli", Some("1.8.2"), Some("1.8.2")),
            row("gh", "cli", None, None),
        ];
        let gs = dep_group_stats(&srows);
        let agent = gs.iter().find(|g| g.category == "agent").unwrap();
        assert_eq!((agent.tools, agent.missing, agent.drift), (2, 1, 1));
        let cli = gs.iter().find(|g| g.category == "cli").unwrap();
        assert_eq!((cli.tools, cli.missing, cli.drift), (2, 1, 0));
        assert!(!gs.iter().any(|g| g.category == "runtime"), "空组不出");
    }

    /// sha 缺失判定：普通资产型缺 sha 命中；uv-git（无下载资产）豁免；已有 sha 不命中。
    #[test]
    fn sha缺失判定_uvgit豁免() {
        let plain = crate::catalog::Tool {
            extract: Some("zip".to_string()),
            ..Default::default()
        };
        assert!(sha_missing_for_tool(&plain));
        let uv_git = crate::catalog::Tool {
            extract: Some("uv-git".to_string()),
            ..Default::default()
        };
        assert!(!sha_missing_for_tool(&uv_git), "uv-git 无 sha 语义不告警");
        #[cfg(windows)]
        {
            let with_sha = crate::catalog::Tool {
                extract: Some("zip".to_string()),
                sha256: Some("ABCD".to_string()),
                ..Default::default()
            };
            assert!(!sha_missing_for_tool(&with_sha));
        }
    }

    /// 派生资产判定：bootstrap 清单、模块引导器、compose/ome 前缀族放行；真孤儿命中。
    #[test]
    fn 派生资产判定_白名单与前缀族() {
        let bootstraps = vec!["7zr.exe".to_string()];
        assert!(is_derived_asset("7zr.exe", &bootstraps));
        assert!(is_derived_asset(crate::vsbuild::BOOTSTRAPPER, &bootstraps));
        assert!(is_derived_asset(crate::rustup::INIT_EXE, &bootstraps));
        assert!(is_derived_asset("docker-compose-v5.5.0.exe", &bootstraps));
        assert!(is_derived_asset(
            "ome-x86_64-pc-windows-msvc.exe",
            &bootstraps
        ));
        assert!(is_derived_asset(
            "ark-x86_64-pc-windows-msvc.exe",
            &bootstraps
        ));
        assert!(is_derived_asset(
            "ark-x86_64-pc-windows-gnu.exe",
            &bootstraps
        ));
        assert!(
            !is_derived_asset("claude-win32-x64.zip", &bootstraps),
            "真孤儿不放行"
        );
        assert!(!is_derived_asset(
            "reader-v0.1.0-x86_64-pc-windows-msvc.zip",
            &bootstraps
        ));
    }

    #[test]
    fn 网络探针_自升级三面与selfupdate常量同源() {
        // D41 自测 7：探针 URL 必须由 REPO 与 mirror_url 派生（防软漂移；旧仓名残留即红）
        let t = net_probe_targets();
        let api = t.iter().find(|(n, _, _)| *n == "net-github-api").unwrap();
        assert!(
            api.1.contains(crate::selfupdate::REPO),
            "API 探针应含 REPO: {}",
            api.1
        );
        assert!(!api.1.contains("ohmyenv-rs"), "旧仓名不得残留: {}", api.1);
        let dl = t.iter().find(|(n, _, _)| *n == "net-github-dl").unwrap();
        assert!(
            dl.1.contains(crate::selfupdate::REPO) && dl.1.contains("ark-aarch64"),
            "下载探针应含 REPO 与主名资产: {}",
            dl.1
        );
        let mirror = t.iter().find(|(n, _, _)| *n == "net-mirror").unwrap();
        assert_eq!(
            mirror.1,
            crate::download::mirror_url("ark", "stable", "ark-x86_64-pc-windows-gnu.exe.sha256"),
            "镜像探针应与 mirror_url 同构"
        );
    }

    /// 死链判定：EnvRoot 域内不存在目录命中；域外与存在目录不命中。
    #[test]
    fn 死链判定_envroot域内() {
        let dir = tempfile::tempdir().expect("临时目录");
        let root = dir.path().join("env");
        std::fs::create_dir_all(root.join("jq")).expect("建目录");
        let entries = vec![
            root.join("jq").display().to_string(),
            root.join("ghost").display().to_string(),
            "C:\\Windows\\System32".to_string(),
        ];
        let row = check_dead_path_entries(&entries, &root);
        assert_eq!(row.status, "WARN");
        assert_eq!(row.detail.len(), 1, "只报域内死链: {:?}", row.detail);
        assert!(row.detail[0].contains("ghost"));
    }

    /// 重复条目判定：尾斜杠与大小写不敏感；唯一条目 OK。
    #[test]
    fn 重复条目判定_大小写与尾斜杠() {
        let entries = vec![
            r"D:\ohmyenv\jq".to_string(),
            r"d:\OHMYENV\jq\".to_string(),
            r"D:\ohmyenv\gh\bin".to_string(),
        ];
        let row = check_dup_path_entries(&entries);
        assert_eq!(row.status, "WARN");
        assert_eq!(row.detail.len(), 1);

        let ok = check_dup_path_entries(&[r"D:\a".to_string(), r"D:\b".to_string()]);
        assert_eq!(ok.status, "OK");
    }

    #[test]
    fn 死链判定_不误伤前缀兄弟目录() {
        let dir = tempfile::tempdir().expect("临时目录");
        let root = dir.path().join("env");
        std::fs::create_dir_all(&root).expect("建 root");
        let sibling = dir.path().join("env-rs");
        std::fs::create_dir_all(&sibling).expect("建兄弟");
        let row = check_dead_path_entries(&[sibling.display().to_string()], &root);
        assert_eq!(row.status, "OK", "env-rs 不是 env 之下: {:?}", row.detail);
    }

    #[test]
    fn probe失败_期望路径未装不算探测失败() {
        let srows = vec![StatusRow {
            name: "jq".into(),
            category: "cli".into(),
            locked: Some("1.8.2".into()),
            installed: None,
            path: false,
            exe: Some(PathBuf::from("/definitely-missing-ome-jq.exe")),
        }];
        let row = check_probe_fail(&srows);
        assert_eq!(row.status, "OK", "未装不应报 probe-fail: {:?}", row.detail);
    }
}
