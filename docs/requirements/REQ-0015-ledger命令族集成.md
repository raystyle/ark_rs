---
id: REQ-0015
title: ledger命令族集成
status: implemented
priority: must
trace: cargo test --release --locked 全绿 239 项（签名基/kid/digest/幂等键/kind 面/饱和/body 哈希七件单测）；实弹五面（new/list/show/publish/attest+promote+list）201 全过，证据见 trace 补充
---

# REQ-0015:ledger命令族集成

总台令（2026-09-20）：集成 ledger 标准的 issue 与 artifact 命令族，替代原 issues.ohmygh.com issue 功能，完毕封小版本。契约权威 ohmycloud REQ-063（Phase 3 CLI 原生集成）。

## Scenario

真源切 ledger.ohmygh.com 仓级公共账本（issue 流管义务、artifact 流即产物共享库本体）；写入走 Ed25519 五头签名道；旧 issues.ohmygh.com 过渡期保役不删不停。

## Criteria

- [x] 签名道：签名基构造（v1/POST/路径/ts/nonce/idem/body-sha256 七行换行连）加幂等键（uuid v4 每写新键）加 nonce；私钥运行时读环境 ARK_LEDGER_KEY 或密档（ARK_LEDGER_KEY_FILE，缺省 ~/.config/ark/ledger.key；不进仓不进 argv）；公钥 JWK 常量内置，kid=sha256hex(规范化 JSON)
- [x] issue 族：new（title/kind=bug|improvement/acceptance/body）、list（家族翻页：默认 100、--before 游标、饱和提示、count 语义）、show（projection 加 timeline）、close（result 引 digest 加 status done 两写两幂等键，半链态如实报）
- [x] artifact 族：publish（name/kind 十五类/digest=sha256 小写 hex 校验/version/git_range/deps/outcome/summary/body）、attest（六型）、promote（糖衣）、list（current/env/kind/name）
- [x] 错误面：409 同键异内容须换键、429 配额、400 校验、401 未注册/验签不过：服务端错误串透传归因
- [x] 帮助面与 --llms 如实（新真源 ledger.ohmygh.com 注明；家族标准保持；零内部编号核对）
- [x] 单测：签名基构造、kid 派生（乱序 JWK 规范化同值）、digest 形边界、幂等键唯一性、kind 面与服务端同口径、饱和判定、body 哈希
- [x] 实弹：issue #1 开单 201（seq 3）加 list 投影对（#1=improvement open）加 artifact publish 201（lesson 真实教训，digest sha256:02d28b410ca289996e510808af7f7be9f79f7026d654f06ffaad68b1a69f1842，seq 4）
- [x] 门禁全绿加封小版本（v1.5.0）加 CHANGELOG 注明 issue 面切 ledger

## trace 补充

- 实弹（2026-09-20，WSL 本机，公钥已注册）：issue new 得 issue=1 seq=3（201）；issue list 得 count=1 `#1=improvement open ledger 集成冒烟`；issue show 出 projection（kind/status）加 timeline；artifact publish（kind=lesson，本仓「批量脚本先落盘后断言」二犯同型真实教训）得 artifact_id 2ffd7f18-5a8c-407c-8ede-696d43d2b2d1 seq=4（201）；attest_dev seq=5、promote seq=6、list --current 得 dev=y current=y。
- 公钥注册：kid 7f3658485ffd07e2e5712374c2a635959c23a4149405d19873a4174acf6aba18（总台入册回执在案，kid 规范形自核算对一致）。

## 设计注记

- kind 十五值与服务端 ARTIFACT_KINDS 同口径（总台单写 16 值系笔误，服务端 Set 实数 15）。
- 事件回执 seq 取 /event/seq（EventRow 全字段回执）。
- artifact_list URL 拼接免尾空参；issue list has_more 仅 before/more 形回执带（服务端同构）。
