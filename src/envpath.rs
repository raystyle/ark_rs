//! envpath：Windows 用户 PATH 管理工具函数，对齐 helpers.ps1 的 Add-EnvPath/Remove-EnvPath（1321-1370 行）。
//! 具体平台实现（Windows 注册表 / Linux profile）集中在 `platform.rs`；本模块保留 Windows 语义纯函数供其调用。
//!
//! 核心语义：读 PATH 原始值（不展开 %VAR%），比较时展开后去重（大小写不敏感），前置插入。

/// %VAR% 环境变量展开（Windows 语义）。
fn expand_env_vars(s: &str) -> String {
    let re = regex::Regex::new("%([^%]+)%").expect("静态正则应合法");
    re.replace_all(s, |caps: &regex::Captures| {
        std::env::var(&caps[1]).unwrap_or_else(|_| caps[0].to_string())
    })
    .into_owned()
}

/// 纯函数：向原始 PATH 串前置插入 dir；已存在（展开后相等）返回 None。
/// 对齐 Add-EnvPath：$parts 存未展开，比较用展开后形式。
fn norm_path(s: &str) -> String {
    expand_env_vars(s)
        .trim_end_matches(['\\', '/'])
        .to_lowercase()
}

/// 向 PATH 文本追加目录（已含则 None，未变不重写）。
pub fn add_path_entry(raw: &str, dir: &str) -> Option<String> {
    let expanded_dir = norm_path(dir);
    let parts: Vec<&str> = raw.split(';').filter(|p| !p.is_empty()).collect();
    let exists = parts.iter().any(|p| norm_path(p) == expanded_dir);
    if exists {
        return None;
    }
    let mut new_parts = vec![dir];
    new_parts.extend(parts);
    Some(new_parts.join(";"))
}

/// 纯函数：从原始 PATH 串移除 dir（展开后相等者全部移除）。对齐 Remove-EnvPath。
pub fn remove_path_entry(raw: &str, dir: &str) -> String {
    let expanded_dir = norm_path(dir);
    raw.split(';')
        .filter(|p| !p.is_empty())
        .filter(|p| norm_path(p) != expanded_dir)
        .collect::<Vec<_>>()
        .join(";")
}

/// 用户 PATH 管理 re-export：业务模块统一调用 `envpath::add_user_path` 等，实际由 `platform.rs` 实现。
pub use crate::platform::{add_user_path, remove_user_path, user_path_contains};

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn add_前置插入_保未展开条目() {
        // %USERPROFILE% 条目必须以未展开形式保留（review 实测降级为字面路径会静默失效）
        std::env::set_var("OME_TEST_HOME", r"C:\Users\demo");
        let raw = r"%OME_TEST_HOME%\bin;C:\tools";
        let new = add_path_entry(raw, r"D:\ohmyenv\jq").expect("应新增");
        assert_eq!(new, r"D:\ohmyenv\jq;%OME_TEST_HOME%\bin;C:\tools");
    }

    #[test]
    fn add_展开后已存在_跳过且大小写不敏感() {
        std::env::set_var("OME_TEST_HOME", r"C:\Users\demo");
        // 已存在条目的展开形式与 dir 相同（变量形式不同也算重复）
        let raw = r"%OME_TEST_HOME%\bin;C:\tools";
        assert_eq!(add_path_entry(raw, r"C:\Users\demo\bin"), None);
        // 大小写不敏感（对齐 PowerShell -contains）
        assert_eq!(add_path_entry(raw, r"c:\users\demo\BIN"), None);
        assert_eq!(add_path_entry(raw, r"C:\TOOLS"), None);
        assert_eq!(
            add_path_entry(r"D:\ohmyenv\jq\", r"D:\ohmyenv\jq"),
            None,
            "尾斜杠应视为同一条目"
        );
    }

    #[test]
    fn add_空串与尾分号边界() {
        assert_eq!(
            add_path_entry("", r"D:\x").as_deref(),
            Some(r"D:\x"),
            "空 PATH 应只含新条目（不产出前导/尾随分号）"
        );
        assert_eq!(
            add_path_entry(r"C:\a;", r"D:\x").as_deref(),
            Some(r"D:\x;C:\a"),
            "尾分号产生的空段应被过滤（对齐 Where-Object {{ $_ }}）"
        );
        // 孤 \ 段是非空串，PowerShell Where-Object { $_ } 判定为真会保留——实现须对齐而非自作聪明清掉
        assert_eq!(
            add_path_entry(r"C:\a;\", r"D:\x").as_deref(),
            Some(r"D:\x;C:\a;\")
        );
    }

    #[test]
    fn remove_展开后匹配_大小写不敏感() {
        std::env::set_var("OME_TEST_HOME", r"C:\Users\demo");
        let raw = r"D:\ohmyenv\jq;%OME_TEST_HOME%\bin;C:\tools";
        assert_eq!(
            remove_path_entry(raw, r"d:\ohmyenv\JQ"),
            r"%OME_TEST_HOME%\bin;C:\tools"
        );
        assert_eq!(
            remove_path_entry(raw, r"C:\Users\demo\bin"),
            r"D:\ohmyenv\jq;C:\tools"
        );
    }

    #[test]
    fn remove_不存在_原样返回() {
        let raw = r"C:\a;C:\b";
        assert_eq!(remove_path_entry(raw, r"C:\nope"), raw);
    }
}
