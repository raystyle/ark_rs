//! toolver：已装版本探测，移植 helpers.ps1 的 Get-InstalledVersion（888-935 行）。
//! 探测参数与输出版本正则自源码硬编码表迁 catalog 字段（D28 入册清单化：每工具
//! `probe_args` 缺省 `["--version"]`、`probe_pattern` 取第 1 捕获组；字段契约见 R001），
//! 新工具入册漏带由 tests/catalog_lint.rs 结构机检拦下，不再靠装后实测暴露。
//! exe 路径解析：official 工具 exe 字段含 %VAR% 环境变量（展开为绝对路径），
//! 其余相对 EnvRoot 拼接。装后读版本带 5 次递增重试（500ms * i，对齐 Install-ToolVersion 末尾）。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use regex::Regex;

use crate::catalog::Tool;

/// exe 路径解析：official（exe 使用环境变量或绝对路径）展开为绝对路径；
/// 其余 Windows 下相对 EnvRoot；Linux / macOS 下有平台专属 exe（`linux_exe`/`mac_exe`，
/// 相对 install_dir）走 dir 展开，回退通用 exe（Windows 风格，自带 dir 段）时相对 EnvRoot。
/// 布局含 `{version}` 占位（D43 zig 版本目录型）：无版本上下文时 glob 扫描占位段取
/// **semver 最大**的在位版本（探测语义；字典序会把 0.9 排 0.16 前，M025 同型）；
/// 无在位版本返回占位填充 0.0.0 的路径（探测 Command 失败即 None=未装）。
pub fn exe_path(tool: &Tool, env_root: &Path) -> Result<PathBuf, String> {
    exe_path_inner(tool, env_root, None)
}

/// 定版形态（D43）：占位以给定版本直替换（装后验证锚定刚装版本，glob 取 max 在
/// 降级场景会锚错）；无占位时与 `exe_path` 等价。
pub fn exe_path_for_version(
    tool: &Tool,
    env_root: &Path,
    version: &str,
) -> Result<PathBuf, String> {
    exe_path_inner(tool, env_root, Some(version))
}

fn exe_path_inner(tool: &Tool, env_root: &Path, version: Option<&str>) -> Result<PathBuf, String> {
    let exe = tool.exe().ok_or_else(|| "工具缺少 exe 字段".to_string())?;
    // npm-tgz 型：bin 落 npm 全局 bin（随各端 node 生态走，无静态路径），PATH 现查；
    // 未在位时返回裸名（探测 None 即未装）
    if tool.extract() == Some("npm-tgz") {
        if let Some(bin) = tool.bin() {
            if let Some(found) = find_on_path(bin) {
                return Ok(found);
            }
            return Ok(PathBuf::from(bin));
        }
    }
    if crate::platform::is_official_exe(exe) {
        return Ok(PathBuf::from(expand_env_vars(exe)));
    }
    // {version} 占位（D43）：定版直替换；探测态 glob 取 semver 最大在位版本
    let fill = |layout: &str, base: &Path| -> PathBuf {
        if !layout.contains("{version}") {
            return base.join(layout);
        }
        if let Some(v) = version {
            return base.join(layout.replace("{version}", v));
        }
        match glob_version_segment(layout, base) {
            Some(seg) => base.join(seg),
            None => base.join(layout.replace("{version}", "0.0.0")),
        }
    };
    #[cfg(windows)]
    {
        Ok(fill(exe, env_root))
    }
    #[cfg(not(windows))]
    if let Some(pexe) = tool.platform_exe() {
        let base = tool
            .dir()
            .map(|d| {
                crate::platform::join_if_relative(env_root, crate::platform::expand_install_path(d))
            })
            .unwrap_or_else(|| env_root.to_path_buf());
        // 专属 exe 允许带子目录（如 python 的 bin/python），将反斜杠统一为正斜杠
        Ok(fill(&pexe.replace('\\', "/"), &base))
    } else {
        // 通用 exe 是 Windows 名录风格：路径自带 dir 段，相对 EnvRoot 直接拼
        Ok(fill(&exe.replace('\\', "/"), env_root))
    }
}

/// 版本目录占位 glob（D43，纯路径扫描）：布局串首个含 `{version}` 的段列其父目录，
/// 前后缀锚定提取中段按 semver 取最大（zig 版本目录型布局的在位发现）。
/// 返回占位段填充后的完整布局串；无在位候选返回 None。
fn glob_version_segment(layout: &str, base: &Path) -> Option<String> {
    let sep = if cfg!(windows) { '\\' } else { '/' };
    let parts: Vec<&str> = layout.split(sep).collect();
    let idx = parts.iter().position(|p| p.contains("{version}"))?;
    let (pre, rest) = parts[idx].split_once("{version}")?;
    let suffix = rest
        .trim_start_matches('{')
        .trim_end_matches('}')
        .to_string();
    let mut best: Option<(Vec<u64>, String)> = None;
    // 前缀段以 base 为根拼接（布局串是相对形态，无 base 会扫到 CWD）
    let mut root = base.to_path_buf();
    for p in &parts[..idx] {
        root = root.join(p);
    }
    let rd = std::fs::read_dir(&root).ok()?;
    for e in rd {
        let Ok(entry) = e else { continue };
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(mid) = name.strip_prefix(pre).and_then(|m| m.strip_suffix(&suffix)) else {
            continue;
        };
        if let Some(key) = crate::resolve::version_key(mid) {
            if best.as_ref().is_none_or(|(k, _)| {
                crate::resolve::semver_cmp(&key, k) == std::cmp::Ordering::Greater
            }) {
                best = Some((key, name));
            }
        }
    }
    let (_, name) = best?;
    let mut out: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
    out[idx] = name;
    Some(out.join(&sep.to_string()))
}

/// 工具是否 official 布局（exe 使用环境变量或绝对路径，installDir/bin 走官方目录，不进 EnvRoot）。
pub fn is_official(tool: &Tool) -> bool {
    tool.exe()
        .map(crate::platform::is_official_exe)
        .unwrap_or(false)
}

/// 当前平台是否管理该工具（平台不适用时 status 出空态行、install/update/pin/query 跳过）：
/// - Windows：通用 `exe` 存在。
/// - Linux/macOS：平台专属 exe（`linux_exe`/`mac_exe`）存在——布局类通用字段是 Windows
///   语义（如 `git\cmd\git.exe`），非 Windows 回退它们会把 Windows-only 工具误判在管
///   （2026-09-01 WSL install all 实证：git/aria2/7z 等走到 resolve 才报未 pin）。
pub fn platform_managed(tool: &Tool) -> bool {
    #[cfg(windows)]
    {
        tool.exe.is_some()
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        tool.linux_exe.is_some()
    }
    #[cfg(target_os = "macos")]
    {
        // 只认 mac_exe：linux_exe 回退会把 shellcheck 等 Linux 资产当成 mac 在管
        tool.mac_exe.is_some()
    }
}

/// 环境变量展开（Windows `%VAR%`；Linux / macOS `$VAR` / `${VAR}`）。
/// 未定义的变量原样保留。
pub fn expand_env_vars(s: &str) -> String {
    crate::platform::expand_env_vars(s)
}

/// 版本探测参数（D28 自源码表迁 catalog `probe_args` 字段；对齐 Get-InstalledVersion 的 switch）。
/// 缺省 `["--version"]`；特例在 catalog 各节：7z `--help`、rmux/openssh `-V`、oscdimg 无参读横幅、
/// vsbuild `-version`、zig/go/lightpanda 子命令 `version`。
pub fn probe_args(tool: &Tool) -> Vec<String> {
    tool.probe_args
        .clone()
        .unwrap_or_else(|| vec!["--version".to_string()])
}

/// 解析版本（纯函数）：正则取 catalog `probe_pattern` 字段（第 1 捕获组）；
/// 逐个非空行找首个命中（多数工具首行即中；shellcheck 类首行是无版本横幅、
/// 版本在次行「version: 0.11.0」——2026-09-01 WSL 实证）。无字段返回 None。
pub fn parse_version(tool: &Tool, output: &str) -> Option<String> {
    let pattern = tool.probe_pattern.as_deref()?;
    let re = Regex::new(pattern).ok()?;
    for line in output.lines().filter(|l| !l.trim().is_empty()) {
        if let Some(caps) = re.captures(line) {
            return Some(caps.get(1)?.as_str().to_string());
        }
    }
    None
}

/// update 的锁定漂移三态（D49）：installed 探针值对 catalog pin 的对照。
/// 解析失败保守按 Behind（补装 pin 版自愈，不静默跳过）。
#[derive(Debug, PartialEq, Eq)]
pub enum PinDrift {
    /// 本机与锁定一致：skip「已是最新」（原行为）
    Current,
    /// 本机落后锁定（或未装）：云端无新版也要真装 pin 版——旧判据在此 skip，
    /// 挡住一票 installed 落后 pin 的真装机（ohmycloud 舰队分型实锤的 a 形态）
    Behind,
    /// 本机领先锁定：保持现状如实报，消提示需数据面滚锁
    Ahead,
}

/// 漂移判定（纯函数）：None 探针（未装）按 Behind 处理（补装自愈）。
pub fn pin_drift(installed: Option<&str>, pin: &str) -> PinDrift {
    let Some(v) = installed else {
        return PinDrift::Behind;
    };
    match (
        crate::resolve::version_key(v),
        crate::resolve::version_key(pin),
    ) {
        (Some(i), Some(p)) if i == p => PinDrift::Current,
        (Some(i), Some(p)) if i > p => PinDrift::Ahead,
        _ => PinDrift::Behind,
    }
}

/// status 的 drift 判定（D49 尾统一口径）：npm-tgz 类不列（版本真源在 npm registry
/// 端上自管，catalog pin 不构成 drift 判据——omc 恒挂形态）；其余走 pin_drift 数值段
/// 口径（git 的 .windows.N 后缀形态数值等，旧字符串比较恒列的口径差）。
pub fn status_drift_hint(
    extract: Option<&str>,
    installed: Option<&str>,
    locked: Option<&str>,
) -> bool {
    if extract == Some("npm-tgz") {
        return false;
    }
    match (installed, locked) {
        (Some(i), Some(l)) => pin_drift(Some(i), l) != PinDrift::Current,
        _ => false,
    }
}

/// 探测已装版本：exe 不存在直接 None；运行 exe 取首行非空输出按 probe_pattern 解析。
pub fn installed_version(exe: &Path, tool: &Tool) -> Option<String> {
    if !exe.exists() {
        return None;
    }
    // 脚本 shim（.cmd/.bat，npm 全局 bin）不能被 CreateProcess 直接拉起，经 cmd /c 运行
    let is_shim = matches!(
        exe.extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .as_deref(),
        Some("cmd") | Some("bat")
    );
    let out = if is_shim {
        Command::new("cmd")
            .arg("/c")
            .arg(exe)
            .args(probe_args(tool))
            .output()
            .ok()?
    } else {
        Command::new(exe).args(probe_args(tool)).output().ok()?
    };
    // stdout 与 stderr 合并取首个非空行（对齐 pwsh 2>&1）
    let merged = format!(
        "{}\n{}",
        decode_output(&out.stdout),
        decode_output(&out.stderr)
    );
    parse_version(tool, &merged)
}

/// PATH 上查找命令（D07 agent 存量纳管判定）：返回首个命中位的完整路径，未命中 None。
/// Windows 依次试 `名.exe` 与裸名；POSIX 无扩展名直试裸名。
pub fn find_on_path(name: &str) -> Option<PathBuf> {
    let mut candidates: Vec<String> = if std::env::consts::EXE_EXTENSION.is_empty() {
        vec![name.to_string()]
    } else {
        vec![
            format!("{name}.{}", std::env::consts::EXE_EXTENSION),
            name.to_string(),
        ]
    };
    // npm 全局 shim 为 .cmd（无 .exe，如 browser-harness 的 bh）：Windows 补脚本候选
    #[cfg(windows)]
    {
        candidates.insert(1, format!("{name}.cmd"));
        candidates.insert(2, format!("{name}.bat"));
    }
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        for cand in &candidates {
            let p = dir.join(cand);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// 子进程输出解码：UTF-8 优先；UTF-16LE（无 BOM，实证 wsl --version 为 W,0,S,0,L,0 字节序）
/// 按 NUL 密度启发式判定后按 UTF-16 解——from_utf8_lossy 会给字符间插 U+FFFD，正则全灭。
fn decode_output(bytes: &[u8]) -> String {
    let head = &bytes[..bytes.len().min(128)];
    let nul = head.iter().filter(|&&b| b == 0).count();
    if nul * 4 > head.len() && bytes.len().is_multiple_of(2) {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        let s = String::from_utf16_lossy(&units);
        if !s.contains('\u{FFFD}') {
            return s;
        }
    }
    String::from_utf8_lossy(bytes).into_owned()
}

/// 装后版本读取：5 次递增重试（500ms * i），对齐 Install-ToolVersion 末尾的重试循环
/// （7zsfx 等解包后文件/杀软可能瞬态未就绪）。
pub fn installed_version_retried(exe: &Path, tool: &Tool) -> Option<String> {
    for i in 1..=5u32 {
        if let Some(v) = installed_version(exe, tool) {
            return Some(v);
        }
        std::thread::sleep(Duration::from_millis(500 * u64::from(i)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UTF-16LE 无 BOM 输出按 NUL 密度启发式解码（wsl --version 实测字节序 W,0,S,0,L,0）。
    #[test]
    fn 解码_utf16无bom输出() {
        let raw: Vec<u8> = "WSL 版本: 2.7.12.0"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let decoded = decode_output(&raw);
        assert!(
            decoded.starts_with("WSL 版本"),
            "应按 UTF-16LE 解: {decoded:?}"
        );
        // 纯 UTF-8 输出不受启发式影响
        assert_eq!(decode_output(b"gh version 2.98.0"), "gh version 2.98.0");
    }

    /// 正则命中：期望值取自各工具真实 --version 输出样例（对照 helpers.ps1 正则逐条）；
    /// 正则本体迁 catalog `probe_pattern` 字段（D28），此处按字段值构造（与真仓各节同值）。
    #[test]
    fn 版本正则_命中真实输出样例() {
        let cases: &[(&str, &str, &str)] = &[
            (r"PowerShell\s+(\d+\.\d+\.\d+)", "PowerShell 7.6.5", "7.6.5"),
            (r"gh version (\d+\.\d+\.\d+)", "gh version 2.98.0 (2026-01-01)", "2.98.0"),
            (r"git version (\S+)", "git version 2.55.0.windows.4", "2.55.0.windows.4"),
            (r"^v?(\d+\.\d+\.\d+)", "v1.3.1", "1.3.1"),
            (r"sops[ -]v?(\d+\.\d+\.\d+)", "sops 3.13.3 (latest)", "3.13.3"),
            (r"aria2 version (\d+\.\d+\.\d+)", "aria2 version 1.37.0", "1.37.0"),
            (r"7-Zip[^\r\n]*?(\d+\.\d+)", "\n7-Zip 26.02 (x64) : Copyright", "26.02"),
            (r"^(\d+\.\d+\.\d+)", "10.0.400", "10.0.400"),
            (r"fnm\s+v?(\d+\.\d+\.\d+)", "fnm 1.39.0", "1.39.0"),
            (r"gsudo\s+v?(\d+\.\d+\.\d+)", "gsudo v2.6.1", "2.6.1"),
            (r"uv (\d+\.\d+\.\d+)", "uv 0.12.6 (abc123 2026-01-01)", "0.12.6"),
            (r"Python (\d+\.\d+\.\d+)", "Python 3.12.11", "3.12.11"),
            (r"ripgrep (\d+\.\d+\.\d+)", "ripgrep 15.2.0", "15.2.0"),
            (r"jq-(\d+\.\d+\.\d+)", "jq-1.8.2", "1.8.2"),
            (r"mq\s+v?(\d+\.\d+\.\d+)", "mq 0.8.4", "0.8.4"),
            // wsl 首行即版本（中文输出实测样例），四段尾 .0 归一为三段对齐 tag 2.7.12
            (
                r"(\d+\.\d+\.\d+)(?:\.\d+)?",
                "WSL 版本: 2.7.12.0\n内核版本: 6.18.33.2-2",
                "2.7.12",
            ),
            (
                r"version v?(\d+\.\d+\.\d+)",
                "yq (https://github.com/mikefarah/yq/) version v4.53.6",
                "4.53.6",
            ),
            (r"starship (\d+\.\d+\.\d+)", "starship 1.26.0", "1.26.0"),
            (r"just\s+v?(\d+\.\d+\.\d+)", "just 1.58.0", "1.58.0"),
            (r"(\d+\.\d+\.\d+)", "ast-grep 0.45.1", "0.45.1"),
            (r"^herdr\s+v?(\d+\.\d+\.\d+)", "herdr 0.8.2", "0.8.2"),
            (r"rumdl\s+(\d+\.\d+\.\d+)", "rumdl 0.2.62", "0.2.62"),
            (r"rmux\s+(\d+\.\d+\.\d+)", "rmux 0.10.0", "0.10.0"),
            (r"go version go(\d+\.\d+\.\d+)", "go version go1.27.0 darwin/arm64", "1.27.0"),
            (r"OSCDIMG\s+(\d+\.\d+)", "\nOSCDIMG 2.56 CD-ROM and DVD-ROM Premastering Utility", "2.56"),
            (r"reader\s+(\d+\.\d+\.\d+)", "reader 0.1.0", "0.1.0"),
            // MSBuild -version 英文首行裸版本与中文横幅均取首段三段号
            (r"(\d+\.\d+\.\d+)", "17.14.51.32402", "17.14.51"),
            (
                r"(\d+\.\d+\.\d+)",
                "适用于 .NET Framework MSBuild 版本 17.14.51+25f168cee",
                "17.14.51",
            ),
            (
                r"version:\s*(\d+\.\d+\.\d+)",
                "ShellCheck - shell script analysis tool\nversion: 0.11.0",
                "0.11.0",
            ),
            (
                r"ffmpeg version n?(\d+\.\d+(?:\.\d+)?)",
                "ffmpeg version 9.0.1-essentials_build-www.gyan.dev Copyright (c) 2000-2026 the FFmpeg developers",
                "9.0.1",
            ),
            (
                r"ffmpeg version n?(\d+\.\d+(?:\.\d+)?)",
                "ffmpeg version n9.0-2026-08-12 Copyright (c) 2000-2026 the FFmpeg developers",
                "9.0",
            ),
        ];
        for (pattern, line, expect) in cases {
            let t = Tool {
                probe_pattern: Some(pattern.to_string()),
                ..Tool::default()
            };
            assert_eq!(
                parse_version(&t, line).as_deref(),
                Some(*expect),
                "{pattern} 应解析出版本"
            );
        }
    }

    /// probe_args 字段值：缺省 --version；显式数组（子命令形态）与空数组（无参横幅）如实传递。
    #[test]
    fn probe_args_缺省与显式() {
        assert_eq!(
            probe_args(&Tool::default()),
            vec!["--version".to_string()],
            "无 probe_args 字段应缺省 --version"
        );
        let subcmd = Tool {
            probe_args: Some(vec!["version".to_string()]),
            ..Tool::default()
        };
        assert_eq!(probe_args(&subcmd), vec!["version".to_string()]);
        let bare = Tool {
            probe_args: Some(vec![]),
            ..Tool::default()
        };
        assert!(probe_args(&bare).is_empty(), "oscdimg 无参形态应为空");
    }

    #[test]
    fn 版本解析_取首个非空行() {
        // 7z --help 首行为空行（pwsh 注释实测）
        let t = Tool {
            probe_pattern: Some(r"7-Zip[^\r\n]*?(\d+\.\d+)".to_string()),
            ..Tool::default()
        };
        assert_eq!(
            parse_version(&t, "\n\n7-Zip 26.02 (x64)").as_deref(),
            Some("26.02")
        );
    }

    #[test]
    fn dies_无字段与垃圾输出解析为none() {
        assert_eq!(
            parse_version(&Tool::default(), "1.2.3"),
            None,
            "无 probe_pattern 字段应返回 None（机检保证在管工具不缺字段）"
        );
        let jq = Tool {
            probe_pattern: Some(r"jq-(\d+\.\d+\.\d+)".to_string()),
            ..Tool::default()
        };
        assert_eq!(parse_version(&jq, "not a version"), None);
        assert_eq!(parse_version(&jq, ""), None);
    }

    #[test]
    #[cfg(windows)]
    fn env变量展开_未定义原样保留() {
        std::env::set_var("OME_TEST_VAR_X", r"C:\somewhere");
        assert_eq!(
            expand_env_vars(r"%OME_TEST_VAR_X%\bin\tool.exe"),
            r"C:\somewhere\bin\tool.exe"
        );
        assert_eq!(
            expand_env_vars(r"%OME_NO_SUCH_VAR%\x.exe"),
            r"%OME_NO_SUCH_VAR%\x.exe"
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn exe路径_专属exe相对安装目录_通用exe相对envroot() {
        let root = Path::new("/tmp/ome-root");
        // 专属 exe（linux_exe，mac 上 mac_exe 同理）相对 install_dir：dir 展开后绝对路径直用
        let platform = Tool {
            dir: Some("~/.local/bin".to_string()),
            exe: Some(r"jq\jq.exe".to_string()),
            linux_exe: Some("jq".to_string()),
            ..Tool::default()
        };
        let home = dirs::home_dir().expect("应能取 home");
        assert_eq!(
            exe_path(&platform, root).expect("应解析"),
            home.join(".local").join("bin").join("jq")
        );
        // 相对 dir（EnvRoot 下子目录语义）拼 EnvRoot 再接专属 exe
        let rel_dir = Tool {
            dir: Some("python".to_string()),
            linux_exe: Some("bin/python".to_string()),
            ..Tool::default()
        };
        assert_eq!(
            exe_path(&rel_dir, root).expect("应解析"),
            root.join("python").join("bin").join("python")
        );
        // 回退通用 exe（Windows 名录风格，路径自带 dir 段）直接相对 EnvRoot——
        // 回归：曾误拼 install_dir 产出 age/age/age.exe（黄金 status.txt 契约：<root>/age/age.exe）
        let generic = Tool {
            dir: Some("age".to_string()),
            exe: Some(r"age\age.exe".to_string()),
            ..Tool::default()
        };
        assert_eq!(
            exe_path(&generic, root).expect("应解析"),
            root.join("age").join("age.exe")
        );
    }

    #[test]
    #[cfg(windows)]
    fn exe路径_official展开_envroot拼接() {
        let root = Path::new(r"D:\sandbox");
        let green = Tool {
            exe: Some(r"jq\jq.exe".to_string()),
            ..Tool::default()
        };
        assert_eq!(
            exe_path(&green, root).expect("应解析"),
            PathBuf::from(r"D:\sandbox\jq\jq.exe")
        );
        assert!(!is_official(&green));

        std::env::set_var("OME_TEST_VAR_Y", r"C:\official");
        let official = Tool {
            exe: Some(r"%OME_TEST_VAR_Y%\rmux\bin\rmux.exe".to_string()),
            ..Tool::default()
        };
        assert_eq!(
            exe_path(&official, root).expect("应解析"),
            PathBuf::from(r"C:\official\rmux\bin\rmux.exe")
        );
        assert!(is_official(&official));
    }
    /// D43：版本目录占位 glob——取 semver 最大在位版本（0.16.0 压过 0.9.0，字典序反例）；
    /// 定版形态直替换；无在位时 0.0.0 填充（探测 None=未装）。
    /// D49：update 漂移三态——一致 skip、落后补装、领先如实报；未装与解析失败保守按落后。
    #[test]
    fn 锁定漂移_三态判定() {
        assert_eq!(pin_drift(Some("1.2.0"), "1.2.0"), PinDrift::Current);
        assert_eq!(pin_drift(Some("1.1.5"), "1.2.0"), PinDrift::Behind);
        assert_eq!(pin_drift(Some("1.2.1"), "1.2.0"), PinDrift::Ahead);
        assert_eq!(pin_drift(None, "1.2.0"), PinDrift::Behind, "未装按落后补装");
        // 数值段比较而非字符串序（v9 不压 v24 的同源教训）
        assert_eq!(pin_drift(Some("9.0.0"), "24.0.0"), PinDrift::Behind);
        assert_eq!(
            pin_drift(Some("garbage"), "1.2.0"),
            PinDrift::Behind,
            "解析失败保守落后"
        );
        // git for windows 四段后缀形态：数值等即 Current（status 旧字符串比较恒列的口径差）
        assert_eq!(
            pin_drift(Some("2.55.0.windows.5"), "2.55.0"),
            PinDrift::Current
        );
    }

    /// D49 尾：status drift 判定统一口径——npm-tgz 排除、数值段不等、缺值不列。
    #[test]
    fn status漂移判定_统一口径() {
        assert!(
            !status_drift_hint(Some("npm-tgz"), Some("0.3.4"), Some("9.9.9")),
            "npm-tgz 不列"
        );
        assert!(
            status_drift_hint(Some("zip"), Some("1.0.0"), Some("1.1.0")),
            "落后列"
        );
        assert!(
            status_drift_hint(Some("zip"), Some("2.0.0"), Some("1.1.0")),
            "领先也列（提示滚锁）"
        );
        assert!(
            !status_drift_hint(Some("msi"), Some("2.55.0.windows.5"), Some("2.55.0")),
            "git 后缀形态数值等不列"
        );
        assert!(
            !status_drift_hint(Some("zip"), None, Some("1.0.0")),
            "未装不列"
        );
        assert!(
            !status_drift_hint(Some("zip"), Some("1.0.0"), None),
            "未锁不列"
        );
    }

    #[test]
    fn 版本目录占位_glob取semver最大与定版替换() {
        let dir = tempfile::tempdir().expect("临时目录");
        let zig_root = dir.path().join("zig");
        for v in ["0.9.0", "0.16.0"] {
            let bin = zig_root.join(format!("zig-x86_64-windows-{v}"));
            std::fs::create_dir_all(&bin).expect("建版本目录");
            std::fs::write(
                bin.join(format!("zig{}", std::env::consts::EXE_SUFFIX)),
                b"fake",
            )
            .expect("写 exe");
        }
        let sep = if cfg!(windows) { "\\" } else { "/" };
        let layout = format!(
            "zig{sep}zig-x86_64-windows-{{version}}{sep}zig{}",
            std::env::consts::EXE_SUFFIX
        );
        let tool = Tool {
            exe: Some(layout),
            ..Default::default()
        };
        let env_root = dir.path();
        // 探测态：glob 取 0.16.0（非字典序 0.9.0）
        let p = exe_path(&tool, env_root).expect("应解析");
        assert!(
            p.ends_with(
                format!(
                    "zig-x86_64-windows-0.16.0{sep}zig{}",
                    std::env::consts::EXE_SUFFIX
                )
                .as_str()
            ),
            "{}",
            p.display()
        );
        // 定版态：直替换
        let pv = exe_path_for_version(&tool, env_root, "0.17.0").expect("应解析");
        assert!(
            pv.ends_with(
                format!(
                    "zig-x86_64-windows-0.17.0{sep}zig{}",
                    std::env::consts::EXE_SUFFIX
                )
                .as_str()
            ),
            "{}",
            pv.display()
        );
        // 无在位（清空重建）：探测路径落到 0.0.0 填充（不存在即未装）
        std::fs::remove_dir_all(&zig_root).expect("清");
        let pn = exe_path(&tool, env_root).expect("应解析");
        assert!(
            pn.ends_with(
                format!(
                    "zig-x86_64-windows-0.0.0{sep}zig{}",
                    std::env::consts::EXE_SUFFIX
                )
                .as_str()
            ),
            "{}",
            pn.display()
        );
        assert!(!pn.exists(), "填充路径应不存在（未装）");
    }
}
