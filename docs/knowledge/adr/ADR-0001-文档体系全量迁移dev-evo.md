---
id: ADR-0001
title: 文档体系全量迁移 dev-evo
status: accepted
date: 2026-09-15
deciders:
  - 用户（三问裁定：方案 A 全量迁移、D 存量冻结索引新立 ADR、mistakes 并入 ADR）
supersedes: []
superseded_by: null
tags: [governance, docs]
---

# ADR-0001 文档体系全量迁移 dev-evo

## Context

本仓自立项起用根原语四件（PRD/GOAL/PLAN/TODO）加 INDEX 加 docs 六目录（research/references/mistakes/diary/guides/proven）体系治理，49 个 D 决策、M 系列 12 犯、R/G/S 全链在役。用户已建 project-evo 插件市场并沉淀 dev-evo skill（文档即代码体系：AGENTS 五节合同、ADR/REQ 状态机、六层模型、llms.txt 检索面），且裁定两个体系统一到 dev-evo。dev-evo 自带存量迁移映射：PRD 条目对应 REQ、PLAN/TODO 对应 REQ 的 Criteria 与 trace、GOAL 定位句并入 AGENTS、INDEX 职责由 AGENTS Read first 加各 README 索引承接、mistakes 并入 ADR。

## Decision

全量迁移，分批逐件对账，不推倒重来：

1. AGENTS.md 重写为五节合同（Commands/Must/Must not/Read first/环境），现行四段全部规则语义无损重排。
2. PRD 的 D01 至 D49 表冻结为历史决策索引（不逐件补篇），D50 起不可逆选择立 ADR 详篇，PRD 表只追加指针行。
3. 需求面立 REQ 制度（draft 到 implemented 带 trace 回填），TODO 现任务与 PLAN 进行中计划转 REQ；GOAL 定位句并入 AGENTS 头部后退役为历史档案。
4. INDEX 职责拆解给 AGENTS Read first 加各目录 README 索引，建 llms.txt agent 检索面，INDEX.md 退役。
5. mistakes 的 M 系列按分类并入 ADR（被否决的选择与流程纪律也是决策），M 文件退役，全仓引用替换。
6. 门禁接 dev-evo check.py（PE-01 至 PE-12）与本仓四件套（rumdl 加 md 三扫描）并存；迁移批同时清 PE-11 禁字存量（Unicode 箭头与连接号）与 PE-12 裸文件名引用。

## Consequences

- 迁移期新旧并存（PRD 冻结只读、INDEX 与 M 文件退役前保留断链过渡），每批跑 check.py 加四件套加断链回归，逐批对账入 diary。
- D 编号与 ADR 编号双轨期间，跨文档引用以「D 编号（历史）」与「ADR 编号（现行）」区分；外部会话（ohmycloud 等）的回执语境不受影响。
- 禁字面收紧（Unicode 箭头入禁）后，diary 历史存量已清但后续写作需守新规；check.py 的 PE-07 空目录 SKIP 为合法起步态。
- 若迁移中途证伪（体系摩擦大于收益），回滚点为每批提交。
