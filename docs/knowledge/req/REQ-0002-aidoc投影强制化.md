---
id: REQ-0002
title: aidoc 投影强制化重构
status: implemented
priority: must
trace: cargo aidoc --check --strict 加 missing_docs deny 加 cargo test 全绿
---

# REQ-0002:aidoc 投影强制化重构

## Scenario

dev-evo 第五十九批将 Rust 栈 aidoc 投影强制化（ADR-0006，bin-only 不再豁免），本仓既有不适用裁定需撤换并落地全链。

## Criteria

- [x] AGENTS 与文档地图的不适用裁定句撤换，改引强制口径
- [x] lib 公开项 /// 契约注释全覆盖（160 项清零：catalog 88、doctor 15、manifest 13、resolve 10、install 9、其余零散）
- [x] missing_docs deny 挂 Cargo.toml lints
- [x] cargo aidoc 生成投影进 Git（docs/aidoc/ 26 件：llms.txt、llms-full.txt、api、模块 md、manifest）
- [x] cargo aidoc --check --strict 入 AGENTS Commands 作漂移门禁
- [x] PEVO_CHECK_ALLOW 指 docs/aidoc/ 在册（AGENTS Commands 标准命令）
- [x] cargo test 保持绿（11 组）
