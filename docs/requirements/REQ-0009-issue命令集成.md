---
id: REQ-0009
title: issue命令集成
status: implemented
priority: must
trace: 总台对齐单（issue 命令集成，REQ-057 契约，2026-09-17）；src/issue.rs（契约校验与 ureq 客户端面，零新增 crate）加 main.rs 三叶接线（new/list/show，catalog 前早期派发）；lib 单测六件全绿（tool 形与 title/body 边界与截断与编码）加全量 cargo test 全绿；真读面冒烟（list 对 issues.ohmygh.com count=0 退出 0）加实弹提交回执见 diary
---

# REQ-0009:issue命令集成

## Scenario

总台建统一 issue 入口 issues.ohmygh.com（Worker 加 D1 真源，每 IP 10 条/时，REQ-057 契约），派单各仓集成自己的 issue 子命令：agent 使用过程中遇缺陷一键反馈，命令自动带上下文（tool=ark 与版本/平台/host）。契约：POST /api/issues JSON 六字段，校验 tool `^[a-z][a-z0-9_-]{0,31}$` 加 title 1 至 200 加 body 至多 20000，回执 201 `{ok,id,url}`；读面 GET 列表与详情。omc 为参考实现。

## Criteria

验收判据，可检验、可勾选：

- [x] `ark issue new "<标题>" [--body] [--tool]`：一键提交自动带 tool=ark（缺省）与版本（Cargo 包版本）/平台（os/arch）/host（HOSTNAME 或 COMPUTERNAME）
- [x] 契约校验客户端面（tool 形拒绝、title trim 后 1 至 200、body 至多 20000、version 40 与 platform/host 64 客户端截断），与 REQ-057 validateIssue 语义对齐
- [x] `ark issue list [--tool] [--status] [--limit]` 与 `ark issue show <id>` 读面（limit 缺省 20）
- [x] 网络面零新增 crate（ureq 既有依赖）；基址 env `ARK_ISSUES_API` 覆盖（测与灰度）
- [x] catalog 前早期派发：无清单环境也能一键反馈（issue 域纯网络面）
- [x] agent 使用纪律入仓合同（AGENTS：遇缺陷即 ark issue new 一键反馈）
- [x] 实弹证据：真提一条 ark issue 且 list 可见（回执入 diary）

实现后回填 frontmatter 的 trace，状态 implemented。
