# mistakes 坑模式页索引

> 三层聚合 knowledge 层（v2 对齐，ADR-0009）：蒸馏产物模式页，M 编号。历史注记：原 M 系列错误模式库并入 ADR-0002 至 0006 附录（ADR-0001 批四，M001 至 M106 在附录内可检）；本目录为 v2 起独立立位的标准承载，**新页编号自 M107 起续号不复用**（编号退役不复用律）。

## 页面形态

- 每页 PROBLEM、ROOT CAUSE、FIX 三段，10 至 30 行；FIX 只许一句非步骤结论（可执行步骤只进 operations 标准页）。
- 写行动模式不抄报错；必带 trace 引文（sources 路径加时点），无引文标 unverified 不得当提案依据。
- frontmatter 登记 version 与 links（target 加 relation，两根词：derived_from 自产、cited_source 外档）。

## 现状

2026-10-10 首轮蒸馏试点入 M107 至 M109 三页（见下表与 distill-log 第 1 轮）。

| 编号 | 文件 | 主题 |
| ------ | ------ | ------ |
| M107 | `M107-copy型资产归档形态未嗅探.md` | copy 型资产上游改归档发放，装出归档本体冒充可执行（REQ-0018 件一） |
| M108 | `M108-evergreen直链无pin解析断层.md` | evergreen 直链无 pin 被当缺版本报错，设计态进失败汇总（REQ-0018 件二） |
| M109 | `M109-同进程并行临时目录互踩.md` | 临时目录仅 pid 命名，同进程并行互踩（REQ-0018 件一踩坑） |
