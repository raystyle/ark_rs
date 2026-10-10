---
id: ADR-0006
title: catalog 转换与数据保真错误模式归档
status: accepted
date: 2026-09-15
deciders:
  - 用户（ADR-0001 批四裁定：mistakes 并入 ADR）
supersedes: []
superseded_by: null
tags: [mistakes-archive]
---

# ADR-0006 catalog 转换与数据保真错误模式归档

## Context

catalog 转换合并与数据保真的行级错误史（原 mistakes 分类文件 M106-catalog转换与数据保真-错误.md，ADR-0001 批四并入）。M001/M003/M014 三类错误模式。错误史封存不改写，同根因聚合条目保留最早编号。

## Decision

错误模式与正解纪律以本 ADR 附录表为唯一权威；新踩坑当场记 diary，构成纪律变更的立新 ADR（M0xx 行级编号制度随本批退役）。

## Consequences

- 查错误模式从 `docs/knowledge/adr/README.md` 入（编号 M0xx 在附录表内仍可检索）。
- 历史 diary 中 M0xx 引用为当时事实记录，不改写。
- 附录原表封存：新错误不再接编 M0xx，以 diary 与新 ADR 承载。

## 附录 原始条目表

> M106-catalog转换与数据保真-错误

> catalog 转换器与 tools.toml 数据保真类错误速查（一行一事，同根因合并聚合）。

## 行级条目

| 编号 | 日期 | 状态 | 现象 | 根因 | 正确处理 |
| --- | --- | --- | --- | --- | --- |
| M001 | 2026-08-31 | 已修正（2026-08-31） | ome status 对 rmux 报 installed=-，pwsh 报 0.10.0（tests\real.rs 真机闸门实测） | import-catalog.ps1 合并规则原为「psd1 Win 侧优先（仅 category/version_pattern 取 New-ToolDef）」，而 pwsh 运行时 Get-EnvLock 是 New-ToolDef 全量覆盖静态字段：rmux 有效 Exe=%LOCALAPPDATA%\rmux\bin\rmux.exe、Extract=rmux、SumsAsset=SHA256SUMS | 已把转换器合并规则对齐 Get-EnvLock（New-ToolDef 定义过的静态字段全量以其为准，含显式置空；未定义字段回退 psd1；pin 字段始终取 psd1），重生成后 diff 仅 rmux（official 生效）与 grok 两节。grok 的 New-ToolDef 资产模式与 CdnUrl 停在 1.0.5，落后于 pin 1.0.13，是 pwsh 侧自身滞后（不能改 ohmypwsh）：grok 走 cdn_url 直链解析，asset_pattern 闲置，按覆盖规则照常合并无害；若日后 grok 版本升级，需先在 ohmypwsh 侧刷新其 tooldef 的 1.0.5 硬编码再重跑转换器。tests\real.rs 的 rmux 豁免已移除，闸门 29/29 一致 |
| M003 | 2026-09-01 | 已修正（2026-09-01） | Windows 真仓 `ome status` 退出 1 报「工具缺少 exe 字段」（mac 三批次合入后首次复现） | M0 补录的 shellcheck 只有 `linux_*` 字段族无通用 `exe`；`exe()` 回退链平台不对称（mac 有 linux 兜底、Windows 仅通用），加夹具三工具全带通用 exe、真机闸门未跑 status，三重盲区 | 代码层凡消费 effective 字段处须容忍平台缺失：`toolver::platform_managed` 判定（effective exe 存在即本平台在管），status 出空态行（exe 渲染 -）、install/update/daily/pin/query 跳过、package 拒绝；测试用「全平台无 exe 的 ghost 工具」做可移植验证 |
| M014 | 2026-09-08 | 已修正（2026-09-08；2026-09-10 增补同型） | lightpanda mac pin 资产 digest 错配：0.4.0 双 mac 资产取错行（x86_64-macos 的 digest 配在 aarch64-macos 上），镜像种子器下载实测 840547bb 对 pin FE50A51D 不符拒收（锚纪律双保险起效）。2026-09-10 同型增补（ohmyenv-rs#10）：claude linux `asset_pattern` 按命名惯例拼成 x86_64，官方资产名实为 claude-linux-x64.tar.gz，linux 端 install 解析报「未找到匹配资产」（lan-pve 实测） | 多架构同名族资产的 digest 表人工转写无机械防错；与 #7329271 单字符笔误同族新形态（跨行错配）。pattern 与 sha 同属人工转写静态面，命名惯例推断不可代替 release 清单逐字核对 | pin 四键落库后跑 `uv run --script .tools/seed.py --plan`：域面 diff 立即暴露 sha-drift / sidecar-missing，入册面机检替代人眼。pattern 面机检（D29）：catalog_lint 加「pin 资产名必被同平台 asset_pattern 命中」，拼写族零网络红灯；入册纪律见 R001 checklist 一 |

## 范围注记

- 2026-08-31：智能体（codex/claude/grok）安装不属 ome 管理域，转换器显式剔除，ome 管理 26 个工具。M001 中 grok 的 New-ToolDef 滞后点对 ome 随之失效（grok 已不在名录），该前置条件仅对 ohmypwsh 侧自身有效。
