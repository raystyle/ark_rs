# ark::selfupdate

selfupdate：ome 自身升级（`ark self update`），三通道：
- **dev**（默认）：pre-release tag `dev` 的滚动资产——CI push main 构建上传，本地测试期升级源；
- **stable**：`releases/latest` 正式版——CI 推 v* tag（封版）触发；
- **git**：源码安装——浅克隆仓库 cargo build 后替换（封版前无任何 release 时的通道，需 git 与 cargo）。

升级判定：release 资产的 API digest（sha256）与运行中 exe 的 sha256 对比，一致即已最新；
不同则经 download_asset 下载到缓存（digest 校验）后替换部署位，并同步数据目录 catalog。
Windows 运行中 exe 可改名不可删：旧 exe 改名 .old 保留、新 exe 就位，下次升级开头清理。

## Functions

- `asset_compat_for_this_platform` — 兼容资产名（`ome-<triple>`，旧二进制认的名；ome/ 分发面已收口停写，仅历史
- `asset_for_this_platform` — 编译目标对应的 CI 资产主名（D41 B：`ark-<triple>`，release 双附主名）。
- `is_ome_self` — 是否自管条目（extract = "ome-self"；D41 起双接受 "ark-self"，数据面改名可单方回退）：无 pin 无资产，升级走 self update 三通道。
- `self_update` — 自升级主流程。

## Types

- `Channel` — 升级通道。
- `SelfUpdateOutcome` — 升级结果。

## Constants

- `REPO` — 自升级源仓库（D41 更名；旧 ohmyenv-rs 名 GitHub 301 兜底，官方路径不断）。

