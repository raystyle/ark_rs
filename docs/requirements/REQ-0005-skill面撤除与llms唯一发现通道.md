---
id: REQ-0005
title: skill面撤除与llms唯一发现通道
status: draft
priority: must
trace: null
---

# REQ-0005:skill面撤除与llms唯一发现通道

## Scenario

agent 发现通道三件并存（根 SKILL.md 静态件、`ark skill` 自适应渲染落盘、`--llms` 紧凑命令图），维护面大且 SKILL.md 命令图与 manifest 双份并行必漂移；用户裁定 2026-09-16「有 --llms 就不需要 skill 命令，--llms 是给 agents 的紧凑说明书」。撤 skill 整面，`--llms` 头部吸收何时用纪律行后成为唯一 agent 发现通道（D50，兑现 D09 预留的命令面收敛另批）。

## Criteria

验收判据，可检验、可勾选：

- [ ] `ark skill` 子命令撤除（Commands 变体、分派臂、cmd_skill），调用报 clap unrecognized subcommand 形态退出码 2
- [ ] selfdeploy 三函数撤除（deploy_skill 与 write_skill 与 render_skill），根 SKILL.md 删除，编译零残留
- [ ] init 幂等清理数据目录旧 SKILL.md（remove_legacy_skill，在则删不在静默，失败降级 WARN 不拦部署）
- [ ] `--llms` 头部吸收何时用纪律行（一律走 ark、幂等检测安装、镜像优先回落官方、有锚必校验），manifest 变单源自持
- [ ] guide 四字段（desc 与 guide_env 与 guide_dirs 与 guide_notes）schema 与访问器保留为数据面，注释改口径，代码零改动
- [ ] tests/cli.rs llms 断言改何时用纪律行并加 not contains；tests/real.rs 缺席名单加 ark skill
- [ ] 文档同步：README 与 llms.txt 与 R013 与 R001 与 lib.rs crate doc；PRD D50 行；CHANGELOG Unreleased；diary 记钩子；cargo aidoc 再生成同提交
- [ ] 门禁全绿：fmt 与 clippy 与 test --release --locked、aidoc --check --strict、md 四件套、check.py

实现后回填 frontmatter 的 trace（测试路径或验收命令），状态改 implemented。
