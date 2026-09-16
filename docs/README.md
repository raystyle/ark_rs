# docs 文档地图

> 本仓文档导航（ADR-0001 迁移后现行体系；INDEX 职责由本地图与 `llms.txt` 承接）。

## 根面

| 文档 | 角色 |
|------|------|
| `../AGENTS.md` | 协作规则五节合同（最高约束） |
| `../README.md` | 项目简介与命令速查 |
| `../llms.txt` | agent 检索面（读序与代码文件位置） |
| `../PRD.md` | D01 至 D49 冻结决策索引（历史） |
| `../CHANGELOG.md` / `../ROADMAP.md` | 版本里程碑与阶段 |
| `../GOAL.md` / `../TODO.md` / `../PLAN.md` / `../INDEX.md` | 历史档案（冻结，见各文件头声明） |

## docs 目录

| 目录 | 定位 | 索引 |
|------|------|------|
| `adr/` | ADR 架构决策（ADR-0001 起现行决策；ADR-0002 至 0006 承接原 M 系列错误模式库） | `adr/README.md` |
| `requirements/` | REQ 需求登记（draft 到 implemented 带 trace） | `requirements/README.md` |
| `references/` | R 编号开发参考细则（选型、测试、数据模式、跨仓协调） | `references/README.md` |
| `research/` | S 编号研究档案（六态标注） | `research/README.md` |
| `diary/` | 项目日记（一天一篇，提交钩子） | `diary/README.md` |
| `guide/` | G 编号元规范（G001 写作、G002 六态、G003 工作流、G004 经验沉淀）与方案模板。**留档理由**：五节合同是摘要层，写作与工作流细则的唯一权威在此，裁撤即摘要层失去细则承接 | `guide/README.md` |
| `proven/` | P 编号方案归档（G004 成功面分治）。**留档理由**：验收全绿方案的封存档案，ADR-0001 后新达成改回填 REQ trace 与关联 ADR，存量留档保历史可查 | `proven/README.md` |

## 红线

- `diary/` 与 `research/` 是保留核心结构，不可裁撤（用户裁定 2026-09-16）：diary 承载过程留痕与踩坑现场，research 承载六态研究档案，二者是六层模型过程面与知识面的实体承载。
- 公开契约双面：命令输出面以黄金文件 oracle 的 regenerate-and-diff 锁定（`tests/golden.rs` 与 `tests/expected/`）；lib 公开项以 aidoc 投影承载（`docs/aidoc/`，ADR-0006 第五十九批强制口径，漂移门禁 `cargo aidoc --check --strict` 在 AGENTS Commands 在册）。
