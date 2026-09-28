---
id: REQ-0016
title: actl适配预览面与llms适配说明
status: implemented
priority: must
trace: cargo test 全绿 231 项（新增六件计划面与 llms 断言）加真机三态冒烟（jq 幂等 skip、hst 委托 delegate、gh 领先 ahead）加门禁四件套绿
---

# REQ-0016:actl适配预览面与llms适配说明

## Scenario

ai-cloud 框架 e76d84d 起 actl 对 ark 做透传适配（lib/passgate.ts，不改 ark 本体）：`actl ark <长尾>` 原样透传，写级动词在 actl 面过写闸预览（`--json` 出 TOON 信封），`--yes` 由 actl 消费剥离。actl 闸只预览 exec 行；agent 要见「真计划」（版本、资产与 sha、解压目标、PATH 注册面）必须工具自带预览（总台派单 2026-09-28，用户令「周知各工位直接做」；hst_rs 侧同类件随其 v2.9.8 落地可参照）。

## Criteria

- [x] `ark install [名] --dry-run`：不落盘（零下载零解压零 PATH 写）出真计划行：版本（tag/version）、资产与 sha（asset/url/fallback/sha256，sha 只给离线已知锚）、解压目标（dir）、PATH 注册面（bin）、缓存落点（cache）、幂等预测（would=install|skip）；解析面照常（pin 驱动零网络）。
- [x] `ark update [名] --dry-run`：同上另出 drift 三态（current/ahead/behind，D49 口径）；家族委托腿只出计划不调用（would=delegate 加 channel=self-update）；未装回落镜像首装如实预测。
- [x] `ark --llms` 含 actl 适配说明行（措辞参照 hst v2.9.8 形）：独立直用完全不变；经 actl 调用时写级动词过其写闸（预览缺省，加 `--yes` 执行），`--json` 出 TOON 信封。
- [x] 帮助示例与 R013 契约表、README 同步；全测绿加门禁四件套绿。

## 决策记录

- sha 不预取官方清单类校验源（sums_asset/asset_sha_suffix/HashiCorp SUMS 均需下载）：预览不落盘红线，空串表示下载期经镜像边车或官方清单校验（R013 注）。
- 幂等预测读探测与 status 同尺（PATH 现查加既装版本探测，只读零写）；未装 exe 探测短路不拉子进程。
- 早退分支（平台不适用/evergreen/自管/hold）计划块与真跑块同源一处构造（skip_rows），防两形漂移。
- 实现位：install.rs 计划面两函数（plan_would/plan_target_rows）、checksum.rs 离线锚（offline_expected_sha256，expected_sha256 复用同前缀）、delegate.rs 定位面（locate_exe，run 与 dry-run 共用）、main.rs 接线（cmd_install/cmd_update 计划分支 + LLMS 手册 actl 行与表行）。

## Trace

- `cargo test --release --locked` 全绿 231 项（新增六件：tests/cli.rs 三件 llms_含actl适配说明行 / install_dryrun_真计划行与零落盘 / install_dryrun_结构化与提示行；tests/install.rs 三件 update_dryrun_未装behind出补装计划 / dryrun_已装同版幂等与current预测（POSIX 门控）/ update_dryrun_委托腿只出计划不调用）。
- 真机冒烟（WSL 本职端）：install jq --dry-run 出 would=skip（幂等预测命中本机已装同版）、update hst --dry-run 出 would=delegate 加 channel=self-update（不调用家族 CLI）、update gh --dry-run 出 drift=ahead（领先如实报）；dry-run 后 EnvRoot 零新建。
- 门禁：cargo fmt --check 与 clippy 无新增告警；rumdl 加 md 三扫描绿（见 diary 当天钩子）。
