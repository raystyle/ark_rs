# R001：catalog 数据模式，tools.toml 字段与 pin 语义

> tools.toml 字段契约（D37 完全解耦 2026-09-10 终版）：权威数据面在 ohmycloud catalog-seed（仓内 catalog/tools.toml 加云端三件套，minisign 签名）；ark 仓持本契约与消费逻辑，权威件已退役为云端消费（tests fixtures 为测试夹具）。pin 字段由数据面维护；端上 `ark pin` 为**临时本地锁**（下次 sync 被云端覆盖），`ark update` 对齐云端 catalog 锁定安装**不回写**（D51 起云端最新定义指镜像与 catalog；锁定单源归数据面）。
>
> 术语（D41 定档）：**泊位（berth）**指 EnvRoot 内某工具的安装位（`dir` 字段解析所得目录）；EnvRoot 为泊位根，物理目录不随更名动。自管条目仅 `ark` 单条目（`ome` 过渡条目已随 ome 关键字剔除批收口，2026-09-18），引擎仅认 `ark-self`（云端权威 catalog 单值），条目演化归 omc 数据面。

## 一、文件级约定

1. 路径：`catalog\tools.toml`，UTF-8 无 BOM。
2. 工具顺序即安装/更新顺序（节序即类序，见 taxonomy）。
3. 每个工具一节 `[tools.<名>]`；字段分两组：静态元数据在前、pin 字段在后。
4. 可选字段为空时整行省略，不写空字符串。

## 二、字段表

### 静态元数据

> 转换器写入，ark 不回写。

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `category` | string | 九类 taxonomy（节序即类序）：agent 智能体依赖（claude/codex/grok/kimi）/ base 操作编排依赖（herdr；自管条目 ark）/ runtime 运行时依赖 / runtime-manager 运行时管理器依赖（uv/fnm）/ compiler 编译器依赖 / mux 多路复用依赖 / service 远程服务依赖（openssh）/ security 密钥安全管理（age/sops）/ cli 命令工具依赖（含 hst，原 oma）。derived 运行时衍生在册 browser-harness 与 omc。旧值 key/project/extras 仅为转换期兜底 |
| `deploy` | string | Deploy.win：envroot / installer / official |
| `dir` | string | EnvRoot 下安装目录（official 工具可省） |
| `bin` | string | 注册进用户 PATH 的目录，相对 EnvRoot（official 可省） |
| `exe` | string | 版本探测 exe 路径，相对 EnvRoot；official 可含 `%VAR%` 环境变量 |
| `extract` | string | 解压/安装方式：zip / targz / targz-bin / tarxz-bin / zip-bin / zip-dir（Windows 版本目录树不展平，zig 用）/ targz-dir / tarxz-dir / copy / gsudo / 7z-extra / 7zsfx / msi / rmux / single / vsbuild（见五） |
| `repo` | string | GitHub 仓库 `owner/name`（纯 cdn 工具可省） |
| `desc` | string | 一行用途说明（D25 入册；D50 起数据面保留，真源云端 ohmycloud，ark 侧无渲染消费者） |
| `guide_env` | string[] | 引导提示的环境变量键清单（用户级优先、进程级兜底；含 TOKEN/KEY/SECRET/PASSWORD 的键只显在否不显值，凭据纪律；D50 起数据面保留） |
| `guide_dirs` | string[] | 引导提示的安装/数据目录清单（支持 `~` 与 `%VAR%` 展开；D50 起数据面保留） |
| `guide_notes` | string | 使用注意事项多行（版本语义、升级例外、PATH 特性等静态知识） |
| `tag_prefix` | string | tag 前缀，剥离后得 version（如 `v`、`release-`） |
| `asset_pattern` | string | 资产名匹配正则（GitHub release 资产筛选） |
| `version_pattern` | string | 可选，从资产名提取版本的正则（python 用） |
| `probe_args` | string[] | 可选，已装版本探测参数（D28 自 toolver 源码表迁字段）；缺省 `["--version"]`；特例：7z `--help`、rmux/openssh `-V`、oscdimg 空数组（无参读横幅）、vsbuild `-version`、zig/go/lightpanda 子命令 `version` |
| `probe_pattern` | string | 可选，已装版本探测输出正则（取第 1 捕获组）；凡平台在管（`exe`/`linux_exe`/`mac_exe` 任一在位）的工具必填，由 `tests/catalog_lint.rs` 结构机检拦漏 |
| `cdn_url` | string | 可选，直链模板，含 `{version}` 占位（dotnet/oscdimg；golang 与 ziglang 不走 GitHub Releases，Windows 侧同 mac 走 go.dev/dl 与 ziglang.org/download 直链） |
| `cdn_index_url` | string | 可选，HashiCorp 式 index.json（夹具仍用 vault 样例；生产 catalog 已无 vault 节） |
| `cdn_asset_pattern` | string | 可选，cdn 系资产名正则 |
| `cdn_version_url` | string | 可选，cdn 系版本查询地址（当前无在用工具，为后续 cdn 系工具预留） |
| `sums_asset` | string | 可选，官方统一校验清单资产名模板（`{version}`/`{tag}` 占位） |
| `sums_pattern` | string | 可选，校验清单内匹配本工具资产行的正则 |
| `asset_sha_suffix` | string | 可选，逐资产校验文件后缀（如 `.sha256`） |
| `bootstrap_asset` | string | 可选，安装自举资产（7z 的 7zr.exe） |

### Linux 平台专属字段

> 缺失时回退到同名的通用字段。ark 不回写。

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `linux_repo` | string | 可选，Linux 专属 GitHub 仓库（通常与 `repo` 相同） |
| `linux_asset_pattern` | string | 可选，Linux 资产名匹配正则 |
| `linux_dir` | string | 可选，Linux 安装目录（可含 `~` 与 `$VAR`） |
| `linux_bin` | string | 可选，Linux 下注册进 PATH 的目录 |
| `linux_exe` | string | 可选，Linux 下版本探测 exe 路径（相对 `linux_dir`） |
| `linux_extract` | string | 可选，Linux 解压方式；支持 `targz-bin`（从 tar.gz 查找二进制）、`tarxz-bin`、`zip-bin`（从 zip 按叶子名取二进制）、`targz-dir` / `tarxz-dir`（全量解压不展平，版本目录/运行时树用） |
| `linux_extra_bins` | string | 可选，补充二进制叶子名（空格/逗号分隔）：`*-bin` 提取与 zip/targz 的 chmod 除 exe 外还覆盖这些成员（如 age-keygen、sg、uvx） |
| `linux_sums_pattern` | string | 可选，Linux 校验清单行匹配正则 |
| `linux_asset_sha_suffix` | string | 可选，Linux 逐资产校验文件后缀 |
| `linux_bootstrap_asset` | string | 可选，Linux 安装自举资产 |
| `linux_cdn_url` | string | 可选，Linux 直链模板，含 `{version}` 占位；Linux 下优先于 `cdn_url` |
| `linux_cdn_asset_pattern` | string | 可选，Linux 的 cdn 索引资产名正则（vault 类各平台资产名不同） |

### macOS 平台专属字段

> 仅 darwin 构建生效；缺失时先回退同义 `linux_*` 字段、再回退通用字段（M1 设立，2026-09-01）。ark 不回写。
> 字段集与 Linux 族同名对称（`mac_*` 前缀，含 `mac_cdn_url`/`mac_cdn_asset_pattern`/`mac_extra_bins`）；语义同上表对应项。
> exe 双语义（`toolver::exe_path`）：有平台专属 exe（`mac_exe`/`linux_exe`）时 exe 相对 install_dir，
> 回退通用 `exe` 时为 Windows 名录风格：路径自带 dir 段、相对 EnvRoot 直接拼。
> `bin`/`exe` 支持 `{version}` 占位（D43 zig 版本目录布局）：探测 glob 取 semver 最大在位版本、装后验证与 PATH 注册按解析版本定版；`dir` 不支持占位（引擎显式拒绝）；分隔符双写法（反斜杠与正斜杠）均接受。

### pin 字段

> ark update/pin 回写，保序保注释用 toml_edit。**按平台分列（M0 起）**：通用四键即 Windows pin，
> Linux/macOS 各有 `linux_*` / `mac_*` 四键，互不覆盖、无跨平台回退，回写只动当前平台四键。

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `tag` | string | Windows 锁定的 release tag |
| `version` | string | Windows 锁定的版本号 |
| `asset` | string | Windows 锁定的资产文件名 |
| `sha256` | string | Windows 锁定资产 sha256，大写；未回填可省略 |
| `linux_tag` / `linux_version` / `linux_asset` / `linux_sha256` | string | Linux 侧对应四键（回源自 psd1 Pos 侧） |
| `mac_tag` / `mac_version` / `mac_asset` / `mac_sha256` | string | macOS 侧对应四键（回源自 psd1 Pos 侧 AssetMac/Sha256Mac） |
| `hold` | bool | 可选，版本锁定（静态元数据、跨平台生效）：true 时 `update`（含 `--force`）、`pin`、带版本选项的 `install` 全部跳过；无选项 `install` 仍按 pin 幂等。解锁 = 删该字段。首个用例 bun 1.3.14（最后一个完全用 Zig 编写核心的版本，2026-09-01 用户裁决） |

## 三、示例

```toml
[tools.age]
category = "tool"
deploy = "envroot"
dir = "age"
bin = "age"
exe = 'age\age.exe'
extract = "zip"
repo = "FiloSottile/age"
tag_prefix = "v"
asset_pattern = '^age-v[0-9.]+-windows-amd64\.zip$'
linux_dir = "~/.local/bin"
linux_bin = "~/.local/bin"
linux_exe = "age"
linux_extract = "targz"
linux_asset_pattern = '^age-v[0-9.]+-linux-amd64\.tar\.gz$'
tag = "v1.3.1"
version = "1.3.1"
asset = "age-v1.3.1-windows-amd64.zip"
sha256 = "C56E8CE22F7E80CB85AD946CC82D198767B056366201D3E1A2B93D865BE38154"
```

## 四、语义要点

1. **唯一 pin 源**：tag/version/asset/sha256 只存在于本文件；解析最新版只读不写，`pin`/`update` 才回写。
2. **sha256 校验优先级**：pin 的 sha256 > 官方校验源（sums_asset / asset_sha_suffix / cdn_index_url 自带 SUMS）；安装成功后可回填空 sha256。清单本体另有 **minisign 签名**（D34，2026-09-10）：sha 边车只证传输与缓存，签名才证来源（公钥内嵌 ark，`.minisig` 随云端清单一并分发）；自举路径同样强校验（镜像优先，官方 raw 只在镜像不可达时兜底且不验签，会打 WARN）。
3. **版本变更清 sha**：pin 到不同 version 时清掉旧 sha256，等 install 回填。
4. **平台边界**：Windows 字段为默认；平台专属字段以 `linux_` / `mac_` 前缀并列，静态字段 Linux 取 `linux_*` 回退通用，mac 取 `mac_*` 回退 `linux_*` 再回退通用；pin 字段按平台分列无回退（平台无 pin 即未锁定，`install` 不带 `--latest` 会提示先 pin）。sha256 随当前平台安装的 asset 回填到本平台键；本平台 pin 的 asset 与解析资产不一致时，该 sha256 不当作校验基准。
5. **数据主权（M0，2026-09-01；D18 确认）**：本文件是唯一权威。历史 psd1 回流已完成，不再对照外部 catalog。
6. **平台不适用（2026-09-01）**：单平台工具是常态数据形状（shellcheck 仅 `linux_*`、aria2/git 仅 Windows）。当前平台 effective exe 缺失即「平台不适用」：status 出空态行（installed 与 exe 渲染为 -）、install/update/pin/query 跳过（见 M106 M003）。
7. **清单来源与部署态纪律（M0 定案；D33 与 D37 两次升格）**：**权威清单在 ohmycloud catalog-seed**（其仓 `catalog/tools.toml`，遵循本契约格式），云端 `env.ohmygh.com/ark/catalog/`（D41 起主键，兼容读 `ome/catalog/`）三件套（清单、`.sha256` 边车、`.minisig`）由 omc 流水发布；本仓已不持有权威件（D37 终态），本仓 `catalog/` 只留 tests fixtures，清单消费一律走 `ark catalog sync`、TTL 自动刷新与裸端自举（三重门：边车锚加解析加内嵌公钥验签）。端上用户数据副本是运行态落点，`ark init` 与 `self update` 的同步只是兜底，内容以云端为准。部署态 pin 回写（`ark pin`）是**临时本地锁**，下一次云端刷新即覆盖，`ark update` 不回写锁定（D51 起对齐 catalog pin 安装）；pin 变更须在 omc 数据面改并由其流水发布（R015 五）。

## 五、入册 checklist

> 新工具入册（D28 清单化，2026-09-09）：catalog 节是唯一要写的清单，漏项有机检。

1. **静态字段**：`category`/`deploy`/`dir`/`bin`/`exe`/`extract`/`repo`/`asset_pattern` 按三平台族补全；平台空态用字段缺席表达（win 空态不写通用 `exe`，见 M003 平台不适用容忍）。`asset_pattern`/`sums_pattern` 对 release 资产清单逐字核对命中，命名惯例不可推断（claude linux 是 x64 非 x86_64，ohmyenv-rs#10 / M014 同型；机检「pin 资产名必被 pattern 命中」在 catalog_lint 常态拦截）。
2. **探测字段**：`probe_pattern` 必填（在管即必填，机检红灯）；参数特例写 `probe_args`；正则必须含第 1 捕获组，期望值来自工具真实输出样例（`toolver.rs` 单测同源）。
3. **pin 四键同 tag**：tag/version/asset/sha256 同一 release；sha 与官方校验源（sums 清单 / 逐资产边车 / GitHub digest）逐字核验，多架构同名族资产逐行核对架构（M014 跨行错配的防复发）。
4. **sha 格式**：64 位 hex（机检红灯）；回填统一大写。
5. **装后验证**：真机 `ark install` 幂等二连、`ark status` 三态齐；探测不过即查 probe 字段（不再需要查源码表）。
6. **镜像对账**：pin 落库后跑 `uv run --script .tools/seed.py --plan` 域面 diff，sha-drift / sidecar-missing 即时暴露（M014 正解：机检替代人眼）。
7. **计数同步**：AGENTS 两处、README 三处（含类表）、INDEX、catalog 头注释。
8. **云端可见性**：pin 与节入仓库推 main 后由 seed-mirror 路线 B 自动入镜（无 sha 不入镜，D34 起同时发布 `.minisig` 签名件，缺密钥即拒发）；部署机随 TTL 自动取得，需立即生效跑 `ark catalog sync`，用 `ark catalog status` 核对 `synced=true` 与 `signature=valid`。

## 六、evergreen 条目

> `extract = "vsbuild"` 型条目（当前仅 vsbuild）不走「版本解析、sha 校验、pin 回写」主流程，规则如下。

1. **无 pin 字段**：tag、version、asset、sha256 整组省略，源是永续直链（aka.ms 引导器），无可锁版本、无官方 sha。`pin`/`update` 对其跳过（提示 evergreen），`query` 只报直链与 evergreen 标记。
2. **安装幂等语义**：cl.exe 在位（`<EnvRoot>\vsbuild\VC\Tools\MSVC\*\bin\Hostx64\x64\cl.exe`）即视为已装，只补机器 PATH；重装等于修复。
3. **需管理员**：未提权且 gsudo 在位时经 gsudo 重跑 `ome install vsbuild`（退出码透传）；两者皆无则报错给出两条出路。
4. **PATH 写机器级**（HKLM，REG_EXPAND_SZ）：MSBuild 与 cl.exe 两个目录，非用户级；status 的 path 态按机器级判定。
5. **版本探测**：`exe` 指向跨版本稳定的 MSBuild.exe，`MSBuild -version` stdout 首行裸版本号（如 17.14.51.32402）取前三段。
6. **Windows 专属**：非 Windows 平台安装即报错。
7. **边界**：Windows SDK 不随 VS 组件（ISO 分离装 Windows Kits），不进 ark。
