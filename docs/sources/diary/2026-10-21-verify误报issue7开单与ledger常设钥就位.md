# 2026-10-21 verify误报issue7开单与ledger常设钥就位

## 任务面

- 总台（lan-win:w19:p1）通报三件：verify 误报 issue 认领进本域、pve 侧实装变更通报（bun 1.4.3 / zig 0.17.0 / rmux 0.10.0 实装、bun zip 嵌套布局坑）、hst 与 ark 自身锁陈化裁量归本格。
- 另格 lan-win:w1G:p1 两问 ledger.key 短事实（开单能力）。
- 本格原为无名格（wP:p1），`herdr agent rename` 正名 ark-rs-claude-1（标准形），回信路由恢复正名。

## ledger 常设钥

- [实证] NOKEY 两轮核验：`~/repos/ark_rs/ledger.key` 与 `~/.ledger.key` 均无（ls exit 2）；`find ~ -maxdepth 5` 全查零命中；`~/.config/ark/` 不存在；EnvRoot（`~/.local/share/ohmyenv`）仅 cache。
- [实证] 总台铸钥落地：`~/.config/ark/ledger.key` 就位（0600、64 字节），公钥 kid 830f2b26 注册 ledger（repo github.com/raystyle/ark_rs，active；旧钥 7f365848 仍 active 不冲突，D34 双钥过渡口径同形）。此钥为本台 ledger 写面常设钥（issue/artifact），勿外传勿进仓。

## issue #7 开单

- [实证] `ark issue new` 直开成功：issue #7（kind bug、status open），url <https://ledger.ohmygh.com/repos/github.com/raystyle/ark_rs/i/7> ，`ark issue show 7` 复核标题与状态在册。
- 单文要点：ark verify 对 bun/zig/rmux 恒 FAIL 而三者实装实跑正常；三疑因 = bun manifest 空锁判 FAIL、zig wrapper 脚本形态 ELF-only 判定、rmux --version usage 形探针解析失败；实证环境 lan-pve（ark 1.7.0、manifest seq 190、PATH 双模式已排除）；验收 = 实装可跑站全 PASS、空锁且已装显 WARN、wrapper 脚本与符号链纳入判定、bun zip 嵌套布局下 install 后置判定能定位 bin。
- 关联注：bun 发行 zip 解出嵌套一层 bun-linux-x64/bun、install 后置判定没找到 bin 需手工 install -m755（pve 侧 2026-10-21 实况），疑与疑因 2 同源（install/verify 共用形态判定）。
- 回执：issue 号已回总台。

## 消息面

- [实证] lan-win 端当窗不可达：对 w19:p1 与 w1G:p1 的 `agents to` 直投 engine_fail 共五次，`agents view` 亦跳过 lan-win（机册双行 lan-win、reachable 与详查矛盾）；按协作惯例回执全落 fleet 流（总台 tail 取阅）。
- [实证] `room/deferred` 无本机代记：engine_fail 不落代记（代记仅门控拒投），回执改走 fleet 补面，消息未静默丢失。
- 待裁：lan-win 引擎故障若持续，总台侧自愈或人工介入（本单不承载）。

## 待办

- verify 误报修复进本域（issue #7）：五步闭环，先搜索引定位 verify/install 判定源码；实质代码改动过 R005 对线门。
- hst（2.15.2 装 / 2.10.3 锁）与 ark 自身（1.7.0 装 / 1.6.0 锁）锁陈化裁量：核 D37 锁定归数据面与 REQ-0014 家族自研通道口径后定，结论回执总台。
