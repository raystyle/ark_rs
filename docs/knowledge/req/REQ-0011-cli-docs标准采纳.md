---
id: REQ-0011
title: cli-docs标准采纳
status: implemented
priority: must
trace: cargo test --release --locked 全绿 218 项（新增裸调用与漂移守卫两测）；cargo fmt --check 与 clippy 存量告警面零新增；md 四件套与 PEVO 绿
---

# REQ-0011:cli-docs标准采纳

总台追加单第三单（2026-09-18，用户裁定「所有仓都 review cli-docs 技能，取/改进封一个版本」）。标准权威：project-evo cli-docs skill（甲面 readme-standard 四节骨架、乙面 agent-face 五件、templates 模板）。改进并入 v1.4.0 封版。

## Scenario

ark 对外面（README、--llms 手册、帮助面、裸调用、自省守卫）对照 cli-docs 标准逐件盘点，缺啥补啥、不适用给理由，单一真源不破。

## 对照表

| 件 | 标准 | ark 现状 | 本批改动 | 不适用与理由 |
| --- | --- | --- | --- | --- |
| README 四节骨架 | 项目介绍/部署/配置/使用加可选尾节 | 原八节平铺（安装/install链/catalog 门/清单/镜像/契约/agent 面） | 重构为四节加目录徽章加排障/开发/许可尾节；供给清单归配置、渐进示例归使用、二进制发行与校验进部署、环境变量表新立 | 无 |
| 徽章 | 三枚内 | 无 | CI、Release、License 三枚 | 无 |
| 何时不用 | 边界诚实 | 无 | 项目介绍 NOTE 块补 | 无 |
| --llms 手册节序 | 定位加版本注入/读序/命令/退出码/输出契约，至多 120 行 | 23 行表形，无版本/读序/退出码节 | 重构为标准节序 41 行；版本 env! 注入禁手写 | 无 |
| --llms --json 机器形 | 结构化命令清单 | 无 | 不动 | 手册是 curated 单源 markdown，机器形将成为第二真相；守卫已锁同源，全派生形改造后再上 |
| 旗标七件 | filter-output/format/full-output/help/llms/json/schema | format/json/help/llms 在位，env-root 自有 | 不动 | R013 对外冻结契约：三格式族与字段集冻结（issue #4，2026-09-02）；加旗标属破坏面，另立 major 窗口 |
| 信封 {ok,data,error,meta} | 统一信封 | kv/json/jsonl 数据块加错误四元组 | 不动 | 同上，R013 冻结裁定在案（2026-09-16 tool-cli-agents 对照） |
| 类型化 CTA | meta.cta 结构 | 文本 HINT（status 漂移、doctor verdict） | 不动 | 随信封裁定同不适用 |
| 退出码 0/1/2 | grep 语义族 | 0/1（R013 冻结） | 不动 | R013 冻结 |
| 帮助面头行 | name@version 加一句描述 | clap 默认 name version 分行 | 根命令 help_template 改 name@version 形（版本注入） | 无 |
| 帮助面节序 | Arguments/Options/Examples/Global Options/Env Vars | clap 原生节序，Examples 尾置 after_help | 根头行对齐；其余维持 clap 原生 | clap 节序模板不逐段可排，Examples 尾置与 Global Options 合并是 clap 形；Env Vars 走 README 表（clap 无原生节）；强行外排帮助生成器属自造第二真相 |
| 自省三面同源 | help/llms/schema 从命令树派生禁手维护 | 手册 curated 手维护、无守卫 | 新增漂移守卫集成测：根帮助命令树逐名与关键旗标必须出现在 --llms，版本注入在位断言 | schema 面无（随旗标七件不适用）；守卫盖 help 与 llms 两面（curated 形必配件补齐） |
| 裸调用面 | 不弹交互不纯报错，紧凑形 exit 恒 0 | 报错 exit 1（「缺少子命令」） | 改紧凑导航形：一行定位加一行指引（--llms 与 --help），exit 0，stdout 纯净 | 无 |
| token 三件与输出过滤 | 可选扩展件 | 无 | 不动 | 2026-09-16 裁定在案（输出体量百行级，format 与 llms 已覆盖；体量增长再立项） |

## 判据

- [x] 对照表覆盖甲面四节与乙面五件全部标准件，缺补与不适用皆有理由在表
- [x] 裸调用面：无参 exit 0、紧凑形含 --llms 指引、stdout 纯净（集成测 `裸调用_紧凑导航_exit0`）
- [x] 漂移守卫：命令树与关键旗标在 --llms 在册断言（集成测 `漂移守卫_help与llms同源`）
- [x] --llms 节序与版本注入、120 行内（41 行实测）
- [x] README 四节骨架与环境变量表、徽章、许可尾节
- [x] 门禁：cargo fmt --check、clippy（存量告警面零新增）、test 全绿、md 四件套、PEVO

## 遗留

- 旗标七件与信封与 0/1/2 退出码的标准化迁移：R013 冻结面，须 major 发版窗口与舰队消费面盘点后另立 REQ。
- 手册全派生形（命令树程序化渲染）：漂移守卫已保 curated 安全，全派生待 clap 命令定义迁 lib 面后再议。
