# ark::status

status：三态对照（locked/installed/path）。
语义：
msi 与 official（exe 含 %）走环境展开，其余 Join EnvRoot；installed 实跑 exe 探测；
path 查用户 PATH 原始值（不展开比较，大小写不敏感）。

## Functions

- `category_label` — 分类中文组名（九类为主，D07 起 agent 与运行时管理器入册；旧值兜底转换期防炸，未知值为空串）。
- `collect_status` — 收集全部工具三态（按 catalog 书写顺序）。
- `collect_status_with` — 流式收集：每探完一个工具立即回调 on_row（status 命令逐行输出的关键——

## Types

- `StatusRow` — status 三态行：locked=pin version，installed=实跑探测，path=用户 PATH 是否含 bin。

## Constants

- `GROUPS` — 七类 taxonomy（catalog category 值 → 中文组名，展示序即数组序）。

