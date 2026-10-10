# S008 模型校验选型：serde 反序列化加 validator 校验

> 2026-09-13。触发：用户供给选型情报（Rust 里最接近 Pydantic「模型加校验」的主流路径）；
> 背景：D42 刚为 manifest DSL 手写键值两谓词（`env_key_sane`/`env_value_sane`），
> 本档定该栈的事实面与 ark 的引入时机，防下次同类需求重新调研。

## 结论

模型校验主流栈为**两步走**：先 serde `Deserialize` 再 `Validate::validate()`，非法对象短暂存在、不进领域层（领域侧 `TryFrom` 收 newtype）。TOML 与 JSON 同型复用（同一 `#[derive(Deserialize, Validate)]` 只换解析入口）。**ark 当前不引入**（校验面是两谓词、HashMap 键为主力且 validator 无 map 键属性，净收益低于依赖成本）；引入触发条件见文末。`toml` crate 不引入：ark 解析面已在 `toml_edit::de`（`toml` 内核同源，manifest.rs 既有选择），R005 最少接线原则维持。

## 事实面

| 事实 | 状态 |
| --- | --- |
| validator 最新 0.21.0、总下载 64,477,515、2026-07-27 更新 | [实证：crates.io API 2026-09-13] |
| toml 最新 1.1.6+spec-1.1.0（TOML 1.1）、总下载 900,771,085、2026-09-10 更新 | [实证：crates.io API 2026-09-13] |
| 新项目钉 validator 0.21（README 示例常写 0.20 已过时）；Axum 生态默认接 axum-valid 的 `Valid<T>`（默认后端即 validator） | [记忆：用户情报 2026-09-13，未本机实跑] |
| 0.18 起嵌套必须 `#[validate(nested)]` 且内层也 `Validate`；`Option` 仅 `Some` 时校验、缺值用 `required`；context 用 `validate_with_args` | [记忆：同上] |
| 常用规则：`email`/`url`/`length`/`range`/`must_match`/`contains`/`regex`/`required`/`nested`/`custom`/`schema`（跨字段）/`non_control_character`；均可加 `message`、`code` | [记忆：同上] |
| 错误聚合 `ValidationErrors`：`Field(Vec<ValidationError>)`、嵌套 `Struct`、列表 `List(BTreeMap<usize, _>)`、schema 错进 `__all__` | [记忆：同上] |

## 与 ark 的适配评估

- **无耦合障碍** [推断]：validator 校验在反序列化之后，与解析 crate 解耦（ark 用 `toml_edit::de` 不受影响）；schema 拒载门（R016 前进兼容红线）照旧先于 validate。
- **覆盖差口** [推断]：ark 主校验面是 **HashMap 键**（`mirror.env` 键、`env_set` 键）与 Vec 键，validator 无 map 键属性，键校验仍要自定义函数；四个 `Option<String>` URL 键可吃 `#[validate(url)]`；`cargo_config` 整文件体（多行带引号）豁免面。故 derive 只覆盖小半面。
- **错误面风格** [推断]：ark 错误走单行 omerr（R013），`ValidationErrors` 的字段路径聚合需再转一层才有价值；当前 lint 与引擎共用两谓词的单一权威形态更贴仓内纪律（双份并行必漂移）。
- **不引入的近因**：D42 刚经三轮对线 CONFIRM 收口，校验面 15 行内，换栈属无收益扰动。

## 引入触发条件

满足其一即立项迁 derive：校验规则超出键值形态进入跨字段或数值域（如端口范围、schema 跨字段一致性）；DSL 新增字段自带标准规则（url/email/length）；或输出面需要结构化校验错误（字段路径聚合）。届时 `Mirror`/`ToolManifest` derive `Validate`，lint 侧复用 `validate()` 收单一权威。

## 迁入陷阱照查

- 必填字段缺失在 serde 层报错，到不了 `validate()`；serde 失败（类型不对）与 validator 失败（值不合法）要分流报告。
- TOML 与 serde_json 同型但**不做宽松强制转换**（`"8080"` 字符串进不了 `u16`，非 Pydantic lax）；要转换走 serde_with。
- 同一 DTO 勿混 garde/validify；未校验值不塞全局状态（热更新同样 from_str 加 validate 后再换入）。
