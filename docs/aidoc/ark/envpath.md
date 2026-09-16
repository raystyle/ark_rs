# ark::envpath

envpath：Windows 用户 PATH 管理工具函数，对齐 helpers.ps1 的 Add-EnvPath/Remove-EnvPath（1321-1370 行）。
具体平台实现（Windows 注册表 / Linux profile）集中在 `platform.rs`；本模块保留 Windows 语义纯函数供其调用。

核心语义：读 PATH 原始值（不展开 %VAR%），比较时展开后去重（大小写不敏感），前置插入。

## Functions

- `add_path_entry` — 向 PATH 文本追加目录（已含则 None，未变不重写）。
- `remove_path_entry` — 纯函数：从原始 PATH 串移除 dir（展开后相等者全部移除）。对齐 Remove-EnvPath。

