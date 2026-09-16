# ark::render

render：单一渲染层（吸收自 incurs 的「handler 结构化产出 + 单一渲染层」模式，S003 扩展三格式）。
纪律：stdout 只走数据，人称提示（[INFO]/[OK]/[WARN]/[HINT]）一律 stderr。
命令产出统一收敛为 Vec<(key, value)> 块交本层输出，不在命令里散写 println!。

格式三态（`--format`，`--json` 为 json 简写）：
- kv（默认）：key=value 逐行，块间空行，分组走 `#` 注释行（header）；
- json：块累积为对象数组，命令收尾 finish 一次性输出（stdout 恒为合法 JSON 文档）；
- jsonl：每块立即输出一行 JSON 对象（流式与结构化兼得，status 逐工具即出）。

值一律字符串：kv 行直转 JSON 值，类型不伪装（gh 的字段类型化留待需要时再做）。

## Functions

- `blank` — 工具组分隔（多工具输出之间的空行；结构化模式下为空操作）。
- `block_value` — 把一组 key=value 转为 JSON 对象（纯函数，便于测试；值一律字符串）。
- `current_format` — 当前格式。
- `emit` — 输出一组 key=value 到 stdout：kv 逐行打印；jsonl 立即单行 JSON；json 累积待 finish。
- `finish` — json 模式收尾：把累积的块输出为一个 JSON 数组（空批输出 []，与 gh 空列表语义一致）。
- `format_rows` — 把一组 key=value 格式化为文本块（纯函数，便于测试）。
- `header` — 组标题：以 `# ` 前缀注释行输出到 stdout（机器可滤，人可读分组）；结构化模式下为空操作。
- `is_structured` — 是否结构化输出（json/jsonl）：错误出口据此切换 stderr 单行 JSON。
- `set_format` — 设定输出格式（main 解析完全局 --format/--json 后调用一次）。

## Types

- `Format` — 输出格式。

