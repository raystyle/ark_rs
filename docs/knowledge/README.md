# knowledge 双链知识层索引

> 三层聚合第二层（v2 对齐，ADR-0009）：带版本、双向链接关联的结构化知识。单向支撑：本层靠 `docs/sources/` 撑，`docs/operations/` 标准面靠本层撑，反向不成立。

| 子根/件 | 承载 | 索引 |
|---------|------|------|
| `adr/` | ADR 架构决策（不可逆裁定；ADR-0002 至 0006 附录承接原 M 系列错误模式） | `docs/knowledge/adr/README.md` |
| `req/` | REQ 需求登记（draft 到 implemented 带 trace 回填） | `docs/knowledge/req/README.md` |
| `references/` | R 编号开发参考细则（事实不解释） | `docs/knowledge/references/README.md` |
| `research/` | S 编号研究档案（六态标注结论页） | `docs/knowledge/research/README.md` |
| `mistakes/` | M 编号坑模式页（蒸馏产物：PROBLEM/ROOT CAUSE/FIX 带 trace 引文） | `docs/knowledge/mistakes/README.md` |
| `skill-impact` | 防重提页（只收被拒提案全文，只由闸门追加，蒸馏与提案角色只读） | `docs/knowledge/skill-impact.md` |
| `distill-log` | 蒸馏流水（按轮次记蒸馏、提案、闸门三行结论） | `docs/knowledge/distill-log.md` |

## 旧名映射注记

| 旧路径 | 新路径 | 迁移方式 |
| ------ | ------ | -------- |
| `docs/requirements/` | `docs/knowledge/req/` | git mv 保史加目录改 req（2026-10-10，ADR-0009；v2 字母表口径，判例实测） |
| `docs/adr/` | `docs/knowledge/adr/` | git mv 保史（同上） |
| `docs/references/` | `docs/knowledge/references/` | git mv 保史（同上） |
| `docs/research/` | `docs/knowledge/research/` | git mv 保史（同上） |
| `docs/guide/`（G 族） | `docs/operations/`（标准面） | git mv 保史加整族一案补闸（归层纠正：how-to 手册属 operations） |
| M 系列（并入 ADR 附录） | `docs/knowledge/mistakes/`（独立立位） | 空态立位起步；ADR 附录 M 编号留存，新页续号不复用 |

## 单仓纪律

knowledge 面先于标准面独立成 commit；闸门拒绝只回滚标准面与加载视图，knowledge 路径永不 reset；知识层永不回滚，否证走新版本或 supersedes 注记，不原地改写。
