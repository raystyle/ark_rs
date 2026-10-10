---
id: REQ-0018
title: 升级轮对齐修复与GitHub反代道
status: implemented
priority: must
trace: 沙盒实弹三面（yq 装出 ELF 4.54.1、bun 滚装 1.4.3、反代下载 sha 与 pin 一致）加 cargo test 全绿 245 项（新增十二件）加门禁四件套绿
---

# REQ-0018:升级轮对齐修复与GitHub反代道

## Scenario

ai-ccoe 总台双机全量升级轮实证两件欠账发对齐单（2026-10-10），随后追加同域第三件（下载通道增强），本 REQ 收束三件为一批：

- 件一（必修）：yq 资产（mikefarah/yq v4.54.1 的 yq_linux_amd64.tar.gz）是 gzip 外层套 tar 内层（内含 ./yq_linux_amd64 平台变体名），catalog 的 `linux_extract = "copy"` 让 ark 把归档本体原样落 ~/.local/bin/yq 冒充可执行，双机同症「安装后未找到可执行文件或无法读取版本」。
- 件二（修噪音）：catalog 的 bun 是撤钉 evergreen 件（CDN latest 直链、无 pin、设计态），update 对它报「需 --version 指定版本（CDN 来源）」并计入 all 失败汇总（query 全量同连坐），设计态当 error 是噪音。
- 件三（新增）：ark 下载道为镜像优先加 GitHub 直连回落，墙内直连回落常慢断；自建 GitHub 反代 proxy.ohmygh.com（前缀形 `<base>/<原URL>` 直通 release 资产）已在位，需接入为中间层。

## Criteria

- [x] 件一：copy/single 型资产落盘前按魔数嗅探形态（gzip `1f 8b`、xz `fd 37 7a 58 5a 00`、zip `PK\x03\x04`、tar 257 偏移 `ustar`）；非归档原样复制（既有语义零变化）；归档剥层到二进制：压缩层先解（flate2/xz2）、内层 tar/zip 展开成员后按 exe 叶子名或平台变体名（`yq_linux_amd64` 形前缀 `_`/`-` 变体、同 stem 的 `yq.exe`）挑真身，多候选取最大者；targz-bin/tarxz-bin 提取同享变体名回退；解包临时目录带纳秒时戳防同进程并行互踩；新增 `extract_tar`（裸 tar 解压）
- [x] 件一测试：离线单测五件（gzip 套 tar 抽 ELF、zip 按叶名挑真身、gzip 裸二进制剥层、非归档原样复制、归档无匹配成员如实报错不落半截）加真网集成一件（真 yq tar.gz 装出物断言 ELF 头与 `--version` v4.54.1）
- [x] 件二：cdn_url 模板无 `{version}` 占位且未 pin 的 evergreen latest 直链（bun 形）重定向解析真实 tag：单跳 302 的 Location 即 `releases/download/<tag>/`（HEAD 不随重定向零 body、GET 终态 URL 兜底，对象直链非 API 无配额面）；install/update/query 默认滚装最新，已装同版幂等 skip；版本化 `{version}` 模板无 pin 仍如实报错（口径不扩大）；doctor pin-missing 异味排除 evergreen CDN 件（同 vsbuild/rust 口径）
- [x] 件二测试：`is_evergreen_cdn` 判定、`tag_from_download_url` 取 tag、版本化模板报错回归锚三件离线单测加真网 query 集成一件（bun 解析出真实 semver 版本行）
- [x] 件三：GitHub 三域（github.com / releases.githubusercontent.com / objects.githubusercontent.com）资产 URL 经反代改写 `<base>/<原URL>` 拉取；api.github.com 与镜像域不改写；通道优先级 = 工具镜像 > 反代 > 直连，反代在 `download_asset` 官方腿内先行单次（锚校验同尺）、失败回落直连完整链；配置面 `ARK_GH_PROXY`（缺省启用 `https://proxy.ohmygh.com/`、自定义覆盖、`0` 或空串关），`ARK_MIRROR=0` 逃逸阀同关反代；selfupdate 官方腿经 `download_asset` 自动受益
- [x] 件三测试：改写与配置纯函数单测三件（三域前缀形、他域 None、尾斜杠归一、配置值域）加真网闸门冒烟一件（yq v4.54.1 linux 包经反代拉取 sha 与 catalog pin 锚一致，直连同 sha 即直通完整性实证，ARK_TEST_MIRROR=1 门控）
- [x] 门禁：cargo fmt 加 clippy 加 test --release --locked 全绿（245 项，新增十二件）；rumdl 加 md 三扫描加 project-evo check 绿；aidoc 投影再生成

## 边界与分工

- catalog 数据面（yq 条目的 `linux_extract = "copy"` 与 bun 撤钉态）归 omc 真源不动：件一在引擎侧把 copy 语义补完到二进制层（数据面如后续改 `targz-bin` 同样可装，变体名回退在位）；件二沿用现有 evergreen 直链键。
- sums/清单类下载（checksum 的 download_fresh）仍直连不走反代：改动面收束，墙内病灶按需后续扩。
- selfupdate 的元数据镜像读序（D51）不在本批：其资产官方腿已经 `download_asset` 自动带反代。
- catalog 全局键配置形未做：`ARK_GH_PROXY` 环境变量已覆盖需求，全局键留待 omc 数据面需要时另立。

## trace 回填区

- 沙盒实弹（release 二进制、HOME 与 EnvRoot 隔离、真网）：`ark install yq` exit 0 出 `action=installed` `version=4.54.1`，装出物 `file` 为 ELF 64-bit LSB executable（非 tar/gzip）、`yq --version` v4.54.1；`ark update bun` exit 0 出 `tag=bun-v1.4.3` `version=1.4.3` `action=installed`，`bun --version` 1.4.3（镜像版本段 404 回落后反代道下载成功）；`ark update yq`（已装同版）出 `action=skipped` exit 0；`ark query bun` 出真实版本行 exit 0。
- 反代真网冒烟（ARK_TEST_MIRROR=1 门控测）：yq v4.54.1 linux 包经 `https://proxy.ohmygh.com/https://github.com/mikefarah/yq/releases/download/v4.54.1/yq_linux_amd64.tar.gz` 拉取，sha256 与 catalog pin 锚 E68A456F90C577AF3FE4960184B3A3CF5C461E0348407C10F107DA3A5FEC8972 一致。
- 测试：cargo test --release --locked 全绿（lib 174 加 cli/catalog_lint/install/golden/linux_install/mirror_fallback/real 计 245 项，本批新增离线单测十件加真网集成三件）；cargo fmt 加 clippy --release --locked --all-targets exit 0。
- 推送前对线（AGENTS 实质代码改动门）：待推前 herdr 驱动 codex 对线，结论回填此处。
