# ark::doctor

doctor：部署异常诊断（2026-09-02 用户需求：检测部署的错误异常）。
与 verify 的分工：verify 答「部署域维度是否 PASS」（pin 对齐断言），doctor 答
「环境里有哪些部署错误与异味」——版本漂移、探测失败、PATH 死链与重复、
pin/sha 缺失、缓存孤儿、EnvRoot 不可写等，输出 check=OK/WARN/FAIL 逐项行，
FAIL 即 exit 1（WARN 不拦退出）。

## Functions

- `check_desc` — check 项的人读描述（TTY 报告面用）：老十项 OK 时 detail 为空，名字本身意义不明，
- `dep_group_stats` — 依赖层九类分组统计（逐组工具数、缺失数、漂移数）。
- `run_doctor` — 收集全量形态（run_doctor_with 的空回调兼容口）。
- `run_doctor_with` — 跑全部诊断项，流式形态：每项算完即经回调输出（三态采集期间先出 envroot 项，
- `run_doctor_with_status` — 同 run_doctor_with，但复用调用方已采集的三态行（cmd_doctor 三层诊断共用一次采集，
- `summarize` — 汇总：FAIL 与 WARN 计数。
- `system_facts` — 采集系统层事实（OS、架构、AVX 族指令集在位性）。

## Types

- `DepGroupStat` — 依赖层分组统计（九类 taxonomy 逐组：工具数、缺失数、漂移数）。
- `DoctorRow` — 单项诊断结果。detail 为该项的明细（stderr 人称提示用）。
- `SysFacts` — 系统层事实（非诊断，不占 OK/WARN/FAIL 三态）。

