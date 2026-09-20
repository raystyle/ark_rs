# R013-Agent友好IO契约-输出格式退出码与冻结面

> ark 对外输出契约的唯一权威（自 README 收敛而来，2026-09-07 用户裁定 README 只留介绍、部署使用与软件清单；D41 前产品名 ome）。
> 吸收自 incurs 研究（S001）与 gh/git 实证（S003 三格式渲染、结构化错误、字段序稳定）。

## 零、功能原语口径

> PRD D10，2026-09-07。

三原语：doctor（检测诊断）、install（幂等安装：下载加 PATH、注册表与配置）、status（三态对照）。
其余命令为派生面，语义挂靠原语：query 为 install 的解析前置、update 为 install 时变、
pin 为锚操作（数据面）、verify 与 heal 为断言与自愈组合、init 与 self 为
辅助通道。命令面演进（增减改名）以原语口径评估归属。`deploy` 已去掉并入 install（D15）；
`daily` 已去掉，升级走 update（D16）；`package` 已去掉（D17）；`skill` 已去掉（D50，2026-09-16：
发现层归 `--llms` 唯一通道，头部含何时用与下载纪律行，SKILL.md 并行面退役）。
`catalog`（D33，2026-09-10）为清单面单一入口：status 看运行态清单来源与云端锚（D39 双轨收口后连 manifest 面一起看：在位、本地锚、年龄、云端锚对比、签名）、sync 立即刷新两件，
语义挂 query（解析前置的数据源）与 pin（锚操作）；自动刷新按 TTL 走，可用 `ARK_CATALOG_TTL`（秒，
0 关）与 `ARK_OFFLINE=1` 关闭，只作用于用户数据副本（仓库与 `ARK_CATALOG` 指定面不动）。
D34（2026-09-10）起，云端清单还须过内嵌公钥的 minisign 签名校验（本地与云端一起校验，见 S006）：
拉取落位前强校验，运行态副本每次加载前巡检；签名不符为错误退出（1），签名缺失只在运行态副本告警。

## 一、输出三格式

全局 `--format kv|json|jsonl`（默认 kv），`--json` 为 json 简写：

- **kv**：`key=value` 逐行，块间空行，`#` 注释行为分组标题（可滤）；
- **json**：整批输出一个 JSON 数组文档，stdout 恒为合法 JSON（无数据为 `[]`）；
- **jsonl**：每块一行 JSON 对象，逐工具/逐维度即出（流式与结构化兼得）。

值一律字符串，字段序与 kv 行序一致（serde_json preserve_order）。

## 二、数据与错误分流

- 数据只走 stdout；进度与提示（`[INFO]/[OK]/[WARN]`）走 stderr；
- 错误出口为 OmeError（code/message/hint/exit_code），main 按 exit_code 退出；
  结构化模式下错误以单行 JSON `{"code","message","hint"?}` 附 stderr 末行，退出码不变形。

## 三、命令数据块字段

| 命令 | 数据块字段 |
| --- | --- |
| `query` | tool, tag, version, asset, size, url, sha256（size 仅 API 解析路径有效；D51 起 pin 驱动零 API 恒 0） |
| `pin` | tool, tag, version, asset, sha256 |
| `install` / `update` | tool, action, version, dir（update 委托腿另出 channel=self-update 与 action=failed 值：家族自研 CLI 走其自身自升级；镜像腿缺省无 channel 字段、失败不出行只汇总） |
| `status` | tool, locked, installed, path, exe |
| `init` | action, exe, bin_dir, catalog, path |
| `verify` | name, verdict |
| `heal` | dim, action, params, result, detail |
| `issue new` | filed, issue, seq, kind, endpoint（REQ-0015 起真源 ledger.ohmygh.com；list 另出 count 与 `#n` 概览行加 has_more，count 是本次返回条数非在册总数，默认 limit 100 即服务端上限，恰打满出 stderr 截断提示，`--before <id>` keyset 游标翻更早一页；show 出 projection 与 timeline） |
| `artifact publish` | filed, artifact_id, kind, digest, endpoint（REQ-0015 产物共享库；attest 出 action 与 seq；list 出 count 与条目行） |
| `query`（D51 注） | pin 驱动默认零 GitHub API、镜像直装（url 如实呈现镜像资产域地址；`fallback_url` 官方直链由下载层兜底）；显式 `--latest`/`--tag`/`--version` 才走 GitHub API |
| `doctor` | check, status, detail；两层节 sys.* / dep（D30 起原 agent 节移除，装态对账归 omc、token 归 oma diagnose）；收尾 verdict（ready/degraded/broken）。TTY 为人读面，数据面不变 |
| `catalog` | status：path, origin, local_sha256, cloud_sha256, synced, age_secs, ttl_secs, offline, signature, pubkey, manifest_path, manifest_present, manifest_local_sha256, manifest_cloud_sha256, manifest_synced, manifest_age_secs, manifest_signature, manifest_cloud_error, cloud_error；sync：action, reason, sha256, path, origin |
| `--llms` | Markdown 命令清单（不经 render，先于 catalog 加载；D50 起唯一 agent 发现通道，头部含何时用与下载纪律行） |

## 四、退出码

| 码 | 语义 |
| --- | --- |
| 0 | 成功（含裸调用导航面：无参出紧凑指引，REQ-0011 cli-docs 采纳） |
| 1 | 失败（verify/doctor 有 FAIL 项、heal 有 fail/partial、安装出错） |

## 五、对外冻结契约

> issue #4，2026-09-02 冻结。

`query` 与 `status` 的 `--format json` 字段集与退出码（0/1）为对外契约。契约演进只做**增量字段**（消费方按名
取值不受影响），删除或改名视为 breaking，需在提交与 diary 显式标注。
query 的 `sha256` 字段语义：解析 tag 与资产同 pin 时给锁定 sha256（未回填为空串），否则空串。

**D41 更名 Ark 登记（2026-09-12 breaking；兼容层终态见 2026-09-18 剔除批）**：命令名 `ome` 改 `ark`
（数据块字段集与退出码零变化）。兼容层三面终态：旧部署位与旧 `ome` 别名停建、init 与 self update
顺带清扫存量（不重建）；环境变量仅 `ARK_*` 主名（`OME_*`/`OHMYENV_ROOT` 旧名读回已撤）；
镜像仅 `ark/` 段与 `ark-*` 资产名（`ome/` 段与 `ome-*` 兼容层已收口）。omc 侧冻结调用
`ome install` 的契约面换 `ark install`（herdr 知会在案）。
