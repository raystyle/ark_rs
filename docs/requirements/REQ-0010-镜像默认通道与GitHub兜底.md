---
id: REQ-0010
title: 镜像默认通道与GitHub兜底
status: implemented
priority: must
trace: cargo test --release --locked 全绿 220 项；cargo fmt --check 与 cargo clippy --release --locked 零告警新增；离线集成测 query_pind驱动_零api镜像直装url；设计决策 ADR-0008
---

# REQ-0010:镜像默认通道与GitHub兜底

总台派单（飞轮正式单 2026-09-18），用户裁定两连发：ohmygh.com 为默认元数据与下载通道，GitHub 降兜底。设计决策与待裁清单见 `docs/adr/ADR-0008-镜像默认通道与GitHub兜底.md`。

## Scenario

五端跑 `ark update` 与 `ark install` 全量时，解析与自更新元数据不应打 api.github.com（匿名 60 次/时配额、gh 凭证失效即批量失败）；镜像与 catalog 是版本与资产的默认真源，GitHub 仅在镜像缺数据时兜底。

## Criteria

- [x] A. 解析面：pin 驱动（无 latest/tag/version 选项）解析零 GitHub API，asset_url 为镜像资产域直拼、fallback_url 承载官方确定性直链（resolve.rs pin_direct，纯函数单测三件）
- [x] B. 下载面：D44 镜像优先机制无回归（既有测试面全绿，官方回落地址对 pin 驱动路径取 fallback_url）
- [x] C. 自更新面：元数据镜像段读序先行、GitHub API 兜底；ARK_MIRROR=1 兼容 no-op、ARK_MIRROR=0 官方优先逃逸阀（is_mirror_off 纯核入测，下载两条链同阀）
- [x] update 对齐 catalog pin：解析 pin 驱动，D49 补装三态与不回写锁定不动（cmd_update 改默认 ResolveOptions）
- [x] 显式 latest/tag/version 与 pin 三键缺一仍走 GitHub API 兜底（上游最新语义保留，待裁 1 见 ADR）
- [x] 测试按 R004 三层：纯函数单测（pin_direct 资格与构造、逃逸阀值域）加离线集成（query 零 API 走镜像、断网面）加既有真网闸门面（D38/D44/D41 gated 全数保留）
- [x] 门禁：cargo fmt --check、cargo clippy --release --locked、cargo test --release --locked 全绿；文档四件套与 PEVO 合规

## trace 补充

- 行为对照（改后）：update 全量与 install pin 驱动零 API 走镜像；install --latest 与 query --latest 走 GitHub API（兜底语义）；self update dev/stable 镜像段读序先行、官方 API 兜底、ARK_MIRROR=0 反转。
- 遗留：五端舰队实弹复跑（总台安排）；latest 语义与上游跟进两问待用户裁定（ADR-0008 待裁节）。
