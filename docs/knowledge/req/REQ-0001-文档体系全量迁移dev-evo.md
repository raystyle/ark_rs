---
id: REQ-0001
title: 文档体系全量迁移 dev-evo
status: implemented
priority: must
trace: check.py 全 PASS 加四件套绿加断链零
---

# REQ-0001:文档体系全量迁移 dev-evo

## Scenario

本仓治理体系（根原语四件加 INDEX 加 docs 六目录）需统一到用户沉淀的 dev-evo 文档即代码体系，迁移中不丢任何在役规则语义、不断任何历史决策可查性。

## Criteria

- [x] AGENTS.md 五节合同齐备（PE-01 PASS），现行四段规则语义无损重排进五节（批一 69f1103）
- [x] docs/knowledge/adr 与 docs/knowledge/req 骨架与索引在位（PE-02、PE-03 PASS，批一）
- [x] PRD D 表冻结头注记落位，D50 起走 ADR（ADR-0001 自证，批一）
- [x] TODO 与 PLAN 进行中面转 REQ，GOAL 定位句并入 AGENTS（批二：GOAL 与 TODO 冻结档案化，PLAN 裁量为命令语义参考保留在役）
- [x] INDEX 职责拆解（AGENTS Read first 加各 README 索引），llms.txt 建成（批三：五目录 README 索引、llms.txt 33 行代码表、INDEX 退役指针页）
- [x] M 系列并入 ADR，全仓引用替换，断链回归零（批四：五 M 文件转 ADR-0002 至 ADR-0006、活引用切换 AGENTS 与 llms.txt 与 INDEX、M0xx 编号制度退役）
- [x] PE-11 禁字存量清零（Unicode 箭头、连接号），PE-12 断链清零（批一，八处存量清剿）
- [x] check.py 全 PASS（12/12 零 SKIP）加本仓四件套绿（批一基线 5P5F 到 12P）
