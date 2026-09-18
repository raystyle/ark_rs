# ark::selfupdate

selfupdate：ark 自身升级（`ark self update`），三通道：
- **dev**（默认）：pre-release tag `dev` 的滚动资产——CI push main 构建上传，本地测试期升级源；
- **stable**：`releases/latest` 正式版——CI 推 v* tag（封版）触发；
- **git**：源码安装——浅克隆仓库 cargo build 后替换（封版前无任何 release 时的通道，需 git 与 cargo）。

D51 镜像默认通道：release 元数据镜像段读序先行（边车即锚），GitHub API 兜底；
`ARK_MIRROR=0` 官方优先逃逸阀反转读序（旧 =1 强制镜像已转正为默认）。
升级判定：release 资产的 digest（sha256）与运行中 exe 的 sha256 对比，一致即已最新；
不同则经 download_asset 下载到缓存（digest 校验）后替换部署位，并同步数据目录 catalog。
包形资产（REQ-0008 窗口，ark-<target>.zip/.tar.gz）digest 为归档 sha：归档经锚校验下载后
解包取内层二进制（拼名契约 ark-<target>/ark），判新等值在解包后的二进制间进行（机制不动）。
Windows 运行中 exe 可改名不可删：替换全程用 rename（备份 ark-old-<pid>、暂存 ark-new-<pid>），
成功清备份、删不动留待启动收割。
REQ-0012 家族自更新统一标准：stable 通道官方 API 判新加 semver 门（localNewer 不动）加镜像
stable 段下载优先（digest 锚不符硬拒不回落）；自替换带 pid 锁、陈旧收割与 --version 自证回滚。

## Functions

- `asset_for_this_platform` — 编译目标对应的 CI 资产主名（D41 B：`ark-<triple>`，release 双附主名）。
- `asset_package_for_this_platform` — 包形资产名（REQ-0008 窗口，对线裁定）：`ark-<triple>` 单顶层目录（净 triple），win
- `is_ark_self` — 是否自管条目（extract = "ark-self"）：无 pin 无资产，升级走 self update 三通道。
- `self_update` — 自升级主流程。

## Types

- `Channel` — 升级通道。
- `SelfUpdateOutcome` — 升级结果。

## Constants

- `REPO` — 自升级源仓库（D41 更名；旧 ohmyenv-rs 名 GitHub 301 兜底，官方路径不断）。

