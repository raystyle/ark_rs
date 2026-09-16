//! 真机闸门测试（R004 / D18）：ARK_TEST_REAL=1 才执行，否则整体 skip。
//! 对照基准是本机部署态：catalog 名录 + 默认 EnvRoot。只读，不写注册表、不改 EnvRoot。

use std::collections::HashMap;
use std::process::{Command, Output};

fn gated() -> bool {
    ark::platform::env_var_or("ARK_TEST_REAL", "OME_TEST_REAL")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

fn skip(reason: &str) {
    eprintln!("[SKIP] {reason}（设 ARK_TEST_REAL=1 启用真机闸门）");
}

fn ome() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin("ark"))
}

fn run(args: &[&str]) -> Output {
    ome()
        .args(args)
        .output()
        .unwrap_or_else(|_| panic!("ome {} 应可运行", args.join(" ")))
}

fn stdout_text(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// 跑 ark status（真实 catalog + 默认 EnvRoot），解析 key=value 三态。
fn ome_status() -> HashMap<String, (String, String)> {
    let out = run(&["status"]);
    assert!(out.status.success(), "ark status 应成功");
    let text = stdout_text(&out);
    let mut map: HashMap<String, (String, String)> = HashMap::new();
    let mut cur: Option<String> = None;
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("tool=") {
            cur = Some(name.to_string());
        } else if let Some(locked) = line.strip_prefix("locked=") {
            if let Some(name) = &cur {
                map.entry(name.clone()).or_default().0 = locked.to_string();
            }
        } else if let Some(inst) = line.strip_prefix("installed=") {
            if let Some(name) = &cur {
                map.entry(name.clone()).or_default().1 = inst.to_string();
            }
        }
    }
    map
}

#[test]
fn real_status_catalog_三态可解析() {
    if !gated() {
        skip("真机闸门未开");
        return;
    }
    let ome = ome_status();
    assert!(!ome.is_empty(), "ark status 应解析出工具行");
    for agent in ["claude", "codex", "grok", "kimi"] {
        assert!(ome.contains_key(agent), "ome 应纳管 {agent}");
    }
    assert!(!ome.contains_key("reader"), "reader 已出册");
    assert!(!ome.contains_key("vault"), "vault 已出册");
    let has_installed = ome.values().any(|(_, installed)| installed != "-");
    assert!(has_installed, "本机 status 应至少有一项已安装");
}

#[test]
fn real_query_jq_有tag() {
    if !gated() {
        skip("真机闸门未开");
        return;
    }
    let out = run(&["query", "jq"]);
    assert!(out.status.success(), "ark query jq 应成功");
    let tag = stdout_text(&out)
        .lines()
        .find_map(|l| l.strip_prefix("tag="))
        .map(str::to_string)
        .expect("ark query 输出应含 tag= 行");
    assert!(!tag.is_empty(), "query jq 的 tag 不应为空");
}

#[test]
fn real_pin_可跑() {
    if !gated() {
        skip("真机闸门未开");
        return;
    }
    let out = run(&["pin", "jq"]);
    assert!(out.status.success(), "ark pin jq 应成功");
    let text = stdout_text(&out);
    assert!(
        text.lines().any(|l| l.starts_with("tool=")),
        "pin 输出应含 tool= 行"
    );
}

#[test]
fn real_doctor_可跑() {
    if !gated() {
        skip("真机闸门未开");
        return;
    }
    let out = run(&["doctor"]);
    let code = out.status.code().unwrap_or(255);
    assert!(
        code == 0 || code == 1,
        "ark doctor 退出码应为 0 或 1，实际 {code}"
    );
    let text = stdout_text(&out);
    assert!(
        text.lines().any(|l| l.starts_with("verdict=")),
        "doctor 输出应含 verdict= 行"
    );
}

#[test]
fn real_verify_可跑() {
    if !gated() {
        skip("真机闸门未开");
        return;
    }
    let out = run(&["verify"]);
    let code = out.status.code().unwrap_or(255);
    assert!(
        code == 0 || code == 1,
        "ark verify 退出码应为 0 或 1，实际 {code}"
    );
    let text = stdout_text(&out);
    assert!(
        text.lines().any(|l| l.contains('=') && !l.starts_with('#')),
        "verify 输出应含收割行"
    );
}

#[test]
fn real_heal_dryrun_可跑() {
    if !gated() {
        skip("真机闸门未开");
        return;
    }
    let out = run(&["heal", "all", "--dry-run"]);
    assert!(out.status.success(), "ark heal all --dry-run 应成功");
    let text = stdout_text(&out);
    assert!(
        text.contains("action=install") || text.contains("dim="),
        "heal dry-run 应含动作行"
    );
}

#[test]
fn real_llms_清单含三原语() {
    if !gated() {
        skip("真机闸门未开");
        return;
    }
    let out = run(&["--llms"]);
    assert!(out.status.success(), "ome --llms 应成功");
    let text = stdout_text(&out);
    for cmd in ["ark doctor", "ark install", "ark status"] {
        assert!(text.contains(cmd), "--llms 应含 {cmd}");
    }
    assert!(!text.contains("ome package"), "--llms 不应再含 package");
    assert!(!text.contains("ome deploy"), "--llms 不应再含 deploy");
    assert!(!text.contains("ome daily"), "--llms 不应再含 daily");
    assert!(
        !text.contains("ark skill"),
        "--llms 不应再含 skill（D50 撤面）"
    );
}
