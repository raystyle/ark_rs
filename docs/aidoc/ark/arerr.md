# ark::arerr

arerr：机器可读错误结构（code/message/hint/exit_code 四元组，吸收自 incurs 的 IncurError 模式）。
内部各模块保持 Result<T, String> 风格；边界（main 出口、需要特殊退出码的命令）转换为 ArkError，
main 按 exit_code 退出。

## Types

- `ArkError` — 机器可读错误：code 稳定标识，message 人称描述，hint 下一步提示，exit_code 进程退出码。

