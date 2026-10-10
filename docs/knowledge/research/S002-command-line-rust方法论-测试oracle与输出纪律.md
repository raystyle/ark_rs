# S002：Command-Line Rust 方法论，测试 oracle 与输出纪律

> 用 reader 对《Command-Line Rust》（Ken Youens-Clark, O'Reilly 2022）全书做定向研究，提炼 Rust CLI 开发方式，评估对 ome 的可用性。页码换算：PDF 页 = 书页 + 20（[实证] 多处页脚核对）。

## 一、研究方式

1. 工具：`reader search` 定位 + `reader extract --pages` 精读（[实证] 2026-09-01，reader 0.1.0，本书即其首个真实负载）。
2. 本书形态：14 章各克隆一个 Unix coreutils 工具（echo/cat/head/wc/uniq/find/cut/grep/comm/tail/fortune/cal/ls），教学法以「原工具即 ground truth oracle」为轴（[实证] 前言 xv，PDF 15）。

## 二、书的方法论要点

> 每条的页码均为 [实证]（书页，括号内 PDF 页）。

1. **结构**：1-2 章纯 bin，第 3 章起固定 bin+lib 双目标（lib 拆分「makes it easier to test and grow applications」，p50，PDF 70）；测试三件套 `tests/cli.rs` + `tests/inputs/` + `tests/expected/`；main.rs 极薄入口 `if let Err(e) = get_args().and_then(run) { eprintln!(); exit(1) }`（p53，PDF 73）。
2. **参数解析**：clap 2.33 builder（全书无 derive）；Config struct + `get_args() -> MyResult<Config>` + `run(config)` 三件套；静态互斥交 clap（conflicts_with），值域验证在 get_args 内手写验证函数并配单元测试（p51-52、p75-76，PDF 71-72、95-96）。
3. **测试**：assert_cmd + predicates + `type TestResult = Result<(), Box<dyn Error>>`（p37-38、p48，PDF 57-58、68）；oracle 由 mk-outs.sh 对原始工具批量生成 expected 文件，测试 `fs::read_to_string(expected)` 全量比对（p38-40，PDF 58-60）；负例统一 `dies_` 前缀可成组过滤（`cargo test dies`，cutr 10 个负例，PDF 201）；只断稳定列（Testing Underground，p355-357，PDF 375-377）；Windows 差异用 `.windows` 双 oracle + `#[cfg(not(windows))]` 门控负例（p164-165，PDF 184-185）。
4. **错误**：无 anyhow 无自定义错误类型，全书 `type MyResult<T> = Result<T, Box<dyn Error>>`（p50，PDF 70）；成功 0、错误一律 1；多文件输入部分失败 eprintln 播报后继续（p58，PDF 78）。
5. **IO**：`open()` 单点抽象，`"-"` 接 stdin，统一 `Box<dyn BufRead>`（p56-57，PDF 76-77）；`lines()` 剥行尾、要精确字节用 `read_line`（p57、p109，PDF 77、129）；夹具混 Windows 编码文件逼程序处理行尾差异（前言 xv，PDF 15）。
6. **输出纪律**：数据走 stdout、诊断走 stderr；退出码正确上报是「well-behaved」标准（p15、p51，PDF 35、71）；测试严格分通道断言。
7. **教学法**：coreutils 克隆 = 已知规范 + 免费 oracle；第 3 章起 test-first（自评更接近 test-first 的 TDD）（前言 xi、p48，PDF 13、68）。

## 三、对 ome 的可用性评估

> 对照 ome 现状（AGENTS.md、R004、src 与 tests 实证核查）。

### 已对齐

> 以下各项无需动作。

- bin+lib 拆分、assert_cmd/predicates/cargo_bin、TestResult + `?`、`dies_` 前缀、只断稳定字段、正负例成对、oracle 对照思想（ome 的 OME_TEST_REAL 真机闸门与书的 mk-outs.sh 同源，期望值均来自独立来源），逐项对应（[实证] 双侧文件比对）。

### 值得吸收

> 按价值排序。

1. **expected 文件 oracle + run helper**（[推断] 价值高）：ome 目前 `contains` 逐行断言；status/daily 多行输出用 `tests/expected/*.txt` 黄金文件全量比较更防回归，比 insta 快照透明可审。
2. **dies_ 成组过滤**：参数矩阵负例用统一前缀收拢，`cargo test dies` 一把跑（[推断] 随 daily/update 参数增多收益递增）。
3. **CRLF 双 oracle 与 cfg 门控负例**：ome 三端开发（Windows/WSL/mac）后可直接照搬 `.windows` 双期望文件模式（[推断]）。
4. **部分失败继续 + stderr 播报 + 汇总退码**：install/update all 的多工具场景适用，须与 OmeError 的 exit_code 体系调和（daily 的 exit 2 优先）（[推断]）。
5. **open() 单点抽象**：后续若读 stdin/文件混合输入可借（[直觉] 当前非紧急）。

### 冲突不宜采用

- clap 2.33 builder：2022 年历史局限，ome 的 clap 4 derive 严格更好，勿回退（[实证] 全书无 derive）。
- `Box<dyn Error>` 一把梭 + 错误一律 exit(1)：ome 的 Result<T,String> + OmeError（code/hint/exit_code）更严格，书的形态会丢掉 daily exit 2 语义（[实证] 对照 p51 与 src\omerr.rs）。
- 验证函数 `Err(From::from(val))` 把输入当错误：无上下文，不符 ome with_hint 形态（[实证] p76）。
- 真机 chmod 000 造负例：违反 ome 沙盒纪律，负例在临时 EnvRoot 造（[实证] p165 对照 R004）。

## 四、关键结论

1. ome 的错误体系、clap 形态、闸门纪律均在这本 2022 年教科书的水平之上；书真正值得搬运的是**测试侧三件套**：expected 文件 oracle、dies_ 成组过滤、cfg 双平台期望（[推断] 综合全章证据）。
2. 三者与 R004 既定演进路线衔接，落地时先改 R004 补「expected 文件 oracle」段落，再改造 status/daily 测试（[推断] 属后续目标，不在本研究落地）。
3. reader 作为研究工具链实证可用：search 定位 + extract 精读 + 页码证据闭环（[实证] 本轮全程）。
