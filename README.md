# ark

**Ark（Agent Runtime Kit，命令 `ark`）**：本机跨平台（Windows / Linux / macOS）环境部署管理 CLI。管 47 个工具与 agent 运行时的版本解析、下载校验、PATH 注册、pin 锁定、更新与 doctor 诊断。

## 安装

需要 Rust 工具链（[rustup](https://rustup.rs)）。三平台同一套流程：克隆、构建、自部署。

Windows（PowerShell 7）：

```powershell
git clone https://github.com/raystyle/ark_rs
cd ark_rs
cargo build --release
.\target\release\ark init
```

Linux / WSL / macOS：

```bash
git clone https://github.com/raystyle/ark_rs
cd ark_rs
cargo build --release
./target/release/ark init
```

`ark init` 自部署：二进制进用户程序目录（Windows `%LOCALAPPDATA%\Programs\ark`，POSIX `~/.local/bin`）、同步 catalog、注册 PATH，幂等可重跑。重开终端后 `ark doctor` 验证。

- 被管理工具装在 EnvRoot：Windows 默认 `D:\ohmyenv`（无 D: 盘则 `C:\ohmyenv`），Linux / macOS 默认 `~/.local/share/ohmyenv`；可用 `--env-root` 或 `ARK_ROOT` 改
- ark 自身装用户目录，与 EnvRoot 解耦；旧 `ome` 部署位在升级时自动清扫（别名已停建，2026-09-14 全舰队水位清零收口；旧环境变量读回已撤，2026-09-18 剔除批）
- 升级自身：`ark self update`（dev / stable / git 三通道；默认镜像段读序、GitHub API 兜底）

## install 链

检测驱动，一条链走完：

1. `ark doctor` 体检：系统 / 依赖两层诊断加 check 节，列缺口与修复建议
2. `ark install [名]` 幂等安装：版本解析、下载、sha 校验、解压、PATH 注册与注册表 / 配置写入一次完成；已装且版本一致即跳过（configure 面照跑）；省略名则全量
3. `ark verify` 部署域验收：逐维度 PASS/FAIL，FAIL 退出码非零，可进脚本
4. `ark heal` 自愈：PATH 修复、镜像源补写等，幂等；`--dry-run` 预览

配套查询与更新：

```powershell
ark status                # 锁定 / 已装 / PATH 三态对照
ark query ffmpeg --latest # 只解析最新版与资产，不下载
ark update [名]           # 对齐云端锁定安装（catalog pin 即目标、零 GitHub API；落后补装、领先如实报，D49/D51；不回写锁定）
ark pin [名]              # 查看 / 设置版本锁定（lock 为别名）
```

## catalog 四重门

清单数据权威在云端（ohmycloud catalog-seed 与镜像三件套），本仓持格式契约与消费逻辑，新增软件不用换二进制。拉取链过四重门，任一不过即拒收：

1. 边车 sha 锚：清单 sha256 与边车逐字对上
2. schema 解析：`Catalog::load` 解析不过不收
3. minisign 验签：公钥内嵌二进制，签名不过不收
4. seq 单调门：顶层 seq 低于已见拒收（防回滚重放旧但签名有效的清单对）

```powershell
ark catalog        # 看 catalog 与 manifest 两面：在位、锚、年龄、签名、同步态
ark catalog sync   # 立即从云端刷新（默认 TTL 24h 自动刷新；ARK_CATALOG_TTL 改，ARK_OFFLINE=1 关）
```

## issue 反馈

统一 issue 入口 issues.ohmygh.com（fleet 自管，REQ-057 契约）：使用中遇缺陷一键反馈，自动带上下文（tool=ark 与版本/平台/host），每 IP 限速防刷。

```powershell
ark issue new "doctor 报 PATH 重复" --body "重跑步骤与输出"   # 一键提交，回执 id/url
ark issue list --tool ark    # 集中列表（按 tool/status 过滤，新到旧）
ark issue show 3             # 单条详情（含正文）
```

## 供给清单

50 个工具（清单权威在云端 seed 与镜像三件套），agent 四家二进制 PATH 在位即跳过、存量原地纳管：

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
| 命令工具（22） | git、gh、aria2、7z、gsudo、oscdimg、rg、jq、mq、yq、starship、just、ast-grep、rumdl、shellcheck、zoxide、sheldon、ffmpeg、rclone、reader、lightpanda、typst |
| 运行时衍生（1） | browser-harness（bin 名 bh） |

平台空态如实表达：sheldon 上游无 Windows 资产、shellcheck 仅 Linux 入册、ffmpeg 官方 mac 构建仅 Intel、lightpanda 上游无 Windows 构建。

## 镜像源

**下载与元数据默认走 env.ohmygh.com 自建镜像、GitHub 兜底**（D44 下载链反转，2026-09-13；D51 全链转正，2026-09-18：解析面 pin 驱动零 GitHub API、self update 元数据镜像段读序先行、update 对齐 catalog pin）；镜像未命中或失败秒级回落官方（下载走官方直链与完整链，无 API 配额面）；有 sha 锚（catalog pin、镜像边车或官方清单）必校验，锚不符视同失败回落。镜像故障逃逸：`ARK_MIRROR=0` 官方优先（`=1` 已是默认、兼容保留）。

运行时工具族的中国源配置走 manifest `mirror` 节（数据面声明、引擎落源，D42）：fnm 面 FNM_NODE_DIST_MIRROR 与 npm registry（npmmirror）、uv / pip 清华 TUNA 加 python 安装 NJU、bun npmmirror；win 落用户环境变量与各工具原生配置位、POSIX 落 shell rc（profile env 块）与 XDG 配置位。rust 由 rustup 接管模块原生落 rsproxy 全量（三平台：装走 rsproxy rustup-init，RUSTUP_DIST_SERVER / RUSTUP_UPDATE_ROOT 加 cargo config；POSIX 用 `~/.rustup` 与 `~/.cargo` 系统标准位）。幂等：内容一致零重写，存量端 `ark install` / `ark update` 即得。

## 输出契约

数据走 stdout、提示走 stderr、错误为单行 JSON；`--format kv|json|jsonl` 可选。字段与退出码全量契约见 `docs\references\R013-Agent友好IO契约-输出格式退出码与冻结面.md`。

## agent CLI 面

发现通道与输出契约现状与裁定（dev-evo tool-cli-agents 第十一节对照，2026-09-16）：

- 发现通道：`--llms` 命令图唯一通道（D50 撤 skill 面：`ark skill` 与 SKILL.md 退役，头部含何时用与下载纪律行）；mcp add 通道不适用（ark 是部署 CLI 非 MCP server，agent 编排归 ohmyagents）。
- 输出信封：key=value 数据面加结构化错误四元组加退出码，契约冻结于 R013（对外冻结面），不迁移 {ok,data,meta} 同构信封（改造破坏冻结契约，裁定不适用）。
- CTA：status 漂移提示（D09-3）与 doctor verdict 面在位。
- token 计量与分页与输出过滤：裁定不适用（输出体量为五十工具全量百行级，`--format` 与 `--llms` 已覆盖检索需求；体量增长再立项）。
