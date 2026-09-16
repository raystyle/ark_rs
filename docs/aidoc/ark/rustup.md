# ark::rustup

rustup：Rust 接管（自 ohmypwsh `scripts\set-rust.ps1` 平移，2026-09-02；POSIX 接管 D42，2026-09-13）。
rsproxy 引导器直链（无版本无 sha，evergreen 语义）：rustup-init 引导 stable 工具链，
无 pin（`rustup update stable` 即更新语义，官方 6 周滚动，stable 只收安全补丁）。
安装根双形态：Windows 重定位 EnvRoot（RUSTUP_HOME=`<EnvRoot>\rustup`、CARGO_HOME=`<EnvRoot>\cargo`，
用户环境变量）；POSIX 尊重既有（进程 env > 用户级 > 默认 `~/.rustup` 与 `~/.cargo`，
不持久化两 HOME 变量、进程内钉解析值供引导器）。
rsproxy 双镜像：rustup 分发（RUSTUP_DIST_SERVER/RUSTUP_UPDATE_ROOT，win 注册表 / POSIX profile env 块）
加 cargo sparse（config.toml，Windows 落 EnvRoot 重定位位、POSIX 落 `~/.cargo`，rsproxy 全量形态）；
cargo bin 进用户 PATH（ome 惯例尾部追加；set-rust 为前置，已装机器由 ps1 前置位保持不变）。
幂等：rustc 在位不重跑 init（update stable 照跑保最新）；config.toml 内容一致不重写。

## Functions

- `cargo_home` — CARGO_HOME：Windows `<EnvRoot>\cargo`；POSIX 与 manifest `cargo_home_path` 同一解析序
- `init_args` — rustup-init 引导参数（纯函数可测）。Windows 对齐 set-rust.ps1（stable + msvc host + 不动 PATH）；
- `install` — 安装（幂等）：download 只落二进制（进程内重定位供 rustup-init 写入安装根）；
- `is_rustup` — 是否 rustup 引导器型条目（extract = "rustup"）。
- `rustc_exe` — rustc 可执行（`<cargo home>\bin\rustc[.exe]`，verify dev-rust 维度断言位）。
- `rustup_home` — RUSTUP_HOME：Windows `<EnvRoot>\rustup`；POSIX 尊重既有（进程 env > 用户级 > 默认 `~/.rustup`）。

## Constants

- `INIT_EXE` — 引导器缓存文件名（平台各异）。

