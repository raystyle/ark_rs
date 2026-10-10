# R016：云端清单与 manifest 标准

> 角色：标准（**v0.2 正式版**，2026-09-11 定稿，D39；v0.1 草案经 omc 评审三待定点全裁 CONFIRM）。ark 云端软件**清单（catalog）**与**安装 manifest** 的数据标准：字段与 DSL 规范、信任与版本化、跨仓执行分工。**制定在 ark 仓（消费引擎定义契约；D41 前名 ome），维护与数据运营在 omc（云端管理全平台软件清单、分发、部署逻辑，用户裁 2026-09-11）**；流程面见 R015，研究依据见 S007。字段语义底册吸收 R001。

## 一、总则

1. **两件分离**：catalog（`tools.toml`：资产、版本、pin、探测）与 manifest（`manifest.toml`：安装配置部署逻辑）关注点分离、独立演进、同批同签发布（云端 `ark/catalog/`（D41 主键，兼容读 `ome/catalog/`）下三件套扩为 catalog 与 manifest 各带边车加 `.minisig`，签发由 omc catalog-seed 流水）。
2. **全平台第一约束**（S007 五节四原则）：原语优先（三平台语义引擎统一保证）、受控命令三键齐备 lint、平台不适用容忍（M003 空态语义）、平台矩阵即验收（R004 三.6）。
3. **信任链复用 D34**：minisign 端到端签名，签名清单即信；lint（omc 侧 vitest 与 ark 侧 catalog_lint 同规则）装前静态可验。
4. **版本化**：两件各带 `schema_version` 字段（`1` 起）；引擎遇高版本拒载并提示升级 ark（前进兼容红线：不猜语义）。
5. **刷新语义**：两件独立演进，catalog 锚未变（含 `--force` 同步命中同锚）也必须拉 manifest，否则 omc 单方面上线 manifest 后台端永远拿不到；云端无 manifest（404）或拉取失败只告警不拦目录同步，保留本地版本走内建回退（双轨期供给不断），**撤内建分支前必须补 manifest 新鲜度门**（拒绝在陈旧 manifest 上宣称已迁移）。

## 二、A 层：catalog schema

字段语义以 R001 为底册（吸收为标准实现注记），标准面新增：

| 域 | 字段 | 规范 |
| --- | --- | --- |
| 元 | `schema_version` | 正整数；缺省视 `1` |
| 资产解析 | `repo`/`asset_pattern`/`sums_*`/`cdn_*`/`version_pattern` 平台族 | 同 R001；pattern 必命中 pin 资产名（lint 规则） |
| pin | `tag`/`version`/`asset`/`sha256` 平台族 | 四键同 tag；sha 64 hex 大写；锁定单源数据面（D37） |
| 布局 | `extract`/`dir`/`bin`/`exe`/`extra_bins` 平台族 | 同 R001 九分派 |
| 探测 | `probe_args`/`probe_pattern` | 在管必有；正则可编译含第 1 捕获组 |
| manifest 引用 | `manifest` | 可选；值即 `manifest.toml` 节键，缺省同名节；引擎与 `catalog_lint` 同一解析（值给键、节缺即零原语回落内建） |

## 三、B 层：manifest DSL 三层

**L1 声明原语**（无脚本，静态可 lint；三平台语义由 ark 引擎统一实现，每原语须过三平台矩阵才准入标准）：

| 原语 | 形态 | win 语义 | POSIX 语义 |
| --- | --- | --- | --- |
| `env_set` | 键值表 | 注册表 HKCU（platform.rs 通道） | profile 标记块 |
| `shims` | 别名表（键=别名名，值=同目录源名） | 硬链接（`{别名}.exe`，要求同卷同 NTFS）失败回落 `{别名}.cmd`（内容 `%~dp0{源}.exe` 相对定位加引号） | 符号链接（`{别名}` 指向同目录 `{源}`） |
| `mirror` | 镜像源节（D42，键值见下段） | env 类走注册表、文件类走各工具 win 原生发现位 | env 类走 profile env 块（shell rc）、文件类走 XDG 位 |
| `persist`（**reserved**） | 目录表（相对安装目录） | **v1 不启用**：与 oma（端上 agent 治理）加 omc（金库与 manifest 运营）配置域重叠，且无真实工具需求；字段名与语义保留，真需求以 schema_version 递增引入（omc 评审裁） | 同左 |
| `machine_path`/`elevate` | 布尔 | 触发引擎内建（HKLM 加 gsudo） | 空态容忍（M003） |
| `service` | 服务描述 | 触发引擎内建服务注册 | systemd 用户单元（按需准入） |

**L1 `mirror` 节（D42，2026-09-13）**：运行时工具族（fnm/uv/bun 等）的**中国镜像源统一数据面**，全集口径对齐 ohmypwsh 遗产（set-mirror.ps1 / P0017 五端统一 / P0019 镜像兜底）；值一律数据面声明，落点与合并语义引擎按类型实现一次，不散在各端脚本。键全可选：`env`（用户级镜像变量，通道同 `env_set`）、`env_unset`（旧通道变量撤除：env 值会盖过文件配置，如 UV_INDEX_URL 盖 uv.toml，收口必须撤）、`npm_registry`（`~/.npmrc` **行级 upsert** `registry=<url>`：认证行等其余行逐字保留，npmrc 常载凭据不可整文件重写）、`bunfig_registry`（`~/.bunfig.toml` 整文件，含本 URL 即不重写，URL 比对去尾斜杠）、`uv_index`（uv.toml 整文件 `[[index]]` default 形态：POSIX `~/.config/uv/`、win `%APPDATA%\uv\` 各平台原生发现位）、`pip_index`（同左：POSIX `pip.conf`、win `pip.ini`）、`cargo_config`（CARGO_HOME 解析位 config.toml 整文件内容比对，rsproxy 全量形态；CARGO_HOME 解析序=进程 env > 用户级变量 > `~/.cargo`，ark Windows rust 接管的重定位位由此生效）、`goproxy`（D44 补位：go 代理语义键，行级 upsert 落 GOENV 文件（win `%APPDATA%\go\env`、POSIX `~/.config/go/env` 即 `go env -w` 持久位，直写不依赖 go 在位），配套 GOSUMDB=sum.golang.google.cn，用户键逐字保留）。**执行语义**：在 `apply_manifest_primitives` 内 **env_set 后、post_install 前**落源（镜像 env 进程内即设，fnm install 等 L2 命令立即经镜像拉包）；失败硬错（L1 写不进就是没配上）；幂等分支同样执行（存量端升级即得，共识③同款）；整面受 `ARK_TEST_NO_PATH_REG` 测试隔离闸门（用户环境写入面第五面）。**rust 特例**：rust 的镜像面由 rustup 专用模块原生落（引导器通道耦合：rustup-init 拉工具链需进程内 RUSTUP_DIST_SERVER 先行；`cargo_config` 键为 rust 数据面化预留，omc 数据就绪并验收后按 2026-09-11 撤内建同款流程收口；**启用 `cargo_config` 键前 rustup 原生写不撤，双写期数据面值必须与 rustup 模块 CARGO_CONFIG 逐字一致**，真正收口即撤原生写单源化，对线 R4）。**值形态红线**：mirror 各单值键与 `env_set` 键值（确认轮收口，D39 起的未校验面）拒绝换行与双引号、env 键须标识符形态（引擎落源前硬错拒整节；本地面无签名门，写入前自拒防配置注入，对线 R5；lint 同规则单一权威复用）。**验收口径**：POSIX 持久 env 落点随 `$SHELL` 选 rc 文件（bash `.bashrc`、zsh `.zshrc`、fish config），wsl 总台 A13 抓 `~/.bashrc`，zsh / fish 宿主上验收脚本须按同规则定位 rc（对线 R6）。**版本化**：mirror 节不升 `schema_version`（serde 容忍未知字段，旧引擎静默忽略零动作，前向兼容同红线）；lint 增 mirror 面（节在但全键空应省略整节）。

**L1 落点与判定粒度**：`shims` 源与别名都取**目标二进制所在目录**（与 PATH 注册面同一目录，故入口传 exe 父目录而非工具根）；**npm-tgz 族落点即「当前」npm 全局 bin，不具跨会话保证**：Windows 无版本管理器时稳定在 `%APPDATA%\npm`，但 fnm multishell 下每 shell 一个 `fnm_multishells\<pid>_<ts>` 目录（本机实测积了 6556 个），POSIX 随 node 版本走（`node-versions\vX\installation`）；且 npm 在 Windows 只产 `{name}`/`{name}.cmd`/`{name}.ps1` 无 `.exe`，而引擎源名按 `{source}.exe` 查，故 npm-tgz 的 shims 在 Windows 实际不生效（WARN 跳过，等于 POSIX-only）。npm 型的 npm 本体可以不在 PATH：引擎先 `which npm`，失败则经 fnm 解析（`aliases/default` 优先，否则**数值段最大**且含 npm 的版本目录；字符串序会把 v9 排到 v24 前，属已记教训），并把解析出的 node bin 注入子进程与自身进程 PATH 后重解析 exe；**锚定只锚静态位**（会话级 shim 如 fnm multishell 路径随会话回收，锚上去必悬空；PATH 命中的 npm 若非会话级则维持 PATH 优先，不抢用户既定 node 生态）；`fnm` 装后另写交互 shell hook（独立标记块**自含 PATH 导出**：`export PATH="$HOME/.local/bin:$PATH"` 后 `if command -v fnm >/dev/null 2>&1; then eval "$(fnm env)"; fi`，守卫缺席静默且不污染 source 退出码；块体感知幂等，存量旧块体原位重写升级，M027；非交互由进程内解析治本）。node 版本本身的供给建议归数据面 manifest `post_install`（`fnm install <ver>` 加 `fnm default <ver>`，fnm 在 ~/.local/bin，非交互同样可跑）。结论：npm 族别名交包自身 bin map，manifest shims 不承诺 npm 族跨会话可用；引擎侧已加防线「落点非绝对路径即跳过加 WARN」（防按 CWD 拼出相对别名）。`env_set` 与 `shims` **逐原语回退**：节存在但该原语缺省时只该原语走内建回退，不是整工具全有全无；撤内建以「数据面键集与内建键集一致」为 parity 前提。

**L2 受控命令数组**：分平台键 `post_install.win`/`.linux`/`.mac`，值为 argv 数组（非 shell 字符串，无元字符解释）；**类型即契约**：数组里每条是参数数组（`win = [["fnm","install","24.20.0"]]`），写成扁平字符串数组（`win = ["fnm","install"]`）会让整份 manifest 反序列化失败而被整体拒载（引擎 WARN 保留本地版本，影响面是该清单全部节而非单节），故 omc 侧 lint 必须含类型/schema 校验（引擎解析器即参考实现，2026-09-11 实测）；三键或显式 `skip` 齐备才过 lint；逐条执行、超时 300s 杀进程、失败**只报不回滚**且报告含退出码与 stdout/stderr 尾行（omc 评审裁）。**数据面约束**：`post_install` 各条命令必须**幂等**（重装与重试都会再跑：幂等分支同样执行，是失败后的重试路径）；失败语义为 **stderr WARN 加不拦安装收尾**（退出码不变，工具本体已装成），报告仍含退出码与尾行。执行期两路管道**并发抽干**（只留最近 64KiB 尾窗）：子进程输出超管道缓冲（约 64KB）时写端会阻塞，等其退出后再读会把正常命令误判超时为 300s（M017 实证）；尾窗回传等 2s 上限后放弃（后台孙进程持写端不关时不影响退出码与超时判定）。超时终止须**先杀进程树再杀直接子进程**：Windows 的 kill 只杀直接子进程，而 `taskkill /T` 靠「父进程在位」找树，父先死则报 not found 且孙进程存活（M021 本机 A/B 实证；正确顺序为 taskkill /PID x /T /F 后 kill 加 wait 兜底与回收）。POSIX 侧无进程组杀为已知残留（孙进程会存活至自然结束，需 pre_exec setpgid 加 killpg 才能根治，未落）。

**L3 任意脚本不进**：例外场景走引擎内建原语由数据字段触发（S007 三节取舍）。

## 四、跨仓执行分工

| 面 | omc（维护与运营） | ark（制定与执行） |
| --- | --- | --- |
| catalog 与 manifest 数据 | 唯一权威：条目、pin、DSL 内容维护 | 消费；改消费面时以标准 herdr 知会 |
| lint 门 | vitest 四规则加 DSL 语法机检（CI 首步） | catalog_lint 同规则（夹具回归） |
| 签名与发布 | catalog-seed 流水（两件各三件套） | 端上三重门与巡检 |
| 引擎 | 无 | manifest 解释器与原语三平台实现 |

## 五、演进治理

- 标准变更（新原语、新字段）由 ark 仓 PR 修订本文件，`schema_version` 递增；omc 数据面跟进 lint；旧版本数据按兼容窗口保留。
- 专用模块（vsbuild/rustup/docker/msi）渐进迁移为原语组合（按工具渐进，S007 四.3 默认），迁移完成一个撤一个内建分支。

## 六、定案与实现状态

三待定点已裁（2026-09-11 omc 评审回执，全 CONFIRM）：persist 不入 v1 且字段 reserved；L2 超时 300s 加只报不回滚加尾行退出码报告；manifest 单件 `manifest.toml`（粒度瓶颈时以 schema_version 变更窗口拆分）。

引擎首波已落（ome 仓）：`src/manifest.rs`（解析与 schema 版本拒载、L1 env_set/shims 三平台、L2 执行链含超时与失败报告）；install 双轨接线（manifest 节优先、无节内建回退，omc 数据上线后撤内建）；catalog sync 扩拉 manifest 三件套（同锚同签，云端未上线 404 静默跳过）；lint 扩 manifest 面（解析、三键齐备、catalog 引用一致性）；fixtures 样例（pwsh/dotnet env_set、bun shims、demo post_install）。omc 已承接首发（2026-09-11，run 34570660675：manifest.toml 三节初值加 manifest-lint 三面机检入 CI 首步、两件各三件套签名推桶）。ome 侧双轨收口完成（2026-09-11）：catalog sync 拉到云端 manifest 并独立验签 ok（顺修三处 URL 双 scheme 笔误）；install pwsh/dotnet/bun 实证 manifest 节优先加载；**内建双轨已撤**（ensure_user_env_overrides 内建表与 ensure_bunx_shim 删除，用户级配置与别名唯一来源是 manifest 节；无节零动作），真机复验绿。omc 侧配套已落（2026-09-11 终报）：bun guide_notes 改「bunx 别名来自 manifest.bun.shims」口径随下轮六件套刷云端；orphan 反向 lint 入 manifest-lint 第四面（manifest 节须对应 catalog 在管工具，首发三节反向清）。D39 全程闭环确认：R016 v0.2 定稿、引擎三平台、双轨收口撤内建、端上 manifest 四门、数据面首发。

D42 已落（2026-09-13）：L1 `mirror` 节引擎与 lint 落码（`src/manifest.rs` 的 `Mirror`/`apply_mirror` 与各键落源函数、catalog_lint mirror 面、fixtures fnm/uv/bun 三节基准形态）；接线在 `apply_manifest_primitives`（env_set 后、post_install 前，幂等分支与早退通道共用同点）；platform 增 `remove_user_env_var`（win 注册表删值、POSIX env 块摘行，`env_unset` 通道）；rust 接管扩 POSIX（`rustup.rs` 系统标准位 `~/.rustup`/`~/.cargo`、rsproxy rustup-init 引导、持久 RUSTUP_DIST_SERVER/RUSTUP_UPDATE_ROOT、cargo config rsproxy 全量、PATH `~/.cargo/bin`；Windows 行为零变化）；verify 增 POSIX dev-rust 维度、heal dev-rust 键开 POSIX；bunfig 写语义收编 manifest 单一权威（heal_bunfig 委托）。Windows 真机实证（2026-09-13）：uv 0.12.6 只读 `%APPDATA%\uv\uv.toml` 不读 `~/.config/uv`（known folder API），故 win 落 config_dir 平台位；四键终态（npmrc/bunfig/uv.toml/pip.ini）与镜像 env、env_unset 撤旧全链幂等绿。omc 数据面待办：manifest mirror 节数据（fnm/uv/bun）、tools.toml rust POSIX 字段（linux/mac cdn_url、exe、probe）、镜像桶 `rustup-init` POSIX 资产名、bootstrap 兜底撤点；验收面 wsl 总台 verify A13 四件断言。

三轮对线补审修正（2026-09-11，codex 审 claude 的 `fc196b8..2b75a86`）：L2 执行链改并发抽干两路管道（原「等退出再读尾行」在输出超管道缓冲时死锁误判超时，M017 实证）；win `.cmd` 兜底改 `%~dp0` 相对定位（原嵌绝对路径并自做反斜杠转义，转义无必要、绝对路径不可重定位）；`sync_manifest_if_present` 结果不再 `let _ =` 吞错（改告警），并把调用点提前到同锚早退之前（否则 catalog 锚未变时 manifest 永不刷新）；install 的 shims 原语判定改按原语（原按节存在判定，会吞掉 bun 内建回退）、shim 落点改 exe 父目录（原用工具根，POSIX 嵌套布局与 official 型下与 PATH 面不一致）；catalog 的 `manifest` 字段值（节键）改由引擎与 lint 共同解析（原字段被忽略）。新鲜度门已落（2026-09-11 双轨收口）：`ome catalog status` 增 manifest 面（manifest_path/present/local_sha256/cloud_sha256/synced/age_secs/signature/cloud_error，与 catalog 面同源探活），install 与 update 在 manifest 缺位时装前打一行 WARN（用户级配置与别名本次不应用），真空面从静默变可诊断。①自动跟进已落（2026-09-11）：auto_refresh 三路径（TTL fresh 加 InSync 加 Updated）补 manifest 拉取，fresh 判据用 manifest 文件 mtime 对 TTL：mtime 即上次成功落位时刻（显式 sync 与自动补拉都会刷新，故显式拉一次即自动续期），文件缺失视为过期首拉，mtime 取不到则保守不拉；TTL=0 或 `OME_OFFLINE=1` 时整条自动路径关闭（真机实证：mtime 触旧 48h 后任意命令 fresh 早退即补拉并验签落位，mtime 刷新；离线态 mtime 不动）。仍待共识一枚：撤内建的 parity 门（数据面键集与内建键集一致才算迁完）。orphan 节反向 lint 已由 omc 入 manifest-lint 第四面。安装侧另有 POSIX 可发现性兜底（非 manifest 原语，引擎行为）：嵌套布局工具的 exe 不在 `~/.local/bin` 时补 `~/.local/bin/<name>` 直链：profile 里的 PATH 注册在非交互 shell（omc hostExec）不加载，XDG 用户 bin 才是非交互可达基建；直链以 ome 装的那份为准（既存链接指向别处即重指，真文件不动），但**效度受宿主 PATH 约束**：只剩最小 PATH 的 env（本机 WSL `env -i` 实测 `/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin`）里它同样不可见，兜底不等于把 PATH 送进去。另记覆盖缺口：uv-git 与 npm-tgz 两条早退通道在 install 主链前 return，manifest 原语目前只覆盖绿色与 msi 主链加重装幂等分支，迁这两族前须把原语应用点上提。
