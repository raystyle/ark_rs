//! 集成测试共享设施（S002 测试三件套之一）：ark 二进制运行 helper 与 expected 文件 oracle 断言。
//! `tests/common/mod.rs` 是 cargo 约定的共享模块，不会被当成独立测试目标（R004 一）。

use std::path::{Path, PathBuf};
use std::process::Output;

/// 夹具 catalog 路径（三种下载来源形态各一，字段契约见 R001）。
pub fn fixture_catalog() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tools.toml")
}

/// 跑 ark 二进制：注入 ARK_CATALOG 指向夹具，外加调用方给的环境变量，返回完整 Output。
pub fn run_ark(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut cmd = assert_cmd::Command::cargo_bin("ark").expect("ark 二进制应已构建");
    cmd.env("ARK_CATALOG", fixture_catalog());
    // O5：测试隔离——禁止沙盒 install 写真实用户 PATH（HKCU / profile）
    cmd.env("ARK_TEST_NO_PATH_REG", "1");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.args(args);
    cmd.output().expect("ark 应可运行")
}

/// 归一化：CRLF 归 LF、沙盒临时路径替换为 <SANDBOX>、home 绝对路径替换为 <HOME>、
/// 路径分隔符 `\` 归 `/`。分隔符归一是跨平台单 oracle 的取舍（路径内容照常比对）；
/// <HOME> 归一用于 posix 布局工具（exe 解析到 ~/.local/bin 绝对路径，CI 与本机 home 不同）。
fn normalize(text: &str, sandbox: Option<&Path>) -> String {
    let mut s = text.replace("\r\n", "\n");
    if let Some(root) = sandbox {
        s = s.replace(&root.display().to_string(), "<SANDBOX>");
    }
    if let Some(home) = dirs::home_dir() {
        s = s.replace(&home.display().to_string(), "<HOME>");
    }
    s.replace('\\', "/").trim_end().to_string()
}

/// expected 文件 oracle：读 `tests/expected/<name>` 与 stdout 全量比对。
/// 黄金文件内以 `##` 起首的行为来源注释，比对前剥除（ark 自身组标题是 `# ` 单行，不冲突）。
/// 平台双 oracle（2026-09-01 M0 起用）：pin 视图显示当前平台 pin（R001 平台分列），
/// 输出实质跨平台不同——非 Windows 优先 `<stem>.<platform><ext>`，无平台文件再回退原名。
pub fn assert_stdout_eq_golden(name: &str, out: &Output, sandbox: Option<&Path>) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/expected");
    let platform = {
        #[cfg(target_os = "macos")]
        {
            "macos"
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            "linux"
        }
        #[cfg(windows)]
        {
            ""
        }
    };
    let mut chosen = name.to_string();
    if !platform.is_empty() {
        if let Some(stem) = name.strip_suffix(".txt") {
            let candidate = format!("{stem}.{platform}.txt");
            if dir.join(&candidate).exists() {
                chosen = candidate;
            }
        }
    }
    let path = dir.join(&chosen);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读黄金文件失败: {}: {e}", path.display()));
    let expected = raw
        .lines()
        .filter(|l| !l.starts_with("##"))
        .collect::<Vec<_>>()
        .join("\n");
    let actual = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        normalize(&actual, sandbox),
        normalize(&expected, sandbox),
        "stdout 应与黄金文件 tests/expected/{chosen} 全量一致（## 注释行与沙盒路径已归一化）"
    );
}
