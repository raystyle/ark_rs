# sources 唯一出处层索引

> 三层聚合第一层（v2 对齐，ADR-0009）：自产轨迹、自产证据件、外档信源原档，带时点、只增不改。一层一索引盖全根（三类出处分根收口）。

| 子根 | 承载 | 索引 |
|------|------|------|
| `diary/` | 日轨迹（一天一篇活轨迹：当日用户令、裁定、坑、门禁退出码、终态） | `docs/sources/diary/README.md` |
| `proven/` | P 编号自产证据面（判例实弹记录、验收全绿方案封存） | `docs/sources/proven/README.md` |
| `external/` | 外档信源原档（verbatim 留底；cited_source 关系的物证位） | `docs/sources/external/README.md` |

## 旧名映射注记

| 旧路径 | 新路径 | 迁移方式 |
| ------ | ------ | -------- |
| `docs/diary/` | `docs/sources/diary/` | git mv 保史（2026-10-10，ADR-0009） |
| `docs/proven/` | `docs/sources/proven/` | git mv 保史（同上） |
| `docs/external/`（无） | `docs/sources/external/` | 空态立位（同上） |

历史 diary 内的旧路径字样按「历史轨迹不回写」保留原貌；现役面引用一律走新路径。
