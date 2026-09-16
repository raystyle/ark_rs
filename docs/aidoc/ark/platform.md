# ark::platform

platform：跨平台抽象层。

Windows：EnvRoot 为 `D:\ohmyenv` 或 `C:\ohmyenv`，工具集中安装；PATH 通过注册表管理。
Linux / macOS：各软件按系统标准目录安装；PATH 通过当前 shell 的 profile 文件（如 `~/.bashrc`）管理。
ark 自身与 EnvRoot 解耦：二进制进用户程序目录（Windows `%LOCALAPPDATA%\Programs\ark`）、
元数据进用户数据目录（Windows `%LOCALAPPDATA%\ark`；Linux `~/.local/share/ark`；
macOS `~/Library/Application Support/ark`）；迁移过渡期旧 `ohmyenv` 目录在而新目录未建时读回旧位。

## Functions

- `absorb_legacy_env_block` — D41：旧 ome env 块收口迁移（值不变的块体搬迁；无旧标记时原样返回）。纯函数。
- `add_user_path` — 将 dir 注册进用户 PATH；返回是否实际新增。
- `default_env_root` — 默认 EnvRoot：显式参数与环境变量已在 `catalog::resolve_env_root` 处理，此处仅返回平台默认值。
- `ensure_profile_hook` — 设置用户级环境变量（幂等）。Windows 写 HKCU\Environment 并同步当前进程；
- `env_var_or` — 读环境变量（D41 更名 Ark）：主名优先，主名未设或纯空白时读回旧名；都未设返回 None。
- `exe_suffix` — 可执行文件后缀。
- `expand_env_vars` — 展开环境变量引用。
- `expand_install_path` — 展开安装路径中的 `~` 与环境变量引用。
- `get_user_env_var` — 读用户级环境变量（未设置返回 None）。Windows 读 HKCU\Environment；
- `is_elevated` — 当前进程是否管理员（以写权限打开 HKLM Environment 判定；非 Windows 恒 false）。
- `is_official_exe` — 判定 exe 字段是否表示 official 布局（使用环境变量展开 / 绝对路径，不纳入 EnvRoot 管理）。
- `join_if_relative` — 展开后的路径若为相对路径则拼到 EnvRoot 下（Windows 名录是相对 dir/bin；Linux/macOS 多为 ~/ 绝对）。
- `legacy_deploy_dir` — 旧 ome 部署位（D41 C 接管清单：Windows `Programs\ome`，在位时 Some）。
- `legacy_metadata_dir` — 旧元数据目录（D41 迁移源 `<data>\ohmyenv`；只在存在时 Some，旧目录始终只读保留）。
- `machine_path_add` — 把缺失目录追加进机器 PATH（REG_EXPAND_SZ，需管理员）；返回是否写入。非 Windows 恒不写入。
- `machine_path_contains` — 机器 PATH 是否已含 dir（非 Windows 恒 false）。
- `merge_env_exports` — 向 profile env 块合并 export 行（已有键行级 upsert，无则追加；幂等不重写）。
- `merge_hook_block` — O3（S017）：自定义钩子块 upsert（fnm 等；纯函数）。幂等且块体感知：
- `merge_path_entries` — PATH 条目合并（纯函数）：把缺失目录追加到 raw 尾部（大小写不敏感、忽略尾反斜杠比较）。
- `metadata_dir` — ark 自身元数据目录（独立 app 数据目录，与 EnvRoot 解耦）：
- `migrate_legacy_env_block_once` — D41：检测旧 ome env 标记即收口迁移一次（init 与 self update 收尾钩子；
- `migrate_legacy_metadata` — 元数据七件套搬迁（D41 C）：旧 `ohmyenv\catalog` 在而新 `ark\catalog` 缺件时逐件复制
- `migrate_legacy_metadata_in` — 搬迁核心（传新旧根便于测）。
- `ome_alias_target` — `ome` 别名落点（D41 C 过渡载体已停建，2026-09-14 全舰队 ome 水位清零收口；
- `path_entries_eq` — 标准化 PATH 条目比较：展开环境变量、大小写不敏感（Windows）/ 敏感（Unix）。
- `path_separator` — PATH 环境变量条目分隔符。
- `remove_env_export` — env 块摘除单键（纯函数，D42 env_unset 通道）：**全文件撤变量不分写入者**——
- `remove_ome_alias` — 清理 `ome` 别名副本（幂等：不在位返回 false 静默；best-effort：失败返回 Err 由调用方告警，
- `remove_user_env_var` — 撤除用户级环境变量（D42 mirror env_unset 通道；幂等）。返回是否真的撤了。
- `remove_user_path` — 从用户 PATH 移除 dir；返回是否实际移除。
- `self_deploy_target` — 自部署目标路径（D41：ark 接管部署位；旧 ome 位与 `ome` 别名已于 2026-09-14 收口停建）。
- `set_user_env_var` — Linux/macOS 写 profile 的 ome 标记块。用于装后遥测关闭等运行时开关。
- `user_env_write_blocked` — 用户面写入总闸门（测试隔离）：`ARK_TEST_NO_PATH_REG=1`（读回 `OME_TEST_NO_PATH_REG`）时，**所有**用户环境写入面
- `user_path_contains` — 用户 PATH 是否已含 dir。
- `user_path_entries` — 用户 PATH 原始条目（保序不去空；Windows 读 HKCU\Environment，非 Windows 读 profile 标记块）。

## Constants

- `METADATA_MIGRATION_FILES` — 元数据七件套（D41 C 搬迁清单：catalog 域运行态全套）。

