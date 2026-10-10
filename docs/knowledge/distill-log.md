# distill-log 蒸馏流水

> 三层聚合 knowledge 层（v2 对齐，ADR-0009，con-04 件3 立位）：按轮次记蒸馏、提案、闸门三行结论。蒸馏完即提交 knowledge（先于提案与闸门，拒绝不回收）。只增不改。

## 轮次记录

### 第 1 轮 con-04 首轮试点

2026-10-10；取样窗最近 3 日加首轮收口豁免援引。

- 蒸馏：M107、M108、M109 三页入 knowledge/mistakes（trace 引文与 links 在页；knowledge 提交先于提案与闸门）
- 点名义务（开工登记）：入册工具的集成测试断言装出物为可执行二进制而非归档本体（此前无标准盖不住；REQ-0018 件一 yq 双机病灶实证）
- 提案：一案 create，标准 skill 候选 ark-catalog-addition（`docs/operations/skills/ark-catalog-addition/SKILL.md`，页头闸门记录段三证据自取）
- 闸门：**受**（用户裁定接受，2026-10-10 con-04-A1；初记待审，A1 收口改受）
- 结论行（A1 收口）：候选 ark-catalog-addition 接收入标准集；页头状态改受。触发对实测（隔离环境 CLAUDE_CONFIG_DIR=/tmp/claude-iso-config，同提示词两轮）：正例「新入册 hexyl 参照 yq 形态」初测与复测均实调 Skill（invoke=1）成立；近失负例「本机安装修复不动 catalog」初测误触发（invoke=1，揭出 When to Apply 面过宽），收窄 description 与 When NOT to Apply（补普通安装诊断与重构两条）后复测三对全符（正例 invoke=1、两负例 invoke=0，转录 /tmp/a1-r-p1|n1|n2.jsonl 当轮）；llms-guard 复跑 exit 0（投影同步含收窄版）
