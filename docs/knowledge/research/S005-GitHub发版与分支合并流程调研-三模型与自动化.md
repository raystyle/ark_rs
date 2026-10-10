# S005 GitHub 发版与分支合并流程调研：三模型与自动化

> 来源：用户 2026-09-08 提供调研全文（镜像侧协作产出，含官方文档与工具星数实证）；本仓整理归档，供 ROADMAP 阶段七封版发布决策引用。
> 六态标注沿用原文：[实证] 为官方文档抓取或当日实测，[推断] 为原文作者综合判断。

## 一、结论速览

1. 没有「唯一标准」，主流三选一：GitHub Flow（官方轻量流，持续交付类首选）、Git Flow（显式版本化、多版本并存）、Trunk-Based（大团队高吞吐）。Git Flow 作者 2020 反思注明确：做持续交付的 Web 应用别用它，选更简单的 GitHub Flow [实证：nvie.com 原文反思注]。
2. 分支开发合并的落地单元是 PR 加分支保护：建分支、提交、开 PR、评审、合并、删分支六步 [实证：docs.github.com github-flow 全文]。
3. 合并方式三种（merge commit / squash / rebase），选型决定历史形态；发版官方载体是基于 Git tag 的 Release，配自动生成 notes [实证：docs.github.com about-releases]。
4. 自动化发版生态成熟：semantic-release（24.0k 星）、GoReleaser（16.0k 星）、release-please（7.5k 星，action 另 2.5k），共同前提是 Conventional Commits 规范提交 [实证：2026-09-08 gh search 星数]。

## 二、三种分支模型

| 维度 | GitHub Flow | Git Flow | Trunk-Based |
| --- | --- | --- | --- |
| 长命分支 | main 一条 | master 加 develop 两条 | trunk（main）一条 |
| 短命分支 | feature 分支 | feature / release-* / hotfix-* | 单人短命分支（存活小于一天） |
| 发布动作 | 合并即部署候选 | release 分支定版本号，合回 master 并打 tag | 从 trunk 即时切 release 分支或直接从 trunk 发，fix forward |
| 适用 | Web 应用、持续交付 | 显式版本化、多版本并存（客户端、库） | 大团队、高合并吞吐 |
| 出处 | GitHub 官方文档 | nvie 2010 博文 | trunkbaseddevelopment.com |

GitHub Flow 六步（官方）[实证]：建描述性命名的分支；提交推送（每个 commit 是独立完整变更，便于回滚）；开 PR（带变更摘要、链接 issue）；评审（可 draft PR 提前要反馈）；合并；删分支（官方明确要求，防误用旧分支）。

Git Flow 关键规则 [实证：nvie 原文含全部命令]：feature 从 develop 出回 develop；release 从 develop 出、在此定版本号、回 master 打 tag 同时回 develop；hotfix 从 master 对应 tag 出、回 master 加 develop（有 release 分支存在时合入 release 分支）。合并一律 `--no-ff` 保留特征分支历史存在感，便于整组回滚。

Trunk-Based 要点：每人每天至少合一次 trunk（CI 的核心要求）；未完成功能靠 feature flags 与 branch by abstraction 藏住；Google 35000 人单一 monorepo trunk。官网明说 GitHub Flow 与它高度相似，唯一差别是从哪发布 [实证：trunkbaseddevelopment.com]。

## 三、合并三式

| 方式 | Git 行为 | 得 | 失 |
| --- | --- | --- | --- |
| Merge commit（默认） | `--no-ff`，全部提交入 base 加合并节点 | 保留全部历史与分组 | 历史分叉，log 噪音大 |
| Squash and merge | 压成单提交，fast-forward | 历史最干净 | 丢原始 SHA 与逐提交时间；同分支再开 PR 会重复列出已压提交、反复解冲突 |
| Rebase and merge | 逐提交重放到 base，无合并节点 | 线性历史且保留逐提交 | 重写 committer 与 SHA；产物提交不带签名（要签名须本地 rebase 后手推） |

配套开关：仓库可只允许一种合并方式强制统一风格；分支保护里 Require linear history 可禁 merge commit，强制 squash 或 rebase [实证：about-merge-methods 加 about-protected-branches]。

## 四、分支保护标准配置

官方功能常用组合 [实证：about-protected-branches 全文]：

- Require pull request reviews：N 个批准；可叠加 code owner 必须批准、最新一次 push 必须由非 push 者批准、新 push 使旧批准失效（dismiss stale）。
- Require status checks：CI 必过；strict 模式还要求分支与 base 同步（合并队列是它的无痛替代）。
- Require conversation resolution：所有评审对话已解决才许合。
- Require merge queue：高吞吐分支排队合并，逐个对最新 base 验证，杜绝「合并后 CI 才红」。
- 默认禁 force push、禁删分支；新版形态叫 rulesets（同分支多规则时更可控）。

## 五、发版 GitHub Releases

官方机制 [实证：about-releases]：

1. Release 基于 Git tag，标记仓库历史特定点；tag 日期与发布日期可以不同。
2. Release notes 可手写、默认模板自动生成、自定义模板。
3. GitHub 自动附源码 zip 与 tarball；额外资产上限 1000 个、单文件 2 GiB，总量与带宽无限。
4. 只有 write 权限者可管理 release；read 权限可看可订阅。
5. 修安全漏洞的 release 应同步发 security advisory（进 Dependabot 告警链）。

标准手动序列：定版本号（SemVer），打 tag（`git tag -s` 可签名），建 Release 填 notes，传资产。

## 六、发版自动化工具

| 工具 | 星数（2026-09-08 实测） | 机制 |
| --- | --- | --- |
| semantic-release | 24028 | 解析 Conventional Commits，全自动 bump 版本、生成 CHANGELOG、发 GitHub Release 与 npm |
| goreleaser | 16024 | Go 生态标准，多平台交叉构建加打包加发布一体 |
| release-please | 7462（action 2515） | 以 PR 管理发版：机器人开 Release PR 攒 changelog，合 PR 即打 tag 发 Release，人保留最终点击权 |

三者都以 Conventional Commits（feat/fix/chore 前缀）为输入，标准流程的前置纪律是规范提交信息 [推断：三工具文档共同要求，未逐一抓全文；复核点各自 README quickstart]。

## 七、合成推荐原文

1. main 设保护：PR 必须、1 至 2 批准、CI 必过、禁 force push。
2. 日常开发：短命 feature 或 fix 分支（描述性命名），一天内可合完；不合长命 develop 分支（除非确有多版本需求）。
3. 合并策略：默认 squash and merge 换线性历史；确需保留逐提交历史的功能分支用 merge commit；仓库设置里关掉不用的合并方式。
4. 提交纪律：Conventional Commits，为自动化发版供料。
5. 发版：release-please（保留人工合 PR 决策点）或 semantic-release（全自动）驱动 bump 加 changelog 加 tag 加 Release；无自动化时按第五节手动序列。
6. 热修：持续交付型直接 fix forward（新 PR 修 main 再发）；多版本维护型从生产 tag 切 hotfix 分支，双合回 master 与 develop。

选型速查：Web 或 SaaS 持续交付选 GitHub Flow 加 squash 加 semantic-release；开源库或多版本支持加 release 分支加 release-please；大团队高吞吐上 Trunk-Based 加 merge queue 加 feature flags。

## 八、对本仓的适用速评已裁

2026-09-08 用户裁：分支采 GitHub Flow 单干变体（直推 main 为基线，并行会话短命分支 squash），发版采 tag 驱动手动封版七步（见 ROADMAP 阶段七），release-please 不引入。以下为裁前速评原文。

- 本仓现役形态：单人直推 main（近似 GitHub Flow 的单干变体，无 PR 评审环节），提交纪律 feat/docs/fix/chore 前缀已与 Conventional Commits 同构。
- 封版（ROADMAP 阶段七）已有机制：build.yml 的 v* tag 双通道路由（tag 推送出正式 release），对应第五节手动序列的 CI 化版本；CHANGELOG 手工维护与 release-please 的攒单式同目标，切不切换留封版时裁。
- 分支保护与 PR 门禁在单人仓收益有限；若未来开放协作者再启用（第四节清单可直接引用）。

## 九、来源

| 来源 | 取用 |
| --- | --- |
| docs.github.com：github-flow / about-releases / about-merge-methods / about-protected-branches | 全文抓取 |
| nvie.com Git Flow 原文（2010，2020 反思注） | 全文抓取 |
| trunkbaseddevelopment.com | 首页抓取 |
| 工具星数：semantic-release / goreleaser / release-please | gh search 实测 |
| 辅助对照：codewithmukesh.com 对比文（2026-02）、Medium 对比文、Reddit r/git 讨论 | SERP 摘要 |
