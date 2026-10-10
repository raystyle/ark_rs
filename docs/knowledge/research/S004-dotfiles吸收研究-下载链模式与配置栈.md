# S004-dotfiles吸收研究-下载链模式与配置栈

> 主题：参考吸收 raystyle/dotfiles（macOS 专用装机仓，Intel 机型，2026-09-06 仍活跃）：工具入册裁决、下载链模式、镜像对账与配置栈学习。2026-09-07 用户指令「再参考吸收下」。

## 一、仓概貌

- 定位：Intel macOS 专用开发环境（zsh · starship · sheldon · Agent CLI · iTerm2），`deploy.sh` 模块化部署（`--only` / `--verify` / `--apply`）。
- 与 lan-mac（M1，R011）是两台机器：本仓 `x86_64-apple-darwin`，lan-mac `aarch64`。[实证： README 范围声明与本仓资产注记]

## 二、吸收裁决

> 用户 2026-09-07 裁定。

| 项 | 裁决 | 落位 |
| --- | --- | --- |
| zoxide | 入册 catalog | `catalog\tools.toml` zoxide 节（v0.10.0，win 实装 deploy 绿、WSL 二进制就位） |
| sheldon | 入册 catalog（追加裁） | `catalog\tools.toml` sheldon 节（0.8.5；上游无 Windows 资产，platform_managed 空态语义同 shellcheck） |
| fd / bat / fzf / delta / lazygit / eza / repomix | 不入册：吸收学习的是 zsh、starship、sheldon 的配置方式（配置域成员，dotfiles 侧自理） | 本档第四节记档 |
| 下载链模式 | 吸收为 D08 设计输入 | 本档第三节 |
| 镜像配置 | 对账（一致，无改动） | 本档第五节 |

## 三、下载链四段模式

> D08 设计输入。

dotfiles `lib\download.sh` 的后端链 [实证： 源码通读]：

1. 段序：gh、api.github.com（curl 解析 asset id，匿名 60/h/IP）、aria2c、curl 依次回落；每段最多一次，硬超时（DF_TIMEOUT 180s）+ 失败或超时进下一段，缺后端跳段。
2. 镜像前缀：`GITHUB_DOWNLOAD_MIRROR` 设则 aria2/curl 先试镜像再原 URL。
3. 停滞探测：github.com web 直连段每进程探测一次，TLS 通但无响应即跳过不白烧（2026-07-19 北京联通实测 web 25s 零字节）。

对 D08（env.ohmygh.com 兜底）的输入 [推断： 设计对齐]：ome download 层的段式链语义同构，官方渠道（GitHub release / cdn）为第一段，env.ohmygh.com 镜像为回落段；每段硬超时与失败进段、镜像前缀按 `<tool>/<version>/<asset>` 模板拼。停滞探测（web 假活）是 dotfiles 实证过的真坑，ome 实现时应带。

## 四、zsh / starship / sheldon 配置栈

> 学习记档，不入册。

- 形态：zsh 加 starship（prompt）加 sheldon（插件声明式管理，`plugins.toml`）加 fzf（键绑定与 history）加 zoxide（`zoxide init zsh` 注入）；各 CLI（fd/bat/delta/eza 等）经 shell 配置集成而非独立管理。[实证： 仓内各配置文件]
- 与 ome 边界：二进制本体可入册（zoxide/sheldon 先例），shell 集成与插件配置属配置域不归 ome；oma 状态栏曾对齐 starship 风格（09-02 oma diary），配置形态同源。
- Intel mac 三应对（上游放弃 darwin-intel 时）：pin 末版（fd v10.3.0）、显式 `--allow-compile` 走 cargo、无包跳过（eza/repomix）。ome 未来接管 intel mac 时同款策略。[经验： dotfiles 实证]

## 五、镜像配置对账

| 配置 | dotfiles 值 | ome 现状 | 结论 |
| --- | --- | --- | --- |
| cargo registry | rsproxy sparse（`cargo\config.toml`） | rustup.rs 建模同源 rsproxy | 一致 [实证： 两仓文件对照] |
| GOPROXY / GOSUMDB | goproxy.cn / sum.golang.google.cn（`go\env.sh`） | heal-mirror goproxy 键同款 | 一致 [实证： 对照] |

## 六、入册实证细节

- zoxide 资产名**不带 v 前缀**（tag `v0.10.0`、资产 `zoxide-0.10.0-...`）：首版 pattern 带 v 全不匹配，实测修正。[实证： gh api 资产清单]
- zoxide win zip 解出根级 `zoxide.exe` 加 completions/man（dir=zoxide、exe=zoxide\zoxide.exe 直中）；linux/mac tar.gz 走 targz-bin。
- sheldon 0.8.5 资产仅 linux musl 三架构加 aarch64-darwin：无 Windows 与 darwin-x86_64 包。[实证： gh api]
- toolver 补 zoxide/sheldon 正则（「名 版本」形态；sheldon --version 多行输出取首行命中）。
- 验版本步依赖 toolver 正则：WSL 旧 ome 二进制无新正则报「无法读取版本」误拦，二进制实际就位；新 dev 资产后自愈。[实证： WSL 实跑]

## 结论

- 吸收三件落地：zoxide 与 sheldon 入册（43 工具）、下载链四段模式进 D08 设计、镜像对账记录一致；配置栈（zsh/starship/sheldon）记档学习不入册。[实证： 本批提交]
- `已验证`: zoxide win 侧 install 幂等加 sha 回填加 deploy 全绿；WSL 双二进制就位（状态回写待新 dev 资产自愈）。
