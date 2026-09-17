# CHANGELOG

> 版本里程碑。SemVer `vMAJOR.MINOR.PATCH`。

## [Unreleased]

v1.3.0 封版收尾批补审（herdr codex 评审闸门，2026-09-17 两轮 CONFIRM）。

- **aidoc 版本面漂移根治**（补审 F1，v1.2.3/v1.3.0 二犯）：封版滚版本未再生成 aidoc 四件投影的漂移补滚至 1.3.0；CI 文档门禁新增 `cargo aidoc --check --strict` 岗（linux 岗）；REQ-0006 封版件判据补 aidoc 同提交项。
- **镜像 stable 段空转护栏与补推**（补审 F2）：tag run 的 mirror-r2 岗先于 release 发布跑、draft 窗口三 skip 零上传仍绿，致 ark/stable 段漏切 1.3.0（镜像 fallback 面在此期间仍解析 1.2.3）；seed.py ark-stable skip 面护栏：部分 skip 落 WARN 列名、零上传红灯拦（三段报数；ARK_SEED_ALLOW_SKIP=1 豁免旧 tag 窗口期重灌）；stable 段三边车 dispatch 补推（run 35180668359）。附 G 批四条文档收口（aria2 三平台口径、REQ-0006 判据形态、AGENTS D46 口径、diary 索引欠账）。
- **build-release 标准对齐批一播种面**（REQ-0007，总台核准差4 裁 a）：新 r2-seed workflow（release published 触发加 dispatch tag 补推口加 prerelease 过滤）双段同灌收归自家：版本段 copy 加 immutable 长缓存头、stable 段 sync 清旧带排除清单保 D46 msvc 回退件一窗（rclone 语义实证：排除即保护、不带 --delete-excluded）、双段逐名核对齐备红灯；资产齐备轮询闸防 published 先于资产传完竞态；seed.py 段制 sync 化（--ark-stable 与 --ark-dev 同形、失败整体不灌、空源拒 sync）；dispatch run 35187398380 实证双段六件逐名在位。
- **build-release 标准对齐批二发布面**（REQ-0007）：`.tools/release.ps1` 本地三段式发布命令面（tag 锚与工作树洁净闸、发布预检三件照 hst 1b 族规、版本一致性闸、测试闸含 md 四件套与 aidoc、linux 本职与 win-gnu 交叉本地构建、lan-mac 实机腿、逐件 .sha256 边车、跨实机冒烟逐字对、draft 挂件传齐后 edit --draft=false --latest 直发）；build.yml 撤 v* 正式构建岗与 stable 灌段留 dev 轻岗豁免（总台裁同 hst）；.gitignore 收 dist。批三包形专项立项 REQ-0008（动工前与总台对线 asset 形）。
- **issue 命令集成**（REQ-0009，对齐 ohmycloud REQ-057 契约）：`ark issue new "<标题>"` 一键提交到统一入口 issues.ohmygh.com（自动带 tool=ark 与版本/平台/host；host 取值梯 HOSTNAME 加 COMPUTERNAME 加 hostname 子命令兜底），`ark issue list [--tool] [--status] [--limit]` 与 `ark issue show <id>` 读面；catalog 前早期派发（无清单环境可反馈）；契约校验 UTF-16 语义对齐（emoji 计 2、截断不劈代理对）；ureq 错误面分诊（429 限速与 400 校验带服务端文案，不误报不可达）；`--timeout 0` 不设整体超时；AGENTS 入遇缺陷一键反馈纪律、R013 冻结表补 issue 行；实弹 #7（抓 host 缺陷）与 #8（AI-LAB）。

## [1.3.0] - 2026-09-16

minor：D50 skill 面撤除批（命令面行为变化取 minor，判据与验收入壳见 REQ-0006）。

- **skill 面撤除与 `--llms` 唯一发现通道**（D50/ADR-0007，REQ-0005）：`ark skill` 子命令、根 SKILL.md 静态件与 D25 自适应渲染退役；`ark --llms` 头部新增何时用与下载两行纪律行（一律走 ark、幂等检测安装、镜像优先回落官方、有锚必校验），成为唯一 agent 发现通道；`ark init` 幂等清理数据目录旧 SKILL.md（remove_legacy_skill）；catalog guide 四字段（desc 与 guide_env 与 guide_dirs 与 guide_notes）转数据面保留，schema 与真源（云端 ohmycloud）不动。
- **对线修正批**（herdr codex 两轮终审 CONFIRM）：manifest 表行裸管道转义统一（pin 与 self update 行对齐 catalog 行口径）；remove_legacy_skill 换 symlink_metadata 三态（NotFound 静默、其余错误上抛，顺修悬空符号链接漏删旧漏）；抽 remove_legacy_skill_at 可测核加三态单测；manifest 头行去计数（摘静态数字，计数权威归云端 catalog）。

## [1.2.3] - 2026-09-16

patch：舰队对线修复批加 dev-evo 治理对齐批（update 漂移三态、逗号串、status 口径、temp PATH 闸、契约注释 89 处与三 clippy lint、aidoc 投影、dev-evo 体系全量迁移）。

- **update 漂移收口**（D49，ohmycloud 舰队分型对线实锤）：`ark update` 的 skip 判据从「云端无新版（resolve==pin）」扩为锁定漂移三态：本机一致才 skip；落后或未装真装 pin 版（旧判据在此 skip，挡住一票 installed 落后 pin 的真装机）；领先如实报并指数据面滚锁。status 的 HINT 文案「版本落后锁定」改「版本与锁定不一致」（判据为双向不等）。尾修 bug4：工具参数收逗号串（status HINT 的 join 逗号串此前整串被当单名拒收，自产提示不可执行；cat.select 一处改，update/install/query/pin/heal 统一，保序去重滤空段）。尾二修：status 的 drift 判据从字符串等值统一为 pin_drift 数值段口径（git for windows 的 .windows.N 后缀形态数值等不再恒列）；npm-tgz 类条目不进 HINT（版本真源在 npm registry，端上自管）。
- **临时 envroot 的 PATH 注册闸**（ohmycloud 舰队回执报障：lan-win 注册表沉淀 Temp 下 zig 与 jq 多条）：`add_user_path` 拒绝系统临时目录下的注册面（install 与 rustup 与 selfdeploy 全调用面一处闸），探测与沙盒装的 Temp 段不再进用户持久 PATH；profile 注册语义的集成测试沙盒挪出系统 temp（`target/` 下，tempdir 自动清理）。

## [1.2.2] - 2026-09-14

patch 加 minor 面：ome 兼容面收口（D41 过渡窗关闭）与 **D46 Windows 构建切 gnu 交叉编译**（用户裁定摆脱 VC；stable 通道自此 gnu 资产）。

- **ome 兼容面收口**（D41 过渡窗关闭；全舰队 ome 水位清零，2026-09-14 omc 舰队对账回执）：CI 撤 ome-* compat 资产双附与 ome/ 段灌写（seed.py 撤 `--ome-*` 参数族，UA 统一 ark-seed）；selfdeploy 停建 `ome` 别名并在 init 与 self update 顺带清理既有副本（幂等 best-effort，Windows 文件占用留待下次再收，lan-win 残留副本随下次升级自动收）。引擎读面不动：`is_ome_self` 双接受与镜像段 ome/ 读回落保留，历史 release 的 ome-* 资产与桶内残量仍可解析。
- **Windows 构建切 gnu 交叉编译**（D46，用户裁定摆脱 VC）：CI windows msvc 岗退役、ubuntu 交叉岗（mingw-w64）产 `ark-x86_64-pc-windows-gnu.exe`（CRT 静态零 DLL 依赖，ohmycloud 实证）；self update 资产读序三层：gnu 主名先、`ark-x86_64-pc-windows-msvc.exe` 回退名次（stable 段与历史 release 仅剩 msvc 资产的窗口期保供）、ome 兼容名殿后，官方 API 与镜像边车链同构；旧 msvc 二进制对新 gnu 源无回退（升级走 omc catalog 通道重装）；镜像段读序尝试表单测补三层顺序断言，ome 段删桶后锚链失效的两个 gated 用例退役。

## [1.2.1] - 2026-09-13

patch：D45 sync 的 tools seq 消费收口（对岸 CF 三犯根治的端上认领面）。

- catalog sync：current 判定从「sha 同锚」收紧为「sha 同锚**且**云端 seq 不高于已见」（`is_current` 纯函数入测）：锚探测（sha 边车）与 toml 本体拉取异源，CF 残影下边车旧本体新时原判定会丢弃已拉到的新档（zig 数据复验实锤缺口）；对岸基建侧已 `no-cache` 全覆盖重传，本修为端上第二道保险。

## [1.2.0] - 2026-09-13

minor：**D43 zig 版本去锁**（用户裁定「不再锁定 zig 版本」）。

- zig 版本去锁（D43，用户裁定「不再锁定 zig 版本」）：resolve 分支 a 泛化 ziglang index 形态（顶层版本键滤 master 取 semver 最大、per-target tarball 直取、`shasum` 官方直值锚进 `Resolution.official_sha256`，checksum 官方链最前）；无 pin 条目默认 latest；布局字段 `{version}` 占位（探测 glob 取 semver 最大在位版本、装后验证与 PATH 注册按解析版本定版）；`cdn_url` 模板退役。真机实证：ziglang.org index 解析 latest 0.16.0、镜像桶命中（官方 sha 锚校验）、zip-dir 占位布局安装、幂等二连 skip、update 同版 skip。

## [1.1.1] - 2026-09-13

patch：**D44 下载链反转镜像优先**（用户裁定「ark 安装默认走 ohmygh，其他官方渠道是兜底」）加 go 代理语义键补位。

- download 反转（D44）：两条链（主资产与 evergreen 边车）反转为 env.ohmygh.com 镜像**单次快速首试**（不退避不 curl：镜像未播该版本属常态，须秒级回落）后走官方完整链兜底（retried 加 curl）；校验锚语义不变（pin/边车/digest 照常，锚不符视同镜像失败回落，CF 陈旧对象被锚拦下）；「有锚才回落」红线改写为「镜像为主、官方兜底、有锚必校验」；覆盖 install/npm-tgz/rustup/vsbuild/selfupdate 全调用面；缓存三分支提取 cache_reuse 共用；错误文案镜像在前官方在后。
- manifest goproxy 语义键（D44 补位，omc go.mirror 节数据先行）：`goproxy = "https://goproxy.cn,direct"` 行级 upsert 落 GOENV 文件（win `%APPDATA%\go\env`、POSIX `~/.config/go/env`，即 `go env -w` 持久位，直写不依赖 go 二进制在位），配套 GOSUMDB=sum.golang.google.cn，用户键（GOTOOLCHAIN 等）逐字保留；lint 与 fixtures 补 go 节样例。

## [1.1.0] - 2026-09-13

minor：**D42 运行时源中国镜像统一落 manifest**（用户重投裁定 + 补充裁定）。manifest DSL 扩 mirror 节（数据面声明、引擎落源，不散各端脚本）；rust 接管扩 POSIX。对线 codex 三轮 CONFIRM（六结论全采纳 + 复审残留 + 确认轮收口 env_set 注入面）。omc 数据面四件同批就绪（mirror 三节、rust POSIX 字段、镜像桶 rustup-init、对岸 lint 谓词）。

- 仓再更名对齐：GitHub 侧 ark-rs 已更名 ark_rs（2026-09-13，旧名 301 重定向在读），`REPO` 常量与 seed 流水仓引用、remote、活文档（README clone 与 cd、AGENTS、SKILL、PLAN）统一切新名；镜像段与资产名（`ark/`、`ome/`、`ark-*`、`ome-*`）与仓名解耦不动。旧二进制直连 301 后 Bearer 被 ureq 默认策略丢弃仅回落匿名配额（公开仓 200），gh 回退与镜像段兜底不断源（codex 对线实证）。
- manifest mirror 节（D42 运行时源中国镜像统一落 manifest）：L1 `mirror` 节引擎落码（`env`/`env_unset`/`npm_registry`/`bunfig_registry`/`uv_index`/`pip_index`/`cargo_config` 七键，值数据面声明、落点与合并语义引擎按类型实现；schema 不升，旧引擎静默忽略前向兼容）；接线 `apply_manifest_primitives`（env_set 后、post_install 前，幂等分支与早退通道共用，存量端升级即得）；npmrc 行级 upsert 保留认证行、bunfig 尾斜杠等价比对、uv/pip 落各平台原生发现位（win `%APPDATA%` 实证 uv 只认此处不认 `~/.config/uv`）、cargo config 按 CARGO_HOME 解析序内容比对；platform 增 `remove_user_env_var`（win 注册表删值 / POSIX env 块摘行，块空整块收口）；bunfig 写语义收编 manifest 单一权威（heal_bunfig 委托）；catalog_lint 增 mirror 面（空节应省略）；fixtures 增 fnm/uv/bun 三节基准形态（omc 数据面抄写基准）。Windows 真机实证：四键终态（npmrc/bunfig/uv.toml/pip.ini）加镜像 env 加 env_unset 撤旧全链幂等绿，`npm config get registry` 读出 npmmirror。
- rust 接管扩 POSIX（D42）：`rustup.rs` 拆平台双形态（Windows EnvRoot 重定位模型零变化；POSIX 系统标准位 `~/.rustup`/`~/.cargo` 不重定位），rsproxy rustup-init 引导（evergreen 边车锚、chmod 755、host 三元组自检）、持久 RUSTUP_DIST_SERVER/RUSTUP_UPDATE_ROOT（POSIX 走 profile env 块即 shell rc）、cargo config rsproxy 全量形态（≥3 处 rsproxy 关键标志对齐 wsl 总台 A13 判据）、PATH `~/.cargo/bin`；verify 增 POSIX dev-rust 维度（`~/.cargo/bin/rustc` 加 `~/.rustup/toolchains`）、heal dev-rust 键开 POSIX。R016 增 mirror 节规范；待 omc 数据面：manifest mirror 三节、tools.toml rust POSIX 字段、镜像桶 rustup-init POSIX 资产名。

## [1.0.0] - 2026-09-12

major：更名 **Ark（Agent Runtime Kit）**（D41，用户定夺）。命令 `ark`；旧调用面全线读回兼容；存量供给双写双附过渡。计划经 codex 对线三轮 CONFIRM，四阶段（A 身份核心、B 分发链、C 自举与存量兼容、D 文档发版）逐批对线后落。

- 更名（D41）：crate 与 CLI 切 `ark`，环境变量族 `ARK_*` 主名读回 `OHMYENV_ROOT`/`OME_*` 旧名（同设主名优先）；元数据目录切 `ark` 主名（旧 `ohmyenv` 在而新未建时读回旧位，init 与 self update 搬迁七件套：双清单本体加双 minisig 加双 .seq 加 .last-sync，旧位只读保留）；分发链切 `raystyle/ark-rs` 与资产主名 `ark-*`（兼容名 `ome-*` 同 release 双附），镜像段读序 `ark/` 先 `ome/` 回落、CI 双写双段，self update REPO 与 git 通道产物名随 `CARGO_PKG_NAME` 派生；云端清单键 `ark/catalog/` 主键先、`ome/catalog/` 回落（短探定键，404 不吃重试链）；profile 全写入面（PATH 块、env 块、fnm 钩子）双读迁移旧块退役；init 接管旧 ome 部署位（`Programs\ome` 收口）并留 `ome` 部署位同目录别名（init 与 self update 重建）；doctor 探针与源码常量同源；文档全量更名，正文残留仅限兼容口径与历史记录。
- 兼容红线（停 ome/ 面判据同为存量机水位清零）：镜像 `ome/` 段与 `ome-*` 资产名过渡期双写不断供；`ome-self` 与 `ark-self` extract 双接受（数据面条目更名单方可回退）；EnvRoot 物理目录与泊位语义不动；签名密钥位保留 `~/.config/ome/`（密钥材料零迁移，R015 注明）。
- 验收（D40 端到端终验，omc manifest seq 4 双步 fnm 供给）：WSL 拆链重装全绿：fnm post_install 两步（fnm install 加 fnm default）重建 aliases/default；omc 0.3.2 端到端（fnm 静态位 npm、直链锚静态位、运行绿）；旗标双源纠偏（--default 不存在、--use 是版本解析策略非设默认、fnm default 正路）。
- 验收（D40 端到端，omc manifest seq 3 双层）：WSL 拆链重装全通：seq 门拉通（tools seq 2 加 manifest seq 3 各自记）、omc 0.3.2 端到端装成（fnm 静态位解析、npm 直装、直链锚静态位、静态位运行绿）；引擎 post_install 失败降级链真机实证（fnm 参数错降 WARN 不拦安装、尾行退出码报告、可重试提示）。
- D40 快审修正（codex 十轮，`bc70316` 之上）：seq 门原来只挂显式 `sync` 与自举，**默认自动路径（`auto_refresh`）没挂门也没记已见基线**：本机同状态实证：旧行为接受「已见 99 vs 云端 2」的回滚内容并打印 `[OK] catalog 已刷新`，新行为 `[WARN] tools.toml 回滚重放拒收` 且内容原封不动、退避标记防刷屏。补齐三处：`auto_refresh` 更新分支过门 + 落位后 `write_seen_seq`、同锚分支把在位件 seq 补记成基线（否则记录恒 0，门形同虚设）、`bootstrap_catalog` 无基线可比（鸡生蛋）但也把拉到的 seq 记成基线；`seq_gate` 出路提示的 `.{what}.*.seq` 改正为实际文件名 `.{what}.seq`；补 `seq_gate`/`toplevel_seq`/已见记录读写的单测（缺/负/非整数视 0、低拒含出路、等过、高过、坏记录按 0、在位件补记、无 seq 不建记录）。
- catalog（D40 回滚重放防护，S006 候选 B 引擎侧落地）：tools.toml 与 manifest.toml 各自独立 seq 门：拉取链在 sha 加解析加验签三重门后读顶层 seq（缺省 0 兼容首发前件），低于已见即拒收报错不降级（防「重放旧但签名有效的清单对」与镜像桶回滚），等于幂等重放、大于即收并记；已见 seq 存各清单同目录 `.<名>.seq`（tools 与 manifest 各一，同 R016 同目录原则）；omc 数据面已先行落地（签发取 R2 本体现值 +1，三轮 dispatch 实证 seq 1 -> 2）。WSL 真机实证：sync 拉通记 seq 2、伪降已见复现拒收报错。

## [0.4.2] - 2026-09-11

patch：S017 O7（npm-tgz 直链字面自引用悬空修复）。

- install（S017 O7）：npm-tgz 早退分支直链改锚定重解析后的绝对路径：外层 exe_path 是 fnm 注入前解析的裸名，symlink 建出字面自引用（~/.local/bin/omc -> omc 悬空，非 login shell 无 npm 环境实测）；现于 install_npm_tgz 注入静态位后重解析锚定。WSL 精确复现（PATH 无 npm）实证直链锚静态位绝对路径。

## [0.4.1] - 2026-09-11

patch：S017 六项收口（ohmycloud lan-linux 全量 install 实测驱动，codex 九轮对线）。

- S017 O6 快审修正（codex 九轮，`00bdc08` 之上）：原「fnm 静态位无条件优先」会把**用户既定 node 生态抢走**（装了 fnm 但默认用 Homebrew/system node 的机器，包里会落到 fnm 版本目录，macOS 上 ~/.local/bin 又常不在 PATH，装成功却不可见），改为**条件优先**：`which npm` 命中的若是会话级 shim（`is_session_shim` 认 `fnm_multishells` 路径段，跨平台纯函数）才让静态位插队，命中真安装则维持 PATH 优先（不抢生态环境）；静态位不可用时回落 PATH、再回落裸名报错。顺补两处：`fnm_node_bin_in` 的 `aliases/default` 分支要求目标版本真含 npm，否则落回版本扫描（残缺默认别名不再断路）；新增「会话级 shim 判据」用例（含 Windows 反斜杠形态与 homebrew 反例）。
- install（S017 O6 收尾）：npm 型 fnm 解析改静态位优先锚定：交互 shell 的 fnm hook 把会话级 multishell 目录（/run/user/<uid>/fnm_multishells/...）前置 PATH，which 命中的 npm/exe 是会话目录（注销即回收），直链锚上去必悬空；现 fnm 静态位（node-versions/<v>/installation/bin）存在即优先注入与锚定，multishell 只做执行期环境。WSL 交互态实证直链锚静态位。
- S017 快审修正（codex 八轮，`3c6f0a4` 之上）：`fnm_node_bin` 无别名时原取「字典序最高」版本目录，**v9.9.9 会压过 v24.20.0**（本机字符串序实证 True），改用 `resolve` 的数值段助手（`semver_cmp` 提 `pub(crate)` 加新增 `version_key` 容忍前导 v），解析核心抽 `fnm_node_bin_in(base)` 便于测并补用例（别名优先、无别名取含 npm 的数值最大、无 npm 的版本跳过）；测试隔离开关从「只管 PATH 注册」扩成**用户面写入总闸门**（`platform::user_env_write_blocked`，覆盖 add_user_path、set_user_env_var、ensure_profile_hook、用户 bin 直链四面，只守一面会让沙盒测试从别的门漏进真实环境）；再删 `.tools/tmp_fix.py`（二犯回仓且内容是旧稿，落 M024）。
- install（S017 三项，ohmycloud lan-linux 全量 install 实测）：O3 npm 型 fnm 解析双面：npm 不在 PATH 时经 fnm（aliases/default 优先、否则最高版本）解析 node bin，注入子进程与自身进程 PATH（装后探测同链生效），fnm 装后写 profile `eval "$(fnm env)"` 钩子（非交互由进程内解析治本）；WSL 剥 fnm PATH 复现 omc 态全链实证（解析 npm 启动 npm install exe 定位绝对路径）。O5 测试注册面隔离：`OME_TEST_NO_PATH_REG=1` 跳过用户 PATH 写（M002 沙盒漏写面同型根治），tests 三处 spawn 注入，不再污染真实 HKCU/profile。O2 rmux 布局与 O4 退出码见回执（数据面提案与引擎语义确认，非本仓改码）。
- POSIX 可发现性兜底补审修正（codex 七轮，`173fb0a` 之上）：直链判据原来只看「悬空」，**有效但指向旧 target 的链接会一直留着**（版本目录型布局如 zig 升级后 ~/.local/bin 直链仍指旧版目录，PATH 顺序在前即陈旧遮蔽），改为比对 target：已指对跳过、指向别处（含悬空）先删再建、落点是真文件（用户自装）绝不覆盖、exe 本就落该目录时不建自指链接；落点语义抽成可测函数 `link_into_user_bin` 并补 POSIX 用例六态（首建/幂等/悬空重指/旧 target 重指/真文件不动/同路径跳过）。另删 `.tools/tmp_fix.py`（一次性行内改写脚本，违 .tools 的命名、PEP 723 头与清单登记三规，且属 M018/M020 同型隐患）。
- install（ohmycloud lan-linux 实测坑修复）：POSIX 嵌套布局用户 bin 直链兜底 `ensure_user_bin_link`：skip 分支注册的 PATH 目录在非交互 shell（omc hostExec）不加载 profile 形同虚设，~/.local/bin 直链才是非交互可达的 XDG 基建；幂等含悬空链接清理（先删再建），挂幂等分支加主链加两早退通道共四处 configure 块。WSL 剥离 PATH 复现 ohmycloud 精确路径实证（MISSING 态 install skipped 自动补链，command -v 命中）。
- manifest 共识② 快审修正（codex 六轮，`8b88074` 之上）：原 helper 只接 shims 加 post_install，**env_set 在 uv-git/npm-tgz 首次装仍被跳过**（`ensure_user_env_overrides` 只挂主链两处，两通道早退拿不到），且同一逻辑当时有**三份并行**（幂等分支与成功尾各一份走硬错、新 helper 一份降 WARN，语义已分叉）。改为全链唯一实现 `apply_manifest_primitives`（env_set 加 shims 加 post_install，主链两处与两通道共用，语义与主链一致：L1 硬错、post_install 降 WARN）；通道侧调用点移到 `install_tool` 的调用处，通道函数签名不再为 manifest 增参（clippy 8/7 超参警告清掉）；shims 落点加「非绝对路径即跳过加 WARN」防线（防裸名 exe 退化成按 CWD 拼相对路径）；R016 补 npm-tgz 落点漂移注（fnm multishell 每 shell 一目录、POSIX 随 node 版本、Windows 无 .exe 源故实际 POSIX-only）。
- manifest（D39 共识②）：应用点上提到 uv-git 与 npm-tgz 早退通道（`apply_manifest_primitives` 与主链同款 WARN 降级，两函数签名穿 ms）：omc/browser-harness 等 npm-tgz 族与 uv-git 族的 manifest 节（shims 与 post_install）不再被 return 跳过；env_set 由既有 configure 块覆盖不变。
- manifest 共识①③④ 快审修正（codex 五轮，`4eb8d11` 之上）：④ 原实现顺序无效：先 `kill` 加 `wait` 之后再 `taskkill /T`，父进程已死致 taskkill 报 not found、`cmd /c` 孙进程存活（本机 A/B 实证：kill 先则无效、taskkill 先则整棵清），改为**先 taskkill /T 再 kill 加 wait**，并给超时用例补「无残留 ping」断言（首版断言大小写敏感，tasklist 输出 `PING.EXE` 会在旧顺序下假绿，已改大小写不敏感并反向验证过判别力）；M021 落 M102。
- manifest（D39 共识①③④落地）：auto_refresh 三路径（fresh 加 InSync 加 Updated）补 manifest 拉取，fresh 判据用 manifest 文件 mtime 对 TTL（无第二标记文件，缺失视为首拉过期）：tools 锚不变而 manifest 已换的端上跟进时效缺口闭合（真机实证）；post_install 失败降 WARN 不拦安装收尾、幂等分支同样执行（重试路径达成）；win 超时 kill 后补 taskkill /T /F 杀进程树（cmd /c 孙进程存活）。
- manifest 配置真空面可见性（R016 六节新鲜度门落地）：`ome catalog status` 增 manifest 面八键（manifest_path/present/local_sha256/cloud_sha256/synced/age_secs/signature/cloud_error，与 catalog 面同源探活，云端整体不可达时复用 catalog 的错不重复探），install 与 update 在 manifest 缺位时装前一行 WARN；manifest 路径推导收敛为 `manifest::path_for` 单点（sync 落位、status 诊断、install 消费、CLI 告警共用）。
- manifest 双轨收口快审修正（codex 四轮，`057e788` 之上）：盲切删 `ensure_bunx_shim` 时**连带删掉 `zip解压_含目录与嵌套文件` 用例**（lib 用例 107 掉到 103，与「删三测试」对不上账）已补回并复核符号表；大输出用例改「读 1MiB 大文件」生成器（原 shell 循环在负载机上从 0.2s 漂到 21s，20s 窗口下假红，本机已复现）；`set_executable_for_tool` 的 bun 特判删除（三端 bun 资产实测只含 `bun`/`bun.exe`，从无 bunx，POSIX 别名是指向 bun 的符号链接，chmod 目标已覆盖）。顺记 M018（盲切连带删用例）与 M019（URL 双 scheme 且被静默跳过掩盖）。
- manifest（D39 双轨收口）：omc 数据面首发（manifest.toml 三节加 lint 三面入其 CI）后 ome 撤内建：`ensure_user_env_overrides` 内建遥测表与 `extract::ensure_bunx_shim`（含 bunx_cmd_content 与三测试）删除，用户级配置与别名唯一来源是 manifest 节（无节零动作）；顺修 `sync_manifest_if_present` 三处 URL 双 scheme 笔误（MIRROR_BASE 已含 scheme）；验收：云端三件套四门拉取落位、catalog-sign 独立验签 ok、install pwsh/dotnet/bun manifest 优先加载实证、WSL 加本机双绿。
- manifest（D39，R016 v0.2）：安装配置部署逻辑数据化第一波：`src/manifest.rs` 引擎（schema_version 拒载红线、L1 env_set/shims 三平台原语、L2 受控命令 argv 数组带 300s 超时与失败尾行退出码报告）；install 双轨接线（manifest 节优先、无节内建回退，遥测键与 bunx 为迁移样例）；catalog sync 扩拉云端 manifest 三件套（同锚同签，未上线 404 静默跳过）；catalog_lint 扩 manifest 面（三键齐备与引用一致性）加 fixtures 样例。标准全文 R016（两件分离同批同签、跨仓分工 omc 维护数据 ome 执行引擎）。
- manifest 三轮对线补审修正（R016）：L2 执行链并发抽干 stdout/stderr（尾窗 64KiB）：原「子进程退出后再读尾行」在输出超管道缓冲（约 64KB）时子进程写阻塞、轮询窗口耗尽被误判 300s 超时（M017 实证）；超时路径补 `wait` 回收。win `shims` 的 `.cmd` 兜底改 `%~dp0` 相对定位（撤掉自造反斜杠转义与绝对路径）。`catalog sync` 的 manifest 拉取改在「catalog 同锚早退」之前且失败只告警不静默吞错（原 `let _ =` 吞错，且同锚时 manifest 永不刷新）。install 双轨修正：shims 原语按原语判定（原按节存在判定会吞掉 bun 内建回退）、shim 落点取 exe 父目录（与 PATH 注册面一致）、catalog 的 `manifest` 字段值（节键）由引擎与 lint 同解析。测试增补：大输出不阻塞、失败尾行退出码、超时杀进程、shim 非拷贝与 `.cmd` 内容、引用不一致红灯（单测 8 加 lint 4）。

## [0.4.0] - 2026-09-11

minor：消费面镜像直装（D38，omc 私有仓入册请求）。

- resolve（D38）：pin 四键齐且 GitHub API 失败（私有仓匿名 404、限流、断网）时回落镜像资产域直拼 URL 过 pin 锚安装（与 D08 同源，回落前移到查询段，锚红线不变：无 pin sha 不回落）；`OME_MIRROR=1` 扩展为解析面强制镜像（pin 驱动真跳过 GitHub API）；gated 真网测增补（OME_MIRROR=1 query 私有仓节断言镜像域 URL）。真机双路径实证（omc v0.3.2 私有仓：自然回落与强制直装）。

## [0.3.1] - 2026-09-10

patch：D37 完全解耦终态（清单数据权威与发布门迁 omc、批 3 撤退、pin 定案；codex 二轮对线修复 POSIX CI 红与七项收口，claude 复验 CI 三平台与本机全绿后封版）。

- 清单（D37 评审对线第二轮修复）：POSIX CI 红修复（`tests/linux_install.rs` 取件源改动态解析，版本期望改由被测二进制自身输出，且文件级 cfg 改为只门控 POSIX 用例让助手全平台参与编译，落 M016 矩阵纪律教训）；`seed.py` 清单源改用到时解析（路线 A 的 `--ome-dev` 与 `--ome-stable` 不再因缺清单失败）；两脚本清单源平台化对齐 `dirs::data_local_dir()`（win `%LOCALAPPDATA%`、mac `~/Library/Application Support`、其余 `$XDG_DATA_HOME`）；self update 的 catalog 刷新改走云端三重门（原 raw 与 git 通道取源随权威件退役已死），自举删官方 raw 兜底改 fail-closed；两枚 `.pyc` 出库并忽略 `__pycache__/`；R001 四.7 与 R015 一/三/四 口径对齐 D37 终态。
- 管辖（D37 完全解耦终态）：清单数据权威与 catalog_lint 发布门迁 ohmycloud（权威 tools.toml 加 vitest 四规则门加 catalog-seed 签名与资产域播种，其仓首发验签全绿）；本仓撤退：权威 `catalog\ools.toml` 退役删除（消费走云端三件套与自举，fixtures 留测试夹具）、seed-mirror.yml 整撤（`.tools/seed.py` 留 `--plan` 对账面，清单源回退仓库件到用户数据副本）、catalog_lint 真仓测随权威件退役。
- update（D37 pin 定案，语义变更）：`ome update` 收窄为拉云端最新安装**不回写锁定**（锁定单源归数据面）；`ome pin` 重定位临时本地锁（sync 云端覆盖优先）；doctor version-drift 明细加「实装超前：update 拉新而云端 pin 未升，向数据面反馈」提示。
- 分发（D36 B 承接收口）：签名与播种运营移交 ohmycloud catalog-seed 流水（拉本仓 catalog pin 源、minisign 签名、推云端三件套；首跑 success 并经本仓验签 signature=valid）；本仓撤退：`CATALOG_SIGNING_KEY` Secret 删除、seed-mirror 签名步移除、seed.py 摘 catalog 本体上传（防双轨竞态出锚对签失配窗口态，seed-mirror 只留软件资产域播种）；软件资产域继定 A 形态（omc 回执）：catalog-seed 扩展接管资产播种，双轨期本仓 seed-mirror 续跑至对方首发对账绿后撤整条只留对账脚本；R015 五、R014 五/六.7 同步。

## [0.3.0] - 2026-09-10

minor：软件清单云端化与签名校验（D32/D33/D34，codex 实现、claude 对线验收共识后封版；三仓水位 omc 0.3.1 / ome 0.3.0 / oma v0.5.2）。

- 清单（D34）：云端清单防 MITM 的非对称校验：新增 minisign（Ed25519）分离签名，公钥内嵌 ome，云端发布 `ome/catalog/tools.toml.minisig`；拉取落位前强校验（sha 边车证传输、签名证来源，任一不过拒收），运行态副本每次加载前巡检（签名不符即错误退出，签名缺失告警；本地 pin 回写会撤签名以免留假凭证），`ome catalog status` 新增 `signature` 与 `pubkey` 字段；签名工具 `.tools/catalog-sign`（keygen / pubkey / sign / verify），私钥只在开发机与 CI 密钥库（GitHub Secret），其他机器只需二进制里的公钥；seed-mirror 流水缺密钥即拒发未签名清单。研究对照见 S006。
- 清单（D34 评审对线小修）：自举路径改镜像优先并强校验签名（官方 raw 仅在镜像不可达时兜底且不验签，如实告警）；落位临时名随目标派生（清单与签名件各用各的 tmp，去掉并发互串隐患）；init 与 self update 同步 catalog 时只在签名与内容不符才撤签名件（内容未变则保留，避免凭空变无签名）；seed-mirror 触发路径加签名工具与流水自身。
- 清单（D33）：软件清单云端化与实时刷新：运行态 catalog 以云端 `env.ohmygh.com/ome/catalog/tools.toml`（`.sha256` 边车即锚，先边车后资产）为权威，新增 `ome catalog [status|sync]` 命令面（status 报解析面/来源/本地与云端锚/年龄/TTL/同步态，sync 立即刷新）；query/install/update/status 等命令在解析面为用户数据副本时按 TTL 自动刷新（默认 24h，`OME_CATALOG_TTL` 秒级可调、0 关；`OME_OFFLINE=1` 关），自动路径单次 5s 探活、失败记退避标记不拖慢命令；仓库 cwd、二进制同级与 `OME_CATALOG` 指定面零干扰。新增软件与 pin 变动自此不必等 ome 发版（受既有解压与下载类型约束，新类型仍须改代码）。
- catalog（D32）：typst 入册（文档排版系统 CLI：compile / watch / init 子命令出 PDF 与图片）：三平台单二进制（win zip 展平，linux/mac tarxz-bin 按叶子名取 typst 落 `~/.local/bin`；mac 取 aarch64）；官方 release 无统一校验清单，pin sha 取 GitHub digest（三平台资产本机下载实测哈希逐字核验），pin v0.15.1；计数 46 改 47。
- 分发（D27 小修，M015）：`seed.py` 的 `.sha256` 边车改临时目录暂存后上传（原直写源资产所在目录，catalog 本体场景即仓库工作树，留未跟踪噪声）；域面对象与键名零变化，`--plan` 输出逐字不变。

## [0.2.1] - 2026-09-10

小版本完美状态同发（D31，#10 清零；三仓对齐水位 omc 0.3.1 / ome 0.2.1）。

- catalog（ohmyenv-rs#10 缺口 2）：codex linux/mac 布局修复：tar 包内 `bin/` 嵌套与 codex-path/codex-resources 多载荷，POSIX 改装入 `~/.local/share/codex` 并注册其 `bin` 到 PATH（与 win 同构）；原直解 `~/.local/bin` 致装后验证必败（lan-pve 实测根因）。
- catalog（ohmyenv-rs#10 缺口 3）：裸端 catalog 自举：四级候选全 miss 时自动拉取落位用户数据目录（官方 raw main 分支优先，镜像 `ome/catalog/tools.toml` 边车锚回落，先边车后资产，解析验证防半截）；`seed.py` 路线 B 增量推 catalog 本体加边车。
- 分发（D30 补遗）：ome/stable 段 CI 直推落地（oma 同型，段名与 self update 通道同名）：build.yml mirror job 增 v* tag 与 workflow_dispatch 双入口，`seed.py --ome-stable` 灌正式段（v0.2.0 已回灌，纠正灾备手推的 win 锚漂移与 darwin 边车缺失）；`self update --stable` 镜像回落与 doctor net 探针同步切 ome/stable；ome/latest 段退役归镜像侧下架（封版拆分欠账收口）。

## [0.2.0] - 2026-09-10

首个正式封版（tag 驱动七步，三仓对齐里程碑 D30 急令当日发布）。

- catalog：browser-harness pin 0.5.1 升 0.6.5 三平台回填（bh 自身升级轮换致实装超前，doctor version-drift FAIL 暴露；GitHub digest 锚逐字核验，tgz 平台无关三平台同 sha）。
- doctor（D30 定稿，**输出契约变更**）：agent 层整层移除（binary/version/locked/drift/token 五字段健康块与 token 凭据探测，AgentHealth/agent_health/token_state 删除），收窄为系统 / 依赖两层；依赖层智能体依赖组保留（install 域单机装态事实，omc 舰队对账数据源）；agent 装态对账归 omc、token 检测归 oma diagnose（单机三态仍走 `ome status`）；R013 doctor 数据块契约同步（json 面 agent 节移除）。
- 定位（D29）：三仓对齐共识：omc agent deploy 全面委托 ome（env 镜像拉二进制加 catalog 推端加 `ome install`）；release 后 herdr 会话知会 ohmycloud 同步 omc tool status 镜像锚（R014 六）；集成优先级裁定（诊断与检测、恢复与治愈、安装部署配置之序）落 ROADMAP。
- catalog（D29）：修正 claude linux 资产 pattern 拼写（x86_64 改 x64；ohmyenv-rs#10 缺口 1：resolve 恒按 pattern 对 release 清单重筛，拼写错致 linux 端 install 解析失败；官方 SHASUMS256 与 pin 三平台 sha 逐字核对一致，镜像双资产 HEAD 200；M014 增补同型）；`tests/catalog_lint.rs` 新增机检「pin 资产名必被同平台 asset_pattern 命中」，拼写族零网络红灯。
- 分发（D27）：种子上传自维护：`.tools/seed.py`（uv 单件）域面 diff 加 rclone 直传 R2；路线 A（build.yml mirror job 灌 ome/dev 与 ome/latest 沙滚段）加路线 B（seed-mirror.yml 每日加 catalog push 触发自动补种）；镜像侧 env-seed 退居灾备对账。
- catalog：修正 lightpanda mac pin digest 错配（0.4.0 双 mac 资产转写取错行，镜像种子器锚校验拒收暴露；M014）。
- catalog（D26）：gitleaks 入册（密钥泄漏扫描，security 类第三员；checksums.txt 统一清单锚，三平台单二进制），计数 45 改 46；win 8.30.1 真机安装绿。
- skill（D25）：`ome skill` 逐工具自适应引导：catalog 增 `desc`/`guide_env`/`guide_dirs`/`guide_notes`（45 工具全量起稿），每工具渲染三态标记、用途、exe 实测、env 键实测（凭据类只显在否）、目录实测（在位与否）与注意事项；修 cmd_skill 落盘被静态版覆盖的既有 bug（自适应文本落盘，静态骨架仅 init 兜底）。
- catalog（D24）：lightpanda 入册（无头浏览器，linux/mac 裸二进制 copy、win 空态；官方无 sums 用 GitHub digest 锚；latest 端点被 nightly 占据升级须显式 `--tag`；探测走 `version` 子命令），计数 44 改 45；WSL 本机先行安装绿（sha 逐字过锚）。
- catalog（D23）：browser-harness 与 reader 重入册（撤 2026-09-07 暂不接管裁），计数 42 改 44。reader 三平台 v0.6.0（`.sha256` 后缀边车锚）；browser-harness 走新 **npm-tgz 安装通道**（tgz 过锚下载后 `npm install -g`，bin bh 进 npm 全局 bin；探测走 PATH 现查，`.cmd` shim 经 `cmd /c` 拉起；需 node 与 npm 在 PATH）。
- catalog：herdr win pin 0.8.2 升 0.9.0（sha 双源核验：GitHub digest 与 ome 缓存实测一致）；盘上二进制待面板外重启随 `ome update herdr` 更换（运行中服务器保持 0.8.2，避免新旧协议错配）；linux/mac pin 仍 0.8.2，镜像 0.8.2 段保留。
- catalog（D22）：rclone 入册（ohmycloud#9 请求，env-seed 上传链切 R2 S3 直传）：三平台节（SHA256SUMS 统一清单锚，win zip 展平、linux/mac zip-bin 单二进制）；toolver 加 rclone 探测正则；win 1.75.1 真机安装 pin 回填（sha 与官方 SHA256SUMS 逐字一致），linux/mac pin 按官方清单回填；工具计数 41 改 42。
- 分发（镜像链加固）：CF 边缘缓存击穿：边车请求带 `?t=` 时间戳每次回源、镜像段资产带 `?v=<锚>`（锚变缓存键变，锚同则缓存对象必与锚一致；五端验收发现 CF 对 R2 覆写不失效，陈旧对象被锚校验拦下后的可用性跟进）。
- 验收（五端全过）：ome 安装加更新五端矩阵（本机 win / lan-win / lan-linux / lan-mac / wsl 裸装）ohmycloud 执行全过；B 面 `OME_MIRROR=1`；终态锚三平台与镜像边车全等。
- 升级（B 面验收开关）：`OME_MIRROR=1` self update 镜像优先（跳官方 API 直取边车锚，锚语义不变），供五端断源验收与未来默认切自建过渡。
- 分发（D08 第二批收官，ohmyenv-rs#7 关单）：ohmycloud#9 补种 `ome/dev` 段后补 self update dev 通道断源锚链用例（锚取 ome/dev 边车，产物 sha 逐字一致），`OME_TEST_MIRROR` 7/7 绿。
- 分发（D08 第二批，ohmyenv-rs#7）：evergreen 引导器（rust / vsbuild）官方失败回落镜像 latest 段，`.sha256` 边车即信任锚（先边车后资产，边车取不到拒绝无校验下载）；selfupdate 边车权威统一至 download 层；`OME_TEST_MIRROR` 6/6 绿（zoxide linux 回归、ffmpeg HEAD 在位）。
- 分发（D21）：向 ohmycloud 派三面对齐 ISSUE（#8）：ome 自身与在管软件的分发、更新、安装。
- catalog：回填 zoxide / ffmpeg `linux_sha256`（官方资产哈希与 GitHub digest 一致）。
- 分发（D20）：种子清单用 GitHub ISSUE 向 ohmycloud 派任务并对齐（R014；ohmycloud#5 差集闭环，OME_TEST_MIRROR 绿）。
- 定位（D19）：ohmycloud 是资源分发基建兄弟仓（env.ohmygh.com）；ome 官方失败回落该镜像。
- 定位（D18）：独立仓，不再依赖 ohmypwsh；真机对照为本机 catalog 与 EnvRoot；本机命令面可跑。
- 命令面（D17）：去掉 `package`。
- 命令面：`install` / `update` / `query` / `pin` / `heal` / `verify` 省略名即全量。
- 命令面（D16）：去掉 `daily`；升级一律 `ome update`。
- 命令面（D15）：去掉 `deploy`；`ome install` 一次完成下载、PATH、注册表与配置；`update` 为 install 到最新。
- 入册 ffmpeg（D14）：Windows GyanD/codexffmpeg essentials 9.0.1，Linux BtbN n9.0 GPL 静态，mac ARM 空态（官方 evermeet 仅 Intel）。
- 审查缺陷：pin sha 必须同 tag；install 不污染旧 pin sha；下载 `.part` 提交；CDN 显式版本重算 tag；self update 镜像按通道、替换部署位、catalog 同步保留 pin。
- doctor：probe-fail 看文件存在；死链按路径分量；rust 走 EnvRoot rustup；D13 5s 总超时；网络 WARN 不单独 degraded。
- POSIX PATH 标记块多目录；mac 只认 `mac_exe`；verify localbin16 去掉 vault。
- 立项：自 ohmypwsh `ohmyenv.ps1` 剥离本机 Windows 环境部署管理为 Rust CLI。
- 八命令落地：query / install / deploy / update / pin / status / daily / self-deploy。
- catalog 数据层：`.tools\import-catalog.ps1` 生成 `catalog\tools.toml`（29 工具唯一 pin 源，合并规则对齐 Get-EnvLock）。
- incurs 选型研究（S001）：不迁移，吸收机器可读错误、单一渲染层、帮助元数据化三模式。
- 真机对齐：status 29/29 与 ohmyenv.ps1 逐项一致，query 同 tag（OME_TEST_REAL 闸门测试）。
- 文档体系自 ohmyagents 平移：AGENTS 四段式、G001-G003、R001/R004/R005/R008/R009、.tools md 三件套。
- 开发接管准备：R010 落定 Linux 开发主机的工具链准备、构建验证、平台门控现状与两端验证分工。
- 管理域收窄：智能体（codex/claude/grok）安装剥离出 ome（归 ohmyagents/ohmypwsh），工具名录 29 降至 26，转换器显式剔除。
- Linux 本机部署支持：platform.rs 平台抽象层、catalog linux_* 字段、package 命令、tests/linux_install.rs。
- 开发主机接管：R010（WSL）归档，R011（mac）落定。
- 新增工具 reader（raystyle/reader_rs v0.1.0）：ome 本地名录首个自增工具（27 个），转换器保留本地节；真机 deploy 验证 locked=installed=0.1.0、path=true。
- 承接 ohmypwsh 完整迁移：M0 数据主权（psd1 单向回流、pin 平台分列、转换器改只校验）；M2 四端齐（WSL/lan-linux/lan-win/mac，package 打包下发）；M3 `ome verify` 部署域验收（维度注册表、流式输出）；M4 `ome heal` 自愈移植（42 键四类归宿）。
- `ome self update` 三通道（dev 滚动 / stable 正式版 / git 源码）与 CI 双通道路由，五端自服务闭环。
- Agent 友好 IO（S003）：全局三格式渲染层（kv/json/jsonl）、结构化错误 stderr 单行 JSON、字段序稳定。
- 新接管：rust（rustup.rs 建模，rsproxy 镜像与 EnvRoot 重定位）、Docker Engine（服务注册与 compose 插件）、Windows OpenSSH（MSI 型）、VS Build Tools（evergreen 引导器）、go/zig（cdn 直链）。
- `ome doctor` 部署异常诊断九项；self-deploy 改名 init（兼容别名）。
- 七类 taxonomy 定稿，37 工具（含 ome 自管条目；oma/omcf 预留）。
- 写作规范 G001 v2（四类禁字符硬禁令）与文档门禁四件套（本地与 CI 同口径）。
- 文档体系对齐 project-evo 骨架：补 PRD（D 编号全量追溯）、docs\proven、ROADMAP、AGENTS 文档义务表，INDEX 以磁盘为唯一事实源对账重整（evo check 13 项全绿）。
