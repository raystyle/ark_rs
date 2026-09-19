# ark::download

download：资产下载与缓存复用，语义对齐 helpers.ps1 的 Save-ReleaseAsset。
缓存目录 <EnvRoot>\cache\<asset>：
- 命中且 sha256 一致则复用；不符删除重下；无 sha 基准且文件非空则复用。
- 下载先写 `<asset>.part` 再 rename，失败不留半截 dest。
- 下载走 ureq（3 次指数退避），失败回退系统 curl.exe（--retry 5）。
- sha256 计算用 sha2，比较统一大写。

## Functions

- `cache_path` — 缓存路径：<EnvRoot>\cache\<asset>。
- `download_asset` — 下载资产到缓存并复用：对齐 Save-ReleaseAsset 的缓存三分支（cache_reuse 提取共用）。
- `download_asset_with_mirror` — 带镜像优先的资产下载（D44 反转，2026-09-13 用户裁定「安装默认走 ohmygh，官方是兜底」）：
- `download_fresh` — 强制重下（删旧再下）：校验清单类资产每次取新，不复用缓存。
- `download_latest_with_sidecar` — 带镜像优先的 latest 段资产下载（D08 第二批，evergreen 引导器：rust / vsbuild；D44 反转）：
- `fetch_text_short` — 单次短超时文本取回（自动刷新探活用，D33）：不重试、不走 curl 兜底，失败即 Err。
- `mirror_sidecar_sha` — 镜像 .sha256 边车取锚（digest 替代源）。单次短超时快取（对线 F3：不退避不 curl，
- `mirror_sidecar_url` — 镜像 latest 段边车 URL：`{MIRROR_BASE}/{tool}/latest/{asset}.sha256`。
- `mirror_url` — 镜像段 URL（全局域 env.ohmygh.com）：`{base}/{tool}/{version}/{asset}`。
- `mirror_url_at` — 镜像段 URL 显式基址形（REQ-0013 独立分发域）：基址取 catalog 节键 mirror_domain
- `parse_sidecar_sha` — 边车文本解析 sha：标准清单行 `<sha>  <filename>`，取首 token 大写化（纯函数可测）。
- `sha256_file` — 计算文件 sha256，返回大写 hex（比较基准统一大写）。
- `with_query` — URL 追加 query 参数（已含 query 用 `&` 连接）。

## Constants

- `MIRROR_BASE` — 自建分发镜像基址（种子终态 69/69，ohmycloud#2）。

