# 项目工具Python库选型细则：pypi与uv

> AGENTS 操作规则「写临时脚本时」的 Python 侧细则。与 `R005-选型研究细则-cratesio与github双通道.md` 分工：R005 管产品 Rust 依赖（crates.io + GitHub）；本篇管项目工具（`.tools\` 脚本与自定义 py 工具）的 Python 选库与开发稳定。PowerShell 侧另见 `R009-项目工具PowerShell模块选型细则-psgallery与psresourceget.md`。移植自 ohmyagents 同名细则；素材为 2026-08-31 用户提供的《Python 库选择与开发稳定指南》，关键断言已本机实证（见六态标注）。

## 一、搜索能力对照

> Python 生态与 Rust/PowerShell 最大的差异在此：关键词搜索只留网页，无对等 CLI 与 API。

| 通道 | 有没有 | 用法 |
| --- | --- | --- |
| 网站关键词搜索 | 有 | `https://pypi.org/search/?q=<名>`（仅浏览器） |
| 官方 CLI 关键词搜索 | 无 | `pip search` 对 pypi.org 已失效（旧 XML-RPC search 被刷垮后永久关闭）[经验: 官方文档口径；本机 pip 不在 PATH 未直测] |
| 官方搜索 API | 无 | 同上 |
| 已知名字的元数据 | 有 | `GET https://pypi.org/pypi/<名>/json` [实证: 2026-08-31 本机 curl，httpx 0.28.1 带 requires_python 与 license] |
| 安装器索引 | 有 | `GET https://pypi.org/simple/<名>/`（PEP 503/691，带 `Accept: application/vnd.pypi.simple.v1+json` 得 JSON）[实证: 2026-08-31 本机 curl] |

不要把 `/search/?q=` 当可编程接口，也不要抓网页做选型；不知道名字就开浏览器搜，知道了再走 API。

## 二、稳度四信号

> 对齐 R005 的 crates.io 四信号，Python 侧对应物。

| 信号 | 来源 | 示例 |
| --- | --- | --- |
| 下载量 | pypistats API `https://pypistats.org/api/packages/<名>/recent` [实证: 2026-08-31 本机 curl，httpx last_month 8 亿] | 月下载量级 |
| 最近发版 | PyPI JSON API `info.version` 与 `releases` 键 | 活跃维护 |
| 运行时约束 | `info.requires_python` [实证] | 与本机 Python/uv 版本对齐 |
| 许可 | `info.license` [实证] | 分发合规 |

补充判断：是否 yank、Source/Docs 齐不齐、维护者是谁。PyPI 项目 70 万+，仿冒前缀常见，名字先对浏览器搜索结果核一眼。[经验: 指南]

相对稳妥一类（指南口径）：`httpx` / `requests` / `pydantic` / `pytest` / `ruff` / `numpy` / `fastapi`。[经验]

## 三、工具链与锁定

> 新项目一律 uv；本机 uv 0.12.6 [实证: 2026-08-31 `uv --version`]。`.tools\` 脚本用 PEP 723 内联元数据加 `uv run --script`，不建 venv（见 `.tools\README.md`）。

```powershell
uv python install 3.12         # 运行时
uv add httpx                   # 项目装库
uv add "pydantic>=2.10"        # 带范围
uv add --dev pytest ruff       # 开发依赖
uv lock --upgrade-package httpx
uv sync --frozen               # CI 可复现
```

应用提交 `uv.lock`；库只在 `pyproject.toml` 声明直接依赖的兼容范围。[经验: 指南]

## 四、自研与发布

src 布局加 Hatchling/setuptools；先 TestPyPI 再正式：

```powershell
uv run pytest
uv build
uv publish --index testpypi
uv publish
```

已发布文件不可改，只能 yank 加新版本。[经验: 指南]

## 五、决策树

| 目标 | 做法 |
| --- | --- |
| 不知道叫什么 | 浏览器开 `pypi.org/search`（唯一关键词入口） |
| 已知名字 | PyPI JSON API 核元数据；pypistats 核下载量 |
| 生产可复现 | `uv.lock` 加 `uv sync --frozen` |
| 自研发布 | PEP 621 加 TestPyPI 加 `uv publish` |

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| 本地 | 用户《Python 库选择与开发稳定指南》（Downloads） | 2026-08-31 | 三通道对照、uv 流程、稳度判据骨架 |
| web | pypi.org/pypi/httpx/json、simple index、pypistats API | 2026-08-31 | 元数据、索引、下载量实测 |
| 本地 | 本机 uv 0.12.6 | 2026-08-31 | 工具链在场 |
