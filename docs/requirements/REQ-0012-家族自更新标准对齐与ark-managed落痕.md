---
id: REQ-0012
title: 家族自更新标准对齐与ark-managed落痕
status: implemented
priority: must
trace: cargo test --release --locked 全绿 227 项；门禁全绿；对线两轮加三轮快核（一轮 F1 至 F4 与 G 全修 8190e9b、二轮 F-1/F-2 与 G-a/b/c 收口，见 diary 2026-09-18 REQ-0012 篇）
---

# REQ-0012:家族自更新标准对齐与ark-managed落痕

总台派单（2026-09-18 家族自更新统一标准轮，用户裁「采纳推广」加「两件全派」）。标准权威：browse-rs REQ-005 加 build-release 公共契约第六节。验收判据：同工具 self update 与管理器 update 同报已最新，各端终态同 digest。

## 差异表（ark selfupdate.rs 对照标准口径逐条）

| # | 标准口径 | ark 现状（截至 v1.4.0） | 差异裁决与本批改动 |
| --- | --- | --- | --- |
| 1 | stable 段双通道优先：镜像 stable 滚动段下载腿优先，任一步失败整对回落 GitHub | stable 通道沿用 D51 镜像元数据读序（边车锚）加官方 API 兜底；镜像命中而资产网络性失败补官方链（对线 F2） | **部分差异**：stable 通道改家族形：判新与元数据走官方 latest API 一次（tag 加 digest，browse 同构「判新走 GitHub tag，镜像段资产名不带版本无法反查」在 ark 同理），下载腿镜像 stable 段优先（官方 digest 锚加 CF 击穿 query）；镜像腿任一步失败（未命中或锚不符，含播种滞后常态）整对回落官方、官方终腿 digest 硬校验拒坏件（对线 F1 裁定：ark 资产名不带版本，与 browse 同名 404 整对回落形不同，锚不符含滞后常态故不套用同源对硬拒）。dev 通道（默认）维持 D51 镜像元数据优先（用户裁定 2026-09-18 两连发在先，dev 滚动源无版本语义），家族契约「dev 加 stable 双通道是否随仓开放由仓裁」 |
| 2 | digest 锚硬校验，哈希不符硬拒不回落 | 镜像段锚不符视同镜像失败回落官方（D44 工具面红线沿用）；官方腿终态 digest 硬校验已拒（不符即败） | **差异收口**：dev 腿镜像边车锚为同源对（边车与资产同段），不符硬拒 Err 不回落；stable 腿官方 digest 为跨源锚，不符（含播种滞后）整对回落官方、官方终腿 digest 硬校验（对线 F1 裁定）；网络性失败两腿均整对回落。install 工具面维持 D44「有锚必校验、不符回落」（pin sha 是官方真源锚，语义不同，入档分家） |
| 3 | semver 只升不降，本地领先报 localNewer 不动 | 判新纯 digest 等值（等值 current、不同即装）；镜像滞后会回装旧件（对线 G3 已记 ADR 滞回注记） | **缺失补齐**：stable 通道加 semver 门（远端 tag 去 v 与本地 CARGO_PKG_VERSION 数值段比较）：相等 current、本地新 localNewer 不动、远端新才进下载腿；dev 通道无版本语义（滚动 digest 锚）不适用，入档 |
| 4 | 自替换自证回滚：同目录暂存防跨文件系统 rename 加 pid 锁防并发互踩加陈旧收割加 --version 自证五次重试加证败回滚复核 | 同目录暂存已有（POSIX ark-new 加 win .old）；无 pid 后缀、无更新锁、POSIX 无旧件备份（rename 覆盖即失旧件）、无 --version 自证、win 仅 copy 失败改名回滚无复核 | **缺失补齐**：统一替换链：deploy 同目录暂存 ark-new-\<pid\> 与备份 ark-old-\<pid\>（POSIX 亦有备份可回滚）、锁文件 .ark-selfupdate.lock（pid 活性检测，死锁收割）、启动清扫陈旧暂存/备份残留（win 运行中删不动则跳过留待）、替换后 --version 自证五次重试（stable 断言含远端版本、dev 断言可执行非空）、证败回滚并复核旧件在位、回滚受阻报自救路径 |
| 5 | 边车即镜像锚契约、发布器与升级器同 digest | stable 通道判新锚改官方 digest（与发布器同判据，r2-seed 同源同 digest）；镜像下载腿同锚校验 | 对齐（随 #1/#2 落） |
| 6 | 元数据对齐：管理器以实值探活判 | status installed 走 probe 实测（toolver::installed_version），不依赖登记 | **已达标**：本机实证 browse self update 0.6.0 到 0.7.0 后 `ark status` 的 installed=0.7.0 与 `browse --version` 同报（locked=0.6.0 待总台滚 catalog 对齐） |
| 7 | ark-managed 落痕生产者契约（browse 让位判据依赖） | 无落痕 | **缺失补齐**：install/update 绿色主链与 official 型安装真身落位后，exe 同目录写 `ark-managed` 标记文件（内容 = ark 版本号）；npm-tgz 与 uv-git 与 msi 与 rustup/vsbuild 特型真身在他管域不落（入档分家）。卸载语义：ark 无 uninstall 命令，标记随工具目录重建自然重写、手删目录即连标记消失，无孤儿面（doctor 缓存孤儿检测只扫 cache 目录，不受落痕影响） |

## Criteria

- [x] stable 通道家族形：官方 API 判新（tag+digest）加 semver 门（current/localNewer/升级三态）加镜像 stable 段下载优先加网络性失败整对回落官方
- [x] digest 锚硬校验：dev 镜像腿同源边车锚不符硬拒不回落；stable 镜像腿不符整对回落官方（滞后常态，资产名形差异所致）、官方终腿 digest 硬校验；判型常量共用（对线 G3-1）单测覆盖
- [x] 自替换五小件：同目录暂存 pid 后缀、更新锁与死锁收割、陈旧残留清扫、--version 自证五次重试（stable 版本断言/dev 可执行断言）、证败回滚加复核
- [x] localNewer 机读面：SelfUpdateOutcome.action 增 localNewer 态，CLI kv 输出
- [x] ark-managed 落痕：主链与 official 型落 exe 同目录标记文件（内容 ark 版本），幂等；特型不落入档
- [x] 测试按 R004 三层；门禁全绿；对线 review 后封版

## trace

- 单测：semver门三态与非semver放行、错误判型校验性与网络性、自替换好件自证过并清备份、坏件自证败回滚旧件、死锁收割（pid 超 pid_max 形，u32::MAX 在 linux 回绕 -1 实证踩坑）、陈旧收割、落痕幂等
- 集成：linux_install jq 装后同目录 ark-managed 断言（内容=版本号）
- 实证：browse 0.7.0 self update 后本机 ark status installed=0.7.0 与 browse --version 同报（件三引擎侧达标）
