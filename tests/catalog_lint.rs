//! catalog 结构机检（D28 入册清单化；D37 完全解耦后只保夹具形态）：数据面四规则（在管
//! 必有 probe_pattern、正则可编译含捕获组、sha 64 hex、pin 资产名必被 asset_pattern 命中）。
//! 权威数据与数据门已迁 ohmycloud（vitest 形态），真仓测随权威件退役，本测作消费面格式
//! 的在库回归。D39 增 manifest 面（R016）：解析与 schema 版本、post_install 三键齐备
//! （win/linux/mac 每键有命令或进 skip）、catalog 的 manifest 引用一致性。
//! 常规面红灯，不再靠装后实测暴露。规则即契约（R001 字段表与入册 checklist 的机检化）：
//! 1. 凡平台在管（exe / linux_exe / mac_exe 任一在位）的工具节必须有 `probe_pattern`
//!    ——toolver 正则漏带两犯（rclone D22、gitleaks D26）的根治面；
//! 2. `probe_pattern` 必须可编译且至少含一个捕获组（parse_version 取第 1 组）；
//! 3. sha 族字段（sha256 / linux_sha256 / mac_sha256）在位必为 64 位 hex
//!    ——digest 转写错配（M014）的格式面拦截（值对错由 seed.py --plan 域面 diff 对账）；
//! 4. pin 资产名必被同平台 asset_pattern 命中——resolve 恒按 pattern 对 release 清单重筛
//!    （pin 只锁 tag 不锁资产名），pattern 按命名惯例拼错（claude linux 写 x86_64 实为
//!    x64，ohmyenv-rs#10 / M014 同型）在入册面红灯，零网络即可判（D29 增补）。

use std::path::Path;

use ark::catalog::Catalog;
use regex::Regex;

fn lint_catalog(path: &Path) -> Vec<String> {
    let cat = Catalog::load(path).expect("catalog 应能解析");
    let mut errs = Vec::new();
    for name in &cat.order {
        let t = cat.tool(name).expect("order 与 tools 应一致");
        let managed = t.exe.is_some() || t.linux_exe.is_some() || t.mac_exe.is_some();
        if managed {
            match &t.probe_pattern {
                None => errs.push(format!(
                    "{name}: 在管工具缺 probe_pattern（入册清单漏带，R001 入册 checklist）"
                )),
                Some(p) => match Regex::new(p) {
                    Err(e) => errs.push(format!("{name}: probe_pattern 不可编译: {e}")),
                    Ok(re) => {
                        if re.captures_len() < 2 {
                            errs.push(format!(
                                "{name}: probe_pattern 无捕获组（parse_version 取第 1 组）: {p}"
                            ));
                        }
                    }
                },
            }
        } else if let Some(p) = &t.probe_pattern {
            // 平台不在管的节带正则不拦（字段冗余无害），但坏正则仍拦
            if let Err(e) = Regex::new(p) {
                errs.push(format!("{name}: probe_pattern 不可编译: {e}"));
            }
        }
        for (k, v) in [
            ("sha256", &t.sha256),
            ("linux_sha256", &t.linux_sha256),
            ("mac_sha256", &t.mac_sha256),
        ] {
            if let Some(v) = v {
                if v.len() != 64 || !v.chars().all(|c| c.is_ascii_hexdigit()) {
                    errs.push(format!("{name}: {k} 非 64 位 hex: {v}"));
                }
            }
        }
        for (plat, pat, pinned) in [
            ("win", &t.asset_pattern, &t.asset),
            ("linux", &t.linux_asset_pattern, &t.linux_asset),
            ("mac", &t.mac_asset_pattern, &t.mac_asset),
        ] {
            if let (Some(p), Some(a)) = (pat, pinned) {
                match Regex::new(p) {
                    Err(e) => errs.push(format!("{name}: {plat} asset_pattern 不可编译: {e}")),
                    Ok(re) => {
                        if !re.is_match(a) {
                            errs.push(format!(
                                "{name}: {plat} asset_pattern 不命中 pin 资产名: {p} vs {a}"
                            ));
                        }
                    }
                }
            }
        }
    }
    errs
}

#[test]
fn 夹具catalog_结构机检() {
    let errs = lint_catalog(Path::new("tests/fixtures/tools.toml"));
    assert!(
        errs.is_empty(),
        "fixtures 结构机检未过:\n{}",
        errs.join("\n")
    );
}

/// manifest 面 lint（R016 D39）：文件存在才检；post_install 全平台三键齐备
/// （win/linux/mac 每键有命令或显式进 skip）；catalog 节声明 manifest 引用时目标节必在。
fn lint_manifest(dir: &Path, cat: &ark::catalog::Catalog) -> Vec<String> {
    let path = dir.join("manifest.toml");
    if !path.exists() {
        return Vec::new();
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mf = match ark::manifest::parse(&text) {
        Ok(m) => m,
        Err(e) => return vec![format!("manifest.toml: {e}")],
    };
    let mut errs = Vec::new();
    for (name, m) in &mf.manifest {
        if let Some(pi) = &m.post_install {
            for (plat, cmds) in [("win", &pi.win), ("linux", &pi.linux), ("mac", &pi.mac)] {
                let covered = cmds.is_some()
                    || pi
                        .skip
                        .as_ref()
                        .is_some_and(|s| s.iter().any(|p| p == plat));
                if !covered {
                    errs.push(format!(
                        "manifest.{name}: post_install 平台 {plat} 无命令且未进 skip（R016 三键齐备）"
                    ));
                }
            }
            if pi.win.is_none() && pi.linux.is_none() && pi.mac.is_none() {
                errs.push(format!(
                    "manifest.{name}: post_install 三键全空（应省略整节）"
                ));
            }
        }
        // env_set 面（对线 R5 确认轮收口）：键值形态校验与引擎同源（D39 起的未校验面补齐）
        for (k, v) in m.env_set.iter().flatten() {
            if !ark::manifest::env_key_sane(k) {
                errs.push(format!("manifest.{name}: env_set 键形态非法: {k}"));
            }
            if !ark::manifest::env_value_sane(v) {
                errs.push(format!("manifest.{name}: env_set 值含换行或双引号: {k}"));
            }
        }
        // mirror 面（D42）：空节应省略（与 post_install 三键全空同款形态约束）；
        // 值与键形态校验与引擎同源（对线 R5：单值键拒换行与双引号，env 键标识符形态）
        if let Some(mir) = &m.mirror {
            if mir.is_empty_conf() {
                errs.push(format!("manifest.{name}: mirror 节全键空（应省略整节）"));
            }
            for (k, v) in mir.env.iter().flatten() {
                if !ark::manifest::env_key_sane(k) {
                    errs.push(format!("manifest.{name}: mirror env 键形态非法: {k}"));
                }
                if !ark::manifest::env_value_sane(v) {
                    errs.push(format!("manifest.{name}: mirror env 值含换行或双引号: {k}"));
                }
            }
            for k in mir.env_unset.iter().flatten() {
                if !ark::manifest::env_key_sane(k) {
                    errs.push(format!("manifest.{name}: mirror env_unset 键形态非法: {k}"));
                }
            }
            let url_keys = [
                ("npm_registry", mir.npm_registry.as_deref()),
                ("bunfig_registry", mir.bunfig_registry.as_deref()),
                ("uv_index", mir.uv_index.as_deref()),
                ("pip_index", mir.pip_index.as_deref()),
                ("goproxy", mir.goproxy.as_deref()),
            ];
            for (key, val) in url_keys {
                if val.is_some_and(|v| !ark::manifest::env_value_sane(v)) {
                    errs.push(format!("manifest.{name}: mirror {key} 值含换行或双引号"));
                }
            }
        }
    }
    for name in &cat.order {
        if let Ok(def) = cat.tool(name) {
            // 引用判定与引擎同解析：值即节键，缺省同名节（R016 二）
            if let Some(key) = def.manifest.as_deref() {
                if !mf.manifest.contains_key(key) {
                    errs.push(format!(
                        "{name}: catalog 声明 manifest = \"{key}\" 但 manifest.toml 无该节（引用一致性）"
                    ));
                }
            }
        }
    }
    errs
}

#[test]
fn 夹具manifest_三键齐备与引用一致() {
    let cat = ark::catalog::Catalog::load(Path::new("tests/fixtures/tools.toml"))
        .expect("fixtures catalog 应能解析");
    let errs = lint_manifest(Path::new("tests/fixtures"), &cat);
    assert!(
        errs.is_empty(),
        "fixtures manifest lint 未过:\n{}",
        errs.join("\n")
    );
}

#[test]
fn manifest_引用不一致红灯() {
    // 声明 manifest 节键但 manifest.toml 缺该节：应报（夹具目录自身一致，故用临时目录自造）
    let dir = tempfile::tempdir().expect("临时目录");
    let text = std::fs::read_to_string("tests/fixtures/tools.toml").expect("读夹具 catalog");
    let text = text.replace("[tools.age]", "[tools.age]\nmanifest = \"age-alias\"");
    assert!(text.contains("manifest = \"age-alias\""), "注入应命中");
    std::fs::write(dir.path().join("tools.toml"), text).expect("写临时 catalog");
    std::fs::write(
        dir.path().join("manifest.toml"),
        "schema_version = 1\n[manifest.pwsh.env_set]\nPOWERSHELL_TELEMETRY_OPTOUT = \"1\"\n",
    )
    .expect("写临时 manifest");
    let cat =
        ark::catalog::Catalog::load(&dir.path().join("tools.toml")).expect("临时 catalog 应能解析");
    let errs = lint_manifest(dir.path(), &cat);
    assert!(
        errs.iter()
            .any(|e| e.contains("age") && e.contains("age-alias")),
        "应报引用不一致: {errs:?}"
    );
}

#[test]
fn manifest_缺键与空节红灯() {
    // 三键全空：应报；单平台缺命令未 skip：应报
    let text = "schema_version = 1\n[manifest.a.post_install]\n[manifest.b.post_install]\nwin = [[\"cmd\", \"/c\", \"echo\", \"x\"]]\n";
    let mf = ark::manifest::parse(text).expect("应解析");
    let mut errs = Vec::new();
    for (name, m) in &mf.manifest {
        if let Some(pi) = &m.post_install {
            if pi.win.is_none() && pi.linux.is_none() && pi.mac.is_none() {
                errs.push(format!("manifest.{name}: post_install 三键全空"));
            }
            for (plat, cmds) in [("win", &pi.win), ("linux", &pi.linux), ("mac", &pi.mac)] {
                let covered = cmds.is_some()
                    || pi
                        .skip
                        .as_ref()
                        .is_some_and(|s| s.iter().any(|p| p == plat));
                if !covered {
                    errs.push(format!("manifest.{name}: 平台 {plat} 未覆盖"));
                }
            }
        }
        if m.mirror.as_ref().is_some_and(|mir| mir.is_empty_conf()) {
            errs.push(format!("manifest.{name}: mirror 节全键空"));
        }
    }
    assert!(errs.iter().any(|e| e.contains("三键全空")), "{errs:?}");
    assert!(
        errs.iter().any(|e| e.contains("平台 linux 未覆盖")),
        "{errs:?}"
    );
}

#[test]
fn manifest_mirror空节红灯与有键放行() {
    // 空节：应报；有键（含 env 子表）：放行
    let text = "schema_version = 1\n[manifest.a.mirror]\n[manifest.b.mirror]\nnpm_registry = \"https://registry.npmmirror.com\"\n[manifest.b.mirror.env]\nFNM_NODE_DIST_MIRROR = \"https://npmmirror.com/mirrors/node/\"\n";
    let mf = ark::manifest::parse(text).expect("应解析");
    let mut errs = Vec::new();
    for (name, m) in &mf.manifest {
        if m.mirror.as_ref().is_some_and(|mir| mir.is_empty_conf()) {
            errs.push(format!("manifest.{name}: mirror 节全键空"));
        }
    }
    assert_eq!(
        errs,
        vec!["manifest.a: mirror 节全键空".to_string()],
        "只报空节"
    );
}

#[test]
fn manifest_mirror值形态红灯() {
    // 对线 R5：TOML 转义产出的真实换行与双引号值应被 lint 拒（配置注入面）
    let text = "schema_version = 1\n[manifest.a.mirror]\nnpm_registry = \"https://x/\\nregistry=evil\"\n[manifest.b.mirror.env]\nOK_KEY = \"https://good.example\"\n[manifest.c.mirror.env]\n\"bad-key\" = \"v\"\n";
    let mf = ark::manifest::parse(text).expect("应解析（转义在值内）");
    let mut errs = Vec::new();
    for (name, m) in &mf.manifest {
        if let Some(mir) = &m.mirror {
            for (k, v) in mir.env.iter().flatten() {
                if !ark::manifest::env_key_sane(k) {
                    errs.push(format!("manifest.{name}: mirror env 键形态非法: {k}"));
                }
                if !ark::manifest::env_value_sane(v) {
                    errs.push(format!("manifest.{name}: mirror env 值含换行或双引号: {k}"));
                }
            }
            if mir
                .npm_registry
                .as_deref()
                .is_some_and(|v| !ark::manifest::env_value_sane(v))
            {
                errs.push(format!(
                    "manifest.{name}: mirror npm_registry 值含换行或双引号"
                ));
            }
        }
    }
    assert!(
        errs.iter().any(|e| e.contains("a: mirror npm_registry")),
        "{errs:?}"
    );
    assert!(errs.iter().any(|e| e.contains("bad-key")), "{errs:?}");
    assert!(
        !errs
            .iter()
            .any(|e| e.contains("b:") || e.contains("OK_KEY")),
        "干净节不应报: {errs:?}"
    );
}

#[test]
fn manifest_env_set值形态红灯() {
    // 对线 R5 确认轮：env_set 与 mirror 同一注入面（D39 起未校验），lint 与引擎同源拒
    let text = "schema_version = 1\n[manifest.a.env_set]\nGOOD_KEY = \"1\"\n[manifest.b.env_set]\nEVIL = \"v\\ninjected=1\"\n[manifest.c.env_set]\n\"bad key\" = \"1\"\n";
    let mf = ark::manifest::parse(text).expect("应解析");
    let mut errs = Vec::new();
    for (name, m) in &mf.manifest {
        for (k, v) in m.env_set.iter().flatten() {
            if !ark::manifest::env_key_sane(k) {
                errs.push(format!("manifest.{name}: env_set 键形态非法: {k}"));
            }
            if !ark::manifest::env_value_sane(v) {
                errs.push(format!("manifest.{name}: env_set 值含换行或双引号: {k}"));
            }
        }
    }
    assert!(errs.iter().any(|e| e.contains("b: env_set 值")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("bad key")), "{errs:?}");
    assert!(
        !errs
            .iter()
            .any(|e| e.contains("a:") || e.contains("GOOD_KEY")),
        "干净节不应报: {errs:?}"
    );
    // 引擎面同拒：坏值在写入前硬错（校验先于任何 set_user_env_var，单测可直接跑）
    let m = ark::manifest::ToolManifest {
        env_set: Some([("EVIL".to_string(), "v\ninjected=1".to_string())].into()),
        ..Default::default()
    };
    let e = ark::manifest::apply_env_set(&m).expect_err("坏值应拒");
    assert!(e.contains("env_set 值含换行"), "{e}");
}
