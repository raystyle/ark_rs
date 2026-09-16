# ark::verify

verify：部署域验收维度（P0026 M3 第一批，数据驱动自 catalog 三态）。
维度注册表按平台生效，判定三层：catalog 工具组（locked==installed，强于 ps1 存在性断言）、
文件全在组（存在性，对齐 ps1 弱断言）、任一存在组（aria2 系统位）。
输出 `dim=PASS/FAIL/NA` 收割行（NA 不参与收割）。
密钥、secret-guard、mesh、compileMatrix 等非部署域不在本命令范围。

## Functions

- `dim_names` — 全部维度名（含跨平台分列重复；heal 注册表对齐校验用）。
- `run_verify` — 跑部署域验收（非流式兼容口）：内部走空回调。
- `run_verify_with` — 跑部署域验收（流式）：维度所需工具探完即经 emit 回调输出（保持注册表顺序中的可出即出），
- `summarize` — 汇总（--json 用）。

## Types

- `Verdict` — 维度判定结果。

