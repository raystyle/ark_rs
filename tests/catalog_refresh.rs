//! catalog 云端刷新真网测试（D33，`ARK_TEST_MIRROR=1` 才跑否则整体 skip）。
//! 断言锚链端到端：云端边车锚可取、拉取件 sha 与锚逐字一致、解析通过、含新入册工具（typst，D32），
//! 且同锚二次调用为 current（幂等）、TTL 内不再联网（fresh）。
//! D34 增补：落位件必须同时带来可验的 minisign 签名件，且内容被改后验签失败。

use std::path::PathBuf;

type TestResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn gated() -> bool {
    ark::platform::env_var_or("ARK_TEST_MIRROR", "OME_TEST_MIRROR")
        .map(|v| v == "1")
        .unwrap_or(false)
}

#[test]
fn 云端清单刷新_锚一致且落位幂等() -> TestResult<()> {
    if !gated() {
        eprintln!("skip: ARK_TEST_MIRROR != 1");
        return Ok(());
    }
    let root = std::env::temp_dir().join(format!("ome-catalog-sync-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    let target: PathBuf = root.join("data").join("catalog").join("tools.toml");

    // oracle 独立来源：测内直接取镜像边车首 token（不经被测刷新链）
    let sidecar = ark::download::mirror_sidecar_sha(
        &root,
        &format!(
            "{}/ome/catalog/tools.toml.sha256",
            ark::download::MIRROR_BASE
        ),
    )?;

    let first = ark::catalog::sync_to(&root, &target, true, ark::catalog::DEFAULT_TTL_SECS)?;
    assert_eq!(first.action(), "updated", "首次刷新应落位");
    assert_eq!(
        ark::download::sha256_file(&target)?,
        sidecar,
        "落位件 sha 必须等于云端边车锚"
    );
    let cat = ark::catalog::Catalog::load(&target)?;
    assert!(
        cat.tool("typst").is_ok(),
        "云端清单应含 D32 入册的 typst（配置播种无需换二进制）"
    );
    // D34：签名件随清单落位，且内嵌公钥验得过；改一个字节即失败
    let sig_path = ark::catalog::signature_path(&target);
    assert!(sig_path.exists(), "刷新应同时落位 minisign 签名件");
    assert_eq!(
        ark::catalog::check_signature(&target),
        ark::catalog::SignatureState::Valid,
        "落位件应通过内嵌公钥验签"
    );
    let mut bytes = std::fs::read(&target)?;
    bytes.push(b'#');
    std::fs::write(&target, &bytes)?;
    assert!(
        matches!(
            ark::catalog::check_signature(&target),
            ark::catalog::SignatureState::Invalid(_)
        ),
        "内容被改后必须验签失败"
    );
    // 复原（后续用例要继续用同一沙盒）
    std::fs::copy(root.join("cache").join("cloud-tools.toml"), &target)?;
    assert_eq!(
        ark::catalog::check_signature(&target),
        ark::catalog::SignatureState::Valid
    );

    let second = ark::catalog::sync_to(&root, &target, true, ark::catalog::DEFAULT_TTL_SECS)?;
    assert_eq!(second.action(), "current", "同锚二次刷新应幂等");

    let third = ark::catalog::sync_to(&root, &target, false, ark::catalog::DEFAULT_TTL_SECS)?;
    assert_eq!(third.action(), "skipped", "TTL 内不应联网");
    assert_eq!(third.reason(), "fresh");

    std::fs::remove_dir_all(&root)?;
    Ok(())
}
