---
id: ADR-0008
title: 镜像默认通道与GitHub兜底
status: accepted
date: 2026-09-18
deciders:
  - 用户（2026-09-18 两连发裁定，总台派单转呈）
superseded_by: null
supersedes: []
tags:
  - channel
  - resolve
  - download
  - selfupdate
---

# ADR-0008:镜像默认通道与GitHub兜底

关联 REQ-0010。冻结索引指针 PRD D51。

## Context

用户裁定（2026-09-18 两连发）：「ark 为什么不是默认使用 ohmygh.com 获取元数据和下载！github 应该是兜底，不是默认流程」「各仓的自更新应该也是默认 ohmygh.com」。

五端实弹病灶（总台 2026-09-18 ark update 与 install 全量实跑）：

- 解析面：`resolve.rs` 分支 c pin 驱动解析直打 api.github.com（releases/latest 与 releases/tags）；镜像回落是 API 失败后的补救（D38），仅 `ARK_MIRROR=1` 真跳过 API。
- 后果：四远端 gh 未认证或凭证失效，全量 update 逐工具撞 api.github.com 匿名 60 次/时配额：lan-win 30 项查询失败 exit 1、lan-mac 401 bad credentials、lan-linux 未 login、lan-ubuntu 无 gh；五端共享出口 IP 加剧。
- 自更新面：selfupdate 官方通道元数据 GitHub API 优先，`ARK_MIRROR=1` 才镜像优先。
- 对照组：下载腿 D44 已镜像优先生效。

镜像侧可用元数据面：catalog 三件套（env.ohmygh.com/ark/catalog/tools.toml 加 .sha256 加 .minisig，seq 防回滚，版本 pin 真源）与资产域 `/<tool>/<version>/<asset>` 加 .sha256 边车即锚。数据面 44 个 GitHub 分支工具 pin 三平台全量齐备（2026-09-18 本机同步副本核验），pin 驱动零 API 无数据缺口。

约束：R015「无 sha 不入镜」与「有锚必校验」红线不动；D49 update 漂移三态语义不动；D37 锁定单源归数据面不动。

## Decision

三面翻转，镜像转正为默认通道、GitHub 降为兜底：

1. **解析面（resolve 分支 c）**：pin 驱动（无 latest/tag/version 显式选项，即 install/update/query/heal 的默认路径）零 GitHub API。catalog pin 三键（tag/version/asset）在位即直装：asset_url 为镜像资产域直拼 URL（主通道），新增 `Resolution.fallback_url` 承载 GitHub release 确定性下载直链（`github.com/{repo}/releases/download/{tag}/{asset}`，对象直链非 API、无配额面，下载层镜像失败时回落）。pin 三键缺一（数据面未 pin 完整）回落 GitHub API（镜像缺数据兜底）。D38 的「API 失败回落镜像」路径删除，被默认通道收编（pin 键齐根本不打 API，键缺回落也不成立）。
2. **下载面**：D44 机制不动（镜像单次快速首试、锚校验、官方完整链兜底），仅官方回落地址对 pin 驱动路径改取 fallback_url。核对无回归（既有 D44 测试面全绿）。
3. **自更新面**：release 元数据镜像段读序先行（边车即锚；读序：包形主名、gnu 裸件、msvc 回退，ome 兼容层已随同批剔除收口、段恒 ark/），未命中回落官方 API；双败报两段错误；镜像元数据命中而资产下载失败时补拉官方 API 元数据走真官方链（对线 F2 修复，第二腿不再打同一镜像 URL）。
4. **update 语义对齐**：update 解析从 `--latest` 改 pin 驱动，云端「最新」定义随此改指镜像与 catalog（上游滚版归数据面 omc 滚锁）；D49 三态（一致 skip、落后补装、领先如实报）与不回写锁定不动。
5. **开关设计（你仓设计权）**：`ARK_MIRROR=1` 转正为默认、保留为兼容 no-op（旧名 `OME_MIRROR` 读回已随同批 ome 剔除撤除）；新增 `ARK_MIRROR=0` 官方优先逃逸阀：下载层跳过镜像首试直走官方、selfupdate 元数据反转读序（官方先行镜像回落，即 D41/D44 原序）。解析面无需此阀（pin 驱动本就零 API 且兜底本就是官方直链）。
6. **latest 语义（待裁保留现设计）**：`--latest` 与未 pin 自动解析仍是「上游最新」语义、走 GitHub API 兜底：镜像侧无 latest 元数据（资产域仅版本段与 evergreen latest 段），catalog pin 是唯一版本真源而「比 pin 更新」只有上游知道。临时钉版工作流（`ark pin <tool> --latest` 钉上游新版）依赖此语义，不裁不动。

## Consequences

- 好：默认路径（update/install/query/status/heal/self update）全链零 GitHub API，五端匿名配额面与 gh 凭证依赖拔除；私有仓工具（omc）update 从必败（latest 打 API 404）变镜像直装可用；query 离线纯读（零网络）。
- 好：`url=` 输出契约延续 D38「如实呈现镜像地址」，舰队脚本无感；下载层 D44 机制与锚红线零改动。
- 坏：GitHub 上游新于 pin 时 update 不跟进（对齐 pin；领先仅在 status 漂移提示可见），跟进速度取决于数据面滚锁节奏，需 omc 运营对齐（见待裁 2）。
- 坏：fallback_url 依赖 GitHub 下载直链形态稳定性（公开仓历史稳定；私有仓兜底本就不可达匿名，主通道镜像在位无实害）。
- 坏（对线 G3 注记）：dev 滚动通道判新锚随镜像边车，镜像播种滞后窗口内 self update 可能回装镜像旧件（stable 通道仅判新滞后无害）；追新走 `ARK_MIRROR=0` 逃逸或等播种对齐。
- 后续要跟：aidoc 投影重生成（Resolution 新 pub 字段）；R015 消费侧段与 README/AGENTS/llms 口径同步；五端舰队重跑 update 全量验证零 API 实效。

### 待用户裁定

1. **latest 是否恒等于 catalog pin**：现设计保留「上游最新」语义（显式请求才打 API）。若裁定 latest := pin，可再省一类显式 API 调用，但临时钉版上游新版工作流（pin --latest）失效，需另设出口。
2. **上游新于 pin 是否跟进**：现设计不跟进（滚锁归数据面）。若要引擎跟进需数据面定义同步策略与节流（否则回到全量打 API 的老路）。
3. **ARK_MIRROR 值域终态**：现设计 =1 保留为兼容 no-op、=0 逃逸阀。若要彻底退役 =1（遇值报错）需发版窗口配合舰队清理。
