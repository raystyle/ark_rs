---
id: ADR-0004
title: PATH 与注册表错误模式归档
status: accepted
date: 2026-09-15
deciders:
  - 用户（ADR-0001 批四裁定：mistakes 并入 ADR）
supersedes: []
superseded_by: null
tags: [mistakes-archive]
---

# ADR-0004 PATH 与注册表错误模式归档

## Context

用户 PATH 与注册表面的行级错误史（HKCU、展开、去重、广播）（原 mistakes 分类文件 M103-PATH与注册表-错误.md，ADR-0001 批四并入）。M009/M010/M011/M027 四类错误模式。错误史封存不改写，同根因聚合条目保留最早编号。

## Decision

错误模式与正解纪律以本 ADR 附录表为唯一权威；新踩坑当场记 diary，构成纪律变更的立新 ADR（M0xx 行级编号制度随本批退役）。

## Consequences

- 查错误模式从 `docs/knowledge/adr/README.md` 入（编号 M0xx 在附录表内仍可检索）。
- 历史 diary 中 M0xx 引用为当时事实记录，不改写。
- 附录原表封存：新错误不再接编 M0xx，以 diary 与新 ADR 承载。

## 附录 原始条目表

> M103-PATH与注册表-错误

> 用户 PATH 注册、profile 标记块、死链判定错误速查。

## 行级条目

| 编号 | 日期 | 状态 | 现象 | 根因 | 正确处理 |
| --- | --- | --- | --- | --- | --- |
| M009 | 2026-09-07 | 已修正 | Linux/mac `ome deploy all` 后 profile 里 ome PATH 块只剩最后一个目录，go/pwsh/zig 与 `~/.local/bin` 互覆盖 | `add_user_path` 删除整个 `# >>> ome PATH` 块再写一条 export | 块内多行 append/去重；`remove_user_path` 只删该目录，空块才撤标记 |
| M010 | 2026-09-07 | 已修正 | 默认 EnvRoot `D:\ohmyenv` 时 doctor 把 `D:\ohmyenv-rs` 仓库 PATH 当死链 | 死链判定 `starts_with` 字符串前缀 | 用 `Path::starts_with` 按路径分量比较 |
| M011 | 2026-09-07 | 已修正（2026-09-10 增补同型） | 用户 PATH 已有 `ffmpeg\bin`，当前终端 `ffmpeg` 仍找不到。2026-09-10 增补：用户报「`D:\ohmyenv` 下几十个子目录都注册了 PATH，唯独没加 `typst`」，实测注册表首条已是 `D:\ohmyenv\typst` 且以注册表 PATH 起新进程可解析 `typst 0.15.1` | 写 HKCU 后未广播 `WM_SETTINGCHANGE`；条目已在注册表时不再注入本进程 PATH。增补：广播只对 Explorer 与后续新进程生效，**已打开的终端保持自身快照**（install 内的 `set_var` 只改 ome 进程环境，改不到父 shell），故「已装工具在旧终端看不到」是语义而非注册缺失 | 写入后广播环境变更；已存在也补当前进程 PATH；装完提示新终端生效。增补核实口径：先读注册表 `HKCU\Environment\Path`（含条目即注册成功）再以注册表用户加机器 PATH 起新进程验证解析，避免把旧终端快照误判为写入失败 |
| M027 | 2026-09-13 | 已修正 | 用户实弹：`ark install fnm` 写进 ~/.bashrc 的钩子是裸 `eval "$(fnm env)"`，执行序在 PATH 含 ~/.local/bin 之前，登录 shell 直接报 `Command 'fnm' not found` | 钩子内容依赖执行时 PATH 顺序，而 rc 文件里 ark 的 PATH 块不能保证先于 fnm 钩子执行；且旧 `ensure_profile_hook` 幂等判据是「全文含该行」而非块体感知，块体演进只会追加新块、存量旧块体永不升级。codex 对线时已点过「钩子加 command -v 守卫」nit 未整改 | 钩子块自含 PATH 导出（`export PATH="$HOME/.local/bin:$PATH"`，不依赖 rc 相对执行序）加 `command -v` 守卫 if 形态（缺席静默且不污染 source 退出码）；块引擎改块体感知 upsert（`merge_hook_block` 纯函数：块在位而块体不一致原位重写、旧 ome 标记迁移、重复块收敛、块外原文不动），存量端由重装自愈，另有 omc bootstrap 兜底 |

## 范围注记

- Windows HKCU 比较需展开 `%VAR%` 并去掉尾斜杠，与 `add_path_entry` 同一套归一。
