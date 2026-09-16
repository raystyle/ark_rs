# ark::selfdeploy

selfdeploy：自部署——复制当前 exe 到用户程序目录（Windows `%LOCALAPPDATA%\Programs\ome`，
Linux / macOS `~/.local/bin`），同步 catalog 到用户数据目录，并注册用户 PATH（幂等）。
Windows 顺带清理旧自部署位 `<EnvRoot>\ome\bin` 的 PATH 残留。
幂等：目标与当前 exe 同路径则跳过复制；sha256 一致则跳过复制；PATH 注册由 envpath 幂等处理。

## Functions

- `deploy_copy` — 复制 exe 到目标（纯文件逻辑，可测）：同路径跳过；sha256 一致跳过；否则覆盖复制。
- `remove_legacy_skill` — 清理数据目录已部署的旧 SKILL.md（D50 撤 skill 面的幂等收尾；仿 ome 别名清理模式）。
- `self_deploy` — Linux / macOS：复制当前二进制到 `~/.local/bin/ark`，同步 catalog，并确保 `~/.local/bin` 在用户 PATH 中。

## Types

- `SelfDeployOutcome` — 自部署结果。

