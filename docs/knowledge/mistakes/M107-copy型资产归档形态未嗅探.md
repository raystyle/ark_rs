---
version: 1
links:
  - target: sources/diary/2026-10-10-升级轮对齐三件REQ-0018
    relation: derived_from
  - target: knowledge/adr/ADR-0003-解压与安装错误模式归档
    relation: derived_from
---

# M107:copy 型资产归档形态未嗅探

## PROBLEM

catalog 的 copy 型工具（extract 语义是「资产本体即单个可执行文件」）在上游把单二进制改打成归档发放时（yq v4.54.1 的 yq_linux_amd64.tar.gz 是 gzip 套 tar 内含 ./yq_linux_amd64 平台变体名、yq_windows_amd64.zip 内含 yq.exe），安装器把 tar/gzip/zip 归档本体原样复制到 bin 目录冒充可执行。双机全量升级轮同症暴露：装后验版本报「安装后未找到可执行文件或无法读取版本」，file 显示装出物是归档非 ELF。

## ROOT CAUSE

copy 语义假设资产形态稳定为裸二进制，但「语义在数据面（catalog extract 键）、形态在上游（release 资产布局）」两处真相无嗅探对账；上游资产布局演进（裸件改打包）不经过任何闸门直达安装器。同根因家族：ADR-0003 附录 M102（gh 2.98.0 zip 布局变更误展平），同属「资产实际形态与解包分支假设脱钩」。

## FIX

copy 型落盘前按魔数嗅探资产形态、归档剥层到二进制再落，装出物断言可执行头是入册验收的必过义务（非可选检查）。

## trace

- sources/diary/2026-10-10-升级轮对齐三件REQ-0018.md 件一节（双机病灶实录、修法与测试证据；提交 e63696e）
- knowledge/adr/ADR-0003-解压与安装错误模式归档.md 附录 M102（同根因家族：资产布局演进与解包分支脱钩）
