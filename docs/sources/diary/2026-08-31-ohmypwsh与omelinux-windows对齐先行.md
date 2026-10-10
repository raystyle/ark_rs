# 2026-08-31：ohmypwsh 与 ome Linux/Windows 对齐先行

## 决策

用户决定：在继续推进 mac 开发主机接管之前，先让 ohmypwsh 与 ome 当前的 Linux/Windows 现状对齐。mac 接管文档（R011）已就绪，真机验证暂停，待 ohmypwsh 对齐完成后再继续。

## 需要对齐的接口

ome 侧已落地的 Linux/Windows 能力，需要 ohmypwsh 同步：

1. **catalog 字段**
   - 新增平台专属字段：`linux_repo`、`linux_asset_pattern`、`linux_dir`、`linux_bin`、`linux_exe`、`linux_extract`、`linux_sums_pattern`、`linux_asset_sha_suffix`、`linux_cdn_url`。
   - 字段语义：Linux 优先 `linux_*`，缺失回退通用字段。

2. **工具 Linux 支持状态**
   - 已支持：pwsh、age、sops、gh、rg、jq、mq、yq、starship、just、ast-grep、nushell、herdr、rumdl、7z、dotnet、fnm、bun、uv、python。
   - 明确跳过：aria2、git（官方 release 无适合 Linux x86_64 的预编译资产，由系统包管理器维护）。

3. **部署包目录与结构**
   - ome `package` 命令默认输出：`<EnvRoot>/cache/deploy/<tool>/`。
   - 单二进制额外复制到：`<out>/<tool>/bin/<tool>`，对齐 ohmypwsh mac-tools 输出约定。
   - 需与 ohmypwsh 约定统一的默认部署包根目录。

4. **目录/PATH 策略**
   - Windows：`D:\ohmyenv` 集中目录 + 注册表 PATH。
   - Linux：`~/.local/share/ohmyenv` 元数据 + `~/.local/bin` 单二进制 + `~/.bashrc` PATH 标记块。

## 当前状态

- `GOAL.md` / `PLAN.md` / `TODO.md` 已更新为「ohmypwsh 与 ome Linux/Windows 对齐」。
- mac 接管（R011）保留但真机验证暂停。

## 待续

- 用户在 ohmypwsh 仓库完成 catalog.psd1 / helpers.ps1 / 部署脚本同步。
- 回到 ome 跑 `cargo test` + `cargo check --target x86_64-pc-windows-gnu` 回归验证。
- 记录对齐结果，必要时更新 `docs/references/R001`。
