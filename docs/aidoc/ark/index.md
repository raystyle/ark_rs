# ark 1.4.0

ark：Ark（Agent Runtime Kit）本机跨平台环境部署管理 CLI。
三原语 doctor / install / status；派生 query、update、pin、verify、heal、
init、self update。catalog 为唯一 pin 源。
下载镜像主通道 env.ohmygh.com、官方兜底（D44）；锁定归云端数据面（D37）。
公开契约双面：命令输出走黄金文件 oracle，lib 公开项走 docs/aidoc/ 投影。

## Modules

- [`arerr`](arerr.md): arerr：机器可读错误结构（code/message/hint/exit_code 四元组，吸收自 incurs 的 IncurError 模式）。
- [`catalog`](catalog.md): catalog：清单主功能。数据面（tools.toml 读写与路径解析）加两个子功能：
- [`checksum`](checksum.md): checksum：sha256 校验源解析，语义对齐 helpers.ps1 的 Get-OfficialSha256。
- [`docker`](docker.md): docker：Windows 容器 Docker Engine 接管（自 ohmypwsh set-docker.ps1 完整迁移，2026-09-02）。
- [`doctor`](doctor.md): doctor：部署异常诊断（2026-09-02 用户需求：检测部署的错误异常）。
- [`download`](download.md): download：资产下载与缓存复用，语义对齐 helpers.ps1 的 Save-ReleaseAsset。
- [`envpath`](envpath.md): envpath：Windows 用户 PATH 管理工具函数，对齐 helpers.ps1 的 Add-EnvPath/Remove-EnvPath（1321-1370 行）。
- [`extract`](extract.md): extract：解压/安装分派，对齐 helpers.ps1 Install-ToolVersion 的 switch（1148-1242 行）。
- [`heal`](heal.md): heal：部署域幂等自愈（P0026 M4，heal-map.psd1 的 42 键迁嵌入注册表）。
- [`install`](install.md): install：安装主编排，对齐 helpers.ps1 的 Install-ToolVersion（1019-1267 行）。
- [`issue`](issue.md): issue 域（REQ-0009，对齐 ohmycloud REQ-057 契约）：自研命令仓统一 issue 入口
- [`manifest`](manifest.md): manifest.toml：安装配置部署逻辑的数据面（R016 B 层，D39 第一波引擎）。
- [`platform`](platform.md): platform：跨平台抽象层。
- [`render`](render.md): render：单一渲染层（吸收自 incurs 的「handler 结构化产出 + 单一渲染层」模式，S003 扩展三格式）。
- [`resolve`](resolve.md): resolve：版本解析三分支（cdn_index_url / cdn_url / GitHub REST），
- [`rustup`](rustup.md): rustup：Rust 接管（自 ohmypwsh `scripts\set-rust.ps1` 平移，2026-09-02；POSIX 接管 D42，2026-09-13）。
- [`selfdeploy`](selfdeploy.md): selfdeploy：自部署——复制当前 exe 到用户程序目录（Windows `%LOCALAPPDATA%\Programs\ark`，
- [`selfupdate`](selfupdate.md): selfupdate：ark 自身升级（`ark self update`），三通道：
- [`status`](status.md): status：三态对照（locked/installed/path）。
- [`toolver`](toolver.md): toolver：已装版本探测，移植 helpers.ps1 的 Get-InstalledVersion（888-935 行）。
- [`verify`](verify.md): verify：部署域验收维度（P0026 M3 第一批，数据驱动自 catalog 三态）。
- [`vsbuild`](vsbuild.md): vsbuild：VS Build Tools 接管（自 ohmypwsh `scripts\set-vsbuild.ps1` 平移，2026-09-01）。

