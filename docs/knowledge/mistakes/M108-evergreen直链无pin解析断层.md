---
version: 1
links:
  - target: sources/diary/2026-10-10-升级轮对齐三件REQ-0018
    relation: derived_from
  - target: knowledge/adr/ADR-0002-版本解析与下载错误模式归档
    relation: derived_from
---

# M108:evergreen 直链无 pin 解析断层

## PROBLEM

撤钉 evergreen 的 CDN 工具（bun：cdn_url 是 releases/latest/download 直链、模板无 {version} 占位、无 pin）在全量 update 轮对它报「需 --version 指定版本（CDN 来源）」并计入失败汇总（exit 1）；query 全量同因整轮连坐失败。设计态（恒随官方最新）被当错误面输出。

## ROOT CAUSE

resolve 面的版本选择链只认「显式选项或 pin」两源，没覆盖第三源「URL 本身即 latest 滚动链接」；版本化模板（含 {version} 占位）与 evergreen 直链（无占位）混在同一个 cdn_url 分支里共用同一报错，未按占位性分叉。同根因家族：ADR-0002 附录（版本解析分支与数据面键形态脱钩类）。

## FIX

evergreen 直链以重定向终态取真实 tag 滚装（对象直链 302 非 API），设计态条目不得进失败汇总；版本化模板无 pin 仍如实报错。

## trace

- sources/diary/2026-10-10-升级轮对齐三件REQ-0018.md 件二节（病灶、重定向解析修法与 bun 滚装 1.4.3 实证；提交 e36f606）
- knowledge/adr/ADR-0002-版本解析与下载错误模式归档.md（版本解析分支形态脱钩家族）
