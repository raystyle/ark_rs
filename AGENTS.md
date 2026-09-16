# AGENTS.md

> Ark（Agent Runtime Kit，CLI 名 `ark`；D41 前名 ome/Oh My Env）是本机跨平台环境部署管理 CLI（Windows / Linux / macOS），独立仓库。统一分发体系分工（用户裁 2026-09-16）：omc 管资源分发运维与版本分发管理（catalog 真源、种子签发、镜像运维、版本对齐表），**ark 是执行引擎面**：各端 install/update/status/doctor 的实际执行与验收回执：50 个工具的版本解析、下载、校验、解压、PATH 注册、pin 锁定、更新与 doctor 诊断。一个标准、一个配置；下载默认走兄弟仓 ohmycloud 的 env.ohmygh.com 镜像、官方渠道兜底（D44）。本文件是协作规则的**最高约束**（五节合同，dev-evo 形态，ADR-0001 迁移）；细则唯一权威在对应 G/R 文档（摘要层铁律：双份并行必漂移）。

## Commands

意图与命令映射（参数与语义全表见 `README.md`，`PLAN.md` 为历史规划档；功能原语口径见 PRD 冻结索引 D10/D15/D16：doctor/install/status 三原语，其余为派生面）：

- 查版本：`ark query`（省略则全量；只解析不下载）
- 装工具：`ark install`（省略则全量；下载到 EnvRoot，注册 PATH、写注册表与配置；工具参收逗号串）
- 更新：`ark update`（省略则全量；拉云端最新安装，不回写锁定（锁定归数据面 D37）；云端无新版而本机落后锁定时补装锁定版（D49）；临时钉版走 pin）
- 锁定：`ark pin`（省略则全量；lock 为别名）
- 看状态：`ark status`（锁定 / 已装 / PATH 三态对照）
- 自部署：`ark init`（self-deploy 别名；二进制进用户程序目录、catalog 同步、注册 PATH）
- 查刷软件清单：`ark catalog`（status 看解析面与签名态，sync 立即从云端刷新过 minisign 校验；自动刷新按 `ARK_CATALOG_TTL`，`ARK_OFFLINE=1` 关，旧名 `OME_*` 读回）
- 查文档：先查 `llms.txt`（读序与代码文件位置）与各目录 README 索引再读；搜索方法：`rg -n "关键词" llms.txt`、`rg --files docs | rg 关键词`、`rg -n "关键词" docs/research docs/references`；`mq -F grep '.h2' docs/research/*.md`（section 必带 -A）；`ast-grep outline -l rs src/`（fn 模式必须带 body 通配 `$$$`、可见性写进模式）
- 验证门禁（每次交付必跑，裸跑看退出码）：`rumdl check .` 加 `uv run --script .tools/mdcharlint.py .` 加 `uv run --script .tools/md-ref-scan.py` 加 `uv run --script .tools/md-heading-scan.py`；结构大改加跑 `uv run --script .tools/md-replace.py`；体系合规加 `uv run ~/.claude/skills/dev-evo/scripts/check.py .`
- 测试：`cargo test --release --locked`；真实环境测试按 `ARK_TEST_REAL`（读回 `OME_TEST_REAL`）闸门 skip
- 格式与静态检查：`cargo fmt --check` 与 `cargo clippy --release --locked`（改 Rust 必跑）；lib 面 `missing_docs` 为 deny（Cargo.toml lints，ADR-0006 第五十九批强制口径）
- aidoc 投影：改 pub 项后 `cargo aidoc` 生成并同提交 `docs/aidoc/`（生成物手改被覆盖）；漂移门禁 `cargo aidoc --check --strict`
- 文档骨架合规：`PEVO_CHECK_ALLOW="^docs/aidoc/" uv run ~/repos/ProjectEvo/plugins/project-evo/skills/dev-evo/scripts/check.py .`（退出码 0；豁免在册，aidoc 条目分隔符 em dash 是渲染格式无开关，真门禁是 aidoc --check --strict）
- 提交：`feat:` / `docs:` / `fix:` / `chore:` 前缀加中文描述；一次提交只做一件事；未经指示不推远端

## Must

- 每轮对话先核对任务面（`docs/requirements/` REQ 与 AGENTS 环境节；历史决策查 PRD 冻结索引）；实质推进当场更新，禁止不核对就干活、偏离当前目标、推进了不更新
- 新需求先立 REQ（draft 起，实现回填 trace）；不可逆技术选择先立 ADR（`docs/adr/`，状态机 proposed 到 accepted 到 superseded）
- 踩坑当场落档：构成纪律或决策的立 ADR，过程性的记 diary，同根因同型坑合并（错误模式库 ADR-0002 至 ADR-0006）；深挖落 research。禁止只留在对话里反复试错
- 发现问题走五步闭环（G003）：定位（先搜索引）、归类（错修文档、缺补规则、知识落研究、出错记档、实证进 references）、修正（改在源头，下游同步）、验证（门禁全跑）、提交（一事一提交，diary 记钩子）
- 交付变更时改代码同步对应文档，改文档同步索引与 `docs/diary/`；版本级成果进 CHANGELOG
- 经验沉淀（G004 强规则）：成功方案回填 REQ trace 与关联 ADR；实证做法与多犯沉淀的正确工作流进 `docs/references/` 并挂路由或索引；同型坑二犯以上升格 references 并互指。禁止 `[经验]` 断言只留研究不落 references、错误只记现象不记根因、`[推断]`/`[假设]` 跳级、一条知识两个权威落位
- 写 Rust 先按 R005 双通道查 crates.io / GitHub 选最流行稳定库，最少代码接上，优先组合不自写协议、解压、HTTP、哈希、CLI 解析；**实质代码改动（新模块、跨文件接线、并发与进程管理）推送前必须经对线 review（herdr 驱动 codex 或用户点名复核），对线结论与修复回执入 diary**（用户裁 2026-09-11）
- 写文档遵守 G001（树形、标题干净、文件名即标题、rumdl 与 .tools/mdcharlint.py 禁字机检）；写研究与测试文档事实性断言必标六态之一（G002）：`[实证]`、`[推断]`、`[经验]`、`[记忆]`、`[假设]`、`[直觉]`
- 写测试遵守 R004（三层分层集成优先、期望值来自独立来源、断言只写稳定字段、`TestResult` 加 `?`、真实环境测试闸门 skip）
- 写临时脚本归 `.tools/`（Python PEP 723 头用 `uv run --script`，选库走 R008/R009）
- 文档义务表：新需求澄清完 PRD/REQ 登记；目标立项起 REQ 与 TODO；选型完成 S 文档加索引；改源码同步 README 与 guide；写测试同步测试规范；写脚本同步 `.tools/README.md`；踩坑当场记档（纪律类立 ADR）；方案达成回填 trace 与 GOAL 历史行；每次提交 diary 记钩子；发布后 CHANGELOG 封版加 herdr 知会 ohmycloud（D29）；文档结构变更跑断链回归

## Must not

- 禁止把「没验证」写成「已验证」、断言不标六态、猜测冒充结论
- 禁止跳过定位直接改、只修表象不回写体系、修完不跑验证
- 禁止只改代码不落文档、改了文档不更新索引
- 禁止现成库能完成时从零实现、引入冷门实验 crate、跳过对线直推
- 禁止标题带括号、口号或破折号；整段混杂不成树
- 禁止重言式断言、测试塞 `mod tests{}`、默认 mock、计时进断言、只测 happy path
- 禁止脚本散落、网页当选型接口、sed 批改中文与反斜杠路径（用 .tools/md-replace.py）
- Windows 禁止默认 `powershell.exe` 5.1、无 BOM 中文 ps1 给 5.1 读（用 PowerShell 7 `pwsh`）

## Read first

1. 本文件（五节合同）
2. `PRD.md`（D01 至 D49 冻结决策索引）与 `docs/adr/README.md`（ADR-0001 起现行决策）
3. `docs/requirements/README.md`（REQ 索引；`TODO.md` 为历史任务档案）
4. `README.md`（项目简介与命令；`PLAN.md` 为历史规划档案）
5. `llms.txt`（agent 检索面：读序与代码文件位置；ADR-0001 批三起 INDEX 退役）
6. 细则权威：数据模式 R001；清单标准 R015；测试 R004；选型 R005；协调 R014；元规范 G001 至 G004；错误模式查 ADR-0002 至 ADR-0006（原 M 系列并入，M0xx 编号附录内可检）
7. `ROADMAP.md` / `CHANGELOG.md` 查阶段与历史；`docs/diary/` 当天钩子

## 环境

- 平台矩阵：Windows（PowerShell 7）/ Linux / macOS / WSL（平台常规 shell）；仓库 `D:\ark_rs`（GitHub raystyle/ark_rs）；EnvRoot `D:\ohmyenv`（POSIX `~/.local/share/ohmyenv`；`--env-root` / `ARK_ROOT` 读回 `OHMYENV_ROOT` 可覆盖）
- 编码：Markdown 与 Rust 源码 UTF-8；兼容 5.1 的 ps1 用 UTF-8 BOM
- 验收与运维脚本统一载体 pwsh（五端 7.6.6；非登录 shell 场景带 PATH 兜底；口径见 dev-evo env-platform.md 第十一节）
- 版本载体唯一权威：Cargo.toml（Version 加 InformationalVersion 血统后缀），载体外版本号即第二真相须清理；semver 判据写封版 REQ（dev-evo flow-release 第七节）
- 分支模型：GitHub Flow 单干变体（直推 main 为基线，2026-09-08 裁）；并行会话或危险大改开短命分支，验证后 squash 进 main 并删
- 门禁：dev-evo check.py（PE-01 至 PE-12）加本仓四件套（rumdl 加 md 三扫描）并存
- 全平台直测：四端测试验收在局（2026-09-16），lan-mac 与 lan-ubuntu 与 lan-linux mesh 随时随地；WSL 到宿主恒走 127.0.0.1 回环 ssh 加 interop 直调，不走宿主 mesh IP（口径全文见 dev-evo env-platform.md 第十节）；各端归 ohmycloud 舰队管理，装拆前对齐；验收按需向 ohmycloud 总台要端点支撑，结论 conclusion 自取；多仓飞轮协作协议见 dev-evo flow-flywheel.md（本仓派单回执实践即其实证源）
- 当前阶段：v1.2.3 已发（舰队对线修复批加 dev-evo 治理对齐批，D49 与 bug4 在内）；Unreleased 窗空；D41 ome 兼容面与 D46/D47/D48 fork 分发链均已闭环
