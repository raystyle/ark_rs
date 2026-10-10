# R012：ohmypwsh 与 ome 对齐清单，Linux 与 Windows

> **D41 更名注（2026-09-12）**：产品已更名 ark（命令 `ark`、自部署位 `~/.local/bin/ark`、元数据 `<data>/ark` 旧位读回、环境变量 `ARK_*` 读回旧名）；本文历史记录与 [实证] 条目保留当时口径。

> 状态变更（2026-09-01）：本清单的「共存对齐」路线已被用户裁决的**完整迁移**取代（ohmypwsh 部署/验收/自愈整体迁给 ome，见 ohmypwsh 仓库的 P0026 方案「部署迁移 ohmyenv-rs 差集盘点与分域路线」）。文件降级为 catalog 数据迁移参考：二、catalog 字段（`linux_*` 字段族与回退规则）、四、extract 类型、五、目录与 PATH 策略、七、校验与编码四节仍有效；一、总体原则与八、验证门禁按 P0026 的 M0 与验证门禁执行。
>
> 本文件是 ome 侧的「对外接口契约」：列出 ohmypwsh 需要与 ome 对齐的事项，确保双方对同一工具在同一平台上的版本、资产、目录、PATH 策略一致。本清单随 ome catalog 或平台策略变化而更新。

## 一、总体原则

1. **唯一 pin 源**：ome 以 `catalog/tools.toml` 为唯一 pin 源；ohmypwsh 的 `scripts/catalog.psd1` 应与之等价，字段命名按 ohmypwsh 习惯转换。
2. **平台字段回退**：Linux/mac 字段缺失时回退到通用字段；ohmypwsh 应实现相同的回退逻辑。
3. **跳过工具一致**：ome 明确不管理的 Linux 工具，ohmypwsh 不应期望 ome 提供部署包。
4. **部署包结构**：`ome package <tool> --out <dir>` 输出 `<dir>/<tool>/`，单二进制额外复制到 `<dir>/<tool>/bin/<tool>`。

## 二、catalog 字段对齐

### Linux / macOS 平台专属字段

ome `Tool` 结构新增字段，ohmypwsh `catalog.psd1` 的 `Pos`（或等价）侧应同步：

| ome 字段 | ohmypwsh 建议字段 | 含义 | 示例 |
| --- | --- | --- | --- |
| `linux_repo` | `Repo` 或 `RepoLinux` | Linux 专属 GitHub 仓库 | `PowerShell/PowerShell` |
| `linux_asset_pattern` | `AssetPatternLinux` | Linux 资产名正则 | `^powershell-[0-9.]+-linux-x64\.tar\.gz$` |
| `linux_dir` | `DirLinux` | Linux 安装目录 | `~/.local/share/ohmyenv/pwsh` |
| `linux_bin` | `BinLinux` | Linux PATH 目录 | `~/.local/share/ohmyenv/pwsh` |
| `linux_exe` | `ExeLinux` | Linux 版本探测 exe 路径 | `pwsh` / `bin/python` |
| `linux_extract` | `ExtractLinux` | Linux 解压方式 | `targz` / `zip` / `copy` / `targz-bin` / `tarxz-bin` |
| `linux_sums_pattern` | `SumsPatternLinux` | Linux 校验清单行匹配正则 | `^powershell-[0-9.]+-linux-x64\.tar\.gz$` |
| `linux_asset_sha_suffix` | `AssetShaSuffixLinux` | Linux 逐资产校验后缀 | `.sha256` |
| `linux_cdn_url` | `CdnUrlLinux` | Linux 直链模板 | `https://dotnetcli.azureedge.net/dotnet/Sdk/{version}/dotnet-sdk-{version}-linux-x64.tar.gz` |

### 字段回退规则

- 任何 `linux_*` 字段缺失时，ome 回退到同名通用字段（如 `repo`、`asset_pattern`、`extract` 等）。
- ohmypwsh 应保持相同回退行为，避免 Linux 上因字段缺失而失败。

## 三、工具支持状态对齐

### ome 已支持 Linux 的工具

age、sops、gh、rg、jq、mq、yq、starship、just、ast-grep、nushell、herdr、rumdl、pwsh、7z、dotnet、fnm、bun、uv、python。

### ome 明确跳过 Linux 的工具

| 工具 | 原因 | ohmypwsh 建议做法 |
| --- | --- | --- |
| `aria2` | 官方 release 仅 android，无 Linux x86_64 预编译资产 | Linux 下由 apt/brew 等系统包管理器安装 |
| `git` | `git-for-windows/git` 仅 Windows portable；Linux 无官方预编译 portable 资产 | Linux 下由 apt/brew 等系统包管理器安装 |

## 四、extract 类型对齐

### 跨平台类型

- `zip`：zip 解压，支持展平顶层单包裹目录。
- `targz`：tar.gz 解压，支持展平顶层单包裹目录。
- `copy` / `single`：单二进制直接落地。

### Linux / macOS 新增类型

- `targz-bin`：从 tar.gz 中按 `linux_exe` 叶子名查找并提取单二进制到安装目录。
- `tarxz-bin`：从 tar.xz 中按 `linux_exe` 叶子名查找并提取单二进制到安装目录（7z Linux 用）。

### Windows 专属类型

ome 在 Linux/mac 下返回明确错误；ohmypwsh 若需跨平台复用逻辑，应同样门控：`msi`、`7zsfx`、`gsudo`、`7z-extra`、`7z-archive`、`rmux`。

## 五、目录与 PATH 策略对齐

### Windows

- ome 元数据 + 工具目录：`D:\ohmyenv`（存在 D: 盘时）或 `C:\ohmyenv`。
- PATH：注册表 `HKCU\Environment\Path`，REG_EXPAND_SZ，支持 `%VAR%` 展开。

### Linux

- ome 元数据目录：`~/.local/share/ohmyenv`（XDG_DATA_HOME 回退）。
- 单二进制默认安装目录：`~/.local/bin`。
- 复杂运行时目录：`~/.local/share/ohmyenv/<tool>`（pwsh、dotnet、python）。
- PATH：shell profile 标记块，默认 `~/.bashrc`；按 `$SHELL` 检测可写入 `~/.zshrc` 或 fish config。

### macOS

- ome 元数据目录：`~/Library/Application Support/ohmyenv`（`dirs::data_dir()` 在 mac 上的返回值）。
- 单二进制默认安装目录：与 Linux 一致，使用 `~/.local/bin`。
- PATH：默认 shell 通常为 zsh，写入 `~/.zshrc`。

## 六、部署包目录对齐

### ome `package` 命令输出

```bash
ome package <tool> --out <dir>
```

生成：

```text
<dir>/<tool>/          # 解压后的完整内容
<dir>/<tool>/bin/      # 单二进制时才有，包含主二进制
<dir>/<tool>/bin/<tool> # 主二进制
```

### 待约定项

- 默认部署包根目录：当前 ome 默认 `<EnvRoot>/cache/deploy/<tool>`；ohmypwsh 若已有 `cache/wsl-tools/<tool>` 或 `cache/mac-tools/<tool>`，需双方统一。
- ohmypwsh scp 到远端后，远端 bash 部署脚本应能从 `<tool>/bin/<tool>` 或 `<tool>/<asset>` 找到二进制。

## 七、校验与编码

1. **校验清单编码**：ome 支持 UTF-8 / UTF-16LE / UTF-16BE（PowerShell `hashes.sha256` 为 UTF-16LE）。ohmypwsh 若读取 ome 下载的校验清单，需兼容 UTF-16。
2. **sha256 大小写**：ome 统一大写；ohmypwsh 比较时建议忽略大小写。
3. **版本号提取**：ome 用 `version_pattern` 从资产名提取版本（如 python tag 是日期，版本在 asset 名中）。

## 八、验证门禁

ohmypwsh 对齐后，回到 ome 跑：

```bash
cargo test
cargo check --target x86_64-pc-windows-gnu
uv run --script .tools/md-ref-scan.py
uv run --script .tools/md-heading-scan.py
```

全部通过视为对齐不影响 ome。
