# S001-incurs选型研究-不迁移只吸收三模式

> 2026-08-31，ome 收尾命令阶段（status/daily/self-deploy）设计时的 CLI 框架选型调研。

## 背景

ome 八命令 CLI 用 clap 4 derive 手写。调研社区是否存在更贴合「人/智能体双读」的 CLI 框架，
候选 incurs（Rust）。裁决：不迁移，只吸收三个模式。

## 关键结论

1. 不引入 incurs：它是 douglance 的个人移植项目而非 wevm 官方 Rust 移植，体量与稳定性信号弱
   [实证: 2026-08-31 crates.io 与 GitHub 抓取，见下]；且违反 ome AGENTS 写作编码规则 7
   「禁止为风格引入冷门或实验 crate」。
2. 吸收模式一，机器可读错误结构：code/message/hint/exit_code 四元组，落为 `src\omerr.rs`
   [实证: 已实现，daily 的 exit 2 走 exit_code 通道]。
3. 吸收模式二，handler 结构化产出加单一渲染层：命令产出收敛为 Vec<(key, value)>，统一经
   `src\render.rs` 输出 key=value [实证: 已实现]。
4. 吸收模式三，示例与提示元数据驱动帮助：子命令示例集中为常量，经 clap after_help 挂入
   [实证: 已实现，main.rs 顶部 EX_* 常量]。
5. 附带落一条输出纪律：stdout 只走数据、stderr 走人称提示，写进 main.rs 顶部注释
   [实证: 已实现并执行]。

## 现状或实测

- incurs 是 wevm/incur（TypeScript）的 Rust 移植，作者 douglance 个人项目，非 wevm 官方
  [实证: 2026-08-31 https://crates.io/crates/incurs 与 https://github.com/douglance/incurs 抓取]。
- 体量信号：MIT；最新 0.5.3；总下载 845；GitHub 1 star；22 天跨 4 个 minor 的 breaking 节奏
  [实证: 2026-08-31 同上来源抓取]。
- 依赖面：默认拉 tokio、schemars、tiktoken-rs 共 14 个非可选依赖 [实证: 2026-08-31 crates.io
  依赖清单抓取]。
- 核心价值与 ome 的匹配度：schema-first 命令图多传输暴露（CLI/HTTP/MCP）ome 用不上
  [推断: ome 只有本机 CLI 一个传输面]；结构化输出默认 TOON 而非 ome 契约的 key=value
  [实证: 2026-08-31 incurs README/文档抓取]；IncurError 四元组与 exit_code、CTA 下一步提示
  可平移 [实证: 已平移为 omerr.rs 与 render.rs]。

## 踩坑沉淀

无坑；属多方案对比取舍（保留 clap 4 derive，拒绝换框架）。

## 待办

无。若 incurs 未来转入 wevm 官方或下载量量级跃升，可重估 [假设: 短期内不会发生]。
