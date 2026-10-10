# S007-安装清单DSL研究-winget与scoop实证

> 角色：研究。D39（用户方向 2026-09-11：参考 winget/scoop 运营，安装配置部署逻辑脚本或 DSL 数据化进云端配置管理，ome 简化为只执行清单元数据）的选型研究：两家主流清单 DSL 的实证对照、ome 的能力对位与缺口、DSL 分层提案。六态标注。

## 一、两家实证

### winget：纯声明 DSL 无脚本

[实证: 2026-09-11 拉 microsoft/winget-pkgs Rustlang.Rustup 1.25.1 installer.yaml]

- InstallerType 枚举（exe/msi/zip/portable/burn…）+ InstallerSwitches 声明式参数（Silent/SilentWithProgress）+ UpgradeBehavior；zip/portable 型有 PortableCommandAlias 与 InstallCommands 命令数组
- 复杂安装场景的出路是「wrapper 包或进 InstallerType 枚举」，不在清单里写脚本 [实证: schema 1.2.0 无脚本字段]
- 信任：MS 签名包 + manifest 校验（中心化运营）

### scoop：任意 PowerShell 脚本

[实证: 2026-09-11 拉 ScoopInstaller 主桶 ripgrep.json 与 git.json]

- arch 键（64bit/32bit/arm64）的 url/hash/extract_dir：与 ome catalog 平台字段族同构 [实证]
- bin 映射、env_set/env_add_path 键值与 PATH 声明 [实证: git.json 字段]
- pre_install/post_install：**任意 PowerShell 脚本数组**（git.json 的 pre_install 含持久化目录恢复、正则改写配置等完整逻辑）[实证]
- persist 目录语义（配置跨版本保留）[实证]
- checkver/autoupdate 模板（版本发现与 URL 模板）：ome 的 resolve+pin 回写能力更强（锚定官方 sums/digest）[推断: 能力对照]
- 信任：bucket 是 git 仓可审计 + hash 校验，无端到端签名

## 二、ome 能力对位与缺口

| 位 | winget | scoop | ome 现状 |
| --- | --- | --- | --- |
| 平台资产 | Architecture 键 | arch 键 | 平台字段族（已有） |
| 布局 | InstallerType | extract_dir/bin | extract 九分派 + dir/bin/exe/extra_bins（已有） |
| 版本发现 | 无（人工提 PR） | checkver/autoupdate | resolve+pin 回写（已有，更强） |
| 校验 | InstallerSha256 | hash | pin sha + sums/digest 三型（已有） |
| 环境变量 | 无 | env_set/env_add_path | **缺**（遥测键硬编码 install.rs） |
| 命令别名 | PortableCommandAlias | bin 映射 | bin/extra_bins（已有）；**shim 生成缺**（bunx 是硬编码） |
| 持久化 | 无 | persist | **缺**（无此语义） |
| 装后动作 | InstallCommands 数组 | post_install 脚本 | **缺**（npm-tgz/uv-git 是引擎内建通道） |
| 提权/服务/机器 PATH | InstallerType 吸收 | 脚本 | **引擎内建但数据不可选**（vsbuild/docker 专用模块触发） |
| 信任 | 中心化 | git 审计 | **minisign 端到端签名（最强，D34）** |

[推断: 对照综合；ome 已有面为实证仓内事实]

## 三、DSL 分层提案

**L1 声明原语（无脚本，静态可 lint）**：`env_set`（用户级键值表）、`shims`（生成别名）、`persist`（目录挂载语义）、`machine_path`/`elevate`（布尔触发引擎内建动作）。

**L2 受控命令数组（跨平台分键）**：`post_install = ["npm install -g <包>"]` 形态的声明式命令，win/linux/mac 分平台键，逐条执行失败即报：覆盖 npm-tgz/uv-git 等通道的通用化。

**L3 任意脚本：不进**。scoop 的 PS 脚本面 Windows 单平台且过宽（正则改配置等任意逻辑）；跨平台任意脚本意味着三套脚本或内嵌解释器，复杂度与安全审计面失控。例外场景（rustup 重定位、docker 服务注册这类多步系统编排）留引擎内建原语，由数据字段选择触发。

[推断: 设计取舍；依据 winget（声明子集可运营百万包）与 scoop（脚本表达力的维护代价）两家实证的折中]

**信任模型**：DSL 与 tools.toml 同件同签名（D34 minisign 端到端链零新增信任面：签名清单即信，比 scoop git 审计强）；catalog-lint（omc 侧 vitest）扩展 DSL 语法机检，装前静态可验。

**专用模块去向（vsbuild/rustup/docker/msi 约 943 行）**：拆解为「L1/L2 原语组合 + 引擎内建动作字段触发」，目标是新工具与配置变更零改 ome 代码、数据面单方可配 [推断: 迁移可行性待设计文档验证]。

## 四、待裁点

1. L2 受控命令的边界（白名单动词？任意 argv？超时与失败语义）；三键齐备 lint 已由全平台约束定（五.2）
2. persist 语义是否引入（配置跨版本保留是 scoop 强项，但与「配置归 oma/omc 域」的边界可能重叠）
3. 专用模块迁移节奏（一次性 vs 按工具渐进）
4. DSL 载体（tools.toml 内嵌字段 vs 分离 installer.toml 同签名发布）

## 五、全平台约束与跨平台先例

**用户裁（2026-09-11）：全平台支持为 D39 第一设计约束。** 推论先行修正参照系：winget 与 scoop 都是 Windows 单平台运营，可参照其声明形态与表达力取舍，不可参照其平台性。

跨平台真实先例补充：

- **Homebrew formula（Ruby DSL，macOS 与 Linux 双平台一等公民）** [实证: 2026-09-11 拉 homebrew-core ripgrep.rb 与 git.rb]：`bottle do` 平台预构建矩阵（arm64_golden_gate/tahoe/sequoia/sonoma 加 arm64_linux/x86_64_linux 平台键逐资产给 sha）；`def install` 块是受控 Ruby：`system` 受控命令加 DSL helper（`generate_completions_from_executable`、`bin`/`man1` 路径域），非任意脚本；`on_macos`/`on_linux` 块显式分叉 [记忆: brew 标准特性]；`depends_on` 依赖声明与 `livecheck` 版本发现。
- **Nix（函数式 DSL，全平台）** [记忆]：表达力最强但学习与维护成本最高，不适合作 ome 主参照。
- **ome catalog 自身**：平台字段族（win/linux/mac 三套静态与 pin 字段）加镜像资产域平台分列，已是三平台清单的活例 [实证: 仓内]：DSL 是在此之上扩展，不是另起炉灶。

约束落为四条设计原则：

1. **原语优先**：安装逻辑尽量进 L1 声明原语，三平台语义由引擎统一保证（env_set 在 win=注册表 HKCU、POSIX=profile 标记块，platform.rs 基建已有）；新原语必须过三平台测试矩阵（R004 三.6）才准入。
2. **L2 三键齐备 lint**：受控命令数组声明则 win/linux/mac 三平台键（或显式 skip 标记）必须齐备：机检防「某平台装了没配置」的分叉漂移。
3. **平台不适用容忍复用**：某原语在某平台无意义（如 machine_path 于 POSIX 用户级安装）走 M003 空态语义，如实空态不报错。
4. **平台矩阵即验收**：DSL 引擎实现与每个原语的验收跑三平台矩阵（CI 每推加关键节点本机三运行时），单平台绿不算过（M016 同型纪律）。
