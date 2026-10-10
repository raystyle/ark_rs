# 2026-09-14 D46 Windows 构建切 gnu 交叉编译

## 决策与背景

用户裁定（2026-09-14）：Windows 构建切 gnu 交叉编译，摆脱 VC。ohmycloud 侧先行实证（不必重验）：WSL 加 rustup target x86_64-pc-windows-gnu 加 apt mingw-w64 后 cargo build 一次过（含 ring 汇编与 xz2 的 C）；产物部署 lan-win 实跑 --version 与 query（cloud manifest 拉取、minisign 验签、seq 校验）全过；CRT 静态零 DLL 依赖；尺寸 11.8MB 与 msvc 同量级。

## 落地

- build.yml：windows-latest msvc 岗整体换 ubuntu-latest 交叉岗（rust-toolchain targets 挂 x86_64-pc-windows-gnu、apt 装 mingw-w64）；资产名 ark-x86_64-pc-windows-gnu.exe；三岗统一带 --target 与 target 路径归位；交叉岗跳过测试与文档门禁（PE 不可在 linux 跑，测试面由 linux/mac 双岗覆盖）。
- selfupdate.rs：platform_triple windows 臂切 gnu；新增 asset_msvc_fallback（仅 Windows 有此层）；official_asset_meta 与 mirror_attempts/mirror_fallback_meta 签名扩 msvc 回退参数，读序三层（gnu 主名先、ark/ 段 msvc 回退名次、ome/ 段兼容名殿后）；fallback_seg 注释明确 msvc 名属 ark- 族走主段（逻辑天然覆盖）。
- 单测：镜像段读序尝试表三层顺序断言（gnu 先、msvc 次、ome 殿后）加缺层跳过；段内回落裁定补 msvc 名断言。
- mirror_fallback.rs 补收昨日漏网：ome/ 段删桶后锚链 404 的两个 gated 用例，「兼容段边车锚一致」整删、「双段同内容」对照断言退役（保留主段主名腿）。

## 语义边界

- 新 gnu 二进制对旧源（stable 段与历史 release 仅剩 msvc 资产的窗口期）：主名 miss 后 msvc 回退名命中即保供，下个 stable 封 gnu 版后回退不再触。
- 旧 msvc 二进制对新 gnu 源：读序无 gnu 名全 miss，无回退（有意），升级走 omc catalog 通道重装。
- 版本号不动、不推 tag（stable 封版另裁）；推 main 后 dev 滚动源自动出 gnu 资产。

## 对线双线回执与修正

- **ohmycloud（H1 必修）**：build.yml win 岗 asset 名丢 .exe 后缀（与 selfupdate 主名不逐字一致则 win 主名永久 miss）。修：补 .exe。
- **codex 右侧对线（F1 必修加 G1-G5）**：
  - F1：seed.py `_TRIPLES` 首项仍 msvc 名，镜像灌段面等于没落地。修：切 gnu 名；`download_asset` 三态化（ok/skip/fail），自产灌段 404 记 skip（D46 前 tag 重灌窗口期，旧 tag 无 gnu 资产不红）。
  - G1：Windows 的 ome 兼容名随 gnu 三元组派生从未存在（历史 ome 资产是 msvc 名），该层 Windows 恒 miss、窗口已由 msvc 回退层覆盖。修：注释对齐实况。
  - G2：official_asset_meta 每层各拉一次 release JSON。修：单拉一次本地按名序匹配（asset_in_release 拆分）。
  - G3：native 岗测试步与构建步编译目录分叉（依赖树编两遍）。修：测试步带 --target 复用缓存。
  - G4：文档义务三处（GOAL 当前目标、INDEX diary 一览两笔漏登属 G004 同型二犯、PRD 状态先行中）。修：全补。
  - G5：doctor 探针仍硬编码 msvc 名（域通探针 404 容忍不误报但与主名不同源）。修：切 gnu 名同步 :897 断言；白名单补 gnu 名断言；asset_msvc_fallback 补非 windows None 门控单测。
- 四问答复收悉：裸环境实证（cc crate 自动探测 mingw 交叉器）、桶内 ark 段 msvc 边车双 200（回退链有效）、用例退役无异议、lan-win 升级走 omc 通道（ark 段 msvc 旧对象保留待舰队升级完再清）。
- 修后验证：cargo test --release 138 全绿；md 门禁四件套绿；fixup 并入原两提交（c1ace26 feat 加 9ccaf72 docs）。

## 验证与对线

- 本机交叉复验：cargo build --release --locked --target x86_64-pc-windows-gnu 一次过，PE32+ 12.4MB。
- cargo test --release --locked 全绿（137 单测加集成面）。
- 文档门禁四件套绿。
- 对线：codex 右侧（w3:p2）加 ohmycloud（w4:p1）双线，修正全落（见上节）。
- **CI 验收（终态）**：run 34840170118 四岗全绿（macos aarch64、ubuntu linux、ubuntu windows-gnu 交叉、mirror-r2）；dev release 资产 ark-x86_64-pc-windows-gnu.exe 到货（历史 msvc 与 ome-* 残量资产留存即窗口期回退面）；桶 ark/dev 的 gnu 资产与 .sha256 边车双 200（curl 实测）。

## 封版 v1.2.2

- 版本号 1.2.1 到 1.2.2（Cargo.toml 加 Cargo.lock）；CHANGELOG Unreleased 两条归 1.2.2（ome 兼容面收口加 D46 gnu 切换）；ROADMAP 阶段注记与 TODO 补封版。
- 验收回执四件（终态）：tag run 34841367924 四岗绿；stable release 三资产 ark-x86_64-pc-windows-gnu.exe、ark-x86_64-unknown-linux-gnu、ark-aarch64-apple-darwin（正式版无残量，干净）；镜像 ark/stable 段 gnu 资产与边车双 200；digest 清单入 ohmycloud 回执。
- 曲折实录：首推 tag run 34841367924 前身（34841035162）红在 linux 岗文档门禁，diary 两处括号标题，本地门禁四件套跑子集漏 heading-scan（M006 第八犯，已接编）；修标题重指 tag（c0fcfc7 到 d4e8571）后全绿。
