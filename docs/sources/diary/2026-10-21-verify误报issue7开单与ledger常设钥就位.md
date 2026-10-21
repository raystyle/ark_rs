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

## verify 误报同日修复

- 根因定案（lan-pve 实证，ark 1.7.0 / manifest seq 190，`ark verify` 实跑三 FAIL + status 三态取证 + 布局 ls）：
  - [实证] bun/zig 恒 FAIL 非 PATH 双模式、非 wrapper 形态：bun（evergreen 撤钉，2026-09-14 用户裁）与 zig（D43 去锁）均无 pin，旧判据 `locked != installed` 把已装未锁一律判 FAIL（zig 探针实测 0.17.0 解析正常；`~/.local/bin/zig` 是 78 字节 wrapper 脚本、rmux 用户位是符号链，探针与文件断言均不因形态误判，总台三疑因之二不成立）。
  - [实证] rmux 恒 FAIL 在 verify 注册表 files_all 断言路径陈旧：`~/.local/libexec/rmux/rmux` 不存在（实装在 `~/.local/share/rmux/libexec/rmux/rmux`）、`~/.local/bin/rmux-daemon` 不在装出面（探针 0.10.0 解析正常，总台三疑因之三不成立）。
  - [实证] bun install 后置判定找不到 bin：`bun-linux-x64.zip` 解出 `bun-linux-x64/bun` 嵌一层，展平在共享目录（`~/.local/bin` 多目录）不触发；兼 `extract_zip` 不恢复 zip 条目 unix 执行位、`set_executable_for_tool` 只认通用 exe 叶名（bun.exe）致 POSIX chmod 全落空（pve 侧总台手工 install -m755 才到位，同判）。
- 修法（commit c4bc895）：verify 判定引入 WARN 态（已装未锁如实报，文件组缺失恒 FAIL 压过，收割行四态，汇总附 WARN 名单行，WARN 不拦退出码）；rmux POSIX files_all 对齐实装布局；POSIX zip/targz 装后嵌套上提（平台 exe 与 extras 提齐期望位、zip unix 执行位恢复、变空包裹目录链清理、`{version}` 占位护栏）；`set_executable_for_tool` 期望位改平台 exe 相对位。附 chore 笔（1d97223）清 docker/toolver 预存 13 警（POSIX 门控面）。
- 验证：`cargo test --release --locked` 250 项全绿、fmt/clippy 零警、门禁四件套加 llms-guard 加 PEVO 加 aidoc --check --strict 全过；R005 对线一轮 CONFIRM（评审格 ark-rev-codex-1，零 F、G1 至 G8 过，回执 `/tmp/herdr-flywheel/2026-10-21-issue7-verify误报/review-round1.md`，ledger 产物 fe3c814e 已 attest_dev）。
- [实证] 实弹验收（lan-pve，`/tmp/ark-fix verify`）：8 维 bun=WARN、zig=WARN、rmux=PASS，FAIL 0、exit 0（修复前 bun/zig/rmux 三 FAIL exit 1）。验收判据「实装可跑站全 PASS 加空锁已装显 WARN」达成。

## hst 与 ark 锁陈化裁量

- [实证] pve catalog pin：hst v2.10.3（装 2.15.2）、ark v1.6.0（装 1.7.0）。
- 裁定：滚锁（hst 至 2.15.2、ark 至 1.7.0）。理由：锁定归数据面（D37，update/install 不回写），锁陈化不拦家族自升级（REQ-0014 委托通道）但驱动首装版本落后与 status drift 噪声；滚锁执行面在 omc（catalog 真源与镜像播种，本仓只管引擎消费面）。结论已回执总台，请转数据面执行并刷云端 seq。

## 消息面补记

- 总台 lan-win:w19:p1 推进令（开修 issue #7、实弹验收、锁陈化并行裁）经 fleet 流收讫；回执同走 fleet（lan-win 端 engine_fail 持续，修复回执时直投已恢复 delivered）。
- 总台锁滚转办回执称「hst.ohmygh.com v2.15.x 与 ark.ohmygh.com linux/mac 1.7.0 边车全 404、执行权在我侧资产补传」经实证反转：404 是探针路径形少 `{tool}` 段（引擎消费形 `{mirror_domain}/{tool}/{version}/{asset}`，`download.rs` 的 `mirror_url_at`）；`ark.ohmygh.com/ark/1.7.0/` 与 `hst.ohmygh.com/hst/2.15.2/` 三平台二进制加边车全 200，边车与 GitHub release 原件逐字节等（ark linux `6804d6c3…`、hst linux `0b4b42f9…`）；`hst/stable` 双域滚动段 200 且 stable 边车即 2.15.2 的 sha；`env.ohmygh.com/ark/stable/` 三平台 200。GitHub release 侧 v1.7.0（六件齐、2026-10-11 published）与 v2.15.2（八件齐）均完整（pve gh 凭据核）。结论：分发面零缺口，我侧零资产动作，总台可直接跑 catalog 锁滚（hst 三平台至 2.15.2、ark linux/mac 至 1.7.0，边车直取）加 lint 加 seed-local 刷 seq。
