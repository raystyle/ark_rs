# ark::heal

heal：部署域幂等自愈（P0026 M4，heal-map.psd1 的 42 键迁嵌入注册表）。
语义：verify 判 FAIL 的维度按本表得到幂等修复动作，`ark heal <dim|all> [--dry-run]` 执行；
所有动作幂等、可无脑重跑、不破坏性（对齐 heal.ps1 原则）。
42 键四类归宿（2026-09-01/02 裁决）：
- install 类 16 行（toolRoot/aria2 按平台分列，dev-rust 已建 rustup 模型）→ ome 原生安装
  （catalog pin 驱动，与 verify 断言一致；rust 为 evergreen 引导器稳定滚动）；
- 密钥载体 dsKey/akKey 与镜像源 bunfig/goproxy → heal-keys.py / heal-mirror.py 原生移植；
- agent 域 12 键休眠（四件套配置归 ohmyagents）；
- 非 ome 域路由（secret-guard 密钥防护、POSIX 残留清零、compileMatrix 编译验收编排、
  POSIX aria2 系统位——不在 ome 自愈范围，只提示不越界）。

mac-* 四键为 ps1 远端路由时代的专列；ome 在 mac 本机原生运行，归一为别名指向普通键。

## Functions

- `heal_bunfig` — bunfig npmmirror：写语义单一权威在 manifest mirror 节（D42 收编，防双份漂移）。
- `heal_keys_carrier` — 密钥载体/端点补齐（heal-keys.py 移植）。
- `run_heal_with` — 跑自愈（流式）：维度动作完成即经 emit 回调输出。

## Types

- `HealRow` — 单次自愈执行结果行。

