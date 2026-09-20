# ark::ledger

ledger：仓级公共账本客户端面（REQ-0015，对齐 ohmycloud REQ-063 Phase 3 CLI 原生集成）。
真源 https://ledger.ohmygh.com（issues.ohmygh.com 过渡期保役，本 CLI 面已切新真源）。
issue 流 = 立项修复或改进的义务（open 至 done，关单须先有 result 引 digest）；
artifact 流 = 产物共享库本体（publish 至 attest 至 promote/demote/supersede）。
写入五头签名道：Idempotency-Key 加 X-Key-Id 加 X-Timestamp（正负 60 秒窗）加
X-Nonce（10 分钟不重）加 X-Signature（Ed25519，签名基 v1/POST/路径/时间戳/nonce/
幂等键/body sha256 各行换行连，base64url 编码）；幂等同键同内容回放、同键异内容 409。
私钥运行时读环境 `ARK_LEDGER_KEY` 或密档（`ARK_LEDGER_KEY_FILE`，缺省
`~/.config/ark/ledger.key`，base64url 32 字节 seed）——不进仓不进 argv；
公钥 JWK 以常量内置（身份分发面），kid = sha256hex(规范化 JSON {crv,kty,x})。

## Functions

- `artifact_attest` — artifact 证明：POST /repos/<repo>/artifacts/<id>/attestations {type,...}。
- `artifact_list` — artifact 列表：GET /repos/<repo>/artifacts?current=&env=&kind=&name=。
- `artifact_publish` — artifact 发布：POST /repos/<repo>/artifacts {name,kind,digest,...}。
- `body_sha256_hex` — body sha256（小写 hex）。
- `file_digest` — 文件 digest（本地算锚：artifact publish 的正文或记录哈希即此值）。
- `http_client` — HTTP 客户端（timeout 毫秒；0 = 不限时；ureq 2.x 形，原 issue 域同款迁移）。
- `issue_close` — issue 关单链：先 result（引用 digest 或 artifact_id）再 status done（两写两幂等键；
- `issue_event` — issue 事件：POST /repos/<repo>/issues/<n>/events {type, payload{...}, body?}。
- `issue_list` — issue 列表（家族翻页形）：GET /repos/<repo>/issues?limit=&before=；
- `issue_new` — issue 开单：POST /repos/<repo>/issues {title, kind, acceptance, body?}。
- `issue_show` — issue 详情：GET /repos/<repo>/issues/<n>（projection 加 timeline）。
- `kid_of_jwk` — kid 派生（纯函数）：sha256hex(规范化 JSON，键序字母、无空白）。
- `ledger_get` — 读面 GET（无签名无配额）：回执 JSON。
- `ledger_post` — 写面 POST（五头签名道）：回执 JSON 与状态码（201 为写入，409 同键异内容，429 配额）。
- `list_saturated` — 饱和判定（家族标准 #52 同口径，纯函数）：返回条数恰打满夹取后 limit。
- `load_signing_key` — 私钥装载：`ARK_LEDGER_KEY`（base64url 32 字节 seed）优先，次 `ARK_LEDGER_KEY_FILE`
- `new_idempotency_key` — 幂等键生成（uuid v4；同键同内容回放、同键异内容 409 须换键——每次写入必新键）。
- `new_nonce` — nonce 生成（10 分钟窗不重；uuid v4 形）。
- `signature_base` — 签名基构造（纯函数，服务端 signatureBase 同构）：
- `signed_post_parts` — 签名一次写入（五头齐出）：返回 (headers, body_text)。
- `valid_digest` — digest 形校验：`sha256:<64hex>`（小写；一律正文或记录哈希为身份，库不收二进制实体）。

## Constants

- `ARTIFACT_KINDS` — artifact kind 面（服务端 ARTIFACT_KINDS 同口径，15 值）。
- `ATTEST_TYPES` — artifact 证明面。
- `ISSUE_EVENT_TYPES` — issue 事件面。
- `ISSUE_KINDS` — issue kind 面（服务端 ISSUE_KINDS 同口径）。
- `ISSUE_STATUSES` — issue 状态面（status 事件 to 值）。
- `KEY_ID` — key id = sha256hex(规范化 JSON {crv,kty,x}，键序字母)：注册与 X-Key-Id 即此值。
- `LEDGER_BASE` — 账本服务基址（REQ-063；issues.ohmygh.com 过渡期保役，CLI 面已切此真源）。
- `LIST_LIMIT_MAX` — 列表夹取界（服务端 clampLimit 同口径；家族标准默认 100 即上限）。
- `PUBLIC_JWK` — 本仓公钥 JWK（身份分发面，CLI 内置；对应私钥在提交侧密档，永不进仓）。
- `REPO_ID` — 本仓 repo_id（规范化 remote）。
- `TIMEOUT_MS` — HTTP 超时缺省（毫秒；0 = 不限时）。

