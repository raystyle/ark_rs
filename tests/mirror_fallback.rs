//! 镜像兜底链真网测试（D08，ARK_TEST_MIRROR=1 才跑否则整体 skip）。
//! 镜像优先闸门（D44 反转后）：官方段 URL 故意不可达也不影响（镜像先走不碰官方），
//! 断言 env.ohmygh.com 镜像段命中成功且 sha 锚一致；官方段仅在镜像失败时兜底。
//! 且 sha256 与 catalog pin 锚一致（信任锚即 pin 的端到端实证）。
//! 资产选 zoxide（545KB 小资产，镜像种子 69/69 在位，ohmycloud#2）。
//! D08 第二批（ohmyenv-rs#7）：evergreen 引导器 latest 段以镜像 `.sha256` 边车为锚
//! （rust / vsbuild），先边车后资产，产物 sha 与边车逐字一致；ffmpeg 大件只 HEAD 在位断言。

use std::path::PathBuf;

type TestResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn gated() -> bool {
    ark::platform::env_var_or("ARK_TEST_MIRROR", "OME_TEST_MIRROR")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// 测内独立取边车首 token（oracle 来源为镜像边车内容，不经被测下载链）。
fn sidecar_oracle(sandbox: &std::path::Path, url: &str) -> TestResult<String> {
    let path = ark::download::download_fresh(sandbox, "oracle-sidecar.sha256", url)?;
    let text = std::fs::read_to_string(&path)?;
    Ok(text
        .split_whitespace()
        .next()
        .ok_or("边车 oracle 为空")?
        .to_uppercase())
}

#[test]
fn 镜像优先_darwin资产镜像命中且sha与mac_pin一致() -> TestResult<()> {
    if !gated() {
        eprintln!("skip: ARK_TEST_MIRROR != 1");
        return Ok(());
    }
    // 期望值来源：catalog\tools.toml [tools.rmux] mac 平台键（darwin 分发测试，ohmycloud#3）
    let asset = "rmux-0.10.0-macos-aarch64.tar.gz";
    let pin_sha = "AAC857519071F680BE53AA9A328DC0CD04C2ABE66EC726F78AA9E26337C5EF7B";
    let sandbox = std::env::temp_dir().join(format!("ome-mirror-mac-{}", std::process::id()));
    std::fs::create_dir_all(&sandbox)?;
    let got: PathBuf = ark::download::download_asset_with_mirror(
        &sandbox,
        asset,
        "https://official-invalid.ome-test.invalid/x",
        Some(pin_sha),
        true,
        "rmux",
        "0.10.0",
    )?;
    assert_eq!(
        ark::download::sha256_file(&got)?,
        pin_sha,
        "darwin 资产镜像产物 sha 必须与 catalog mac pin 一致"
    );
    std::fs::remove_dir_all(&sandbox)?;
    Ok(())
}

#[test]
fn 镜像优先_下载命中且sha与pin一致() -> TestResult<()> {
    if !gated() {
        eprintln!("skip: ARK_TEST_MIRROR != 1");
        return Ok(());
    }
    // 期望值来源：catalog\tools.toml [tools.zoxide] pin（独立来源，非被测逻辑回显）
    let asset = "zoxide-0.10.0-x86_64-pc-windows-msvc.zip";
    let pin_sha = "F465AE548F8754C8E7EDBC60B45FBF58C92BFE123DB83D790252D6810FA5DAF1";
    let sandbox = std::env::temp_dir().join(format!("ome-mirror-test-{}", std::process::id()));
    std::fs::create_dir_all(&sandbox)?;
    let got: PathBuf = ark::download::download_asset_with_mirror(
        &sandbox,
        asset,
        // 官方段故意给不可达地址（.invalid TLD 保测不会真通），逼回落镜像段
        "https://official-invalid.ome-test.invalid/x.zip",
        Some(pin_sha),
        true,
        "zoxide",
        "0.10.0",
    )?;
    let actual = ark::download::sha256_file(&got)?;
    assert_eq!(
        actual, pin_sha,
        "镜像段下载产物 sha 必须与 catalog pin 一致（信任锚即 pin）"
    );
    std::fs::remove_dir_all(&sandbox)?;
    Ok(())
}

/// D08 第二批闸门项（ohmycloud#7 补种后）：zoxide linux 资产镜像优先命中，sha 与 linux pin 一致。
#[test]
fn 镜像优先_linux资产镜像命中且sha与linux_pin一致() -> TestResult<()> {
    if !gated() {
        eprintln!("skip: ARK_TEST_MIRROR != 1");
        return Ok(());
    }
    // 期望值来源：catalog\tools.toml [tools.zoxide] linux 平台键（2026-09-08 官方资产哈希回填）
    let asset = "zoxide-0.10.0-x86_64-unknown-linux-musl.tar.gz";
    let pin_sha = "2D93385B99F3E82CF2701609A1BFFCAD863FBEB75AA3FE7EB6BE4D29BE68B1AE";
    let sandbox = std::env::temp_dir().join(format!("ome-mirror-linux-{}", std::process::id()));
    std::fs::create_dir_all(&sandbox)?;
    let got: PathBuf = ark::download::download_asset_with_mirror(
        &sandbox,
        asset,
        "https://official-invalid.ome-test.invalid/x.tar.gz",
        Some(pin_sha),
        true,
        "zoxide",
        "0.10.0",
    )?;
    assert_eq!(
        ark::download::sha256_file(&got)?,
        pin_sha,
        "linux 资产镜像产物 sha 必须与 catalog linux pin 一致"
    );
    std::fs::remove_dir_all(&sandbox)?;
    Ok(())
}

/// D08 第二批（ohmyenv-rs#7）：rust 引导器镜像优先命中 latest 段，产物 sha 与边车逐字一致。
#[test]
fn 镜像优先_rust引导器latest段命中且sha与边车一致() -> TestResult<()> {
    if !gated() {
        eprintln!("skip: ARK_TEST_MIRROR != 1");
        return Ok(());
    }
    let sandbox = std::env::temp_dir().join(format!("ome-mirror-rust-{}", std::process::id()));
    std::fs::create_dir_all(&sandbox)?;
    let got = ark::download::download_latest_with_sidecar(
        &sandbox,
        "rustup-init.exe",
        "https://official-invalid.ome-test.invalid/rustup-init.exe",
        "rust",
    )?;
    let oracle = sidecar_oracle(
        &sandbox,
        "https://env.ohmygh.com/rust/latest/rustup-init.exe.sha256",
    )?;
    assert_eq!(
        ark::download::sha256_file(&got)?,
        oracle,
        "rust 引导器 latest 段产物 sha 必须与镜像边车逐字一致"
    );
    std::fs::remove_dir_all(&sandbox)?;
    Ok(())
}

/// D08 第二批（ohmyenv-rs#7）：vsbuild 引导器镜像优先命中 latest 段，产物 sha 与边车逐字一致。
#[test]
fn 镜像优先_vsbuild引导器latest段命中且sha与边车一致() -> TestResult<()> {
    if !gated() {
        eprintln!("skip: ARK_TEST_MIRROR != 1");
        return Ok(());
    }
    let sandbox = std::env::temp_dir().join(format!("ome-mirror-vsbuild-{}", std::process::id()));
    std::fs::create_dir_all(&sandbox)?;
    let got = ark::download::download_latest_with_sidecar(
        &sandbox,
        "vs_buildtools.exe",
        "https://official-invalid.ome-test.invalid/vs_buildtools.exe",
        "vsbuild",
    )?;
    let oracle = sidecar_oracle(
        &sandbox,
        "https://env.ohmygh.com/vsbuild/latest/vs_buildtools.exe.sha256",
    )?;
    assert_eq!(
        ark::download::sha256_file(&got)?,
        oracle,
        "vsbuild 引导器 latest 段产物 sha 必须与镜像边车逐字一致"
    );
    std::fs::remove_dir_all(&sandbox)?;
    Ok(())
}

/// ffmpeg 双平台大件（win zip 90MB 级、linux tar.xz 121MiB 级）不整下载：
/// HEAD 断言资产与边车在位（入镜闸门复核），完整回落链与 zoxide 同构不再重复。
#[test]
fn ffmpeg双平台资产与边车_在位探测() -> TestResult<()> {
    if !gated() {
        eprintln!("skip: ARK_TEST_MIRROR != 1");
        return Ok(());
    }
    let agent = ureq::AgentBuilder::new().build();
    for url in [
        "https://env.ohmygh.com/ffmpeg/9.0.1/ffmpeg-9.0.1-essentials_build.zip",
        "https://env.ohmygh.com/ffmpeg/9.0.1/ffmpeg-9.0.1-essentials_build.zip.sha256",
        "https://env.ohmygh.com/ffmpeg/9.0/ffmpeg-n9.0-latest-linux64-gpl-9.0.tar.xz",
        "https://env.ohmygh.com/ffmpeg/9.0/ffmpeg-n9.0-latest-linux64-gpl-9.0.tar.xz.sha256",
    ] {
        let resp = agent.request("HEAD", url).call()?;
        assert_eq!(resp.status(), 200, "镜像应在位（HEAD）: {url}");
    }
    Ok(())
}

/// R3 缓办收口（B 首轮 CI 已灌 ark/dev 段，2026-09-12 实证边车 200）：主段主名腿。
/// 锚取 `ark/dev/ark-*.sha256` 边车，资产按边车锚经镜像段下载校验。D41 双写期的
/// 「双段同内容」对照断言已随 ome/ 段删桶退役（2026-09-14 收口）；ome 兼容腿用例同批
/// 删（锚链 ome/dev 边车已 404，无实链可复刻）。D46 起 Windows 资产主名切 gnu。
#[test]
fn 镜像优先_自身dev主段边车锚一致() -> TestResult<()> {
    if !gated() {
        eprintln!("skip: ARK_TEST_MIRROR != 1");
        return Ok(());
    }
    let asset = ark::selfupdate::asset_for_this_platform()?;
    let sandbox = std::env::temp_dir().join(format!("ark-mirror-self-{}", std::process::id()));
    std::fs::create_dir_all(&sandbox)?;
    let anchor = ark::download::mirror_sidecar_sha(
        &sandbox,
        &format!("https://env.ohmygh.com/ark/dev/{asset}.sha256"),
    )?;
    let got: PathBuf = ark::download::download_asset_with_mirror(
        &sandbox,
        &asset,
        "https://official-invalid.ome-test.invalid/ark.exe",
        Some(&anchor),
        true,
        "ark",
        "dev",
    )?;
    assert_eq!(
        ark::download::sha256_file(&got)?,
        anchor,
        "ark dev 段产物 sha 必须与 ark/dev 边车逐字一致"
    );
    std::fs::remove_dir_all(&sandbox)?;
    Ok(())
}

/// D38 消费面镜像直装：ARK_MIRROR=1 时 pin 驱动跳过 GitHub API，私有仓（匿名 404）直取
/// 镜像资产域 URL（真网 gated；断言只锚镜像域前缀与 pin 版本，不依赖具体版本号）。
#[test]
fn mirror_query_私有仓pin锚镜像直装() -> Result<(), Box<dyn std::error::Error>> {
    if ark::platform::env_var_or("ARK_TEST_MIRROR", "OME_TEST_MIRROR").unwrap_or_default() != "1" {
        eprintln!("skip: ARK_TEST_MIRROR 未设置");
        return Ok(());
    }
    let cat = std::env::var("LOCALAPPDATA")
        .map(|l| {
            std::path::PathBuf::from(l)
                .join("ohmyenv")
                .join("catalog")
                .join("tools.toml")
        })
        .map_err(|_| "仅 Windows 本机闸门（用户数据副本作清单源）".to_string())?;
    if !cat.exists() {
        eprintln!("skip: 用户数据副本缺件（先 ark catalog sync）");
        return Ok(());
    }
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_ark"))
        .args(["query", "omc"])
        .env("ARK_MIRROR", "1")
        .env("ARK_CATALOG", &cat)
        .output()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "ARK_MIRROR=1 query omc 应成功: {stdout}"
    );
    assert!(
        stdout.contains("url=https://env.ohmygh.com/omc/"),
        "url 应为镜像资产域直拼: {stdout}"
    );
    Ok(())
}
