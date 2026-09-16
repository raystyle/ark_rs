//! install 链路集成测试：临时 EnvRoot 沙盒 + 动态生成的沙盒 catalog（ARK_CATALOG 指向）。
//! 全程离线：cdn_url 分支解析不触网；幂等跳过在下载前短路；防穿越在校验前拦截。
//! 不碰真实 D:\ohmyenv（除只读借用 jq.exe 作假 exe）与真实注册表（不跑 deploy/update）。

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

/// 沙盒：临时 EnvRoot + 写入 catalog，返回（临时目录守卫， catalog 路径， env_root 路径）。
fn sandbox(catalog_text: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().expect("创建沙盒失败");
    let env_root = dir.path().join("envroot");
    fs::create_dir_all(&env_root).expect("创建 envroot 失败");
    let catalog = dir.path().join("tools.toml");
    fs::write(&catalog, catalog_text).expect("写沙盒 catalog 失败");
    (dir, catalog, env_root)
}

fn ome(catalog: &Path, env_root: &Path) -> Command {
    let mut cmd = Command::cargo_bin("ark").expect("ome 二进制应已构建");
    cmd.env("ARK_CATALOG", catalog);
    // O5（S017）：沙盒 install 不得写真实用户 PATH（HKCU / profile）
    cmd.env("ARK_TEST_NO_PATH_REG", "1");
    cmd.args(["--env-root", &env_root.to_string_lossy()]);
    cmd
}

#[test]
#[cfg(windows)]
fn dies_install_防穿越_目录越出envroot() {
    // dir 越出 EnvRoot：必须在下载前以「危险路径」拒绝（对齐 Test-SafeUnderRoot 语义）
    let catalog_text = r#"
[tools.evil]
dir = '..\evil'
exe = 'evil\evil.exe'
extract = "copy"
cdn_url = "https://example.invalid/evil.exe"
tag = "v1.0.0"
version = "1.0.0"
asset = "evil.exe"
"#;
    let (_guard, catalog, env_root) = sandbox(catalog_text);
    ome(&catalog, &env_root)
        .args(["install", "evil"])
        .assert()
        .failure()
        .stderr(contains("危险路径"));
    // 拒绝后不应在沙盒外留任何目录
    assert!(!_guard.path().join("evil").exists(), "不应创建越界目录");
}

#[test]
fn install_缺manifest时装前打一行warn() {
    // 沙盒 catalog 无同目录 manifest.toml：用户级配置与别名（env_set/shims）不会应用，
    // 装前必须让真空面可见（R016 双轨收口）。用「本平台不适用」条目保持离线与快速。
    let (name, catalog_text) = if cfg!(windows) {
        (
            "posixonly",
            r#"
[tools.posixonly]
linux_dir = 'posixonly'
linux_exe = 'posixonly'
extract = "copy"
cdn_url = "https://example.invalid/posixonly"
tag = "v1.0.0"
version = "1.0.0"
asset = "posixonly"
"#,
        )
    } else {
        (
            "winonly",
            r#"
[tools.winonly]
dir = 'winonly'
exe = 'winonly\winonly.exe'
extract = "copy"
cdn_url = "https://example.invalid/winonly.exe"
tag = "v1.0.0"
version = "1.0.0"
asset = "winonly.exe"
"#,
        )
    };
    let (_guard, catalog, env_root) = sandbox(catalog_text);
    ome(&catalog, &env_root)
        .args(["install", name])
        .assert()
        .success()
        .stderr(contains("manifest.toml 不在位"));
    // 反例：manifest 在位时不该打这条 WARN（防误报）
    fs::write(_guard.path().join("manifest.toml"), "schema_version = 1\n")
        .expect("写同目录 manifest 失败");
    ome(&catalog, &env_root)
        .args(["install", name])
        .assert()
        .success()
        .stderr(contains("manifest.toml 不在位").not());
}

#[test]
fn install_幂等跳过_已装版本一致不触网() {
    // 借真实 jq.exe 当假 exe（只读复制进沙盒）；缺失时闸门跳过
    let real_jq = Path::new(r"D:\ohmyenv\jq\jq.exe");
    if !real_jq.exists() {
        eprintln!("[SKIP] 无 D:\\ohmyenv\\jq\\jq.exe 可借用，跳过幂等测试");
        return;
    }
    // 先探测真实版本，动态生成与之匹配的 pin（期望值来自 exe 自身输出，非被测逻辑）
    let out = std::process::Command::new(real_jq)
        .arg("--version")
        .output()
        .expect("jq --version 应可运行");
    let line = String::from_utf8_lossy(&out.stdout);
    let ver = line
        .trim()
        .strip_prefix("jq-")
        .expect("jq 版本行应为 jq-<ver> 格式")
        .to_string();

    let catalog_text = format!(
        r#"
[tools.jq]
dir = "jq"
bin = "jq"
exe = 'jq\jq.exe'
probe_pattern = 'jq-(\d+\.\d+\.\d+)'
extract = "copy"
cdn_url = "https://example.invalid/jq-windows-amd64.exe"
tag = "v{ver}"
version = "{ver}"
asset = "jq-windows-amd64.exe"
"#
    );
    let (_guard, catalog, env_root) = sandbox(&catalog_text);
    // 预置「已装」假 exe
    let jq_dir = env_root.join("jq");
    fs::create_dir_all(&jq_dir).expect("创建 jq 目录失败");
    fs::copy(real_jq, jq_dir.join("jq.exe")).expect("复制假 exe 失败");

    // cdn_url 指向不可达地址：若未走幂等短路，下载必失败；成功即证明未触网
    ome(&catalog, &env_root)
        .args(["install", "jq"])
        .assert()
        .success()
        .stdout(contains("tool=jq"))
        .stdout(contains("action=skipped"))
        .stdout(contains(format!("version={ver}")));
}

#[test]
#[cfg(windows)]
fn install_幂等分支mirror节点接线与测试闸门() {
    // D42：幂等跳过分支同样落 mirror（存量端升级即得的接线面）。
    // 沙盒闸门开：断言 apply_manifest_primitives 真正调到 apply_mirror（闸门行可见）
    // 且不写任何真实用户配置（npmrc/env 面全被闸门拦下）。
    let real_jq = Path::new(r"D:\ohmyenv\jq\jq.exe");
    if !real_jq.exists() {
        eprintln!("[SKIP] 无 D:\\ohmyenv\\jq\\jq.exe 可借用，跳过 mirror 接线测试");
        return;
    }
    let out = std::process::Command::new(real_jq)
        .arg("--version")
        .output()
        .expect("jq --version 应可运行");
    let ver = String::from_utf8_lossy(&out.stdout)
        .trim()
        .strip_prefix("jq-")
        .expect("jq 版本行应为 jq-<ver> 格式")
        .to_string();
    let catalog_text = format!(
        r#"
[tools.jq]
dir = "jq"
bin = "jq"
exe = 'jq\jq.exe'
probe_pattern = 'jq-(\d+\.\d+\.\d+)'
extract = "copy"
cdn_url = "https://example.invalid/jq-windows-amd64.exe"
tag = "v{ver}"
version = "{ver}"
asset = "jq-windows-amd64.exe"
"#
    );
    let (_guard, catalog, env_root) = sandbox(&catalog_text);
    let jq_dir = env_root.join("jq");
    fs::create_dir_all(&jq_dir).expect("创建 jq 目录失败");
    fs::copy(real_jq, jq_dir.join("jq.exe")).expect("复制假 exe 失败");
    fs::write(
        _guard.path().join("manifest.toml"),
        "schema_version = 1\n[manifest.jq.mirror]\nnpm_registry = \"https://registry.npmmirror.com\"\n",
    )
    .expect("写沙盒 manifest 失败");

    ome(&catalog, &env_root)
        .args(["install", "jq"])
        .assert()
        .success()
        .stdout(contains("action=skipped"))
        // 接线可见：幂等分支进到 mirror 落源，被测试闸门拦下（真实 HOME 零写入）
        .stderr(contains("跳过 jq mirror 落源（测试隔离）"));
}

/// REQ-0003：PATH 存量已达锁定版纳管（update 面）——自管位假 exe（版本输出等于 pin）在 PATH、
/// EnvRoot 未装时，update 视为已达态跳过；cdn_url 为无效域，若走装链必失败，跳过即证未触
/// 下载（纳管分支在 resolve 之前，也不触解析网络）。
#[test]
fn update_自管位path达锁定版纳管跳过() {
    let (exe_name, exe_content, exe_field) = if cfg!(windows) {
        (
            "selftool.cmd",
            "@echo selftool 1.0.0\r\n",
            r#"exe = 'selftool\selftool.exe'"#,
        )
    } else if cfg!(target_os = "macos") {
        // platform_managed 在 mac 只认 mac_exe（linux_exe 回退会把 Linux 资产当 mac 在管）
        (
            "selftool",
            "#!/bin/sh\necho selftool 1.0.0\n",
            r#"mac_exe = 'selftool/selftool'"#,
        )
    } else {
        (
            "selftool",
            "#!/bin/sh\necho selftool 1.0.0\n",
            r#"linux_exe = 'selftool/selftool'"#,
        )
    };
    let catalog_text = format!(
        r#"
[tools.selftool]
dir = "selftool"
{exe_field}
probe_pattern = '(\d+\.\d+\.\d+)'
extract = "copy"
cdn_url = "https://example.invalid/selftool.exe"
linux_cdn_url = "https://example.invalid/selftool-linux.exe"
tag = "v1.0.0"
version = "1.0.0"
linux_version = "1.0.0"
mac_version = "1.0.0"
asset = "selftool.exe"
"#
    );
    let (guard, catalog, env_root) = sandbox(&catalog_text);
    // PATH 假 bin：自管位形态（不在 EnvRoot 下）
    let fake_bin = guard.path().join("fakebin");
    fs::create_dir_all(&fake_bin).expect("创建假 bin 失败");
    let fake_exe = fake_bin.join(exe_name);
    fs::write(&fake_exe, exe_content).expect("写假 exe 失败");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fake_exe, fs::Permissions::from_mode(0o755))
            .expect("假 exe 加执行位失败");
    }
    let path_env = std::env::join_paths(std::iter::once(fake_bin.clone()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .expect("拼 PATH 失败");

    ome(&catalog, &env_root)
        .env("PATH", &path_env)
        .env("ARK_OFFLINE", "1")
        .args(["update", "selftool"])
        .assert()
        .success()
        .stdout(contains("action=skipped"))
        .stderr(contains("纳管跳过"));
}

/// REQ-0003（收窄裁定）：install 是「显式装入管理面」意图，PATH 存量同版不拦；
/// 走装链（无效域下载失败即证穿透，无纳管 INFO）。
#[test]
fn install_自管位path存量不拦显式装() {
    let (exe_name, exe_content, exe_field) = if cfg!(windows) {
        (
            "selftool.cmd",
            "@echo selftool 1.0.0\r\n",
            r#"exe = 'selftool\selftool.exe'"#,
        )
    } else if cfg!(target_os = "macos") {
        // platform_managed 在 mac 只认 mac_exe（linux_exe 回退会把 Linux 资产当 mac 在管）
        (
            "selftool",
            "#!/bin/sh\necho selftool 1.0.0\n",
            r#"mac_exe = 'selftool/selftool'"#,
        )
    } else {
        (
            "selftool",
            "#!/bin/sh\necho selftool 1.0.0\n",
            r#"linux_exe = 'selftool/selftool'"#,
        )
    };
    let catalog_text = format!(
        r#"
[tools.selftool]
dir = "selftool"
{exe_field}
probe_pattern = '(\d+\.\d+\.\d+)'
extract = "copy"
cdn_url = "https://example.invalid/selftool.exe"
linux_cdn_url = "https://example.invalid/selftool-linux.exe"
tag = "v1.0.0"
version = "1.0.0"
linux_version = "1.0.0"
mac_version = "1.0.0"
asset = "selftool.exe"
"#
    );
    let (guard, catalog, env_root) = sandbox(&catalog_text);
    let fake_bin = guard.path().join("fakebin");
    fs::create_dir_all(&fake_bin).expect("创建假 bin 失败");
    let fake_exe = fake_bin.join(exe_name);
    fs::write(&fake_exe, exe_content).expect("写假 exe 失败");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fake_exe, fs::Permissions::from_mode(0o755))
            .expect("假 exe 加执行位失败");
    }
    let path_env = std::env::join_paths(std::iter::once(fake_bin.clone()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .expect("拼 PATH 失败");

    ome(&catalog, &env_root)
        .env("PATH", &path_env)
        .env("ARK_OFFLINE", "1")
        .args(["install", "selftool"])
        .assert()
        .failure()
        .stderr(contains("纳管跳过").not())
        // 失败点锚定在下载段（无效域），防失败点前移到 resolve 时用例空转
        .stderr(contains("example.invalid"));
}
