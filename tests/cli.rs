//! CLI 集成冒烟测试（离线路径）：以 ARK_CATALOG 指向夹具，断退出码与 key=value 标记行。
//! 网络路径（query --latest 等）不在此处测，真机对齐走 ARK_TEST_REAL 闸门。

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tools.toml")
}

fn ark_cli() -> Command {
    let mut cmd = Command::cargo_bin("ark").expect("ark 二进制应已构建");
    cmd.env("ARK_CATALOG", fixture());
    cmd
}

#[test]
fn pin_无选项_打印当前pin_sha截16位() {
    // pin 视图显示当前平台的 pin（夹具三平台 pin 值同 tag/version，sha 各异）
    #[cfg(windows)]
    let (asset, sha16) = ("age-v1.3.1-windows-amd64.zip", "C56E8CE22F7E80CB...");
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let (asset, sha16) = ("age-v1.3.1-linux-amd64.tar.gz", "BDC69C09CBDD6CF8...");
    #[cfg(target_os = "macos")]
    let (asset, sha16) = ("age-v1.3.1-darwin-arm64.tar.gz", "01120EA2CBF0463D...");
    ark_cli()
        .args(["pin", "age"])
        .assert()
        .success()
        .stdout(contains("tool=age"))
        .stdout(contains("tag=v1.3.1"))
        .stdout(contains("version=1.3.1"))
        .stdout(contains(format!("asset={asset}")))
        .stdout(contains(format!("sha256={sha16}")));
}

/// D51 镜像默认通道：GitHub 分支 pin 驱动 query 零 API（离线即证不打 api.github.com），
/// url 如实呈现镜像资产域直拼地址；版本与资产全取 catalog pin（期望值来自夹具）。
#[test]
fn query_pind驱动_零api镜像直装url() {
    #[cfg(windows)]
    let asset = "age-v1.3.1-windows-amd64.zip";
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let asset = "age-v1.3.1-linux-amd64.tar.gz";
    #[cfg(target_os = "macos")]
    let asset = "age-v1.3.1-darwin-arm64.tar.gz";
    // 断网面：代理指向不可达地址加 ARK_OFFLINE 关自动刷新，query 仍应成功（零网络纯读 pin）
    ark_cli()
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .env("https_proxy", "http://127.0.0.1:1")
        .env("ARK_OFFLINE", "1")
        .args(["query", "age"])
        .assert()
        .success()
        .stdout(contains("tool=age"))
        .stdout(contains("tag=v1.3.1"))
        .stdout(contains("version=1.3.1"))
        .stdout(contains(format!("asset={asset}")))
        .stdout(contains(format!(
            "url=https://env.ohmygh.com/age/1.3.1/{asset}"
        )));
}

#[test]
fn lock_别名等价pin() {
    ark_cli()
        .args(["lock", "vault"])
        .assert()
        .success()
        .stdout(contains("tool=vault").and(contains("version=1.20.2")));
}

#[test]
fn dies_query_未知工具() {
    ark_cli()
        .args(["query", "nonexistent"])
        .assert()
        .failure()
        .stderr(contains("未知工具"));
}

#[test]
fn dies_pin_未知工具() {
    ark_cli()
        .args(["pin", "nonexistent"])
        .assert()
        .failure()
        .stderr(contains("未知工具"));
}

#[test]
fn dies_pin_latest与version互斥() {
    ark_cli()
        .args(["pin", "age", "--latest", "--version", "1.3.1"])
        .assert()
        .failure();
}

#[test]
fn dies_pin_tag与version互斥() {
    ark_cli()
        .args(["pin", "age", "--tag", "v1.3.1", "--version", "1.3.1"])
        .assert()
        .failure();
}

#[test]
fn dies_catalog_缺文件() {
    // ARK_CATALOG 指向不存在的路径：Catalog::load 应在读取处失败，不落到任何后续逻辑
    let mut cmd = Command::cargo_bin("ark").expect("ark 二进制应已构建");
    cmd.env("ARK_CATALOG", fixture().with_file_name("nonexistent.toml"));
    cmd.args(["status"])
        .assert()
        .failure()
        .stderr(contains("读取 catalog 失败"));
}

#[test]
fn env_root_ark_root覆盖_端到端() {
    // ARK_ROOT 覆盖默认 EnvRoot（旧名读回已随 ome 关键字剔除批撤除，2026-09-18）
    let primary = std::env::temp_dir().join("ark-root-primary-e2e");
    ark_cli()
        .env("ARK_ROOT", &primary)
        .args(["status"])
        .assert()
        .success()
        .stdout(contains(primary.to_string_lossy().as_ref()));
}

#[test]
fn dies_latest与tag互斥() {
    ark_cli()
        .args(["query", "age", "--latest", "--tag", "v1.3.1"])
        .assert()
        .failure();
}

#[test]
fn status_沙盒_三态输出exe缺失为横线() {
    // 沙盒 EnvRoot：exe 不存在 → installed=-；path 读真实用户 PATH（只读）判定 bin 未注册
    // 组头为七类 taxonomy（夹具节序 python→vault→age 对应 运行时依赖→远程服务依赖→命令工具依赖）
    let dir = tempfile::tempdir().expect("创建沙盒失败");
    ark_cli()
        .args(["status", "--env-root", &dir.path().to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("tool=age"))
        .stdout(contains("locked=1.3.1"))
        .stdout(contains("installed=-"))
        .stdout(contains("path=false"))
        .stdout(contains("# [运行时依赖]"))
        .stdout(contains("# [命令工具依赖]"));
}

#[test]
fn status_format_json_整批数组且组标题不出stdout() {
    // S003：--format json 输出合法 JSON 数组文档，值全字符串，# 组标题是 kv 排版件不进结构化
    let dir = tempfile::tempdir().expect("创建沙盒失败");
    let out = ark_cli()
        .args([
            "status",
            "--format",
            "json",
            "--env-root",
            &dir.path().to_string_lossy(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8_lossy(&out);
    let v: serde_json::Value =
        serde_json::from_str(text.trim()).expect("stdout 应为单一合法 JSON 文档");
    let arr = v.as_array().expect("顶层应为数组");
    assert!(!arr.is_empty(), "夹具至少一个工具");
    let age = arr
        .iter()
        .find(|o| o.get("tool").and_then(|t| t.as_str()) == Some("age"))
        .expect("应含 age 对象");
    assert_eq!(age.get("locked"), Some(&serde_json::json!("1.3.1")));
    // installed 平台相关（posix 实机可能真装着 age），只断字段在
    assert!(age.get("installed").is_some());
    assert!(!text.contains('#'), "结构化输出不含 # 组标题");
}

#[test]
fn json_简写等价格式_数组单对象() {
    let out = ark_cli()
        .args(["pin", "age", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out).trim()).expect("--json 应输出合法 JSON");
    let arr = v.as_array().expect("顶层应为数组");
    assert_eq!(arr.len(), 1, "单工具 pin 应为单元素数组");
    assert_eq!(
        arr[0].get("version"),
        Some(&serde_json::json!("1.3.1")),
        "值应为字符串"
    );
}

#[test]
fn status_format_jsonl_逐块单行json() {
    let dir = tempfile::tempdir().expect("创建沙盒失败");
    let out = ark_cli()
        .args([
            "status",
            "--format",
            "jsonl",
            "--env-root",
            &dir.path().to_string_lossy(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 2, "jsonl 应逐工具多行");
    for line in &lines {
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("每行应为 JSON 对象: {line}: {e}"));
        assert!(v.get("tool").is_some(), "每行应带 tool 字段");
    }
}

#[test]
fn 错误_结构化模式_stderr单行json含code() {
    // S003：--json 下 stdout 恒纯数据（此例无数据块），错误走 stderr 单行 JSON，退出码不变形
    let out = ark_cli()
        .args(["pin", "nonexistent", "--json"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let text = String::from_utf8_lossy(&out.stderr);
    let last = text
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .expect("stderr 应含 JSON 错误行");
    let v: serde_json::Value = serde_json::from_str(last).expect("错误行应为合法 JSON");
    assert!(v.get("code").is_some());
    assert!(v.get("message").is_some());
    // stdout 恒为合法 JSON 文档：无数据块时为空串或空数组（管道解析不炸）
    let stdout_text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !stdout_text.is_empty() {
        let sv: serde_json::Value =
            serde_json::from_str(&stdout_text).expect("失败时 stdout 仍应为合法 JSON");
        assert!(sv.as_array().map(|a| a.is_empty()).unwrap_or(false));
    }
}

#[test]
fn dies_json与format互斥() {
    ark_cli()
        .args(["status", "--json", "--format", "jsonl"])
        .assert()
        .failure();
}

// ── query sha256 契约（issue #4：pin 同 tag 同资产给锁定 sha，离线纯读）──

#[test]
fn query_sha256_同pin给锁定sha() {
    #[cfg(windows)]
    let sha16 = "C56E8CE22F7E80CB";
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let sha16 = "BDC69C09CBDD6CF8";
    #[cfg(target_os = "macos")]
    let sha16 = "01120EA2CBF0463D";
    ark_cli()
        .args(["query", "age"])
        .assert()
        .success()
        .stdout(contains("tool=age"))
        .stdout(contains(format!("sha256={sha16}")));
}

// ── heal（M4：heal-map.psd1 42 键迁嵌入注册表；离线路径断标记行）──

#[test]
fn dies_heal_未知维度() {
    ark_cli()
        .args(["heal", "nonexistent"])
        .assert()
        .failure()
        .stderr(contains("未知自愈维度"));
}

#[test]
fn heal_休眠键_agent裁决提示不执行() {
    // agent 域键按 2026-09-01 裁决休眠：只提示不执行，退出码 0
    ark_cli()
        .args(["heal", "claude"])
        .assert()
        .success()
        .stdout(
            contains("dim=claude")
                .and(contains("action=dormant"))
                .and(contains("result=skip")),
        );
}

#[test]
fn heal_外域键_路由提示() {
    // compileMatrix 属外域（双平台在册，断言 routed）
    ark_cli()
        .args(["heal", "compileMatrix"])
        .assert()
        .success()
        .stdout(contains("dim=compileMatrix").and(contains("action=routed")));
}

#[test]
fn heal_dryrun_install键只出计划() {
    // dry-run 在解析 catalog 之前返回计划行（无网络无落盘）
    ark_cli()
        .args(["heal", "yq", "--dry-run"])
        .assert()
        .success()
        .stdout(
            contains("dim=yq")
                .and(contains("action=install"))
                .and(contains("params=yq"))
                .and(contains("result=dry-run")),
        );
}

#[test]
fn heal_all_dryrun_计划含原生动作类() {
    ark_cli()
        .args(["heal", "all", "--dry-run"])
        .assert()
        .success()
        .stdout(contains("action=install").and(contains("action=keys")));
}

#[test]
fn heal_json_结构化块含休眠理由() {
    let out = ark_cli()
        .args(["heal", "claude", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out).trim()).expect("--json 应输出合法 JSON");
    let arr = v.as_array().expect("顶层应为数组");
    assert_eq!(arr[0].get("dim"), Some(&serde_json::json!("claude")));
    assert_eq!(arr[0].get("action"), Some(&serde_json::json!("dormant")));
    assert!(arr[0].get("detail").is_some(), "休眠理由应入 detail 字段");
}

#[cfg(windows)]
#[test]
fn heal_平台不适用键_提示不报错() {
    // POSIX 专列键 go：Windows 上单维度调用提示不适用（Windows 走 dev-go）
    ark_cli()
        .args(["heal", "go"])
        .assert()
        .success()
        .stdout(contains("dim=go").and(contains("action=inapplicable")));
}

#[cfg(windows)]
#[test]
fn heal_goproxy_windows_dryrun() {
    ark_cli()
        .args(["heal", "goproxy", "--dry-run"])
        .assert()
        .success()
        .stdout(
            contains("dim=goproxy")
                .and(contains("action=goproxy"))
                .and(contains("result=dry-run")),
        );
}

#[cfg(not(windows))]
#[test]
fn heal_别名键_归一规范键() {
    // mac-* 专列在 ome 归一为别名：mac-go → go（POSIX 上规范键生效）
    ark_cli()
        .args(["heal", "mac-go", "--dry-run"])
        .assert()
        .success()
        .stdout(
            contains("dim=mac-go")
                .and(contains("action=alias"))
                .and(contains("params=go")),
        );
}

#[test]
fn llms_打印命令清单无需catalog() {
    let mut cmd = Command::cargo_bin("ark").expect("ark 二进制应已构建");
    cmd.env("ARK_CATALOG", fixture().with_file_name("nonexistent.toml"));
    cmd.arg("--llms")
        .assert()
        .success()
        .stdout(contains("ark doctor"))
        .stdout(contains("何时用"))
        .stdout(contains("ark install"))
        .stdout(contains("省略则全量"))
        .stdout(contains("ark skill").not());
}

#[test]
fn install_帮助_省略则全量() {
    ark_cli()
        .args(["install", "--help"])
        .assert()
        .success()
        .stdout(contains("省略则全量"));
}

#[test]
fn update_帮助_省略则全量() {
    ark_cli()
        .args(["update", "--help"])
        .assert()
        .success()
        .stdout(contains("省略则全量"));
}

#[test]
fn query_pin_heal_帮助_省略则全量() {
    for cmd in ["query", "pin", "heal"] {
        ark_cli()
            .args([cmd, "--help"])
            .assert()
            .success()
            .stdout(contains("省略则全量"));
    }
}

#[test]
fn dies_缺子命令_先于catalog加载() {
    let mut cmd = Command::cargo_bin("ark").expect("ark 二进制应已构建");
    cmd.env("ARK_CATALOG", fixture().with_file_name("nonexistent.toml"));
    cmd.assert().failure().stderr(contains("缺少子命令"));
}
