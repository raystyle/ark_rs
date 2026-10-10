# 2026-09-02 定位定调与 init doctor

> 当天做了什么（项目日记，按日期归档）。

## 定位定调

- **docs/定位（用户裁决，同日三次澄清终版）**：ome 自适应承担**本地本系统**的工具与运行时环境部署、管理、诊断：平台自适配，落在哪台机器就管哪台机器，**不做远程与集成**。三仓分工：**ome** 本地本系统部署/管理/诊断；**ohmypwsh** 集成 ome：本地或远程执行 ome（远程由其下发二进制与 catalog 后 ssh 执行，ohmywsl 的 Sync-OmeToRemote 与 sync-ome-lanwin 即此架构）、负责 ome 自身部署分发、并在 ome 装好密钥工具后承担密钥管理；**Agent 四件套本体部署执行器归 D:\ohmyagents**。前两版表述（「ome 承担本地远程集成」「agent 执行器归 ome」）均为误写，当日修正。clap about、README 一句话定位与边界同步；ome 自身「部署到用户目录、独立用户数据目录、自注册 PATH」三项已就位。

## init 改名

- **feat/**：`self-deploy` 改名 `init`（保留 `self-deploy` 隐藏兼容别名，既有文档与 ohmypwsh 侧脚本零断链）；README/INDEX/R011/AGENTS 同步口径。

## verify 流式

- **feat/**：`run_verify_with` 回调形态：维度所需工具探完即输出（保持注册表顺序可出即出；catalog 缺工具的维度兜底判 NA 出列不静默漏）。json 模式仍整批产出。实证 0.7s 出 5 行（总 26 行含汇总）。

## doctor 部署异常诊断

- **feat/**：新增 `ome doctor [--json]` 九项：EnvRoot 可写、版本漂移（FAIL）、探测失败、装而未上 PATH、pin 缺失（排除 evergreen）、sha 缺失、PATH 死链（EnvRoot 域内）、PATH 重复条目（大小写与尾斜杠不敏感）、缓存孤儿（无 pin 指向资产，计数与体积）。输出 `check=OK/WARN/FAIL`（明细走 stderr），FAIL 即 exit 1、WARN 不拦。
- **实证/首跑即抓真问题**：`gopath\bin` PATH 死链一条、缓存孤儿 13 个共 850.5 MB（claude/codex/grok/kimi 旧安装包、docker、rustup-init、旧 vs_buildtools.exe 等，待用户裁决清理）；pin-missing 首版误报 vsbuild（evergreen 无 pin 属设计）已修。

## 环境

- rustc 1.97.1 / windows msvc；基线 c56c477 加本日提交
