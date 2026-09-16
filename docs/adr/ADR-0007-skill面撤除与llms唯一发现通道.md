---
id: ADR-0007
title: skill面撤除与llms唯一发现通道
status: accepted
date: 2026-09-16
deciders:
  - 用户（裁定 2026-09-16「有 --llms 我们就不需要 skill 参数命令了，--llms 就是一个紧凑版本的给 agents 的说明书」）
supersedes: []
superseded_by: null
tags: [命令面, agent发现层, D50]
---

# ADR-0007:skill面撤除与llms唯一发现通道

## Context

agent 发现层自 D09 起三件并存：根 SKILL.md 静态件（include_str! 进二进制、init 部署到数据目录）、`ark skill` 自适应渲染（D25 逐工具引导，消费 catalog guide 四字段）、`--llms` 紧凑命令图。三件的维护成本大于收益：SKILL.md 命令图与 LLMS_MANIFEST 双份并行必漂移（R013 靠人肉两处同步）；自适应渲染的实装清单、env 与目录实测信息与 doctor、status 输出高度重叠；D09 当年即预留「命令面合并或收敛（增减裁决）另批」未兑现。

备选项：只撤 `ark skill` 子命令保留静态 SKILL.md 双通道（仍留双份同步漂移面）；或三件全留（维护成本不变）。用户裁定取整面撤除，`--llms` 作为唯一发现通道。

## Decision

撤 skill 整面，`--llms` 为唯一 agent 发现通道；两项附带裁定：

1. `ark skill` 子命令、selfdeploy 三函数（deploy_skill 与 write_skill 与 render_skill）、根 SKILL.md 全撤；init 增 remove_legacy_skill 幂等清理数据目录已部署的旧 SKILL.md（仿 ome 别名清理模式）；`--llms` 头部吸收何时用与下载两行纪律（一律走 ark、幂等检测安装、镜像优先回落官方、有锚必校验），manifest 单源自持。
2. catalog guide 四字段（desc 与 guide_env 与 guide_dirs 与 guide_notes，D25 入册）保留为数据面：schema 与解析不动、真源在云端 ohmycloud，ark 侧无渲染消费者，未来其他面可消费。

## Consequences

- 好：发现层单源，消除 SKILL.md 与 manifest 的两处同步漂移面；命令面少一个子命令；五端数据目录旧 SKILL.md 由 init 幂等收敛。
- 坏：D25 的 45 工具引导起稿在 ark 侧暂无消费出口（数据保留云端，不丢失）；下一版发布前旧二进制 init 仍会重建 ark 位 SKILL.md，随 D50 版铺开自然消失。
- 关联：REQ-0005（验收判据与 trace）；历史背景 D09（发现层三件起意）与 D25（guide 字段与自适应渲染）冻结于 PRD D 表不动。
