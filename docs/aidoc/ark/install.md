# ark::install

install：安装主编排，对齐 helpers.ps1 的 Install-ToolVersion（1019-1267 行）。
流程：防穿越校验 → 幂等跳过（顺带补 PATH/sha 回填/滞后锁定）→ sha 基准（pin > 官方源）
→ 下载（含 bootstrap 资产与 MZ 头校验）→ 删旧目录全量重建 → extract 分派
→ 装后验版本（5 次递增重试）→ 回写 lock（write_pin / write_sha256）。

## Functions

- `install_tool` — 安装单工具（下载 → 校验 → 解压 → 验版本 → 回写）。
- `is_safe_under_root` — 防穿越：path 必须在允许的安全根之下。

## Types

- `InstallAction` — 安装结果动作。
- `InstallOptions` — 安装选项（对齐 Install-ToolVersion 的 -RegisterPath / -UpdateLock / -Force）。
- `InstallOutcome` — 安装结果：动作、版本、安装目录（msi 无绿色目录，为 None）。

