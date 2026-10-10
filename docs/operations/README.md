# operations 标准面索引

> 三层聚合第三层（v2 对齐，ADR-0009）：how-to 标准手册（G 族）与标准 skill（`skills/` 子面，过闸才进）。旧目录名 `docs/guide/`，git mv 迁入（2026-10-10）；加载视图 `.claude/skills/` 是 `skills/` 的投影（同步守卫在 `.tools/llms-guard.py`）。

## G 族 how-to 标准手册

| 编号 | 文件 | 主题 |
| --- | --- | --- |
| G001 | `docs/operations/G001-文档标准细则-命名写作规范与rumdl检查.md` | 文档命名、写作规范与 rumdl 检查 |
| G002 | `docs/operations/G002-研究标准细则-结构与六态标记.md` | 研究文档结构与六态标记 |
| G003 | `docs/operations/G003-工作流标准细则-从登记到归档五步.md` | 想法从登记到归档五步工作流 |
| G004 | `docs/operations/G004-经验沉淀细则-成功与错误经验分治.md` | 经验沉淀分治：成功进 proven/references，错误进 mistakes，同型坑升格 |

不编号文档：`docs/operations/template.md`（方案模板）。

## skills 标准面子面

过闸标准 skill 落 `skills/<名>/SKILL.md`（agentskills 官方 spec 形态，双端同形）；候选未过闸不入此面。索引与闸门记录随件页头。

## G 族整族一案补闸记录

存量手册族整族一案补闸（v2 layers.md 建仓五步第 5 步口径；con-04 施工期执行）：

- 证据一（既有门禁无回归）：rumdl 加 mdcharlint 加 md-ref-scan 加 md-heading-scan 加 project-evo check.py（PEVO 豁免 aidoc）全绿退出码 0（同轮实录见 `docs/sources/diary/2026-10-10-升级轮对齐三件REQ-0018.md` con-04 节）。
- 证据二（点名义务翻转）：点名义务「G 族手册有唯一权威落位且与 AGENTS 摘要层不双份并行」。迁移前 guide 目录与 v2 字母表归层（G-NNN 属 operations）不符且无闸门在册，整族迁入本面后落位在册即翻转（存量族补闸口径：翻转对象为归层与在册性，非新增行为义务）。
- 证据三（知识链接成立）：族页头知识底座链在位（G001 至 G004 页首 blockquote 行，链 `../knowledge/` 各锚页）；族级知识底座：`docs/knowledge/adr/ADR-0001-文档体系全量迁移dev-evo.md`（文档体系迁移与留档裁定）与 `docs/knowledge/adr/ADR-0009-三层聚合对齐迁移.md`（本次归层裁定）。
