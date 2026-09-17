# ark::issue

issue 域（REQ-0009，对齐 ohmycloud REQ-057 契约）：自研命令仓统一 issue 入口
issues.ohmygh.com。new 一键提交自动带 tool=ark 与版本/平台/host；list/show 读面。
契约文档真源 = ohmycloud docs/requirements/REQ-057 契约节（omc 客户端与 Worker
服务端共 src/lib/issuecontract.ts 单源；本模块为其 Rust 面等价实现，校验语义对齐）。

网络面统一 ureq（仓内既有依赖，零新增 crate）：提交 POST、列表与详情 GET；
基址 env `ARK_ISSUES_API` 可覆盖（测与灰度，对齐 omc 的 OMC_ISSUES_API）。

## Functions

- `http_client` — HTTP 客户端（超时毫秒；ureq Agent 复用连接池）。`timeout_ms == 0` 不设整体
- `issues_api_base` — API 基址：env `ARK_ISSUES_API` 覆盖供测与灰度，缺省统一入口域。
- `list_issues` — 列表：GET /api/issues?tool=&status=&limit=（新到旧；limit 1 至 100 由服务端封顶）。
- `post_issue` — 提交：POST /api/issues；201 取 `{ok,id,url}`（429 限速与 400 校验不过带服务端文案）。
- `self_host` — 主机名：env `HOSTNAME`（交互 shell 常在但多不导出）与 `COMPUTERNAME`（win）
- `self_platform` — 平台串形 `linux/x86_64`（os/arch 取编译目标常量，运行态恒定）。
- `self_version` — 自身版本（Cargo 包版本单一来源）。
- `show_issue` — 详情：GET /api/issues/<id>（含正文）。
- `validate_issue` — 提交体校验（tool 与 title 规则拒绝；version/platform/host 客户端先截断）。

## Types

- `IssueFields` — 提交体六字段（POST /api/issues 的 JSON 体；回执 201 形 `{ok,id,url}`）。
- `IssueFull` — 详情（列表行加正文）。
- `IssueRow` — 列表行（list 与 show 共用面；created_at 为服务端 UTC ISO 形）。

## Constants

- `BODY_MAX` — 契约上限：body 至多 20000（REQ-057）。
- `ISSUES_DOMAIN` — 统一入口域（fleet 共用 issue 入口，REQ-057）。
- `TIMEOUT_MS` — 默认 HTTP 超时毫秒（对齐 omc 参考实现 20s）。
- `TITLE_MAX` — 契约上限：title trim 后至多 200（REQ-057）。

