---
id: REQ-0007
title: 对齐build-release标准迁移（批一播种面加批二发布面）
status: draft
priority: must
trace: 总台对齐派单与核准单（2026-09-17，批一批二核准、差4 裁 a、差9 保留 GitHub 优先与 hst ADR-0004 族规一致）；实施中，批一批二落地后回填
---

# REQ-0007:对齐build-release标准迁移

## Scenario

总台定稿全仓编译打包发布流程标准（project-evo build-release skill，三段式：本地编译打包到 gh release 直发到 CI 自动播种），派单各仓对号入座自查回执。ark 工位回执差距清单十项，总台核准单（2026-09-17）裁定：批一播种面与批二发布面动工，差4 裁 (a) 版本段收归自家 r2-seed 双段同灌（omc seed run 保留总台代发例外与无自播工具面，catalog 真源治理仍归总台），差5 存量 msvc 对象以排除清单保一窗（退役点为下一正式版封版窗口、五端 1.3.0+ 全实证后），差9 判新读序保留 GitHub 优先（与 hst ADR-0004 族规一致，403 限流背景 fleet-wide，不做镜像优先改造），dev 通道 CI 轻岗豁免保留（同 hst 裁）。批三包形专项另立 REQ-0008。

## Criteria

验收判据，可检验、可勾选：

- [ ] 批一：独立 r2-seed workflow（release published 事件触发加 workflow_dispatch 带 tag 入参补推口，prerelease 不进本岗）
- [ ] 批一：双段同灌（ark/<版本>/ 段 copy 加 immutable 长缓存头；ark/stable/ 段 sync 清旧，排除清单保 D46 msvc 回退件一窗）
- [ ] 批一：双段零上传红灯（版本段与 stable 段分别清点报数，任一零即红）
- [ ] 批一：dispatch 补推 run 对 v1.3.0 实跑，双段边车 curl 实测与 release 资产逐字等，msvc 窗口件存活实证
- [ ] 批二：本地发布命令面（版本一致性闸：tag 对 Cargo.toml 不一致即止红；测试闸先行；linux 本职加 win-gnu 交叉本地构建；mac 实机构建；逐件解包面冒烟 --version 对 tag）
- [ ] 批二：gh release create --latest 直发禁 draft，逐件 .sha256 边车挂 release（Release 与镜像段同源同批）
- [ ] 批二：build.yml 撤 v* tag 触发与正式构建岗与 stable 灌段；dev 轻岗豁免保留（main 推构建挂 dev prerelease 加 ark/dev 段 sync 形灌段）
- [ ] 实施纪律：每批独立提交走 herdr codex 评审闸门至 CONFIRM，回执总台附 run 链接

## 差9 口径存档

判新读序保留 GitHub 优先、失败自动回退镜像腿（ARK_MIRROR=1 可反转为镜像优先），与 hst ADR-0004「不占缺省行为面，ark 先例」互为族规；总台核准单明示不做镜像优先改造。

实现后回填 frontmatter 的 trace，状态 implemented。
