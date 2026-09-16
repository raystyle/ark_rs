# ark::toolver

toolver：已装版本探测，移植 helpers.ps1 的 Get-InstalledVersion（888-935 行）。
探测参数与输出版本正则自源码硬编码表迁 catalog 字段（D28 入册清单化：每工具
`probe_args` 缺省 `["--version"]`、`probe_pattern` 取第 1 捕获组；字段契约见 R001），
新工具入册漏带由 tests/catalog_lint.rs 结构机检拦下，不再靠装后实测暴露。
exe 路径解析：official 工具 exe 字段含 %VAR% 环境变量（展开为绝对路径），
其余相对 EnvRoot 拼接。装后读版本带 5 次递增重试（500ms * i，对齐 Install-ToolVersion 末尾）。

## Functions

- `exe_path` — exe 路径解析：official（exe 使用环境变量或绝对路径）展开为绝对路径；
- `exe_path_for_version` — 定版形态（D43）：占位以给定版本直替换（装后验证锚定刚装版本，glob 取 max 在
- `expand_env_vars` — 环境变量展开（Windows `%VAR%`；Linux / macOS `$VAR` / `${VAR}`）。
- `find_on_path` — PATH 上查找命令（D07 agent 存量纳管判定）：返回首个命中位的完整路径，未命中 None。
- `installed_version` — 探测已装版本：exe 不存在直接 None；运行 exe 取首行非空输出按 probe_pattern 解析。
- `installed_version_retried` — 装后版本读取：5 次递增重试（500ms * i），对齐 Install-ToolVersion 末尾的重试循环
- `is_official` — 工具是否 official 布局（exe 使用环境变量或绝对路径，installDir/bin 走官方目录，不进 EnvRoot）。
- `parse_version` — 解析版本（纯函数）：正则取 catalog `probe_pattern` 字段（第 1 捕获组）；
- `pin_drift` — 漂移判定（纯函数）：None 探针（未装）按 Behind 处理（补装自愈）。
- `platform_managed` — 当前平台是否管理该工具（平台不适用时 status 出空态行、install/update/pin/query 跳过）：
- `probe_args` — 版本探测参数（D28 自源码表迁 catalog `probe_args` 字段；对齐 Get-InstalledVersion 的 switch）。
- `status_drift_hint` — status 的 drift 判定（D49 尾统一口径）：npm-tgz 类不列（版本真源在 npm registry

## Types

- `PinDrift` — update 的锁定漂移三态（D49）：installed 探针值对 catalog pin 的对照。

