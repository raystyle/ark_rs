# ark::docker

docker：Windows 容器 Docker Engine 接管（自 ohmypwsh set-docker.ps1 完整迁移，2026-09-02）。
形态：官方 static zip（download.docker.com CDN 直链，pin 驱动，dotnet/oscdimg 同族）+
Windows 服务注册 + daemon.json 合并 + docker-users 组 + compose 插件 + 机器级 PATH。
需管理员：未提权且 gsudo 在位时经 gsudo 重跑 `ark install docker`（vsbuild 同模式）。
与 vsbuild 的差异：docker 有版本与 pin（非 evergreen），resolve 走 cdn_url 分支。

## Functions

- `install` — 非 Windows 占位：docker-win 仅 Windows 语义。
- `is_docker` — 是否 docker 专用安装条目（extract 标记分发）。

