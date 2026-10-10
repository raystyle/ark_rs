# R014：ohmycloud 种子清单，ISSUE 派任务与对齐

> 角色：现役流程。ark（D41 前名 ome）与兄弟仓 ohmycloud（资源分发基建，env.ohmygh.com）之间，**种子资源清单只走 GitHub ISSUE 派任务和对齐**。catalog 是唯一 pin 权威；ohmycloud 源仓只读、零改动。
>
> **通道更替（2026-09-10 裁）**：跨仓周知与回执一律走 herdr 会话互相同步，不再发 ISSUE；存量 ISSUE（ohmycloud#8、ohmyenv-rs#10 等）不强求跟进，信息以 herdr 同步为准。下文一、三节的 ISSUE 派单步骤自该日起退为历史流程（种子上传已由 D27 CI 自维护承接，ISSUE 通道本已只剩对账与例外通报用）。

## 一、通道

| 侧 | 做什么 |
| --- | --- |
| ome | `catalog\tools.toml` 为唯一清单源；用 `.tools\seed-inventory.py` 抽可入镜对象；向 `raystyle/ohmycloud` 开或回 ISSUE |
| ohmycloud | 按 ISSUE 种子 / 下架 / 刷新 latest 段；通报对象数与差集 |
| 对齐完成 | ark 侧 `ARK_TEST_MIRROR=1` 断官方源回落验收 |

ISSUE 标签用 `coordination` 加 `distribution`。中文排版：短段、列表、留白可换行。

## 二、入镜规则

1. **可入镜**：该平台 `version` + `asset` + `sha256` 三键齐。路径 `https://env.ohmygh.com/<tool>/<version>/<asset>`，同名 `.sha256` 边车。
2. **无锚不入镜**：缺 sha 的对象写「待 ome 回填」不派种。
3. **evergreen**：`ome` / `rust` / `vsbuild` 走 `latest` 段，边车即锚（ohmycloud#4 先例）。
4. **下架**：catalog 删除的工具在 ISSUE 里点名请镜像删除，避免脏对象。
5. **信任锚**：安装校验同 catalog pin，镜像不做第二份 manifest。

## 三、派发步骤

1. 跑 `uv run --script .tools/seed-inventory.py` 得到当期快照。
2. 与上一份已关闭的 ohmycloud 种子 ISSUE 做差集：新增、下架、sha 待回填、latest 段保持。
3. **差集先对域面 HEAD**（M012）：拟补种已 200 则不派；拟下架已 404 则不派。文本差集滞后于镜面是常态。
4. 在 `raystyle/ohmycloud` 开新 ISSUE（或在仍开放的种子 ISSUE 追评），正文含契约、差集、请求、完整清单（可折叠）。
5. 本仓 diary 记 ISSUE 号；ohmycloud 通报后再跑镜像闸门，并复核通报中的出入。

## 四、本仓脚本

```powershell
uv run --script .tools/seed-inventory.py
uv run --script .tools/seed-inventory.py --json
```

退出码 0 成功、2 读 catalog 失败。

## 五、种子上传自维护 D27

- 双仓自维护 S3 种子（用户裁，镜像方 D41 对偶）：本仓 `.tools/seed.py`（uv 单件）读 catalog 出清单，对 env.ohmygh.com 域面 diff（GET 边车带 `?t=` 击穿加 HEAD 资产，`--plan` 模式无凭据可跑）后经 rclone 直传 R2；配方 provider=Cloudflare、NO_CHECK_BUCKET 必带（受限 token 无建桶权）、Cache-Control public max-age=60（消费侧 `?v=`/`?t=` 击穿双保险）；R2 四键在本仓 GitHub Secrets。
- 路线 A（本仓现役唯一播种面）：build.yml mirror-r2 job，ome 自产产物灌 R2 段（oma 同型，段名与 self update 通道同名）：push main 灌 `ome/dev` 沙滚段，push v* tag 或 workflow_dispatch 指定 stable_tag 灌 `ome/stable` 正式段（`ome/latest` 段退役）。路线 B 与 `ome/catalog/` 三件套已整体移交 ohmycloud catalog-seed（D37 终版：权威数据、lint 门、签名、资产域播种皆其仓，批 2 资产域首跑 success；本仓 seed-mirror.yml 已撤，`.tools/seed.py` 留 `--plan` 只读对账面，清单源回退序为仓库件、用户数据副本）。
- ohmycloud env-seed 自此退居灾备对账；ISSUE 派单通道保留（对账与例外通报用）。

## 六、agent 部署委托与发版知会 D29

> 三仓共识（2026-09-10）：ohmycloud 提案、oma 侧 ohmycloud#6 同日确认、ome 侧回执经 herdr 会话同步（通道更替裁当日生效）。

1. **委托口径**：omc agent deploy 全面委托 ark（D41 前名 ome），链路为 env 镜像拉 ark 二进制、catalog 推端、`ark install`（豁免端仅装工具面：claude codex sops age；omc 侧冻结调用由 `ome install` 换 `ark install` 加别名过渡窗口，D41 herdr 知会）；omc 顶层新增 ark / oma 透传命令。ark 只管二进制与工具面，配置、hook、编排归 oma。ohmycloud 侧 2026-09-10 同步实装态：ssh2 舰队钥通道（hostExec / sftp 读写）已通，豁免端 lan-linux 仅工具面，sops v3.13.3 端上实装。
2. **sops / age 三平台槽位持续维护**：catalog 唯一 pin 源纪律（R001）加 D28 结构机检加 D27 种子 CI 域面 diff。2026-09-10 核对：sops 3.13.3 与 age 1.3.1 三平台 pin 齐且镜像 HEAD 200。
3. **claude linux / mac 资产链**：ohmyenv-rs#10 缺口 1 已修（linux pattern x86_64 改 x64，官方 SHASUMS256 与 pin 三平台 sha 逐字一致，linux 包单文件 claude 居根；镜像双资产 HEAD 200）；余量（codex linux/mac 嵌套 bin 布局、catalog 裸端自举通道）挂 #10 按集成优先级排期。
4. **发版知会**：ark 正式 release（tag 推送）后 herdr 会话知会 ohmycloud 同步 omc tool status 镜像锚（AGENTS 义务表发布行）。oma CI 直推 S3 自维护 oma 段（oma/stable 段滚动已被 omc tool 域消费，segment 已切），与 ark D27 双路线同型并行；各仓 release 后锚同步以 herdr 知会为节拍，互不对账对方段域；桶段规范（段命名与桶面布局）变更须同步 ohmycloud（dist 域管桶面）。
5. **集成优先级**（用户裁 2026-09-10）：ome 与 omc / oma 集成的功能优先级为首要诊断与检测（doctor：系统 / 依赖两层，D30 收窄），其次恢复与治愈（heal / verify），最后安装部署配置（install 面）；ROADMAP 同步。
6. **版本里程碑对齐**（D30/D31，2026-09-10 当日两版）：omc 0.3.1 / ome v0.2.0 加 v0.2.1 已发并经 ohmycloud 验收全绿（stable 段边车锚与 omc tool status 对照一致；omc 断言为 URL 段形态不含版本锚，无需改）；oma v0.5.3 已验收（锚同步；此前 v0.5.2 系 v0.5.1 的 M059 快修：claude 本体在 Windows 装 Git Bash 时 hook 经 /usr/bin/bash -c 执行、& 调用操作符注册形态在 bash 是语法错误且退出码 2 等同阻断；v0.5.2 统一为无引号正斜杠绝对路径三吃注册形态，三资产 sha256 边车同源），ohmycloud 锚取 v0.5.2（oma 已直知会，ome 无需转达），三仓水位当日封齐（omc 0.3.1 / ome 0.3.1 / oma v0.5.3）；ohmycloud 终报 2026-09-10 收讫：其 R008 架构档管辖表升完全解耦终版（12b79d4），R015 从定档到实装全链闭环收官。重叠削减已落 ome 侧：doctor agent 层移除（装态对账归 omc、token 归 oma diagnose）、ome 自身装位与升级通道边界澄清（首次装位 omc tool 加 ome init 双通道同位，升级单通道 self update）；omc 调 ome install 契约冻结。另定协调项：yolo 托管端用户级以 omc patchCodexConfig 为准（oma 面向项目级）、pretrust 归 oma init（ome 无对应面，仅登记口径）。ome/stable 段由本仓 CI v* 自动直推；ome/latest 旧段已下架（2026-09-11 dist 域执行，env 桶余 catalog/dev/stable 三段）。
7. **管辖边界**（2026-09-10 用户裁递进定稿：**完全解耦**）：omc 管所有软件清单数据（权威 tools.toml条目与 pin 维护、catalog_lint 发布门、签名与三件套发布、软件资产播种（A 形态，双轨期 ome seed-mirror 续跑至对方首发对账绿后撤整条只留 seed.py --plan 对账面）；ome 只管诊断逻辑与安装部署逻辑加清单格式契约（R001）加自身发版与自更新，仓内权威 catalog 退役为测试夹具、端上一律云端拉取。批 2 资产域首跑 success（101 域面 100 同步）、批 3 权威面首发验签全绿（run 34485506081，云端 sha 零漂移）；ome 侧批 3 撤退已执行（权威件退役、seed-mirror 整撤留对账脚本）；pin 回写定案：update 收窄不回写（锁定单源数据面）、pin 留临时本地锁、drift 如实加滞后提示。三流程标准见 R015（五节终版表）。
