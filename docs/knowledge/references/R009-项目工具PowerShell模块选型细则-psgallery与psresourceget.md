# 项目工具PowerShell模块选型细则：psgallery与psresourceget

> AGENTS 操作规则「写临时脚本时」的 PowerShell 侧细则。与 `R005`（Rust 产品依赖）、`R008`（py 工具）三分项目工具选型。移植自 ohmyagents 同名细则；素材：2026-08-31 用户提供《PowerShell 模块选择与开发稳定指南》，经本机实证并对照 ohmypwsh `scripts\psmodule.ps1` 与 `modules.psd1`（自研模块管理器）的实际配置对齐后落定。

## 一、搜索能力对照

> 与 PyPI 相反：Gallery 网站与官方 CLI 搜索都有。[实证: 2026-08-31 本机 pwsh 7.6.5]

| 通道 | 用法 |
| --- | --- |
| 网站关键词 | `powershellgallery.com`，字段语法 `Id:` / `Tags:` / `Author:` |
| CLI 条件搜索 | `Find-PSResource <名>`（7.4+ 首选，PSResourceGet 1.2.0 本机实证）；旧栈 `Find-Module -Name / -Tag / -Filter`（实证 ImportExcel 7.8.10 可查） |
| 已知名字元数据 | `Find-PSResource <名>` 出版本与描述 |

坑与稳妥做法：`Find-PSResource` 裸调偶发「could not be found in any registered repositories」（本机两次一败一成，冷查询抖动）；显式 `-Repository PSGallery` 可复现，ohmypwsh `psmodule.ps1` 即恒带显式仓。[实证: 2026-08-31 两轮对照]

## 二、稳度判据

> 选型看四项，装前必核名字。

| 信号 | 看什么 |
| --- | --- |
| 下载量 | Gallery 页 Downloads |
| 最近更新 | Last Updated |
| 维护者 | Owner（`azure-sdk`、`msgraph-sdk-powershell` 这类官方账号） |
| 项目页 | ProjectUri 有源码有文档 |

名字必须与官方文档一字不差（2026 出现过大规模仿冒）。[经验: 指南] 相对稳妥一类：`Az` / `Microsoft.Graph` / `Pester` / `ImportExcel` / `PSScriptAnalyzer` / `PSReadLine`。[经验: 指南]

## 三、安装与版本管理

> 选型走 PSGallery；安装用 `Install-PSResource`（不自行 `Install-Module`）。历史模块管理器已退役（D18）。

```powershell
Find-PSResource <名> -Repository PSGallery
Install-PSResource <名> -Repository PSGallery
```

选型纪律：

- **生产钉版本**：`-RequiredVersion`。[经验: 指南]
- **Trusted 语义**：Trusted 只减确认不代表安全；核版本与哈希比 Trusted 更强。[经验: 指南]
- **仓库**：显式 `-Repository PSGallery`，避免冷查询抖动。[实证: 2026-08-31]

## 四、写与打包

- 临时脚本直接 ps1 归档 `.tools\`（AGENTS 写临时脚本规则）；有模块形态价值才 `New-ModuleManifest`（GUID 建后不重建；`FunctionsToExport` 禁 `*`）。[经验: 指南]
- 打包：`Compress-PSResource`。[经验: 指南]
- 发布 Gallery：`Publish-PSResource -ApiKey`（本仓暂无发布需求，流程留指南口径）。[经验: 指南]
- PS5 兼容：含中文的 ps1 统一 UTF-8 BOM（AGENTS 执行命令与写文件规则同源）。

## 五、决策树

| 目标 | 做法 |
| --- | --- |
| 不知道叫什么 | 网站或 `Find-PSResource`（CLI 条件搜索有） |
| 已知名字 | `Find-PSResource <名> -Repository PSGallery` 核版本 |
| 装进本机 | `Install-PSResource <名> -Repository PSGallery` |
| 脚本临时用 | 优先无依赖 ps1；确需模块走 PSResourceGet，不散装 |

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| 本地 | 用户《PowerShell 模块选择与开发稳定指南》（Downloads） | 2026-08-31 | 三通道对照、稳度判据、LTS 口径骨架 |
| 本地 | ohmypwsh `scripts\psmodule.ps1` 与 `modules.psd1`、S028 | 2026-08-31 | 本机实际模块管理机制与锁定清单 |
| web | PSGallery（Find-PSResource / Find-Module / 两套仓库注册表） | 2026-08-31 | 搜索、元数据、注册表现状实测 |
