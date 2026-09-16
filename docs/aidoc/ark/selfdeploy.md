# ark::selfdeploy

selfdeploy：自部署——复制当前 exe 到用户程序目录（Windows `%LOCALAPPDATA%\Programs\ome`，
Linux / macOS `~/.local/bin`），同步 catalog 到用户数据目录，并注册用户 PATH（幂等）。
Windows 顺带清理旧自部署位 `<EnvRoot>\ome\bin` 的 PATH 残留。
幂等：目标与当前 exe 同路径则跳过复制；sha256 一致则跳过复制；PATH 注册由 envpath 幂等处理。

## Functions

- `deploy_copy` — 复制 exe 到目标（纯文件逻辑，可测）：同路径跳过；sha256 一致跳过；否则覆盖复制。
- `deploy_skill` — 同步 SKILL.md 到用户数据目录（D09：agent 发现入口，自适应生成——本机实装清单与
- `render_skill` — 自适应渲染环境 SKILL（D09）：本机实装依赖（十类分组、名称与版本）、类级使用引导、
- `self_deploy` — Linux / macOS：复制当前二进制到 `~/.local/bin/ark`，同步 catalog，并确保 `~/.local/bin` 在用户 PATH 中。
- `write_skill` — 落盘自适应 SKILL 文本（cmd_skill 用；deploy_skill 的静态骨架仅作 init 兜底）。

## Types

- `SelfDeployOutcome` — 自部署结果。

