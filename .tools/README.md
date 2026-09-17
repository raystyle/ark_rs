# .tools：项目自定义脚本工具

> 角色：项目管理与操作过程中**按需自定义、临时编写**的 PowerShell、Python、Rust 工具及代码的归档目录，进 git。用完即归档进此目录，不散落仓库根或会话里。

## 使用规则

1. **归档时机**：会话中为完成某操作临时写出的脚本，若具备复用价值（第二次会用到的），当轮收尾前移入 `.tools\` 并加 PEP 723 头（Python）或用法注释（ps1）；纯一次性的留在对话里不进仓。
2. **Python 统一 uv 载体**：脚本头部带 `# /// script` 内联元数据，运行用 `uv run --script .tools\xxx.py`（不建 venv、不装依赖进环境）。py 选库走 `docs\references\R008-项目工具Python库选型细则-pypi与uv.md`（关键词搜索只有网页；已知名字走 PyPI JSON API 加 pypistats）；ps 模块选型走 `docs\references\R009-项目工具PowerShell模块选型细则-psgallery与psresourceget.md`。
3. **命名**：小写连字符加用途动词或名词（`md-ref-scan.py`、`md-replace.py`）；ps1 同风格；Rust 专用工具（若有）先 `cargo new` 独立子目录再入 `.tools\`。
4. **工具自述**：每个脚本 docstring 写清用法、参数、退出码；改动同步本 README 清单。
5. **门禁联动**：`md-ref-scan.py` 在文档结构大改（改名、编号、移目录）后必跑；退出码非 0 即有断链，先修后提交。

## 工具清单

| 工具 | 用途 | 用法 |
| --- | --- | --- |
| `import-catalog.ps1` | 已退役（D18）：catalog 是唯一权威，不再对照外部 psd1；运行提示后退出 0 | `pwsh -NoProfile -File .tools\import-catalog.ps1` |
| `seed.py` | 软件资产与 ark 产物对 env.ohmygh.com（R2）种子同步：域面 diff（`--plan` 只读无凭据）与 rclone 上传（路线 A 加 B，D27；catalog 三件套归 ohmycloud catalog-seed，B 承接 R015 五）；D41 B 起自产段配 ark-* 主名（`--ark-dev`/`--ark-stable`）；ome/ 段与 ome-* 兼容写已撤（全舰队 ome 水位清零，2026-09-14 收口）；ark-stable 全 skip 零上传落 WARN 加补推指引（draft 窗口防静默丢段，2026-09-17 补审 F2） | `uv run --script .tools/seed.py [--plan\|--ark-dev --tag dev\|--ark-stable --tag <v*>]` |
| `catalog-sign/` | 云端清单 minisign 签名工具（Rust 子工程，D34）：keygen / pubkey / sign / verify；私钥只在本机与 CI 密钥库，公钥进仓库并内嵌 ome | `cargo run --manifest-path .tools/catalog-sign/Cargo.toml -- sign -s <sec> -m catalog/tools.toml` |
| `inject-guide-d25.py` | 已注入完毕留档（D25；guide 内容随 catalog 上云，注入目标仓库路径 D37 退役，guide 字段 D50 起数据面保留） | `uv run --script .tools/inject-guide-d25.py` |
| `inject-probe-d28.py` | D28 探测字段注入：catalog 各节插 probe_pattern/probe_args（幂等；值迁自 toolver.rs 原 match 表，已注入完毕留档） | `uv run --script .tools/inject-probe-d28.py` |
| `seed-inventory.py` | 自 catalog 抽出 ohmycloud 镜像可入镜对象（R014） | `uv run --script .tools/seed-inventory.py`；`--json` 机读 |
| `md-ref-scan.py` | 全仓 markdown 仓内路径引用断链扫描（结构大改后的回归门禁） | `uv run --script .tools/md-ref-scan.py [--root docs] [--allow 豁免.txt]`；退出码 0/1 |
| `md-heading-scan.py` | 标题括号规范扫描（G001 标题干净的机检项；代码围栏内的注释不计） | `uv run --script .tools/md-heading-scan.py [--root docs]`；退出码 0/1 |
| `mdcharlint.py` | 禁用字符机检（G001 v2 四类硬禁令：破折号、箭头、emoji、非法全角；豁免围栏/行内代码/链接目标/裸 URL；默认跳过 diary 与 proven 历史归档） | `uv run --script .tools/mdcharlint.py <文件或目录>... [--all]`；退出码 0/1 |
| `md-replace.py` | 中文与反斜杠路径安全的字面批量替换（规避 sed 转义坑） | `uv run --script .tools/md-replace.py --glob 'docs/**/*.md' --map 映射.txt [--dry]` |

## 历史注记

- 三个 md 脚本自 ohmyagents `.tools\` 原样平移（2026-08-31），其首发由 ohmyagents 文档整编中的内联脚本正式化。
