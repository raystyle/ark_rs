---
id: REQ-0003
title: PATH 存量已达锁定版纳管
status: implemented
priority: must
trace: cargo test --release --locked 全绿 204 项（update 纳管与 install 穿透两用例在内）
---

# REQ-0003:PATH 存量已达锁定版纳管

## Scenario

自管位工具（如 hst 的 self-update 通道装 `~/.hst/bin`）在 PATH 已达锁定版本而 EnvRoot 未装时，`ark update` 仍判「未装真装」走 resolve 与下载，产 EnvRoot 双份副本且解析面网络差时直接失败（2026-09-16 宿主实锤：hst PATH 1.3.0 等于 pin，update 卡 GitHub API 限流三连 403）。

## Criteria

- [x] update 面新增纳管分支（resolve 之前）：PATH 命中且 PATH 探测版本达 pin 且 EnvRoot 未装，报 INFO 纳管跳过，`action=skipped`（不触 resolve 网络与下载）
- [x] PATH 版本落后 pin 时不纳管，仍走 D49 真装（语义边界：只有已达才跳）
- [x] install 面不纳管（收窄裁定）：install 是「显式装入管理面」意图，PATH 同版不拦（agent 类 D07 例外保留）；linux_install 真机闭环测试为此语义哨兵
- [x] 集成测试：PATH 假 exe（版本输出等于 pin）时 update skipped 纳管；install 同条件走装链（无效域失败即证穿透，失败点锚定下载段）
- [x] 对线修正三件（codex）：分支位置在自管与 rustup 与 vsbuild 与平台与 hold 判定之后（防截获 tools.ark 自管条目翻转 version=self-managed 机读契约）；判据用 pin_drift 数值口径（与 D49 同尺，防 v1.3.0 与 1.3.0 形态差异静默失效）；--force 走真装不纳管；EnvRoot 判存用 exists 短路不跑版本子进程
