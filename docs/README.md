# docs 文档地图

> 本仓文档导航（ADR-0001 迁移后现行体系；ADR-0009 起三层聚合 v2 对齐：sources/knowledge/operations 单向支撑，INDEX 职责由本地图与 `llms.txt` 承接）。

## 根面

| 文档 | 角色 |
|------|------|
| `../AGENTS.md` | 协作规则五节合同（最高约束） |
| `../README.md` | 项目简介与命令速查 |
| `../llms.txt` | agent 检索面（读序与代码文件位置） |
| `../PRD.md` | D01 起冻结决策索引（历史） |
| `../CHANGELOG.md` / `../ROADMAP.md` | 版本里程碑与阶段 |
| `../GOAL.md` / `../TODO.md` / `../PLAN.md` / `../INDEX.md` | 历史档案（冻结，见各文件头声明） |

## 三层聚合 v2 与单仓纪律形

| 层 | 目录 | 承载 | 索引 |
|------|------|------|------|
| sources 唯一出处层 | `sources/` | `diary/` 日轨迹、`proven/` P 编号自产证据、`external/` 外档 verbatim 留底 | `sources/README.md` |
| knowledge 双链知识层 | `knowledge/` | `adr/`、`req/`、`references/`、`research/`、`mistakes/` M 编号模式页、`docs/knowledge/skill-impact.md` 防重提页、`docs/knowledge/distill-log.md` 流水 | `knowledge/README.md` |
| operations 标准面 | `operations/` | G 族 how-to 标准手册、`skills/` 标准 skill（过闸才进） | `operations/README.md` |

旧名映射：`requirements/` 改 `knowledge/req/`、`guide/` 迁 `operations/`、`diary/` 与 `proven/` 迁 `sources/`（git mv 保史，历史 diary 不回写，映射注记在各层索引）。

## 红线

- `sources/diary/` 与 `knowledge/research/` 是保留核心结构，不可裁撤（用户裁定 2026-09-16）：diary 承载过程留痕与踩坑现场，research 承载六态研究档案，二者是六层模型过程面与知识面的实体承载。
- 公开契约双面：命令输出面以黄金文件 oracle 的 regenerate-and-diff 锁定（`tests/golden.rs` 与 `tests/expected/`）；lib 公开项以 aidoc 投影承载（`docs/aidoc/`，ADR-0006 第五十九批强制口径，漂移门禁 `cargo aidoc --check --strict` 在 AGENTS Commands 在册）。
