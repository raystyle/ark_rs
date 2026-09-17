# references 开发参考索引

> 开发参考细则唯一索引（R 编号，接最大号不复用）。查 R 文档先搜本文件；guide 元规范（G 编号）见 `docs/guide/README.md`。

| 编号 | 文件 | 主题 |
| --- | --- |  |
| R001 | `docs/references/R001-catalog数据模式-tools-toml字段与pin语义.md` | catalog 清单字段模式与 pin 回写语义（数据面归 ohmycloud） |
| R004 | `docs/references/R004-测试标准细则-分层断言与门禁流程.md` | 测试分层断言与门禁（真机对齐闸门 ARK_TEST_REAL） |
| R005 | `docs/references/R005-选型研究细则-cratesio与github双通道.md` | Rust 库与项目选型双通道 |
| R008 | `docs/references/R008-项目工具Python库选型细则-pypi与uv.md` | 项目工具 Python 选库与 uv |
| R009 | `docs/references/R009-项目工具PowerShell模块选型细则-psgallery与psresourceget.md` | 项目工具 PowerShell 模块选型 |
| R010 | `docs/references/R010-linux开发接管-环境准备与构建验证.md` | Linux 开发主机接管，已归档：工具链、构建验证、平台门控、两端分工 |
| R011 | `docs/references/R011-mac开发接管-环境准备与构建验证.md` | mac 开发主机接管：工具链、构建验证、mac 目录/PATH 策略、三端分工 |
| R012 | `docs/references/R012-ohmypwsh与ome对齐清单-linux-windows.md` | 历史对齐清单：已降级为 catalog 数据迁移参考（2026-09-01 被完整迁移裁决取代；D18 源项目不存在） |
| R013 | `docs/references/R013-Agent友好IO契约-输出格式退出码与冻结面.md` | 输出三格式、数据错误分流、命令数据块字段、退出码与对外冻结契约（自 README 收敛） |
| R014 | `docs/references/R014-ohmycloud种子清单-ISSUE派任务与对齐.md` | 与 ohmycloud 协调（herdr 通道）；agent 部署委托与管辖边界终版（D29/D37：清单数据权威归 omc）；版本对齐水位 |
| R016 | `docs/references/R016-云端清单与manifest标准.md` | 云端 catalog 与 manifest 数据标准（两件分离同签、schema 版本化、三层 DSL 原语、mirror 镜像源节 D42、跨仓分工 omc 维护 ark 执行；D39/D42） |
| R015 | `docs/references/R015-软件清单发布更新与播种标准.md` | 清单发布、更新与软件播种三流程唯一标准（三重门、三通道、双路线、管辖边界两域分治；D35/D36） |

编号注记：R002/R003、R006/R007 号段属 ohmyagents 产品特定，2026-08-31 文档体系平移时留空不复用。
