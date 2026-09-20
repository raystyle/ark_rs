# ark::ledger

 ledger：仓级公共账本命令面接线（REQ-0015；总台修正令 2026-09-20 收口为
 ledger-rs crate 唯一实现——签名道加只增面全在 crate，本仓不再自研客户端副本）。
 真源 https://ledger.ohmygh.com（issues.ohmygh.com 已转只读保役）。
 权限收口：只增不关不删——issue new/list/show 加 artifact publish/attest
（attest_dev|attest_prod|verification_failed）加 list；close/status 推进与
 promote/demote/supersede 及删除唯一道 = 开发工作台 herdr 委托 omc 工位执行。
 私钥密档 `~/.config/ark/ledger.key`（32 字节 hex；`ARK_LEDGER_KEY` 环境或
 `ARK_LEDGER_KEY_FILE` 可覆盖）——不进仓不进 argv。

## Functions

- `artifact_attest` — artifact 证明（crate 只增面，收口后仅三型；promote/demote/supersede 归 omc 工作台）。
- `artifact_list` — artifact 列表（crate 读面：current/env 过滤；kind/name 过滤归网页面）。
- `artifact_publish` — artifact 发布（crate 只增面）：回执 artifact_id。
- `client` — 装配账本客户端：私钥读环境 `ARK_LEDGER_KEY`（32 字节 hex）优先，次
- `content_digest` — 正文或记录哈希（digest 即身份；crate content_digest 同源）。
- `issue_list` — issue 列表（crate 读面，家族翻页 more=1 恒带；免私钥）：回执含 issues 与 has_more。
- `issue_new` — issue 开单（crate 只增面）：回执 issue 号。
- `issue_show` — issue 详情（crate 读面）：projection 加 timeline。
- `list_saturated` — 饱和判定（家族标准 #52 同口径，纯函数）：返回条数恰打满夹取后 limit。
- `valid_digest` — digest 形校验：`sha256:<64hex>`（小写；一律正文或记录哈希为身份，库不收二进制实体）。

## Constants

- `LIST_LIMIT_MAX` — 列表夹取界（服务端 clampLimit 同口径；家族标准默认 100 即上限）。
- `REPO_ID` — 本仓 repo_id（规范化 remote）。

