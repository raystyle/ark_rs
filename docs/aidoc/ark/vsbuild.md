# ark::vsbuild

vsbuild：VS Build Tools 接管（自 ohmypwsh `scripts\set-vsbuild.ps1` 平移，2026-09-01）。
永续引导器（aka.ms 直链，无版本无 sha 可 pin，属 evergreen 语义）；
组件三件套 VCTools、VC.Tools.x86.x64、VC.CMake.Project（不带 --includeRecommended；
Windows SDK 走 ISO 分离装 Windows Kits，不进 VS 组件，见 set-windows-sdk.ps1）。
需管理员：未提权且 gsudo 可用时经 gsudo 重跑 `ark install vsbuild`（退出码透传）；
PATH 写机器级（MSBuild 与 cl.exe 目录），非用户级。
幂等：cl.exe 在位只补机器 PATH（PATH 齐备则完全无操作，无需管理员）。

## Functions

- `bootstrapper_args` — 引导器安装参数（纯函数可测；--wait 等安装器完成，3010 = 成功需重启）。
- `find_cl_exe` — 最新 MSVC 工具集的 cl.exe（存在即视为已安装）。
- `install` — 安装（幂等）。download 只跑引导器落二进制；deploy（configure）才写机器 PATH。
- `install_root` — 安装根：`<EnvRoot>\vsbuild`。
- `is_vsbuild` — 是否 vsbuild 型条目（extract = "vsbuild"）。
- `machine_path_dirs` — 机器 PATH 应包含的目录（MSBuild 与 cl.exe 目录，存在才返回）。
- `msbuild_exe` — MSBuild.exe（路径跨版本稳定，版本探测用：`MSBuild -version` stdout 首行裸版本号）。

## Constants

- `BOOTSTRAPPER` — 引导器缓存文件名。
- `COMPONENTS` — 安装组件（VCTools/x86.x64/CMake）。

