# Requirements 索引

> 需求登记：新需求先立 REQ 再实现，实现回填 trace（测试或验收命令）。状态机 draft 到 implemented 或 rejected。模板见同目录 0000-template.md 文件。
> 历史需求（迁移前）冻结于 PRD.md D 表与 GOAL.md 历史行，自 REQ-0001 起新需求立 REQ。

| id | 状态 | 标题 | trace |
|---|---|---|---|
| REQ-0001 | implemented | 文档体系全量迁移 dev-evo | check.py 全 PASS 加四件套绿 |
| REQ-0002 | implemented | aidoc 投影强制化重构 | cargo aidoc --check --strict 加 missing_docs deny 加 cargo test 全绿 |
| REQ-0003 | implemented | PATH 存量已达锁定版纳管 | cargo test 全绿 204 项 |
| REQ-0004 | rejected | 权威边车拉取时间戳击穿 | 对线实证 fetch 层已穿双写撤除 |
