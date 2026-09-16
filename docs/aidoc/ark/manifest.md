# ark::manifest

manifest.toml：安装配置部署逻辑的数据面（R016 B 层，D39 第一波引擎）。

结构：`schema_version` 加每工具一节 `[manifest.<tool>]`，含 L1 声明原语
（`env_set` 用户级键值表、`shims` 别名表、`mirror` 镜像源节 D42）与 L2 受控命令
（`post_install` 分平台 argv 数组——每条是参数数组非 shell 字符串，无元字符解释）。

生命周期：与 tools.toml 同目录（catalog sync 顺带拉取三件套，同锚同签）；
文件或工具节缺失时零原语、零动作（内建双轨已于 2026-09-11 撤除，omc 数据面为唯一来源）。
高 `schema_version` 拒载并提示升级 ome（R016 前进兼容红线）。
mirror 节不升 schema（serde 容忍未知字段，旧引擎静默忽略零动作，前向兼容同红线）。

## Functions

- `apply_env_set` — L1：应用用户级环境变量（逐键幂等）。键值先过形态校验（与 mirror 同一红线：
- `apply_mirror` — L1：镜像源节落源（幂等）。env 先行（post_install 子进程继承，FNM_NODE_DIST_MIRROR
- `apply_shims` — L1：生成别名（win=硬链接加 `.cmd` 兜底、POSIX=符号链接；目标已存在即跳过，幂等）。
- `cargo_home_path` — CARGO_HOME 解析：进程环境变量 > 用户级变量（win 注册表 / POSIX profile 块，
- `ensure_bunfig` — bunfig registry 落 `~/.bunfig.toml`（整文件；含本 URL 即不重写，heal-mirror 同语义；
- `ensure_cargo_config` — 生产入口（cargo）。
- `ensure_cargo_config_in` — cargo 镜像配置落 CARGO_HOME/config.toml（整文件内容比对；rsproxy 全量形态等内容即零重写；
- `ensure_go_env` — 生产入口（go）。GOENV 尊重（对线 F7）：进程/用户级 `GOENV` 设 `off` 时跳过文件面
- `ensure_go_env_file` — GOENV 文件直写（对线 G1）：path 即目标文件本身（GOENV 自定义值就是文件路径，
- `ensure_go_env_in` — go 代理落 GOENV 文件（win `%APPDATA%\go\env`、POSIX `~/.config/go/env`，
- `ensure_npmrc` — npm registry 落 `~/.npmrc`（UTF-8 无 BOM）。返回是否写入。
- `ensure_pip_conf` — 生产入口（pip）。
- `ensure_pip_conf_in` — pip index 落用户配置目录下 `pip/`（目录注入便于测）。返回是否写入。
- `ensure_uv_toml` — 生产入口（uv）：用户配置目录解析（dirs 同源，POSIX=XDG ~/.config、win=%APPDATA%）。
- `ensure_uv_toml_in` — uv index 落用户配置目录下 `uv/uv.toml`（POSIX `~/.config/uv/`、win `%APPDATA%\uv\`，
- `env_key_sane` — 环境写入面键形态校验：标识符形态（防键里带 `=` 或元字符破坏 export 行）。lint 同规则复用。
- `env_value_sane` — 环境写入面值形态校验（对线 R5，注入面；env_set 与 mirror 单值键共用）：值拒绝换行与
- `go_env_upsert` — GOENV 文件的行级 upsert（纯函数）：GOPROXY 与 GOSUMDB 两键原位替换或追加，
- `load` — 载入 manifest.toml（传 catalog 路径，落位与消费同一推导）；缺文件返回空（零原语）；
- `npmrc_upsert` — npmrc 的 registry 行 upsert（纯函数）：有 registry 行原位替换（键名大小写与空白容忍），
- `parse` — 解析（纯函数可测；反序列化走 toml_edit serde feature，零新增依赖）。
- `path_for` — manifest.toml 路径：与 catalog（tools.toml）同目录（R016 两件分离同批落位）。
- `pip_conf_content` — pip.conf 目标内容（纯函数）。
- `pip_conf_name` — pip 配置文件名（win=pip.ini、POSIX=pip.conf；pip 各平台原生发现位）。
- `platform_commands` — 当前平台键选 L2 命令（纯函数可测）。
- `platform_covered` — 平台是否被显式跳过（三键齐备语义：有命令或进 skip 即齐备）。
- `run_post_install` — L2：逐条执行受控命令；超时 300s 杀进程；失败只报不回滚，报告含退出码与输出尾行（R016 三节）。
- `shim_cmd_content` — win `.cmd` 兜底内容（纯函数可测）：`%~dp0` 相对定位（
- `uv_toml_content` — uv.toml 目标内容（纯函数）：`[[index]]` default 形态（uv 0.4.23+ 数组表）。

## Types

- `ManifestFile` — manifest.toml 根结构。
- `Mirror` — L1 镜像源节（D42，全集对齐 ohmypwsh set-mirror.ps1 / P0017 五端统一口径）。
- `PostInstall` — L2 受控命令：每平台一组 argv 数组；`skip` 显式声明「该平台无命令」（三键齐备 lint 依据）。
- `ToolManifest` — 单工具节：L1 声明原语加 L2 受控命令（未识别字段由 serde default 容忍，lint 白名单把关）。

## Constants

- `SUPPORTED_SCHEMA_VERSION` — 引擎支持的 manifest schema 大版本（R016 v0.2 定稿）。

