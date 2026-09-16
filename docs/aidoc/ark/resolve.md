# ark::resolve

resolve：版本解析三分支（cdn_index_url / cdn_url / GitHub REST），
语义对齐 helpers.ps1 的 Resolve-ToolVersion / Get-HashiCorpIndex / Get-GitHubRelease。
本模块只解析不下载；网络调用统一 30s 超时、3 次指数退避（2^n 秒），
api.github.com 在 403/限流等失败时回退 `gh api`（认证通道）。

## Functions

- `extract_version_by_pattern` — 用 version_pattern 正则从资产名提取版本（取第 1 捕获组）。
- `pick_max_semver` — 从版本名列表里按语义版本取最大（对齐 pwsh Get-HashiCorpIndex 的
- `resolve_tool` — 解析工具目标版本与资产：uv-git > cdn_index_url > cdn_url > GitHub release 四分支。
- `strip_tag_prefix` — tag_prefix 剥离（大小写不敏感，对齐 pwsh 的 OrdinalIgnoreCase）。

## Types

- `Resolution` — 解析结果：与 pwsh Resolve-ToolVersion 的返回字段对应（Release/Shasums 等运行时对象除外）。
- `ResolveOptions` — 版本选择：--latest / --tag / --version 三选一（都不给则用 pin 的锁定版本）。

