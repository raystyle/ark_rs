---
name: ark-catalog-addition
description: 往 ark catalog 入册或修一个工具的标准流程。资产形态实证、extract 类型与装出物断言、版本验证与 pin、evergreen 直链语义、镜像播种衔接与门禁收口。用于新增工具、修 catalog 条目、修引擎装出物与解析缺陷时;不用于不动 catalog 与引擎的本机安装修复、日常写码与重构。
---

# ark-catalog-addition:catalog 入册与条目修复标准流程

> 闸门记录（第 1 轮，2026-10-10，状态**受**：用户裁定接受 2026-10-10，con-04-A1 收口）。
> A1 触发对补录（2026-10-10，隔离环境 CLAUDE_CONFIG_DIR 私有配置实测）：正例新工具入册触发 Skill 调用成立；近失负例「本机安装修复」初测误触发，揭出 When to Apply 面过宽，本批收窄(description 与 When NOT to Apply 补普通安装诊断与重构两条)后复测三对全符(证据行见 distill-log 第 1 轮)。
> 证据一（门禁无回归）：rumdl 加 mdcharlint 加 md-ref-scan 加 md-heading-scan 加 llms-guard 加 PEVO check 全 0，cargo fmt 加 clippy 加 test --release --locked 全绿（实录见 `docs/sources/diary/2026-10-10-升级轮对齐三件REQ-0018.md` con-04 节）。
> 证据二（点名义务翻转）：点名义务「入册工具的集成测试断言装出物为可执行二进制（ELF/PE）而非归档本体」。此前无标准盖不住（yq v4.54.1 双机装出 tar 归档本体，REQ-0018 件一实证）；候选下本页步骤 4 明文该断言为必过义务，yq 与 bun 示范测试在位（tests/linux_install.rs）。
> 证据三（知识链接）：本页回指 `docs/knowledge/mistakes/M107-copy型资产归档形态未嗅探.md`（带 sources trace 引文）与 `docs/knowledge/mistakes/M108-evergreen直链无pin解析断层.md`。
> 目的说明：把 M107/M108 两次双机病灶的修法固化为入册验收义务，防同型坑再犯。

## When to Apply

- 往 catalog 新入册一个工具（新 [tools.X] 节）
- 修既有条目的资产名、extract 类型、pin、probe 字段
- 排查「安装后未找到可执行文件或无法读取版本」类缺陷且修复对象是 catalog 条目或引擎代码(要动数据面或代码)

## When NOT to Apply

- 普通安装诊断与本机修复(不动 catalog 条目与引擎代码:走重装、ark doctor、issue 反馈)
- 日常写码与重构(与入册无关的代码改动)
- 家族自研 CLI（hst/browse/reader/officecli）：走委托自升级通道，不入册资产面
- 特型条目（vsbuild/rustup/docker）：走各自专用安装模块，不走通用 extract 分支
- agent 类条目：PATH 存量纳管语义优先（D07），入册面只管二进制分发

## 步骤

1. **资产形态实证**：从官方 release 的 checksums 清单抽锚三平台 sha（不臆测资产名）；真实下载一次核对 sha 与 magic（gzip `1f 8b`、zip `PK`、tar `ustar`、裸 ELF/PE）。资产名与形态以实测为准，不以历史条目类推。
2. **extract 类型与形态对账**：copy 只适用裸二进制资产；归档形资产选 targz-bin/zip 等对应型或依赖引擎剥层。上游把裸件改打包时（M107 形），copy 型靠引擎魔数嗅探兜底，但条目面应如实反映形态。
3. **版本验证面**：probe_pattern 与真实 `--version` 输出对上（捕获组返回版本号）；装后验版本语义（期望等于实际）必须成立。
4. **装出物断言（必过义务）**：新增或修条目必须带一条真网集成测试，断言装出物头部是可执行二进制（POSIX 断言 `\x7fELF`、Windows 断言 `MZ`）而非归档本体，且 `--version` 命中目标版本（判例：tests/linux_install.rs 的 yq 用例）。
5. **evergreen 直链判定**：cdn_url 模板无 {version} 占位即 evergreen 滚动语义（无 pin、重定向解析真实 tag，M108）；版本化模板必须有 pin 或显式选项，两形不得混写。
6. **pin 与播种衔接**：pin 三键（tag/version/asset）加 sha256 大写；镜像版本段播种在 ohmycloud 侧（agent-upstream 清单加行），本仓只管引擎消费面。
7. **门禁收口**：cargo test --release --locked 加门禁四件套加 llms-guard 全绿；改 pub 项同步 aidoc；diary 记钩子。
