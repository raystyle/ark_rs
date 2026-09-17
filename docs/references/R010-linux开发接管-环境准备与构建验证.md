# R010：linux 开发接管，环境准备与构建验证

> **D41 更名注（2026-09-12）**：产品已更名 ark（命令 `ark`、自部署位 `~/.local/bin/ark`、元数据 `<data>/ark` 旧位读回、环境变量 `ARK_*` 读回旧名）；本文历史记录与 [实证] 条目保留当时口径。

> 已归档：2026-08-31 开发主机由 WSL/Linux 切换到 mac，后续开发接管流程见 `docs\references\R011-mac开发接管-环境准备与构建验证.md`。本文件保留为 2026-08-31 Linux/WSL 接管阶段的历史记录。
>
> ome 已扩展支持 Linux 本机部署。本文件涵盖：工具链准备、构建验证、Linux 目录/PATH 策略、平台门控现状、两端验证分工。事实性断言标六态。

## 一、前提与仓库

1. 仓库：`https://github.com/raystyle/ohmyenv`，主分支 `main`（[实证] 2026-08-31 已推送，含 Cargo.lock）。
2. 开发主机：本机 WSL 的 Linux 发行版（[实证] 2026-08-31 裁决，由「lan-linux 或 WSL」收窄）。clone 落 WSL 原生文件系统（家目录，如 `~/ohmyenv`），不放 `/mnt/d`：跨文件系统性能差、权限语义不同（[经验]）；本机 WSL 曾有 fnm cd hook 损坏致 /mnt/d 假象的坑，cd 行为异常先查 `~/.bashrc.d/node.sh`（[记忆] 详见 ohmypwsh S039）。
3. 目标平台包括 Windows 与 Linux（WSL 为 Linux 验证主机）；Linux 已纳入管理域（[实证] 2026-08-31 更新 AGENTS.md 边界节）。mac 被管侧策略为系统标准目录、不进 ohmyenv 目录，属后续扩展。

## 二、工具链准备

> WSL 发行版内的工具准备清单（全部装在 WSL 里，不依赖 Windows 侧同名工具）。

| 工具 | 用途 | 准备方式 |
| --- | --- | --- |
| rustup / cargo | 构建与测试 | rustup 官方脚本装 stable；WSL 侧 cargo 1.98.0 / rustc 1.98.0（[实证] 2026-08-31 本机 WSL） |
| uv + python | 跑 .tools 三个 md 扫描脚本 | uv 官方脚本；`uv run --script .tools/<名>.py`（[实证] 脚本带 PEP 723 头，无需预装依赖） |
| rumdl | markdown lint | 可选；linux x64 二进制在 ohmypwsh catalog Pos 侧有 pin（[记忆] 待复核资产名） |
| gh | GitHub REST 限流回退 | 可选；匿名解析不依赖（[实证] resolve.rs 仅 403 时回退 gh api） |

## 三、首次构建验证

```bash
git clone https://github.com/raystyle/ohmyenv ~/ohmyenv
cd ~/ohmyenv
cargo build
cargo test
```

验收标准：`cargo build` 零错误；`cargo test` 全绿，Windows 专属测试按平台 cfg 跳过（[实证] 2026-08-31 WSL 侧：cargo build / cargo build --release 均零错误；cargo test 全绿；新增 `tests/linux_install.rs` 验证 jq install/deploy/status 闭环；`cargo check --target x86_64-pc-windows-gnu` 通过）。Windows 专属跳过项见第五节。

## 四、Linux 目录/PATH/提取策略

1. **ome 元数据目录**：`~/.local/share/ohmyenv`（XDG_DATA_HOME 回退），含 `cache/`、`logs/`（[实证] 2026-08-31 运行 `ome install jq --latest`）。
2. **软件安装目录**：默认单二进制绿色工具安装到 `~/.local/bin`；多二进制工具（如 age）通过 `targz` 展平后也落入 `~/.local/bin`（[实证] 2026-08-31 age 安装后 age/age-keygen 均在 ~/.local/bin）。
3. **PATH 管理**：写入 `~/.bashrc`（或按 `$SHELL` 检测的 `.zshrc` / fish config），用 `# >>> ome PATH` / `# <<< ome PATH` 标记块包裹，幂等更新（[实证] `tests/linux_install.rs`）。
4. **extract 类型**：
   - 跨平台：`zip`、`targz`、`copy`、`single`。
   - Linux 新增：`targz-bin`（解压 tar.gz 后按 `linux_exe` 叶子名查找并 copy 单二进制到安装目录）。
   - Windows 专属（Linux 下返回明确错误）：`msi`、`7zsfx`、`gsudo`、`7z-extra`、`7z-archive`、`rmux`（[实证] 源码 `extract.rs` 分支）。

## 五、平台门控现状

> 2026-08-31 完成：Windows 侧源码核查 + WSL 首跑实证 + 门控补齐。

1. 新增平台抽象层（[实证] 2026-08-31）：
   - `src\platform.rs`：集中封装 EnvRoot 默认值、PATH 管理（Windows 注册表 / Linux profile）、环境变量展开、`self-deploy` 目标、official 判定。
   - `src\envpath.rs`：保留 Windows PATH 纯函数；`add_user_path` / `remove_user_path` / `user_path_contains` re-export `platform.rs`。
2. catalog 平台扩展（[实证] 2026-08-31）：
   - `Tool` 增加 `linux_repo`、`linux_asset_pattern`、`linux_dir`、`linux_bin`、`linux_exe`、`linux_extract`、`linux_sums_pattern`、`linux_asset_sha_suffix`、`linux_bootstrap_asset`。
   - 提供平台感知方法（`repo()`、`asset_pattern()`、`dir()`、`bin()`、`exe()`、`extract()`、`sums_pattern()`、`asset_sha_suffix()`、`bootstrap_asset()`），Linux 优先 `linux_*`，缺失回退通用字段。
3. 安装/提取/状态适配（[实证] 2026-08-31）：
   - `src\install.rs`：`install_dir` / `bin_dir` 支持 `~` 与环境变量展开；Linux 不整删共享 `~/.local/bin`；防穿越允许 `$HOME` 与 EnvRoot 之下。
   - `src\extract.rs`：Windows 专属 extract 类型加 `#[cfg(windows)]`；Linux 下 `copy` / `zip` / `targz` 设置可执行权限；新增 `targz-bin`。
   - `src\toolver.rs`：Linux 下 `exe_path` 基于 `linux_dir` 解析；`expand_env_vars` 支持 `$VAR` / `${VAR}`。
   - `src\checksum.rs`：跨平台时若 pin 的 `asset` 与当前解析资产不一致，不将 pin 的 sha256 作为校验基准。
   - `src\selfdeploy.rs`：Linux 下复制到 `~/.local/bin/ome` 并注册 `~/.local/bin` 到 PATH。
   - `src\status.rs`：PATH 查询与 exe 路径解析走平台抽象。
4. 测试门控（[实证] 2026-08-31）：
   - Windows 语义测试加 `#[cfg(windows)]`：`envpath` 全部、`toolver::exe路径_official展开_envroot拼接`、`install::防穿越_*`、`extract::bunx_shim_硬链接优先_幂等`、`tests/install.rs::dies_install_防穿越_目录越出envroot`。
   - 新增 `tests/linux_install.rs`：WSL 下 jq install/deploy/status 闭环 + PATH 幂等。
5. 禁改项（[实证] AGENTS.md 边界节）：不为 Linux 构建改动 Windows 行为语义；门控只隔离平台差异，两平台共享逻辑保持单份。
6. 交叉检查限制（[实证] 2026-09-01 Windows 侧）：从 Windows 做 `cargo check --target x86_64-unknown-linux-gnu` 不可行：xz2（lzma-sys）与 ureq native-tls 需 C 工具链，报 `x86_64-linux-gnu-gcc` 缺失；属环境前提非代码问题，Linux 构建前置需 build-essential。winreg 已在 `[target.'cfg(windows)'.dependencies]`。

## 六、两端验证分工

| 侧 | 负责 | 命令 |
| --- | --- | --- |
| WSL（开发主机） | Linux 部署闭环：解析、下载、校验、安装、PATH、status 的 build + test | `cargo build`、`cargo test` |
| Windows（验收机） | Windows 专属行为：注册表 PATH、msi/7zsfx、self-deploy、真机闸门对照本机 EnvRoot | `ARK_TEST_REAL=1 cargo test` |

## 七、文档与提交纪律

1. 文档验证三件套在 WSL 同样跑：rumdl check、md-ref-scan、md-heading-scan（[实证] 三脚本与 rumdl 均有 linux 形态；rumdl 若缺可暂缓，两个 py 扫描必跑）。
2. 提交规范不变：`feat:`/`docs:`/`fix:`/`chore:` 前缀 + 中文描述、一事一提交；推送前确认本地全绿（[实证] AGENTS.md 规则 5）。
3. 踩坑与发现按 G003 五步闭环落 docs，跨仓事项（如 grok 的 New-ToolDef 滞后）查 `docs\mistakes\M106` M001（[实证] 已记录前置条件）。
