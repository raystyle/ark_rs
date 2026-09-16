//! omerr：机器可读错误结构（code/message/hint/exit_code 四元组，吸收自 incurs 的 IncurError 模式）。
//! 内部各模块保持 Result<T, String> 风格；边界（main 出口、需要特殊退出码的命令）转换为 OmeError，
//! main 按 exit_code 退出。

use std::fmt;

/// 机器可读错误：code 稳定标识，message 人称描述，hint 下一步提示，exit_code 进程退出码。
#[derive(Debug)]
pub struct OmeError {
    /// 机器可读错误码
    pub code: &'static str,
    /// 人读错误信息
    pub message: String,
    /// 修复提示（可选）
    pub hint: Option<String>,
    /// 进程退出码
    pub exit_code: i32,
}

impl OmeError {
    /// 普通错误（exit 1）。
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        OmeError {
            code,
            message: message.into(),
            hint: None,
            exit_code: 1,
        }
    }

    /// 附下一步提示（CTA）。
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// 指定退出码。
    pub fn with_exit_code(mut self, code: i32) -> Self {
        self.exit_code = code;
        self
    }
}

impl fmt::Display for OmeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, "\n  提示: {hint}")?;
        }
        Ok(())
    }
}

/// 内部 String 错误到边界错误结构的默认转换：code=error、exit 1。
impl From<String> for OmeError {
    fn from(message: String) -> Self {
        OmeError::new("error", message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 错误四元组_display含code与hint() {
        let e = OmeError::new("verify-fail", "有 2 项 FAIL")
            .with_hint("ark heal all")
            .with_exit_code(1);
        let text = e.to_string();
        assert!(text.contains("[verify-fail]"), "应含 code: {text}");
        assert!(text.contains("FAIL"), "应含 message: {text}");
        assert!(text.contains("提示: ark heal all"), "应含 hint: {text}");
        assert_eq!(e.exit_code, 1);
    }

    #[test]
    fn string错误默认转换_exit1() {
        let e = OmeError::from("something broke".to_string());
        assert_eq!(e.code, "error");
        assert_eq!(e.exit_code, 1);
        assert!(e.hint.is_none());
    }
}
