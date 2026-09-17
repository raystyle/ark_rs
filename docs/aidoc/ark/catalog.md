# ark::catalog

catalog：清单主功能。数据面（tools.toml 读写与路径解析）加两个子功能：
`ark catalog status`（看解析面与云端同步态）与 `ark catalog sync`（从云端刷新用户数据副本，D33）。

数据契约见 `docs/references/R001`：读用 serde（字段同 R001），
写（pin 回写）用 toml_edit DocumentMut 直接改文档树，保住字段顺序与注释。
路径解析优先级：
- EnvRoot：`--env-root` 参数 > `ARK_ROOT` 环境变量 > 存在 D:\ 则 D:\ohmyenv 否则 C:\ohmyenv
- catalog：`ARK_CATALOG` 环境变量 > exe 上级的 catalog\tools.toml > cwd\catalog\tools.toml
  > 用户数据目录；四级全 miss 时自举拉取（镜像边车锚，键读序 `ark/catalog` 主先、`ome/catalog` 兼容回落，#10）
  > 用户数据目录 catalog\tools.toml（自部署布局）

## Functions

- `auto_refresh` — 自动刷新（仅用户数据副本路径）：TTL 判定在联网之前；网络异常单次探活即退化，
- `auto_refresh_if_user_data` — 命令入口接线：解析面**就是**用户数据副本时按 TTL 刷新；跳过与失败都不拦命令。
- `auto_ttl` — 当前 TTL（读 `ARK_CATALOG_TTL` / `ARK_OFFLINE`）。
- `catalog_state` — 子功能 status 采集（云端不可达时如实标注 error 字段，不报错退出）。
- `check_signature` — 校验磁盘清单与其分离签名（`<清单>.minisig`）。
- `classify_origin` — 解析面来源分类（纯函数，供 status 子功能报告）：userdata / repo / env / other。
- `cloud_sha` — 云端清单锚（边车首 token，大写；走下载链重试与 curl 兜底，命令面用）。
- `fetch_cloud` — 取锚后拉取（sync 子功能用；锚与键同源解析）。
- `fetch_with_anchor` — 按给定锚拉取云端清单到缓存（键读序：主键先、兼容键回落；任一键全链通过即返回），
- `is_user_data_catalog` — 该路径是否为用户数据副本（运行态权威落点；签名巡检只对它告警）。
- `marker_text` — 标记文件文本（纯函数，与 parse_marker 对称；小写写入，读出一律大写）。
- `needs_check` — 是否需要联网比对（纯函数）：目标缺失、无标记、标记过期、或本地已被改写（标记锚与本地不符）。
- `parse_marker` — 标记文件解析（纯函数）：`<unix 秒>\n<sha256>\n`。
- `pin_key` — 当前平台 pin 字段的 TOML 键名（Windows 无前缀，Linux/mac 加平台前缀）。
- `resolve_catalog_path` — catalog 路径解析：`ARK_CATALOG` > exe 上级的 catalog\tools.toml（仓库与旧自部署布局）
- `resolve_env_root` — EnvRoot 解析：显式参数 > ARK_ROOT > 平台默认。
- `resolve_ttl` — TTL 解析（纯函数）：离线优先，其次显式秒数（0 关），非法值回落默认。
- `seq_gate` — seq 门（纯函数可测）：拉到 seq 低于已见即拒收（报错不降级）；等于幂等重放；
- `signature_path` — 清单的分离签名路径（`<清单>.minisig`）。
- `sync_to` — 子功能 sync 的核心：刷新到指定目标（命令面与自动路径共用；target 独立解析，便于沙盒测试）。
- `toplevel_seq` — 顶层单调序号（回滚重放防护，S006 候选 B / D40）：签发侧每批 +1，
- `user_data_catalog_path` — 用户数据副本路径：`<metadata>\catalog\tools.toml`（self-deploy 同步位，运行态权威的落点）。
- `verify_with_embedded_keys` — 用内嵌公钥集合验签（任一公钥通过即可，供密钥轮换过渡期使用；纯函数可测）。
- `write_pin` — pin 回写：用 toml_edit 直接改文档树，只动当前平台的 tag/version/asset/sha256 四个键
- `write_sha256` — sha256 回填：只写当前平台的 sha256 一个键（install 成功后回填空 sha；统一大写）。

## Types

- `Catalog` — 已加载的 catalog：order 保工具书写顺序（即安装/更新顺序），tools 按键查值。
- `CatalogState` — 子功能 status 的数据面：解析面路径与来源、本地与云端锚、检查年龄、TTL 与离线态。
- `CloudCatalog` — 已拉到缓存的云端清单（含分离签名件路径）。
- `ManifestState` — manifest 面状态：在位与本地锚、年龄、云端锚对比与签名态（两件各自可诊断）。
- `Outcome` — 刷新结果。
- `SignatureState` — 清单签名状态（D34）：valid 通过内嵌公钥验签；invalid 有签名但验不过；missing 无签名件。
- `Tool` — 工具条目：字段与 R001 一一对应；可选字段为空时整行省略，故全部 Option。

## Constants

- `CLOUD_CATALOG_KEY` — 云端清单在镜像里的键（D41 C：主键 `ark/catalog/`；兼容键 `ome/catalog/` 为 omc 铺段前
- `CLOUD_CATALOG_PUBKEY_ID` — 内嵌公钥的 key id（人读标注，来自 `catalog-sign pubkey` 输出）。
- `CLOUD_MANIFEST_KEY` — 云端 manifest 键（R016 两件分离：与 tools.toml 同批同签；双键读序同上）。
- `DEFAULT_TTL_SECS` — 自动刷新默认 TTL（秒）：一天一次锚比对。

