# ark::delegate

delegate：家族自研 CLI 的 update 委托腿（用户裁定 2026-09-19「我们自己维护的
自升级的 CLI，ark 对齐也走 CLI 自升级渠道；其余不是我们维护的 CLI 对齐 omc 维护 env」）。
家族五员（ark 自身、hst、browse、reader、officecli）在 `ark update <tool>` 面委托
调用该 CLI 自身的自升级命令，吃其独立域通道与既有锚校验、回滚与自证（家族标准形）；
ark 不为五员自建镜像下载腿。其余非自研工具照旧镜像优先加官方兜底（D44 面不变）。
让位契约联动（家族仓零改动）：家族自更新检出 ark-managed 落痕即拒自升，故委托前
临时撤落痕（rename 挪开）、自升级完成（含失败）后恢复落痕。

## Functions

- `run` — 委托执行一次家族自升级：定位真身 exe → 临时撤 ark-managed 落痕 → 调家族自升级命令
- `self_update_args` — 家族五员自升级命令路由（exe 后参数；改令 2026-09-19）：返回 None 即走镜像腿。

## Types

- `DelegateOutcome` — 委托腿结果。

