# ark

[![CI](https://github.com/raystyle/ark_rs/actions/workflows/build.yml/badge.svg)](https://github.com/raystyle/ark_rs/actions/workflows/build.yml)
[![Release](https://img.shields.io/github/v/release/raystyle/ark_rs)](https://github.com/raystyle/ark_rs/releases)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Ark（Agent Runtime Kit，命令 `ark`）**：本机跨平台（Windows / Linux / macOS）环境部署管理 CLI：管 53 个工具与 agent 运行时的版本解析、下载校验、PATH 注册、pin 锁定、更新与 doctor 诊断，一个标准一个配置，五端（Windows/WSL/mac/ubuntu/linux）同一套验收。

## 目录

- [项目介绍](#项目介绍)
- [部署](#部署)
- [配置](#配置)
- [使用](#使用)
- [排障与反馈](#排障与反馈)
- [开发](#开发)
- [许可](#许可)

## 项目介绍

- 53 工具全量纳管：agent 四家、运行时七家、编译器四家到命令工具廿余家（清单见[供给清单](#供给清单)表，权威在云端 catalog）
- 幂等安装：检测驱动，已装且版本一致即跳过，重跑零副作用；PATH、注册表与配置一次写对
- doctor 三层诊断：系统 / 依赖两层加 check 节（环境错误、配置健康、部署深诊、网络通连），列缺口与修复建议
- 镜像默认通道：下载与元数据默认 env.ohmygh.com、GitHub 兜底、有 sha 锚必校验（五端不再撞 GitHub 匿名配额）
- 清单云端权威：catalog 四重门（边车锚、schema、minisign 验签、seq 单调），新增软件不用换二进制

```powershell
ark doctor          # 体检：列缺口与修复建议
ark install         # 全量幂等安装（53 工具）
ark status          # 锁定 / 已装 / PATH 三态对照
```

> [!NOTE]
> 何时不用：ark 管的是「本机工具链部署」，不管 CI 侧容器镜像构建（各仓 CI 自管），也不管远端服务器编排（归 ohmycloud 舰队）。

## 部署

二进制发行版（推荐，三平台资产带 `.sha256` 边车）：

```bash
# 从 Release 下载对应平台资产（ark-<triple>.zip/.tar.gz 或裸件）
curl -LO https://github.com/raystyle/ark_rs/releases/latest/download/ark-x86_64-unknown-linux-gnu
curl -LO https://github.com/raystyle/ark_rs/releases/latest/download/ark-x86_64-unknown-linux-gnu.sha256
sha256sum -c ark-x86_64-unknown-linux-gnu.sha256   # 校验后进 PATH
```

源码构建自部署（需要 [rustup](https://rustup.rs)；三平台同一套流程）：

```powershell
git clone https://github.com/raystyle/ark_rs
cd ark_rs
cargo build --release
.\target\release\ark init     # Windows（PowerShell 7）
```

```bash
git clone https://github.com/raystyle/ark_rs
cd ark_rs
cargo build --release
./target/release/ark init     # Linux / WSL / macOS
```

`ark init` 自部署：二进制进用户程序目录（Windows `%LOCALAPPDATA%\Programs\ark`，POSIX `~/.local/bin`）、同步 catalog、注册 PATH，幂等可重跑。重开终端后 `ark doctor` 验证。

升级：`ark self update`（dev 滚动默认 / `--stable` 正式 / `--git` 源码三通道；默认镜像段读序、GitHub API 兜底）。

- 被管理工具装在 EnvRoot：Windows 默认 `D:\ohmyenv`（无 D: 盘则 `C:\ohmyenv`），Linux / macOS 默认 `~/.local/share/ohmyenv`；可用 `--env-root` 或 `ARK_ROOT` 改
- ark 自身装用户目录，与 EnvRoot 解耦；旧 `ome` 部署位在升级时自动清扫（别名已停建，2026-09-14 全舰队水位清零收口；旧环境变量读回已撤，2026-09-18 剔除批）

## 配置

ark 无配置文件：行为配置走环境变量，软件清单与工具级配置走云端 catalog 与 manifest（数据面 ohmycloud 权威）。

| 环境变量 | 作用 | 默认 |
| --- | --- | --- |
| `ARK_ROOT` | EnvRoot 覆盖（被管理工具安装根） | 平台默认（见部署节） |
| `ARK_CATALOG` | 清单文件显式指定（开发/测试用） | 四级解析（用户数据副本优先） |
| `ARK_CATALOG_TTL` | 清单自动刷新间隔秒数，0 关 | 86400（24h） |
| `ARK_OFFLINE` | `1` 关清单自动刷新（离线态） | 未设 |
| `ARK_MIRROR` | `0` 官方优先逃逸阀（镜像故障时用） | 镜像默认通道 |
| `ARK_TEST_REAL` / `ARK_TEST_MIRROR` | 真机/真网测试闸门（开发用） | 未设即 skip |

清单数据权威在云端（ohmycloud catalog-seed 与镜像三件套），本仓持格式契约与消费逻辑。拉取链过四重门，任一不过即拒收：

1. 边车 sha 锚：清单 sha256 与边车逐字对上
2. schema 解析：`Catalog::load` 解析不过不收
3. minisign 验签：公钥内嵌二进制，签名不过不收
4. seq 单调门：顶层 seq 低于已见拒收（防回滚重放旧但签名有效的清单对）

**下载与元数据默认走自建镜像、GitHub 兜底**（D44 下载链反转，2026-09-13；D51 全链转正，2026-09-18：解析面 pin 驱动零 GitHub API、self update 元数据镜像段读序先行、update 对齐 catalog pin；有专属分发域的工具（catalog 节键 `mirror_domain`，如 reader.ohmygh.com）镜像腿走专属域、缺省回落 env.ohmygh.com）；镜像未命中或失败秒级回落官方（下载走官方直链与完整链，无 API 配额面）；有 sha 锚（catalog pin、镜像边车或官方清单）必校验，锚不符视同失败回落。

运行时工具族的中国源配置走 manifest `mirror` 节（数据面声明、引擎落源，D42）：fnm 面 FNM_NODE_DIST_MIRROR 与 npm registry（npmmirror）、uv / pip 清华 TUNA 加 python 安装 NJU、bun npmmirror；win 落用户环境变量与各工具原生配置位、POSIX 落 shell rc（profile env 块）与 XDG 配置位。rust 由 rustup 接管模块原生落 rsproxy 全量。幂等：内容一致零重写，存量端 `ark install` / `ark update` 即得。

### 供给清单

53 个工具（清单权威在云端 seed 与镜像三件套，2026-09-18 实况：browser-harness 已出册、reader 已回册、browse/officecli/oxvg/resvg/agent-svgtools 入册），agent 四家二进制 PATH 在位即跳过、存量原地纳管：

| 类 | 工具 |
| --- | --- |
| 智能体（4） | claude、codex、grok、kimi |
| 自管（1） | ark（自管条目，原 ome；ome 过渡条目已收口 2026-09-18） |
| 操作编排（1） | herdr（多 agent 并行会话宿主） |
| agent 配置与诊断（1） | hst（原 oma；Hooks、Statusline、Trace 与只读观测） |
| 云端控制台（1） | omc（云与内网控制台 CLI，npm-tgz 通道） |
| 运行时（7） | pwsh、wsl、docker、dotnet、bun、python、nushell |
| 运行时管理器（2） | fnm（node）、uv（python） |
| 编译器（4） | vsbuild（含 C 编译器）、rust、go、zig |
| 多路复用（1） | rmux |
| 远程服务（1） | openssh |
| 密钥安全（3） | age、sops、gitleaks |
| 命令工具（27） | git、gh、aria2、7z、gsudo、oscdimg、rg、jq、mq、yq、starship、just、ast-grep、rumdl、shellcheck、zoxide、sheldon、ffmpeg、rclone、reader、lightpanda、typst、browse、officecli、oxvg、resvg、agent-svgtools |

平台空态如实表达：sheldon 上游无 Windows 资产、shellcheck 仅 Linux 入册、ffmpeg 官方 mac 构建仅 Intel、lightpanda 上游无 Windows 构建。

## 使用

任务型渐进示例（每步幂等可重跑）：

```bash
ark doctor                    # 1 体检：系统/依赖诊断列缺口
# 诊断缺口会带下一步建议命令（CTA）
ark install                   # 2 全量安装（或单装：ark install rg）
ark status                    # 3 三态对照：锁定/已装/PATH
ark verify                    # 4 部署域验收，FAIL 退出码非零可进脚本
```

查与更新：

```bash
ark query ffmpeg --latest     # 只解析最新版与资产，不下载（显式 latest 走 GitHub API 兜底）
ark update [名]               # 家族自研 CLI（hst/browse/reader/officecli）委托其自身自升级；其余对齐云端锁定（零 GitHub API；落后补装、领先如实报，D49/D51）
ark pin rg --version 14.1.1   # 临时本地锁（下次 sync 被云端覆盖；lock 为别名）
```

清单与自愈：

```bash
ark catalog                   # 看 catalog 与 manifest 两面：在位、锚、年龄、签名、同步态
ark catalog sync              # 立即从云端刷新（默认 TTL 24h 自动刷新；ARK_CATALOG_TTL 改，ARK_OFFLINE=1 关）
ark heal aria2 --dry-run      # 部署维度幂等自愈预览（PATH 修复、镜像源补写等）
```

issue 反馈（统一入口 issues.ohmygh.com，REQ-057 契约；自动带 tool=ark 与版本/平台/host）：

```bash
ark issue new "doctor 报 PATH 重复" --body "重跑步骤与输出"   # 一键提交，回执 id/url
ark issue list --tool ark    # 集中列表（默认 limit 100 即服务端上限；count 是本次返回条数非在册总数）
ark issue list --limit 3 --before 51   # keyset 游标翻更早一页（取 id 51 之前三条）
ark issue show 3             # 单条详情（含正文）
```

输出契约：数据走 stdout、提示走 stderr、错误为单行 JSON；`--format kv|json|jsonl` 可选；裸调用 `ark` 出导航指引 exit 0。完整字段与退出码契约见 `docs/references/R013-Agent友好IO契约-输出格式退出码与冻结面.md`。

### agent CLI 面

- 发现通道：`ark --llms` 命令手册唯一通道（D50 撤 skill 面；REQ-0011 cli-docs 采纳：版本注入、读序、退出码节，漂移守卫测试锁与 `--help` 同源）；mcp add 通道不适用（ark 是部署 CLI 非 MCP server，agent 编排归 ohmyagents）。
- 输出信封：key=value 数据面加结构化错误四元组加退出码，契约冻结于 R013（对外冻结面），不迁移 {ok,data,meta} 同构信封（改造破坏冻结契约，裁定不适用）。
- CTA：status 漂移提示（D09-3）与 doctor verdict 面在位（文本 HINT 形；类型化 CTA 结构随信封裁定同不适用）。
- token 计量与分页与输出过滤：裁定不适用（输出体量为五十工具全量百行级，`--format` 与 `--llms` 已覆盖检索需求；体量增长再立项）。

完整选项：`ark --help`；agent 手册：`ark --llms`。

## 排障与反馈

| 问题 | 对策 |
| --- | --- |
| 装完命令找不到 | 重开终端（PATH 注册面新会话生效）；`ark doctor` 查 PATH 卫生 |
| GitHub 限流报错 | 默认路径零 GitHub API（D51）；显式 `--latest` 触发时可等配额窗口或先 `ark catalog sync` |
| 镜像故障 | `ARK_MIRROR=0` 官方优先逃逸阀 |
| 清单拒收（验签/seq） | `ark catalog sync` 重拉；持续失败 `ark issue new` 反馈 |

## 开发

克隆后 `cargo build --release`；验证门禁与协作规则见 [AGENTS.md](AGENTS.md)（五节合同）与 [llms.txt](llms.txt)（agent 检索面）。测试 `cargo test --release --locked`；真机闸门 `ARK_TEST_REAL=1`、真网镜像闸门 `ARK_TEST_MIRROR=1`。

## 许可

[MIT](LICENSE)
