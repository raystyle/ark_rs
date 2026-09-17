# R011：mac 开发接管，环境准备与构建验证

> **D41 更名注（2026-09-12）**：产品已更名 ark（命令 `ark`、自部署位 `~/.local/bin/ark`、元数据 `<data>/ark` 旧位读回、环境变量 `ARK_*` 读回旧名）；本文历史记录与 [实证] 条目保留当时口径。

> 开发主机由 WSL/Linux 切换到本机 mac。ome 的 `#[cfg(not(windows))]` 分支同时覆盖 Linux 与 macOS，因此代码层面以 Linux 实现为基线，mac 侧重点在于工具链、构建验证与目录/PATH 习惯差异。本文件涵盖：工具链准备、首次构建验证、mac 目录/PATH 策略、与 Linux/Windows 的验证分工。事实性断言标六态。
>
> D18（2026-09-07）：ohmypwsh 已不存在；真机对照为本机 EnvRoot。下文 2026-09-01 接力段为历史快照。

## 一、前提与仓库

1. 仓库：`https://github.com/raystyle/ohmyenv-rs`（2026-09-01 由 ohmyenv 改名），主分支 `main`，含 Cargo.lock（[实证]）。
2. 开发主机：本机 mac（Apple Silicon 或 Intel）。clone 落 mac 原生文件系统（家目录，如 `~/ohmyenv-rs`），不放外置/网络卷：权限、大小写敏感、Spotlight 索引都可能引发异常（[经验]）。
3. 目标平台仍包括 Windows 与 Linux；mac 自身同时承担「开发主机」与「被管理目标」两个角色（[推断] 由用户决策）。
4. **接力状态（2026-09-01 mac 收官，Windows 回接）**：mac 侧批次全部收盘：M1 mac 字段族与真机六项验证、M0 数据主权与回流（psd1 Pos 侧一次性回流、pin 平台分列、转换器只校验不再生、go/zig/shellcheck 补录共 31 工具）、mac 完美收敛（目录型运行时 pwsh/rmux/zig/go 实证布局、extra_bins 多二进制、18 工具三态全等）。**开发接力回 Windows（D:\ohmyenv-rs）**，方向为 ohmypwsh 联动验收（对齐 P0026 M3/M4/M6 口径）。Windows 侧回接要点：
   - `git pull` 后基线门禁：`cargo test`（Windows 专属 cfg 测试回归）、`cargo fmt --all --check`、`cargo clippy --all-targets -- -D warnings`、md 三件套、`pwsh .tools\import-catalog.ps1`（校验器，PwshRoot 缺省 D:\ohmypwsh 即命中）。
   - catalog 已平台 pin 分列：Windows 侧回写只动通用四键，mac/linux 键（含 mac 真机写入的 sops/uv/vault/go pin 与 sha）**不得覆盖**；转换器已改只校验不再生，勿再跑旧版生成脚本。
   - tag 前缀语义变更：`tag_prefix` 未写即无前缀（uv/nushell/zig 等 tag 是裸版本号），Windows 侧 `--version` 解析同步受益。
   - mac 侧遗留观察：pwsh/go 的 path 态按 ome 注册语义如实 false（符号链接与 ohmypwsh hook 注入不计入）；`~/.ohmyenv/bin` 历史 PATH 残留属 ohmypwsh 退役范畴。
   - mac 侧环境快照见 R011 六与本机部署态：`ome` 已自部署 `~/.local/bin/ome`，catalog 同步 `~/Library/Application Support/ohmyenv/catalog/`，后续 Windows 批次无需 mac 侧配合即可继续（mac 需要时按 R011 三重建门禁）。

## 二、工具链准备

> mac 内的工具准备清单（不依赖 Linux/Windows 侧同名工具）。

| 工具 | 用途 | 准备方式 |
| --- | --- | --- |
| rustup / cargo | 构建与测试 | rustup 官方脚本装 stable（[经验] mac 上 Homebrew 的 rust 版本常滞后，推荐 rustup；本机 1.97.0 [实证]） |
| uv + python | 跑 `.tools` 三个 md 扫描脚本 | uv 官方脚本；`uv run --script .tools/<名>.py`（[实证] 2026-09-01 本机两扫描脚本跑通） |
| rumdl | markdown lint | mac 二进制由 ohmypwsh 部署在 `~/.local/bin/rumdl`，`rumdl check .` 31 文件通过（[实证] 2026-09-01） |
| gh | GitHub REST 限流回退 | 可选；匿名解析不依赖（[实证] resolve.rs 仅 403 时回退 gh api） |
| mingw-w64 | `cargo check --target x86_64-pc-windows-gnu` 交叉回归 | `brew install mingw-w64`（ring 构建脚本要 `x86_64-w64-mingw32-gcc`，[实证] 2026-09-01） |

## 三、首次构建验证

```bash
git clone https://github.com/raystyle/ohmyenv-rs ~/ohmyenv-rs
cd ~/ohmyenv-rs
cargo build
cargo test
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
uv run --script .tools/md-ref-scan.py
uv run --script .tools/md-heading-scan.py
```

验收标准：`cargo build` 零错误；`cargo test` 全绿（2026-09-01 基线 74 项），Windows 专属测试按平台 cfg 跳过（[实证] 2026-09-01 mac 真机 70 项全绿，差额为 Windows 专属 cfg 测试；黄金 status 测试曾因 exe 路径双拼失败，随 M1 修复转绿）；vsbuild 相关测试在 mac 上按 cfg 门控跳过或走纯函数路径（[实证] vsbuild 安装分支非 Windows 即报错，测试仅覆盖参数与探测纯函数）。clippy `-D warnings` 在 mac 需专门过一遍：Windows 专属代码的 import 与 `return` 若无 cfg 门控只在非 Windows host 报错（[实证] 2026-09-01 修 13 处）。Windows 交叉回归建议 `cargo check --target x86_64-pc-windows-gnu`（mac 需 `rustup target add` + `brew install mingw-w64`；Windows 主机侧无该 target 且 R010 已实证其交叉 check 不可行，故 gnu 交叉回归以 mac/Linux host 为准）。

## 四、mac 目录/PATH/提取策略

1. **ome 自身（2026-09-01 安装形态整改后）**：自部署二进制 `~/.local/bin/ome`（`init` 复制并注册 PATH）；元数据与部署态 catalog 在 `~/Library/Application Support/ohmyenv`（`dirs::data_local_dir()/ohmyenv`，三平台统一），`self-deploy` 会把仓库 `catalog\tools.toml` 同步到 `<数据目录>/catalog/`，部署态二进制按解析四级（ARK_CATALOG、exe 相邻、cwd、数据目录）找 catalog，不依赖 clone 目录（[实证] Windows 侧同构实现已验证，mac 侧待真机）。
2. **被管理软件安装目录**：默认单二进制绿色工具安装到 `~/.local/bin`，与 Linux 一致；复杂运行时按平台字段族解析（M1 已设 `mac_*` 专属族，回退链依次为 mac、linux、通用，见 R001；[实证] 2026-09-01 jq/fnm 首批补录）。本机 `~/.local/bin` 已有 ohmypwsh 部署的 26 个二进制，ome 按「同版本在位即跳过」幂等接管（[实证] 2026-09-01 `ome install jq --latest` 对已装 jq-1.8.2 报 skipped、零重装、仓库 catalog 零改动）。
3. **PATH 管理**：默认 shell 在 mac 上通常为 zsh，`platform.rs` 按 `$SHELL` 检测并写入 `~/.zshrc`；若使用 bash 则回退到 `~/.bashrc`（[推断] 由 `platform.rs::profile_path()` 逻辑）。
4. **extract 类型**：与 Linux 一致，跨平台 `zip`/`targz`/`copy`/`single`，Linux/mac 新增 `targz-bin`/`tarxz-bin`；Windows 专属类型（含 2026-09-01 新增的 `vsbuild`）在 mac 下返回明确错误。

## 五、与 Linux/Windows 的验证分工

| 侧 | 负责 | 命令 |
| --- | --- | --- |
| mac（开发主机） | 非 Windows 逻辑：解析、下载、校验、安装、PATH、status 的 build + test | `cargo build`、`cargo test` |
| Linux/WSL（验证机） | Linux 本机部署闭环验证（保留，防止 mac 与 Linux 行为分叉） | `cargo test`（[记忆] 2026-08-31 已全绿） |
| Windows（验收机） | Windows 专属行为：注册表 PATH、msi/7zsfx、self-deploy、真机闸门对照本机 EnvRoot | `ARK_TEST_REAL=1 cargo test` |

## 六、mac 真机验证结果

> 2026-09-01 六项全部通过，实测细节如下。

1. `cargo build` / `cargo test` 全绿（[实证] 70 项，Windows 专属 cfg 跳过；fmt / clippy -D warnings / md 三件套同绿）。
2. `ome install jq --latest` 幂等接管（[实证] ohmypwsh 已装 jq-1.8.2 在位即报 skipped，dir 解析 `~/.local/bin`，仓库 catalog 零改动；沙盒全装路径由 tests\linux_install.rs 覆盖，mac 首跑即绿）。
3. `ome deploy jq` 后新 shell 直接调用 `jq`（[实证] `zsh -ic 'jq --version'` 出 jq-1.8.2；`~/.zshrc` 已有同文 `export PATH="/Users/ray/.local/bin:$PATH"`，add_user_path 识别后零改写）。
4. `ome init` 后从家目录跑 `ome status` 命中 `~/Library/Application Support/ohmyenv/catalog/tools.toml`（[实证] 部署态独立解析；二进制 `~/.local/bin/ome`；jq 三态 locked=installed=1.8.2、path=true）。
5. `ome package fnm --out <dir>` 输出 `<dir>/fnm/bin/fnm`（[实证] universal 二进制含 arm64，缓存落 `~/Library/Application Support/ohmyenv/cache/`）。
6. `cargo check --target x86_64-pc-windows-gnu` 通过（[实证] 22.75s；前置 `rustup target add` + `brew install mingw-w64`）。

遗留观察（非阻塞）：无 mac 字段的工具（如 pwsh）在 mac `ome status` 的 exe 行显示 Windows 风格回退路径（`%ProgramFiles%\...`），installed/path 判定失真，待 M0 回流后逐工具补 `mac_*` 字段消化；PATH 中存在历史残留 `~/.ohmyenv/bin`（非 ome 自部署位，self-deploy 清理逻辑不覆盖，属 ohmypwsh 侧退役范畴）。

## 七、文档与提交纪律

1. 文档验证三件套在 mac 同样跑：rumdl check、md-ref-scan、md-heading-scan（[推断] 与 WSL 同理）。
2. 提交规范不变：`feat:`/`docs:`/`fix:`/`chore:` 前缀 + 中文描述、一事一提交；推送前确认本地全绿（[实证] AGENTS.md 规则 5）。
3. 踩坑与发现按 G003 五步闭环落 docs；mac 特有问题接编 `docs/mistakes` 并登记 `INDEX.md`。
