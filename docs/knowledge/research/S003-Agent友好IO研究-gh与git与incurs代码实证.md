# S003 Agent 友好 IO 研究：gh 与 git 与 incurs 代码实证

> 研究背景：用户裁决 ome 的 CLI 输入输出应保证 Agent 友好，指定三家参照：GitHub CLI（gh）、
> git clone、douglance/incurs（要求 clone 源码做代码级研究）。本文提炼三家模式、对照 ome 差距、
> 记录吸收裁决与实施结果。

## 一、三家 oracle 的模式提炼

### 1. gh 官方手册

实证来源：cli.github.com/manual/gh_pr_list。

| 模式 | 实证细节 | Agent 价值 |
| --- | --- | --- |
| `--json <fields>` 显式字段选择 | `gh pr list --json number,title,state`，可用字段全量列在手册 JSON Fields 节 | 输出形状由调用方声明，schema 可预测、字段集稳定可枚举 |
| `--jq <expression>` | 内置 jq 过滤，一条命令完成选择投影 | 免管道拼装，agent 单命令取精确值 |
| `--template <string>` | Go 模板格式化（gh help formatting 专文） | 人类可读定制不牺牲数据层 |
| 默认值与值域进选项类型 | `-L, --limit <int> (default 30)`、`-s, --state <string> (default "open")`、值域 `{open\|closed\|merged\|all}` | agent 读 help 即知默认行为，无需试错 |
| 错误与数据分stream | 数据 stdout、错误 stderr，空列表返回 `[]` 而非空输出 | 管道恒可解析 |
| 继承全局选项 | `-R, --repo` 全子命令可用 | 上下文一次声明 |

### 2. git clone 官方手册

实证来源：git-scm.com/docs/git-clone。

| 模式 | 实证细节 | Agent 价值 |
| --- | --- | --- |
| 进度恒 stderr | 「Progress is not reported to the standard error stream」在 --quiet 下的表述；数据与进度分stream | stdout 干净，`2>/dev/null` 即静默 |
| TTY 条件进度 | 默认 attached to terminal 才报进度；`--progress` 强制、`--quiet` 抑制，`--verbose` 与进度正交 | 非交互（agent）环境天然低噪 |
| `--[no-]xxx` 显式否定 | `--no-single-branch`、`--no-tags`、`--no-shallow-submodules` 等 | 布尔默认值不用猜，可显式反向 |
| SYNOPSIS 全签名 | 手册首列完整签名 | agent 一屏读全命令面 |
| 破坏性操作 NOTE | --shared 危险性专段警示 | 高危操作前置声明 |
| `--` 终止选项 | 选项与位置参数显式分界 | 防参数注入歧义 |

### 3. incurs 源码

实证来源：浅克隆 douglance/incurs 逐文件阅读，2026-09-02。

| 模式 | 源码位置 | Agent 价值 |
| --- | --- | --- |
| 多格式同源渲染 | src/Formatter.ts：`Format = toon\|json\|yaml\|md\|jsonl`，一个 format(value, fmt) 纯函数 | 一份结构化产出，多种镜头；jsonl 天然流式 |
| `--json` 为 `--format json` 简写 | src/Cli.ts:2498 extractBuiltinFlags | 调用方肌肉记忆兼容 |
| TTY 自动判别 agent | src/Cli.ts:696 `human = tty && !formatExplicit`；ctx.agent = stdout 非 TTY | 管道下自动切机器格式 |
| 错误尊重格式 | src/Cli.test.ts:1126：--format json 下错误输出 `{code, message}`，exit 1 不变 | 失败也可结构化消费 |
| 机器可读错误四元组加 retryable | src/Errors.ts IncurError：code/message/hint/retryable/exitCode | agent 可程序化分流重试 |
| ValidationError 逐字段 | src/Errors.ts FieldError：path/expected/received/message | 参数错误结构化定位 |
| `--full-output` 信封 | Cli.test.ts:1116：默认出数据本体，全量出 `{ok, data, meta}` | 数据与元数据分层 |
| `--filter-output` 轻量路径选择 | src/Filter.ts：`a.b,c[0,10]` 解析为 Segment 遍历 | 内置投影，不引 jq 依赖 |
| `--token-limit/--token-offset/--token-count` | Cli.ts:2528-2540 | 输出按 token 分页，agent 上下文预算管理 |
| `--llms/--llms-full` 与 `--schema` | Cli.ts:2492-2497 | 命令清单与 JSON Schema 自描述，供 agent 发现 |
| outputPolicy | Cli.ts:538：`all` 与 `agent-only` | TTY 模式抑制数据输出 |

## 二、ome 差距分析与吸收裁决

### 已吸收本批实施

| 裁决 | 来源 | 落点 |
| --- | --- | --- |
| 全局 `--format kv\|json\|jsonl` 三格式，`--json` 为 json 简写且互斥报错 | incurs Format + gh --json | render.rs Format 三态 + main.rs 全局参数 |
| json 整批数组、jsonl 逐块单行（流式与结构化兼得） | incurs jsonl | render.rs emit/finish；status 逐工具即出 |
| stdout 恒为合法 JSON 文档（无数据块输出 `[]`） | gh 空列表语义 | render.rs finish 空批语义 + 测试断言 |
| 结构化模式错误走 stderr 单行 JSON `{code,message,hint}`，stdout 保持纯数据；错误前已产出数据块照常上 stdout（daily exit 2 预览行） | gh 错误恒 stderr 与 incurs 错误尊重格式的合流取舍 | main.rs 错误出口 |
| 结构化富字段：verify 出 name/verdict、doctor 出 check/status/detail（kv 单行形状不动，ohmypwsh 正则消费不受影响） | gh JSON Fields 字段语义化 | cmd_verify/cmd_doctor |
| 值一律字符串，字段序与 kv 行序一致（serde_json preserve_order） | gh 字段序稳定 | Cargo.toml + render.rs block_value |
| `bin_name = "ome"` 钉死 usage 前缀，Windows 不再显示 ome.exe | gh/git 跨平台一致 usage | main.rs clap 元数据 |

### 不吸收与理由

| 候选 | 来源 | 不吸收理由 |
| --- | --- | --- |
| `--jq`/`--filter-output` 过滤 | gh/incurs | jq 已在工具清单；过滤属管道域，CLI 内置投影收益低 |
| `--token-limit/--token-offset` 分页 | incurs | ome 输出体量小（31 工具全量 status 数十行），无分页需求；出现爆量输出时再议 |
| `--llms/--llms-full/--schema` 自描述清单 | incurs | ome 是单命令面 CLI 非框架，clap help 已可自省；引入即维护双份事实源 |
| TTY 自动判别切格式 | incurs | ome 默认 kv 本身即机器可读（行式 key=value 正则友好），无人类排版可切 |
| toon/yaml/md 格式 | incurs | 数据形状简单，kv/json/jsonl 三态覆盖；格式越多测试面越大 |
| `{ok,data,meta}` 信封 | incurs --full-output | ome 数据即数据，退出码与 stderr 已承担信号通道，信封冗余 |
| retryable 字段 | incurs | ome 错误域小（网络/IO/校验），exit code 与 hint 已够分流 |
| MCP/HTTP/OpenAPI 多传输暴露 | incurs | 与定位冲突：远程与集成编排归 ohmypwsh（README 边界） |

## 三、实施记录

1. **render.rs**：`Format{Kv,Json,Jsonl}` 三态 + thread-local 持格式与 json 块累积；`emit` 按格式分流（kv 逐行、jsonl 立即单行、json 累积）；`finish` json 模式整批输出（空批 `[]`）；`header/blank` 结构化模式空操作（# 组标题是 kv 排版件）；`block_value` 纯函数（值全字符串）。
2. **main.rs**：全局 `--json`（conflicts_with format）与 `--format`；run 首行 set_format；错误出口先 finish 再按格式分流（结构化 stderr 单行 JSON，kv 人称行）；verify/doctor 子命令级 `--json` 退役（全局统一，用法不变）；verify kv 单行 `dim=VERDICT` 原样（ohmypwsh `^([\w-]+)=(PASS|FAIL|NA)$` 正则消费不回归），结构化出 name/verdict 块；doctor 结构化出 check/status/detail 块（明细入字段，agent 免读 stderr）；`bin_name = "ome"`。
3. **Cargo.toml**：serde_json 开 preserve_order（默认 BTreeMap 字母序会打乱字段序，与 kv 行序不一致，实证见 status jsonl 首行）。
4. **测试**：render 纯函数四测（kv 行、块转对象值字符串、空组、格式切换）；cli.rs 集成五测（status --format json 合法数组且无 # 标题、--json 简写单对象数组、--format jsonl 逐行对象、结构化错误 stderr 单行 JSON 且 stdout 恒合法文档、--json 与 --format 互斥）。
5. **真机实证**：verify --json 数组、doctor --format jsonl 逐行、pin nonexistent --json 出 `[]` 与 `{"code":"error",...}` exit 1、verify kv 行原样、status jsonl 字段序与 kv 一致。

## 四、遗留与后续

- **（假设）**字段类型化（如 installed 版本号出数值、path 出布尔）暂缓：kv 直转字符串诚实且可解析，出现类型敏感消费再议。
- **（假设）**help 内字段清单（gh JSON Fields 节模式）暂缓：字段名即 kv 键名自描述，README 承担清单职责。
- env root 等上下文信息不进结构化输出（gh 模式数据即数据）；如 agent 需要可用 `--env-root` 自证。

## 五、结论

- **（实证）**三家 oracle 的公约数是：stdout 纯数据可解析、错误可结构化、退出码语义稳定、空结果可预测；差异在格式丰富度与自描述深度。
- **（实证）**ome 本批次达成公约数：三格式全局统一、错误结构化、字段序稳定、跨平台 usage 一致；kv 兼容形状零破坏（ohmypwsh 消费正则实测不回归）。
- **（经验）**不吸收清单的判断依据是输出体量与命令面规模：工具 CLI 的 Agent 友好在「可预测」而非「功能全」。
