# 2026-09-17 build-release标准对齐批一批二

## 派单与核准

总台定稿全仓编译打包发布流程标准（project-evo plugins/evo-adr/skills/build-release/，六型路由加七铁则加护栏三件），派单对号入座自查回执。ark 回执差距十项（三 P0 架构、四 P1 契约、三 P2 细节）加整改三批计划。总台核准单（同日）：批一批二动工、差4 裁 (a) 版本段收归自家双段同灌、差5 msvc 存量排除清单保一窗（退役点下一封版窗口）、差9 保留 GitHub 优先与 hst ADR-0004 族规一致不改造、dev 通道 CI 轻岗豁免同 hst 裁、批三包形另立 REQ-0008 且动工前先与总台对线 asset 形方案。

## 批一 播种面：r2-seed 双段同灌

三笔（271d9bf 加 210b3fc 加 9210a63），评审两轮至 CONFIRM。

- 新 r2-seed workflow：release published 事件触发（prerelease 不进岗）加 workflow_dispatch tag 补推口；调 seed.py --ark-published。
- seed.py 双段模式：版本段 copy 加 immutable 31536000 头（差8）、stable 段 sync 清旧带 STABLE_PROTECT 排除清单（差5）、双段逐名核对齐备红灯（lsf 按暂存件名核对，防「只剩保窗件」假绿）；--ark-stable 与 --ark-dev 同步升 sync 形（源即段终态、失败整体不灌防半失败、空源拒 sync）；skip 面护栏沿用（零到件红灯加 ARK_SEED_ALLOW_SKIP 豁免口，限 stable 与 dev 面）。
- rclone 语义实证钉死：excluded 默认不删、--delete-excluded 反清保护面，故保窗组合是排除加不带该旗标（与标准模板白名单清垃圾场景不同形，核准单「排除清单保一窗」的正确落地）。
- 一轮 2F：published 先于资产传完（gh release create 先建档后传件，v1.3.0 实测窗口约 3 分钟），修以资产齐备轮询闸（30 轮乘 10 秒有界超时红）；msvc 存活判据按桶态探底改可检形（stable 与 dev 段现 404 无该件、仅 ark/1.2.1 版本段在，排除清单为空集防御形）。二轮 CONFIRM 加 G 顺采。
- 首跑红一次：CONFIRM 后手改顺采把 jq 参数收尾引号吃掉（bash 语法错退 2，b70116a 自纠：jq 改逐行输出加 grep -qx 精确行匹配，全部 run 块 bash -n 加真 gh 实跑验证后复绿）。
- 实跑回执：dispatch run 35187398380 success，mirror objects ver 与 stable 各 6 件逐名在位；双段六边车 curl 带 bust 实测 18c657ea 加 6018e1f0 加 6cfef166 与 1.3.0 锚逐字等。

## 批二 发布面：本地三段式直发

三笔（9f7fdbf 加 42447e8 加 cee851d），评审两轮至 CONFIRM。

- .tools/release.ps1（pwsh 7）：tag 锚闸加工作树洁净闸（全形含未跟踪，忘 git add 的漏形态也拦）加发布预检三件（照 hst 1b 族规：gh 登录、远端 tag 已推且锚本 sha 全等、无既有 release 报恢复路径）加版本一致性闸（Cargo.toml 对 tag）加测试闸（cargo test 加 md 四件套加 aidoc 漂移，v* 不再触发 CI 本闸即最后门禁）加 linux 本职与 win-gnu 交叉本地构建加 lan-mac 实机腿（rsync 源码树免管远端 git 态）加逐件 .sha256 边车（原生格式）加跨实机冒烟逐字对（win 走 WSL interop 直调 PE、mac 实机 chmod 后跑）加 gh draft 挂六件传齐后 edit --draft=false --latest 发布（published 即齐备信号，一轮评审 F1 采纳形）。
- build.yml 撤岗：v* tag 触发删、stable_tag 输入删（补推口归 r2-seed）、发布步与 mirror 只留 dev 轻岗（prerelease 滚动加 ark/dev sync 灌段，总台裁豁免保留）。
- 一轮 2F（远程 tag 预检缺位：gh 会自动钉默认分支 HEAD；dist 未入 gitignore，距 27MB 产物回仓一步之遥）加 4G 全修；二轮 CONFIRM 加 G 顺采（洁净闸全形出豁免块加 ls-remote 分诊加冒烟注）。
- 演练抓真缺陷一枚：mac 冒烟件 scp 丢执行位（permission denied），修 chmod 后跑；全链演练真退出码 0（三件冒烟全 ark 1.3.0、六件摘要出、发布步 DryRun 止步）。

## REQ 与收尾

- REQ-0007 立项至 implemented 回填（判据八条全勾，trace 含 run 与演练证据）；REQ-0008 包形专项 draft 立项（动工前置：与总台对线 catalog [tools.ark] 随迁与 selfupdate 兼容窗口与退役清理三面；批一 G2 顺采「r2-seed 齐备闸名单随包名同迁」判据在册）。
- 差9 口径存档：判新保留 GitHub 优先、镜像自动回退（ARK_MIRROR=1 可反转），与 hst ADR-0004 互为族规。

## 遗留与断网态

- 断网插曲：批二收尾时出口 TLS 整面被重置（GitHub 与 Cloudflare 域全 000、TCP 可达、网关无代理口），推送重试环挂至网络自愈后四笔一次推齐（b70116a 至 0f6ebb0）。
- 真形排练（评审收尾建议，断网恢复后补录）：真推排练 tag rehearse-req0007（非 v* 名不触发任何工作流）后无豁免跑 -DryRun，0a 工作树洁净闸与 0b 发布预检三件首次真执行通过（gh 登录、远端 tag 锚本 sha、无既有 release），版本一致性闸对非版本 tag 正确红止（exit 1）；排练 tag 两端已清。
- msvc 排除清单退役点：下一正式版封版窗口（五端 1.3.0+ 全实证后撤清单清段）。
- 批三包形专项：待总台对线 asset 形方案后立项动工（REQ-0008）。
