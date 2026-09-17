# Requirements 索引

> 需求登记：新需求先立 REQ 再实现，实现回填 trace（测试或验收命令）。状态机 draft 到 implemented 或 rejected。模板见同目录 0000-template.md 文件。
> 历史需求（迁移前）冻结于 PRD.md D 表与 GOAL.md 历史行，自 REQ-0001 起新需求立 REQ。

| id | 状态 | 标题 | trace |
|---|---|---|---|
| REQ-0001 | implemented | 文档体系全量迁移 dev-evo | check.py 全 PASS 加四件套绿 |
| REQ-0002 | implemented | aidoc 投影强制化重构 | cargo aidoc --check --strict 加 missing_docs deny 加 cargo test 全绿 |
| REQ-0003 | implemented | PATH 存量已达锁定版纳管 | cargo test 全绿 204 项 |
| REQ-0004 | rejected | 权威边车拉取时间戳击穿 | 对线实证 fetch 层已穿双写撤除 |
| REQ-0005 | implemented | skill面撤除与llms唯一发现通道 | cargo test 全绿加 CLI 冒烟三件加五端旧 SKILL.md 清扫回执 |
| REQ-0006 | implemented | 封版v1.3.0 | 对线两轮 CONFIRM 加 tag run 与资产验收（回填 diary 封版篇） |
| REQ-0007 | draft | 对齐build-release标准迁移（批一播种面加批二发布面） | 实施中（批一批二，trace 见 REQ 内） |
| REQ-0008 | draft | 包形专项批三 | 对线合流三点全准（2026-09-17）；窗一版动工在即（开关形双挂加包形读序），设计决策见 REQ 内 |
| REQ-0009 | implemented | issue命令集成 | lib 单测六件加全量全绿加真读面冒烟加实弹回执（trace 见 REQ 内） |
| REQ-0010 | implemented | 镜像默认通道与GitHub兜底 | cargo test 全绿加离线集成测零 API 面加门禁绿（trace 见 REQ 内） |
| REQ-0011 | implemented | cli-docs标准采纳 | 对照表全件加裸调用面与漂移守卫两测加 README 四节重构（trace 见 REQ 内） |
