//! ark CLI 入口：query / pin（lock 别名）/ install / update / status / init（self-deploy 别名）
//! / verify / heal / doctor / self。
//!
//! 输出纪律（吸收自 incurs 研究 S001，S003 扩展三格式）：
//! - stdout 只走数据：默认 key=value 逐行，`--format json|jsonl` 或 `--json` 切结构化，
//!   统一经 render 层输出，命令里不散写 println!；
//! - 人称提示（[INFO]/[OK]/[WARN]/[HINT]/[跳过] 等）一律 stderr；
//! - 错误出口为 ArkError（code/message/hint/exit_code），main 按 exit_code 退出；
//!   结构化模式下错误以单行 JSON 附 stderr 末行，stdout 保持纯数据。

use std::path::Path;

use clap::{Parser, Subcommand};

use ark::arerr::ArkError;
use ark::catalog::{self, Catalog};
use ark::install::{install_tool, InstallOptions, InstallOutcome};
use ark::render;
use ark::resolve::{resolve_tool, Resolution, ResolveOptions};
use ark::status;
use ark::toolver;

/// --llms 紧凑命令清单（D09 发现层；D50 起唯一 agent 发现通道：头部含何时用与下载
/// 纪律行，单源自持，不再有 SKILL.md 并行面；REQ-0011 cli-docs 采纳节序：定位加版本
/// 注入、读序、命令、退出码、输出契约；版本从 Cargo.toml 载体 env! 注入禁手写）。
/// 三原语（PRD D10/D15）：doctor / install / status；其余派生面。
const LLMS_MANIFEST: &str = concat!(
    "# ark：命令清单（工具与 agent 二进制的部署管理诊断）\n",
    "\n",
    "> Ark（Agent Runtime Kit）：本机跨平台环境部署管理 CLI。版本 ",
    env!("CARGO_PKG_VERSION"),
    "。手册 curated 单源，与 --help 同源由漂移守卫测试锁定。\n",
    "\n",
    "## 读序\n",
    "\n",
    "装环境从 doctor 起、装工具 install、对齐更新 update、三态对照 status；其余按命令表直达。\n",
    "原语三件：doctor 检测诊断、install 幂等安装、status 三态对照；其余为派生面。\n",
    "全局：--format kv|json|jsonl、--json、--env-root PATH、--llms。数据 stdout、提示 stderr、错误单行 JSON。\n",
    "命令级常用旗标：query/pin/install 共用 --latest、--tag、--version；install/update 另有 --force；verify 有 --check；heal 有 --dry-run。\n",
    "何时用：装、查、管、诊断环境一律走 ark（不手拼官方 URL、不裸 curl release 资产、不手写 PATH 注册表）；install/update 幂等检测安装，重跑零副作用。\n",
    "下载：默认自建镜像（工具专属分发域或 env.ohmygh.com）、未命中秒级回落官方；有 sha 锚（catalog pin、镜像 .sha256 边车或官方清单）必校验，锚不符即换道。\n",
    "\n",
    "## 命令\n",
    "\n",
    "| 命令 | 语义 | 关键输出 | 退出码 |\n",
    "| --- | --- | --- | --- |\n",
    "| ark doctor | 原语·检测诊断（系统/依赖两层+check 节：环境错误/配置健康/部署深诊/网络通连） | sys.* dep= check= verdict | 1=check 有 FAIL |\n",
    "| ark install [名] | 原语·幂等安装（下载+PATH/注册表/配置；省略则全量） | tool,action,version,dir | 0/1 |\n",
    "| ark status | 原语·三态对照（锁定/已装/PATH） | tool,locked,installed,path,exe | 0/1 |\n",
    "| ark query [名] [--latest] | 解析版本与资产不安装（省略则全量） | tool,tag,version,asset,sha256 | 0/1 |\n",
    "| ark update [名] | 家族自研 CLI（hst/browse/reader/officecli）委托其自身自升级通道（channel=self-update 标注，--force 不作用）；其余工具对齐云端锁定安装（镜像优先零 GitHub API；落后补装、领先如实报，不回写锁定）；省略则全量混合 | 同 install 加 channel | 0/1 |\n",
    "| ark pin [名] [--latest\\|--version V] | 查看/设置锁定（省略则全量；lock 别名） | tool,tag,version,sha256 | 0/1 |\n",
    "| ark init | 部署自身到用户目录并同步 catalog（幂等） | action,exe,catalog,path | 0 |\n",
    "| ark verify [--check a,b] | 部署域验收维度（省略则全量） | name,verdict | 1=有 FAIL |\n",
    "| ark heal [维度] [--dry-run] | 部署维度幂等自愈（省略则全量） | dim,action,result | 1=有 fail |\n",
    "| ark catalog [status\\|sync] | 派生·运行态软件清单：status 看解析面/云端锚/同步态与 manifest 面（在位/本地锚/年龄/云端锚/签名），sync 立即从云端刷新两件（边车锚，ARK_CATALOG_TTL 与 ARK_OFFLINE 只管自动刷新） | path,origin,local_sha256,cloud_sha256,synced,manifest_present,manifest_local_sha256,manifest_cloud_sha256,manifest_synced 或 action,sha256 | 0/1 |\n",
    "| ark self update [--stable\\|--git] | 升级自身三通道（默认镜像段读序、边车即锚、官方 API 兜底；ARK_MIRROR=0 官方优先逃逸阀） | exe,sha256 | 0/1 |\n",
    "| ark issue new\\|list\\|show\\|close | 统一 issue 入口（新真源 ledger.ohmygh.com）：new 开单（kind 为 bug 错误任务或 improvement 改进优化任务加 acceptance 验收条件）；list 读面（默认 limit 100 即上限，count 是本次返回条数非在册总数，--before 翻更早一页）；show 详情；close 关单（result 引 digest 加 status done） | filed,issue,seq 或 count,#行 或 单条或 action=closed | 0/1 |\n",
    "| ark artifact publish\\|attest\\|promote\\|list | 产物共享库（ledger.ohmygh.com）：publish 发布（kind 十五类加 digest=sha256 正文哈希，库不收二进制实体）；attest 证明（attest_dev/attest_prod/verification_failed/promote/demote/supersede）；promote 晋级当前版；list 列表（current/env/kind/name 过滤） | filed,artifact_id,seq 或 count 行 | 0/1 |\n",
    "\n",
    "## 退出码\n",
    "\n",
    "| 码 | 义 |\n",
    "| --- | --- |\n\n",
    "| 0 | 成功（含裸调用导航面） |\n",
    "| 1 | 失败（verify/doctor 有 FAIL 项、heal 有 fail/partial、安装出错） |\n",
    "\n",
    "## 输出契约\n",
    "\n",
    "细契约：仓库 docs\\references\\R013（输出格式/退出码/冻结面）。\n",
);

// ── 帮助示例元数据（各子命令示例集中于此，经 after_help 挂进帮助）──
const EX_QUERY: &str = "示例:\n  ark query\n  ark query gh --latest";
const EX_PIN: &str = "示例:\n  ark pin\n  ark pin git --latest\n  ark lock git --version 2.55.0";
const EX_INSTALL: &str = "示例:\n  ark install\n  ark install git\n  ark install --force";
const EX_UPDATE: &str = "示例:\n  ark update\n  ark update gh";
const EX_STATUS: &str = "示例:\n  ark status";
const EX_INIT: &str = "示例:\n  ark init";
const EX_VERIFY: &str = "示例:\n  ark verify\n  ark verify --check toolRoot,localbin16 --json";
const EX_HEAL: &str = "示例:\n  ark heal\n  ark heal aria2 --dry-run";
const EX_DOCTOR: &str = "示例:\n  ark doctor\n  ark doctor --json";
const EX_CATALOG: &str = "示例:\n  ark catalog\n  ark catalog status --json\n  ark catalog sync";
const EX_SELF: &str =
    "示例:\n  ark self update\n  ark self update --stable\n  ark self update --git";
const EX_ARTIFACT: &str = "示例:\n  ark artifact publish --name \"镜像通道教训\" --kind lesson --digest sha256:<64hex> --summary \"一句话\"\n  ark artifact attest <id> --attest-type attest_dev\n  ark artifact promote <id>\n  ark artifact list --current";
const EX_ISSUE: &str = "示例:\n  ark issue new \"doctor 报 PATH 重复\" --kind bug --acceptance \"复现步骤与修复验证\"\n  ark issue list --limit 3 --before 51\n  ark issue close 3 --digest sha256:<64hex>";

#[derive(Parser)]
#[command(
    name = "ark",
    bin_name = "ark",
    version,
    about = "Ark（Agent Runtime Kit）：全平台 Agent 工具及运行时依赖环境的部署、管理、验收与诊断 CLI",
    // cli-docs 帮助面头行（REQ-0011）：name@version 连接一句描述（版本注入勿手写）
    help_template = "{name}@{version} {about}\n\n{usage-heading} {usage}\n\n{all-args}{after-help}"
)]
struct Cli {
    /// 环境根目录覆盖，默认读取 ARK_ROOT 或平台默认路径
    #[arg(long, global = true)]
    env_root: Option<String>,

    /// 以 JSON 数组输出数据，等价 --format json
    #[arg(long, global = true, conflicts_with = "format")]
    json: bool,

    /// 输出格式：kv 逐行键值、json 整批数组、jsonl 逐块单行
    #[arg(long, global = true, value_enum)]
    format: Option<FormatArg>,

    /// 打印 agent 紧凑命令清单（markdown 表）后退出，零安装可用
    #[arg(long, global = true)]
    llms: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

/// --format 取值（映射 render::Format）。
#[derive(clap::ValueEnum, Clone, Copy)]
enum FormatArg {
    Kv,
    Json,
    Jsonl,
}

impl From<FormatArg> for render::Format {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Kv => render::Format::Kv,
            FormatArg::Json => render::Format::Json,
            FormatArg::Jsonl => render::Format::Jsonl,
        }
    }
}

/// --latest / --tag / --version 三选项（query / pin / install 共用）。
#[derive(clap::Args, Clone, Default)]
struct VersionOpts {
    /// 解析最新版
    #[arg(long, conflicts_with_all = ["tag", "version"])]
    latest: bool,
    /// 指定 release tag
    #[arg(long, conflicts_with = "version")]
    tag: Option<String>,
    /// 指定版本号
    #[arg(long)]
    version: Option<String>,
}

impl VersionOpts {
    fn is_empty(&self) -> bool {
        !self.latest && self.tag.is_none() && self.version.is_none()
    }
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// 解析工具版本与下载资产，不落盘；省略工具名则全量
    #[command(after_help = EX_QUERY)]
    Query {
        /// 工具名；省略则全量
        #[arg(default_value = "all")]
        tool: String,
        #[command(flatten)]
        opts: VersionOpts,
    },
    /// 查看或设置版本锁定；省略工具名则全量
    #[command(visible_alias = "lock", after_help = EX_PIN)]
    Pin {
        /// 工具名；省略则全量
        #[arg(default_value = "all")]
        tool: String,
        #[command(flatten)]
        opts: VersionOpts,
    },
    /// 安装工具：下载解压、注册 PATH、写注册表与配置；省略工具名则全量
    #[command(after_help = EX_INSTALL)]
    Install {
        /// 工具名；省略则全量
        #[arg(default_value = "all")]
        tool: String,
        #[command(flatten)]
        opts: VersionOpts,
        /// 强制重装，跳过幂等检查
        #[arg(long)]
        force: bool,
    },
    /// 对齐云端锁定安装（锁定归数据面，不回写 pin；临时钉版走 pin）；省略工具名则全量
    #[command(after_help = EX_UPDATE)]
    Update {
        /// 工具名；省略则全量
        #[arg(default_value = "all")]
        tool: String,
        /// 强制重装，跳过幂等检查
        #[arg(long)]
        force: bool,
    },
    /// 对照锁定版本、已装版本与 PATH 三态
    #[command(after_help = EX_STATUS)]
    Status,
    /// 安装自身到用户程序目录，同步 catalog 并注册 PATH，幂等
    #[command(alias = "self-deploy", after_help = EX_INIT)]
    Init,
    /// 按部署维度验收环境一致性，失败返回非零；省略则全量
    #[command(after_help = EX_VERIFY)]
    Verify {
        /// 只检查指定维度，逗号分隔；省略则全量
        #[arg(long)]
        check: Option<String>,
    },
    /// 幂等自愈指定部署维度；省略则全量
    #[command(after_help = EX_HEAL)]
    Heal {
        /// 维度名；省略则全量
        #[arg(default_value = "all")]
        dim: String,
        /// 只打印将执行的动作，不执行
        #[arg(long)]
        dry_run: bool,
    },
    /// 诊断部署异常：版本漂移、PATH 死链重复、锁定缺失、缓存孤儿等，失败返回非零
    #[command(after_help = EX_DOCTOR)]
    Doctor,
    /// 运行态软件清单：查看解析面与云端同步态，或立即从云端刷新（D33）
    #[command(after_help = EX_CATALOG)]
    Catalog {
        #[command(subcommand)]
        cmd: Option<CatalogCmd>,
    },
    /// ark 自身管理
    #[command(name = "self", after_help = EX_SELF)]
    ArkSelf {
        #[command(subcommand)]
        cmd: SelfCmd,
    },
    /// 统一 issue 入口：new 一键提交（自动带 tool=ark 与版本/平台/host）、list/show 读面
    #[command(after_help = EX_ISSUE)]
    Issue {
        #[command(subcommand)]
        cmd: IssueCmd,
    },
    /// 产物共享库（ledger.ohmygh.com：publish 至 attest 至 promote）
    #[command(after_help = EX_ARTIFACT)]
    Artifact {
        #[command(subcommand)]
        cmd: ArtifactCmd,
    },
}

/// `ark catalog` 子命令面（缺省 status）。
#[derive(Subcommand, Clone)]
enum CatalogCmd {
    /// 打印清单状态：解析面路径与来源、本地与云端锚、检查年龄、TTL、是否同源
    Status,
    /// 立即从云端刷新用户数据副本（先边车锚后资产；不受 ARK_CATALOG_TTL 与 ARK_OFFLINE 限制）
    Sync,
}

/// `ark self` 子命令面。
#[derive(Subcommand, Clone)]
enum SelfCmd {
    /// 升级自身：默认 dev 滚动源，--stable 拉 latest 正式版，--git 源码构建
    #[command(alias = "upgrade")]
    Update {
        /// 拉 latest 正式版（v* tag 封版产物）
        #[arg(long, conflicts_with = "git")]
        stable: bool,
        /// 源码安装：浅克隆仓库 cargo build 后替换（封版前通道，需 git 与 cargo）
        #[arg(long, conflicts_with = "stable")]
        git: bool,
    },
}

/// `ark issue` 子命令面（REQ-0015，ledger.ohmygh.com 仓级公共账本；issues.ohmygh.com 过渡期保役）。
#[derive(Subcommand, Clone)]
enum IssueCmd {
    /// 开单（kind 为 bug 错误任务或 improvement 改进优化任务；新真源 ledger.ohmygh.com）
    New {
        /// 标题（1 至 200）
        title: String,
        /// 任务性质：bug=BUG 错误任务（修复）或 improvement=改进优化任务
        #[arg(long, default_value = "bug")]
        kind: String,
        /// 验收条件（完成判据描述）
        #[arg(long, default_value = "")]
        acceptance: String,
        /// 补充正文
        #[arg(long)]
        body: Option<String>,
        /// HTTP 超时毫秒（缺省 20000；0 = 不限时）
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// 集中列表（新到旧；count 是本次返回条数非在册总数；--before 翻更早一页）
    List {
        /// 至多行数（1 至 100 服务端封顶；默认 100 即上限）
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// keyset 游标：取该 issue 号之前（更旧）一页
        #[arg(long)]
        before: Option<i64>,
        /// HTTP 超时毫秒（缺省 20000；0 = 不限时）
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// 单条详情（projection 加 timeline）
    Show {
        /// issue 号
        n: i64,
        /// HTTP 超时毫秒（缺省 20000；0 = 不限时）
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// 关单（先 result 引 digest 再 status done；digest 为产物或正文 sha256）
    Close {
        /// issue 号
        n: i64,
        /// 结果引用（sha256:<64hex> 小写；`ark artifact publish` 回执的 digest）
        #[arg(long)]
        digest: String,
        /// 备注（result 与 status 两事件的 body）
        #[arg(long)]
        note: Option<String>,
        /// HTTP 超时毫秒（缺省 20000；0 = 不限时）
        #[arg(long)]
        timeout: Option<u64>,
    },
}

/// `ark artifact` 子命令面（REQ-0015，产物共享库：publish 至 attest 至 promote）。
#[derive(Subcommand, Clone)]
enum ArtifactCmd {
    /// 发布产物（共享库本体；digest 是正文或记录哈希，库不收二进制实体）
    Publish {
        /// 产物名（1 至 200）
        #[arg(long)]
        name: String,
        /// 产物类（binary|image|wasm|sbom|schema|openapi|eval-set|benchmark|runbook|decision|attested-report|experience|lesson|research|prototype）
        #[arg(long)]
        kind: String,
        /// 身份摘要（sha256:<64hex> 小写；正文或记录文件哈希）
        #[arg(long)]
        digest: String,
        /// 版本信息（实现成果的 tag 或版本号）
        #[arg(long)]
        version: Option<String>,
        /// 开发记录区间（如 v1.4.2..v1.4.3）
        #[arg(long)]
        git_range: Option<String>,
        /// 依赖出处（artifact_id 逗号串，回溯链即证据链）
        #[arg(long)]
        deps: Option<String>,
        /// 结果倾向（experience 类用 success|failure）
        #[arg(long)]
        outcome: Option<String>,
        /// 一行摘要
        #[arg(long)]
        summary: Option<String>,
        /// 补充正文
        #[arg(long)]
        body: Option<String>,
        /// HTTP 超时毫秒（缺省 20000；0 = 不限时）
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// 证明（attest_dev|attest_prod|verification_failed|promote|demote|supersede）
    Attest {
        /// artifact id
        id: String,
        /// 证明类型
        #[arg(long)]
        attest_type: String,
        /// HTTP 超时毫秒（缺省 20000；0 = 不限时）
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// 晋级当前版（promote 糖衣）
    Promote {
        /// artifact id
        id: String,
        /// HTTP 超时毫秒（缺省 20000；0 = 不限时）
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// 产物列表（current=1 只看当前版）
    List {
        /// 只看当前版（未被 supersede）
        #[arg(long)]
        current: bool,
        /// 按环境过滤：dev|prod
        #[arg(long)]
        env: Option<String>,
        /// 按类过滤
        #[arg(long)]
        kind: Option<String>,
        /// 按名过滤
        #[arg(long)]
        name: Option<String>,
        /// HTTP 超时毫秒（缺省 20000；0 = 不限时）
        #[arg(long)]
        timeout: Option<u64>,
    },
}

fn main() {
    match run() {
        Ok(()) => render::finish(),
        Err(e) => {
            // 错误前已产出的数据块照常上 stdout
            render::finish();
            if render::is_structured() {
                let mut obj = serde_json::Map::new();
                obj.insert("code".to_string(), serde_json::json!(e.code));
                obj.insert("message".to_string(), serde_json::json!(e.message));
                if let Some(hint) = &e.hint {
                    obj.insert("hint".to_string(), serde_json::json!(hint));
                }
                if let Ok(line) = serde_json::to_string(&serde_json::Value::Object(obj)) {
                    eprintln!("{line}");
                }
            } else {
                eprintln!("ark: {e}");
            }
            std::process::exit(e.exit_code);
        }
    }
}

fn run() -> Result<(), ArkError> {
    let cli = Cli::parse();
    // --llms：打印紧凑命令清单后退出（agent 零安装可用；D50 起唯一 agent 发现通道，
    // R013 契约）。早于一切子命令与格式初始化。
    if cli.llms {
        print!("{}", LLMS_MANIFEST);
        return Ok(());
    }
    let format = if cli.json {
        render::Format::Json
    } else {
        cli.format
            .map(render::Format::from)
            .unwrap_or(render::Format::Kv)
    };
    render::set_format(format);
    // 子命令可选；裸调用面（cli-docs 采纳，REQ-0011）：无参进入是导航事件非错误——
    // 紧凑形一行定位加一行指引，exit 恒 0，不弹交互不纯报错。
    let Some(cmd) = cli.command else {
        eprintln!("ark：本机跨平台环境部署管理 CLI（装、查、管、诊断 47 工具）");
        eprintln!("命令清单：ark --llms；详情：ark --help");
        return Ok(());
    };
    // issue 与 artifact 域纯网络面（REQ-0009/REQ-0015）：早期派发，不经 catalog
    // 加载与签名巡检（无清单环境也能一键反馈与发布）。
    if let Commands::Issue { cmd } = cmd.clone() {
        return cmd_issue(cmd).map_err(ArkError::from);
    }
    if let Commands::Artifact { cmd } = cmd.clone() {
        return cmd_artifact(cmd).map_err(ArkError::from);
    }
    let env_root = catalog::resolve_env_root(cli.env_root.as_deref()).map_err(ArkError::from)?;
    let cat_path = catalog::resolve_catalog_path().map_err(ArkError::from)?;
    // D33：仅当解析面就是用户数据副本时按 TTL 刷新云端清单（仓库与 ARK_CATALOG 指定面零干扰；
    // catalog 子命令自身除外，其状态与刷新显式可控）。失败与跳过都不拦命令。
    if !matches!(cmd, Commands::Catalog { .. }) {
        catalog::auto_refresh_if_user_data(&env_root, &cat_path);
    }
    // D34：本地清单签名巡检（内嵌公钥，见 S006 候选 A）。有签名但验不过必须拦下；
    // 无签名件只在运行态副本上告警（本地回写会撤签名，仓库开发面不参与签名）。
    // catalog 子命令自身豁免（status 要如实报状态、sync 就是修复通道），否则损坏时无法自愈。
    if !matches!(cmd, Commands::Catalog { .. }) {
        match catalog::check_signature(&cat_path) {
            catalog::SignatureState::Valid => {}
            catalog::SignatureState::Invalid(e) => {
                return Err(ArkError::from(format!(
                    "清单签名校验不过: {}（{e}）；修复: `ark catalog sync` 取回云端签名件，或设 ARK_CATALOG 指定本地清单；内嵌公钥 {}",
                    cat_path.display(),
                    catalog::CLOUD_CATALOG_PUBKEY_ID
                )));
            }
            catalog::SignatureState::Missing => {
                if catalog::is_user_data_catalog(&cat_path) {
                    eprintln!(
                        "[WARN] 运行态清单无签名件（本地回写已撤签名或尚未同步签名件）: {}；`ark catalog sync` 可取回云端签名件",
                        cat_path.display()
                    );
                }
            }
        }
    }
    let cat = Catalog::load(&cat_path).map_err(ArkError::from)?;
    match cmd {
        Commands::Query { tool, opts } => cmd_query(&cat, &tool, &opts).map_err(ArkError::from),
        Commands::Pin { tool, opts } => cmd_pin(&cat, &tool, &opts).map_err(ArkError::from),
        Commands::Install { tool, opts, force } => {
            cmd_install(&cat, &env_root, &tool, &opts, force).map_err(ArkError::from)
        }
        Commands::Update { tool, force } => {
            cmd_update(&cat, &env_root, &tool, force).map_err(ArkError::from)
        }
        Commands::Status => cmd_status(&cat, &env_root).map_err(ArkError::from),
        Commands::Init => cmd_init(&env_root).map_err(ArkError::from),
        Commands::Verify { check } => {
            cmd_verify(&cat, &env_root, check.as_deref()).map_err(ArkError::from)
        }
        Commands::Heal { dim, dry_run } => {
            cmd_heal(&cat, &env_root, &dim, dry_run).map_err(ArkError::from)
        }
        Commands::Doctor => cmd_doctor(&cat, &env_root).map_err(ArkError::from),
        Commands::Catalog { cmd } => cmd_catalog(&env_root, &cat_path, cmd).map_err(ArkError::from),
        Commands::ArkSelf {
            cmd: SelfCmd::Update { stable, git },
        } => {
            let channel = if git {
                ark::selfupdate::Channel::Git
            } else if stable {
                ark::selfupdate::Channel::Stable
            } else {
                ark::selfupdate::Channel::Dev
            };
            cmd_self_update(&env_root, channel).map_err(ArkError::from)
        }
        // issue 已在 catalog 前早期派发（纯网络面），此臂不可达
        Commands::Issue { .. } => unreachable!("issue 已早期派发"),
        Commands::Artifact { .. } => unreachable!("artifact 已早期派发"),
    }
}

/// issue 域（REQ-0009）：统一 issue 入口三叶（new/list/show，issues.ohmygh.com）。
fn cmd_issue(cmd: IssueCmd) -> Result<(), String> {
    use ark::ledger;
    match cmd {
        IssueCmd::New {
            title,
            kind,
            acceptance,
            body,
            timeout,
        } => {
            let agent = ledger_http(timeout);
            let (n, resp) = ledger::issue_new(&agent, &title, &kind, &acceptance, body.as_deref())?;
            let seq = resp
                .pointer("/event/seq")
                .and_then(serde_json::Value::as_i64)
                .map(|x| x.to_string())
                .unwrap_or_else(|| "-".to_string());
            render::emit(&[
                kv("filed", &n.to_string()),
                kv("issue", &n.to_string()),
                kv("seq", &seq),
                kv("kind", &kind),
                kv("endpoint", ledger::LEDGER_BASE),
            ]);
            eprintln!("[OK] issue #{n} 已开单（seq {seq}，kind {kind}）");
            eprintln!(
                "[HINT] 复核：ark issue show {n}；网页面：{}/repos/{}/i/{n}",
                ledger::LEDGER_BASE,
                ledger::REPO_ID
            );
            Ok(())
        }
        IssueCmd::List {
            limit,
            before,
            timeout,
        } => {
            let agent = ledger_http(timeout);
            let (issues, has_more) = ledger::issue_list(&agent, limit, before)?;
            let mut out = vec![
                kv("count", &issues.len().to_string()),
                kv("endpoint", ledger::LEDGER_BASE),
            ];
            // has_more 恒出（more=1 索取；缺省 false）；饱和提示以 has_more 为准，
            // 无 has_more 材料时退回条数判定（对线 F2：has_more=false 不再出截断提示）
            let hm = has_more.unwrap_or(false);
            out.push(kv("has_more", if hm { "true" } else { "false" }));
            let saturated =
                has_more.map_or_else(|| ledger::list_saturated(issues.len(), limit), |h| h);
            for r in &issues {
                let n = r
                    .get("issue_n")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                let title = r
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let status = r
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let kind = r
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                out.push(kv(&format!("#{n}"), &format!("{kind} {status} {title}")));
            }
            render::emit(&out);
            if saturated {
                eprintln!("[HINT] issue list 尚有更早条目；下一步：--before <id> 翻更早一页");
            }
            eprintln!(
                "[HINT] 网页面：{}/repos/{}",
                ledger::LEDGER_BASE,
                ledger::REPO_ID
            );
            Ok(())
        }
        IssueCmd::Show { n, timeout } => {
            let agent = ledger_http(timeout);
            let v = ledger::issue_show(&agent, n)?;
            let mut rows = vec![
                kv("issue", &n.to_string()),
                kv("endpoint", ledger::LEDGER_BASE),
            ];
            for key in ["kind", "status", "assignee"] {
                let val = v
                    .pointer(&format!("/projection/{key}"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("-");
                rows.push(kv(key, val));
            }
            // title 不在服务端 projection 面，从 timeline 首个 issue_open 事件 payload 取
            let title = v
                .get("timeline")
                .and_then(serde_json::Value::as_array)
                .and_then(|tl| {
                    tl.iter().find(|e| {
                        e.get("type").and_then(serde_json::Value::as_str) == Some("issue_open")
                    })
                })
                .and_then(|e| e.get("payload"))
                .and_then(|p| {
                    serde_json::from_str::<serde_json::Value>(p.as_str().unwrap_or("")).ok()
                })
                .and_then(|p| {
                    p.get("title")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })
                .unwrap_or_else(|| "-".to_string());
            rows.insert(2, kv("title", &title));
            let has_result = v
                .pointer("/projection/hasResult")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            rows.push(kv("has_result", if has_result { "true" } else { "false" }));
            if let Some(tl) = v.get("timeline").and_then(serde_json::Value::as_array) {
                rows.push(kv("events", &tl.len().to_string()));
                for ev in tl {
                    let seq = ev
                        .get("seq")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0);
                    let t = ev
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    rows.push(kv(&format!("e{seq}"), t));
                }
            }
            render::emit(&rows);
            eprintln!(
                "[HINT] 网页面：{}/repos/{}/i/{n}",
                ledger::LEDGER_BASE,
                ledger::REPO_ID
            );
            Ok(())
        }
        IssueCmd::Close {
            n,
            digest,
            note,
            timeout,
        } => {
            let agent = ledger_http(timeout);
            let (result, status) = ledger::issue_close(&agent, n, &digest, note.as_deref())?;
            let rs = result
                .pointer("/event/seq")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            let ss = status
                .pointer("/event/seq")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            render::emit(&[
                kv("issue", &n.to_string()),
                kv("action", "closed"),
                kv("result_seq", &rs.to_string()),
                kv("status_seq", &ss.to_string()),
                kv("digest", &digest),
                kv("endpoint", ledger::LEDGER_BASE),
            ]);
            eprintln!("[OK] issue #{n} 已关单（result seq {rs} + status done seq {ss}）");
            Ok(())
        }
    }
}

/// ledger HTTP 客户端（timeout 毫秒；0 = 不限时）。
fn ledger_http(timeout: Option<u64>) -> ureq::Agent {
    ark::ledger::http_client(timeout.unwrap_or(ark::ledger::TIMEOUT_MS))
}

/// artifact 命令族（REQ-0015 产物共享库面）。
fn cmd_artifact(cmd: ArtifactCmd) -> Result<(), String> {
    use ark::ledger;
    match cmd {
        ArtifactCmd::Publish {
            name,
            kind,
            digest,
            version,
            git_range,
            deps,
            outcome,
            summary,
            body,
            timeout,
        } => {
            let agent = ledger_http(timeout);
            let mut extras = serde_json::Map::new();
            if let Some(v) = version {
                extras.insert("version".into(), serde_json::json!(v));
            }
            if let Some(g) = git_range {
                extras.insert("git_range".into(), serde_json::json!(g));
            }
            if let Some(d) = deps {
                let list: Vec<&str> = d
                    .split(',')
                    .map(str::trim)
                    .filter(|x| !x.is_empty())
                    .collect();
                extras.insert("deps".into(), serde_json::json!(list));
            }
            if let Some(o) = outcome {
                extras.insert("outcome".into(), serde_json::json!(o));
            }
            if let Some(sm) = summary {
                extras.insert("summary".into(), serde_json::json!(sm));
            }
            if let Some(b) = body {
                extras.insert("body".into(), serde_json::json!(b));
            }
            let (id, resp) = ledger::artifact_publish(
                &agent,
                &name,
                &kind,
                &digest,
                &serde_json::Value::Object(extras),
            )?;
            let seq = resp
                .pointer("/event/seq")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            render::emit(&[
                kv("filed", &id),
                kv("artifact_id", &id),
                kv("seq", &seq.to_string()),
                kv("kind", &kind),
                kv("digest", &digest),
                kv("endpoint", ledger::LEDGER_BASE),
            ]);
            eprintln!("[OK] artifact 已发布（seq {seq}，kind {kind}）：{id}");
            eprintln!("[HINT] 证明：ark artifact attest {id} --attest-type attest_dev；网页面：{}/repos/{}/a/{id}", ledger::LEDGER_BASE, ledger::REPO_ID);
            Ok(())
        }
        ArtifactCmd::Attest {
            id,
            attest_type,
            timeout,
        } => {
            let agent = ledger_http(timeout);
            let resp = ledger::artifact_attest(&agent, &id, &attest_type, &serde_json::json!({}))?;
            let seq = resp
                .pointer("/event/seq")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            render::emit(&[
                kv("artifact_id", &id),
                kv("action", "attested"),
                kv("attest_type", &attest_type),
                kv("seq", &seq.to_string()),
                kv("endpoint", ledger::LEDGER_BASE),
            ]);
            eprintln!("[OK] artifact {id} 已证明（{attest_type}，seq {seq}）");
            Ok(())
        }
        ArtifactCmd::Promote { id, timeout } => {
            let agent = ledger_http(timeout);
            let resp = ledger::artifact_attest(&agent, &id, "promote", &serde_json::json!({}))?;
            let seq = resp
                .pointer("/event/seq")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            render::emit(&[
                kv("artifact_id", &id),
                kv("action", "promoted"),
                kv("seq", &seq.to_string()),
                kv("endpoint", ledger::LEDGER_BASE),
            ]);
            eprintln!("[OK] artifact {id} 已晋级当前版（promote，seq {seq}）");
            Ok(())
        }
        ArtifactCmd::List {
            current,
            env,
            kind,
            name,
            timeout,
        } => {
            let agent = ledger_http(timeout);
            let arts = ledger::artifact_list(
                &agent,
                current,
                env.as_deref(),
                kind.as_deref(),
                name.as_deref(),
            )?;
            let mut out = vec![
                kv("count", &arts.len().to_string()),
                kv("endpoint", ledger::LEDGER_BASE),
            ];
            for a in &arts {
                let id = a
                    .get("artifact_id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let short: String = id.chars().take(8).collect();
                let k = a
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let n = a
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let dv = a
                    .get("dev_verified")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let pv = a
                    .get("prod_verified")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let cur = a
                    .get("current")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                out.push(kv(
                    &short,
                    &format!(
                        "{k} {n} dev={} prod={} current={}",
                        if dv { "y" } else { "n" },
                        if pv { "y" } else { "n" },
                        if cur { "y" } else { "n" }
                    ),
                ));
            }
            render::emit(&out);
            eprintln!(
                "[HINT] 网页面：{}/repos/{}",
                ledger::LEDGER_BASE,
                ledger::REPO_ID
            );
            Ok(())
        }
    }
}
fn cmd_self_update(env_root: &Path, channel: ark::selfupdate::Channel) -> Result<(), String> {
    let out = ark::selfupdate::self_update(env_root, channel)?;
    render::emit(&[
        kv("action", out.action),
        kv("channel", out.channel),
        kv("asset", &out.asset),
        kv("sha256", &out.sha256),
        kv("exe", &out.exe.display().to_string()),
        kv(
            "catalog",
            if out.catalog_synced {
                "synced"
            } else {
                "skipped"
            },
        ),
    ]);
    Ok(())
}

/// doctor：核心诊断命令（D07 起三层，D30 收窄两层 2026-09-10）：系统层（os/arch/指令集）
/// 到依赖层（九类分组统计）再到环境错误 check 节。kv 输出
/// name=OK/WARN/FAIL（明细走 stderr）；结构化输出同序块。FAIL 即 exit 1（专属 check 节，
/// 依赖缺口走 WARN 不拦退出，检测驱动安装）。agent 装态对账归 omc、token 检测归 oma
/// diagnose（D30 削减；agent 单机三态走 `ark status`）。
fn cmd_doctor(cat: &Catalog, env_root: &Path) -> Result<(), String> {
    use std::io::IsTerminal;
    // TTY 人读面：一条一条描述报告；非 TTY（管道/agent）走 kv 数据面（两副面孔，oma status 同款）
    let tty = std::io::stdout().is_terminal() && !render::is_structured();
    let mut first = true;
    // ══ 一层：系统 ══
    let sys = ark::doctor::system_facts();
    if tty {
        let mut caps = Vec::new();
        if sys.avx {
            caps.push("avx");
        }
        if sys.avx2 {
            caps.push("avx2");
        }
        if sys.avx512f {
            caps.push("avx512f");
        }
        let caps_s = if caps.is_empty() {
            "none".to_string()
        } else {
            caps.join("/")
        };
        println!("[系统] {} {}，指令集 {caps_s}", sys.os, sys.arch);
    } else {
        render::emit(&[
            ("sys.os".into(), sys.os.into()),
            ("sys.arch".into(), sys.arch.into()),
            ("sys.avx".into(), sys.avx.to_string()),
            ("sys.avx2".into(), sys.avx2.to_string()),
            ("sys.avx512f".into(), sys.avx512f.to_string()),
        ]);
        render::blank();
    }
    // ══ 二层：依赖分组（D30 削减后仅此一层事实陈述；原 agent 层归 omc/oma）══
    let srows = ark::status::collect_status(cat, env_root)?;
    for g in ark::doctor::dep_group_stats(&srows) {
        if tty {
            if g.missing == 0 {
                println!("[依赖] {}：{} 项全在", g.label, g.tools);
            } else {
                println!(
                    "[依赖] {}：{} 项在装，缺 {} 项（ark install 补）",
                    g.label,
                    g.tools - g.missing,
                    g.missing
                );
            }
        } else {
            render::emit(&[
                ("dep".into(), g.category.clone()),
                ("label".into(), g.label.into()),
                ("tools".into(), g.tools.to_string()),
                ("missing".into(), g.missing.to_string()),
                ("drift".into(), g.drift.to_string()),
            ]);
            render::blank();
        }
    }
    // ══ check 节：环境错误 + 配置健康 + 部署深诊 + 网络通连 ══
    let rows = ark::doctor::run_doctor_with_status(cat, env_root, &srows, |r| {
        if tty {
            // 一条一条描述报告：OK 一行人话；待修/故障首行主描述，其余 detail 明细缩进续行
            let desc = r
                .detail
                .first()
                .cloned()
                .unwrap_or_else(|| ark::doctor::check_desc(r.name).to_string());
            match r.status {
                "OK" => println!("[通过] {desc}"),
                "WARN" | "FAIL" => {
                    let tag = if r.status == "WARN" {
                        "待修"
                    } else {
                        "故障"
                    };
                    println!("[{tag}] {desc}");
                    for d in r.detail.iter().skip(1) {
                        println!("       {d}");
                    }
                }
                other => println!("[故障] {desc}（未知状态 {other}）"),
            }
        } else if render::is_structured() {
            let mut block = vec![kv("check", r.name), kv("status", r.status)];
            if !r.detail.is_empty() {
                block.push(kv("detail", &r.detail.join("; ")));
            }
            emit_block(&mut first, block);
        } else {
            render::emit(&[(r.name.to_string(), r.status.to_string())]);
        }
    })?;
    let (fails, warns, fail_names, _) = ark::doctor::summarize(&rows);
    // 网络通连 WARN 是渠道可达性，不单独把本机环境打成 degraded（官方不通会走镜像）
    let local_warns = rows
        .iter()
        .filter(|r| r.status == "WARN" && !r.name.starts_with("net-"))
        .count();
    let verdict = if fails > 0 {
        "broken"
    } else if local_warns > 0 || missing_total(&srows) > 0 {
        "degraded"
    } else {
        "ready"
    };
    if !tty {
        render::emit(&[("verdict".into(), verdict.into())]);
    }
    let missing: Vec<&str> = srows
        .iter()
        .filter(|r| r.exe.is_some() && r.installed.is_none())
        .map(|r| r.name.as_str())
        .collect();
    if tty {
        let verdict_desc = match verdict {
            "ready" => "环境就绪".to_string(),
            "degraded" => format!("环境可用，{warns} 项待修"),
            _ => format!("环境故障：{fails} 项 FAIL（{}）", fail_names.join("、")),
        };
        println!("\n结论: {verdict_desc}");
        if !missing.is_empty() {
            println!("建议: ark install {} 补缺", missing.join(" "));
        }
    } else if !missing.is_empty() {
        let head: Vec<&str> = missing.iter().take(5).copied().collect();
        let tail = if missing.len() > 5 {
            format!(" 等 {} 项", missing.len())
        } else {
            String::new()
        };
        eprintln!(
            "[HINT] 检测到缺失，补装: ark install {}{}",
            head.join(","),
            tail
        );
    }
    match verdict {
        "broken" => {}
        "degraded" => eprintln!("[HINT] 环境可跑但有缺口（degraded），见待修项"),
        _ => eprintln!("[OK] 环境就绪（ready）"),
    }
    if fails > 0 {
        return Err(format!(
            "诊断发现 {fails} 项 FAIL: {}",
            fail_names.join(", ")
        ));
    }
    Ok(())
}

/// verify：部署域验收维度检查，kv 输出 dim=PASS/FAIL/NA 收割行，
/// 结构化输出 name/verdict 块。维度就绪即出（流式）。FAIL 即 exit 1。
fn cmd_verify(cat: &Catalog, env_root: &Path, check: Option<&str>) -> Result<(), String> {
    let filter: Vec<String> = check
        .map(|c| {
            c.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let mut first = true;
    let rows = ark::verify::run_verify_with(cat, env_root, &filter, |name, verdict| {
        if render::is_structured() {
            emit_block(
                &mut first,
                vec![kv("name", name), kv("verdict", verdict.as_str())],
            );
        } else {
            render::emit(&[(name.to_string(), verdict.as_str().to_string())]);
        }
        Ok(())
    })?;
    let (total, fails) = ark::verify::summarize(&rows);
    eprintln!("[汇总] {total} 项，FAIL {} 项", fails.len());
    if !fails.is_empty() {
        return Err(format!("验收失败 {} 项: {}", fails.len(), fails.join(", ")));
    }
    Ok(())
}

/// heal：部署维度幂等自愈。kv 输出 dim/action/params/result 收割行（明细走 stderr），
/// 结构化输出 dim/action/params/result/detail 块。有 fail/partial 结果即 exit 1。
fn cmd_heal(cat: &Catalog, env_root: &Path, dim: &str, dry_run: bool) -> Result<(), String> {
    let mut first = true;
    let rows = ark::heal::run_heal_with(cat, env_root, dim, dry_run, |r| {
        if render::is_structured() {
            let mut block = vec![
                kv("dim", &r.dim),
                kv("action", r.action),
                kv("params", &r.params),
                kv("result", &r.result),
            ];
            if !r.detail.is_empty() {
                block.push(kv("detail", &r.detail.join("; ")));
            }
            emit_block(&mut first, block);
        } else {
            let mut block = vec![
                kv("dim", &r.dim),
                kv("action", r.action),
                kv("result", &r.result),
            ];
            if !r.params.is_empty() {
                block.insert(2, kv("params", &r.params));
            }
            emit_block(&mut first, block);
        }
        for d in &r.detail {
            eprintln!("[{}] {}: {}", r.result, r.dim, d);
        }
        Ok(())
    })?;
    let healed = rows.iter().filter(|r| r.result == "healed").count();
    eprintln!(
        "[汇总] {} 项：healed {healed}、ok {}、fail/partial {}",
        rows.len(),
        rows.iter().filter(|r| r.result == "ok").count(),
        rows.iter()
            .filter(|r| r.result == "fail" || r.result == "partial")
            .count(),
    );
    let bad: Vec<String> = rows
        .iter()
        .filter(|r| r.result == "fail" || r.result == "partial")
        .map(|r| r.dim.clone())
        .collect();
    if !bad.is_empty() {
        return Err(format!(
            "自愈未完全成功 {} 项: {}",
            bad.len(),
            bad.join(", ")
        ));
    }
    Ok(())
}

/// 组装解析选项。
fn resolve_opts(opts: &VersionOpts) -> ResolveOptions {
    ResolveOptions {
        latest: opts.latest,
        tag: opts.tag.clone(),
        version: opts.version.clone(),
    }
}

/// query：只解析不下载，每工具输出 tool/tag/version/asset/size/url 六行 key=value。
fn cmd_query(cat: &Catalog, tool: &str, opts: &VersionOpts) -> Result<(), String> {
    let names = cat.select(tool)?;
    let ropts = resolve_opts(opts);
    let mut first = true;
    for name in &names {
        let def = cat.tool(name)?;
        if ark::vsbuild::is_vsbuild(def) {
            eprintln!("[INFO] {name} 为永续引导器条目，无远端版本解析");
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("version", "evergreen"),
                    kv("asset", ark::vsbuild::BOOTSTRAPPER),
                    kv("url", def.cdn_url().unwrap_or("")),
                ],
            );
            continue;
        }
        if ark::rustup::is_rustup(def) {
            eprintln!("[INFO] {name} 为 rustup 引导器条目（stable 滚动），无远端版本解析");
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("version", "evergreen"),
                    kv("asset", ark::rustup::INIT_EXE),
                    kv("url", def.cdn_url().unwrap_or("")),
                ],
            );
            continue;
        }
        if ark::selfupdate::is_ark_self(def) {
            eprintln!("[INFO] {name} 为自管条目，版本走 self update 三通道（dev/stable/git）");
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("version", "self-managed"),
                    kv("asset", "ark self update"),
                ],
            );
            continue;
        }
        if !ark::toolver::platform_managed(def) {
            eprintln!("[INFO] {name} 当前平台不适用（无本平台 exe 字段），跳过");
            emit_block(&mut first, vec![kv("tool", name), kv("action", "skipped")]);
            continue;
        }
        let r = resolve_tool(name, def, &ropts)?;
        let mut rows = resolution_rows(&r, true);
        // 对外契约字段（issue #4）：pin sha 仅在解析结果与锁定同 tag 同资产时给出，否则空串
        rows.push(kv("sha256", &query_sha(def, &r)));
        emit_block(&mut first, rows);
    }
    Ok(())
}

/// query 的 sha256 契约值：解析 tag/资产与 pin 一致时给 pin 的 sha256（未回填则空），不一致给空串。
fn query_sha(def: &ark::catalog::Tool, r: &Resolution) -> String {
    let same_asset =
        def.pin_asset().unwrap_or("").is_empty() || def.pin_asset() == Some(r.asset_name.as_str());
    if def.pin_tag() == Some(r.tag.as_str()) && same_asset {
        def.pin_sha256().unwrap_or("").to_string()
    } else {
        String::new()
    }
}

/// pin：无选项打印当前 pin（sha256 截前 16 位加 ...），未 pin 的自动解析最新并回写；
/// 有选项则解析并回写 tag/version/asset（版本变化时清 sha256）。
fn cmd_pin(cat: &Catalog, tool: &str, opts: &VersionOpts) -> Result<(), String> {
    let names = cat.select(tool)?;
    let ropts = resolve_opts(opts);
    let mut first = true;
    for name in &names {
        let def = cat.tool(name)?;
        if ark::vsbuild::is_vsbuild(def) {
            eprintln!("[INFO] {name} 为 evergreen 引导器条目，无 pin 语义（install 幂等）");
            emit_block(&mut first, vec![kv("tool", name), kv("pin", "evergreen")]);
            continue;
        }
        if ark::rustup::is_rustup(def) {
            eprintln!(
                "[INFO] {name} 为 rustup 引导器条目（stable 滚动），无 pin 语义（install 即更新）"
            );
            emit_block(&mut first, vec![kv("tool", name), kv("pin", "evergreen")]);
            continue;
        }
        if ark::selfupdate::is_ark_self(def) {
            eprintln!("[INFO] {name} 为自管条目，无 pin 语义（self update 按资产 sha 滚动）");
            emit_block(
                &mut first,
                vec![kv("tool", name), kv("pin", "self-managed")],
            );
            continue;
        }
        if !ark::toolver::platform_managed(def) {
            eprintln!("[INFO] {name} 当前平台不适用（无本平台 exe 字段），跳过");
            emit_block(&mut first, vec![kv("tool", name), kv("action", "skipped")]);
            continue;
        }
        // 版本锁定（hold）：pin 不动（显示当前锁定并提示解锁方式）
        if def.is_held() {
            eprintln!("[INFO] {name} 已锁定（hold），pin 不变；解锁需删 catalog 的 hold 字段");
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("pin", def.pin_version().unwrap_or("held")),
                ],
            );
            continue;
        }
        if opts.is_empty() && def.pin_tag().is_some() {
            // 已 pin：只打印当前锁定
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("tag", def.pin_tag().unwrap_or("")),
                    kv("version", def.pin_version().unwrap_or("")),
                    kv("asset", def.pin_asset().unwrap_or("")),
                    kv("sha256", &short_sha(def.pin_sha256())),
                ],
            );
            continue;
        }
        // 未 pin 且无选项：自动解析最新并回写；有选项：按选项解析并回写
        let eff = if opts.is_empty() {
            eprintln!("[INFO] {name} 未 pin，自动 pin 最新版");
            ResolveOptions {
                latest: true,
                ..ResolveOptions::default()
            }
        } else {
            ropts.clone()
        };
        let r = resolve_tool(name, def, &eff)?;
        let version_changed = catalog::write_pin(&cat.path, name, &r)?;
        eprintln!(
            "[OK] {name} 已 pin: {}{}",
            r.version,
            if version_changed {
                "（sha256 已清除，将在 install 时回填）"
            } else {
                ""
            }
        );
        emit_block(&mut first, resolution_rows(&r, false));
    }
    Ok(())
}

/// catalog 与 manifest 同目录（R016 两件分离）：manifest 缺位时用户级配置与别名
/// （env_set/shims）不会应用，装前一行 WARN 让真空面可见（撤内建双轨后的安全网）。
fn warn_if_manifest_missing(cat: &Catalog) {
    let path = ark::manifest::path_for(&cat.path);
    if !path.exists() {
        eprintln!(
            "[WARN] manifest.toml 不在位（{}）: 用户级配置与别名（env_set/shims）本次不会应用；`ark catalog sync` 后重跑 install（R016 双轨收口）",
            path.display()
        );
    }
}

/// install：解析（默认锁定版本）→ 下载解压 → PATH、注册表与配置。
fn cmd_install(
    cat: &Catalog,
    env_root: &Path,
    tool: &str,
    opts: &VersionOpts,
    force: bool,
) -> Result<(), String> {
    let names = cat.select(tool)?;
    warn_if_manifest_missing(cat);
    let ropts = resolve_opts(opts);
    let iopts = InstallOptions {
        configure: true,
        update_lock: false,
        force,
    };
    let mut first = true;
    let mut errors: Vec<String> = Vec::new();
    for name in &names {
        let def = cat.tool(name)?;
        // 平台不适用（无本平台 exe，如 shellcheck 在 Windows、Windows-only 工具在 Linux）：跳过不安装
        if !ark::toolver::platform_managed(def) {
            eprintln!("[INFO] {name} 当前平台不适用（无本平台 exe 字段），跳过");
            emit_block(&mut first, vec![kv("tool", name), kv("action", "skipped")]);
            continue;
        }
        // vsbuild：evergreen 引导器（无版本解析、需提权、机器级 PATH），走专用安装模块
        if ark::vsbuild::is_vsbuild(def) {
            match ark::vsbuild::install(def, env_root, true) {
                Ok(out) => emit_block(&mut first, install_rows(name, &out)),
                Err(e) => skip_or_fail(tool, name, e, &mut errors)?,
            }
            continue;
        }
        // rust：rustup 引导器（rsproxy 直链、stable 滚动、EnvRoot 重定位），走专用安装模块
        if ark::rustup::is_rustup(def) {
            match ark::rustup::install(def, env_root, true) {
                Ok(out) => emit_block(&mut first, install_rows(name, &out)),
                Err(e) => skip_or_fail(tool, name, e, &mut errors)?,
            }
            continue;
        }
        // ark：自管条目（self update 三通道），install 提示走 self update
        if ark::selfupdate::is_ark_self(def) {
            eprintln!("[INFO] {name} 自管理：升级走 `ark self update`（dev/stable/git 三通道）");
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("action", "skipped"),
                    kv("version", "self-managed"),
                ],
            );
            continue;
        }
        // docker：static zip + Windows 服务注册 + daemon.json + compose 插件（set-docker.ps1 迁移），走专用模块
        if ark::docker::is_docker(def) {
            let step = resolve_tool(name, def, &ropts)
                .and_then(|r| ark::docker::install(def, env_root, &r, true));
            match step {
                Ok(out) => emit_block(&mut first, install_rows(name, &out)),
                Err(e) => skip_or_fail(tool, name, e, &mut errors)?,
            }
            continue;
        }
        // 版本锁定（hold）：带版本选项的安装拒绝漂移；无选项按 pin 走（幂等）
        if def.is_held() && !opts.is_empty() {
            eprintln!(
                "[INFO] {name} 已锁定（hold）：{}，拒绝按选项安装；解锁需删 catalog 的 hold 字段",
                def.pin_version().unwrap_or("")
            );
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("action", "skipped"),
                    kv("version", def.pin_version().unwrap_or("")),
                ],
            );
            continue;
        }
        let step = resolve_tool(name, def, &ropts)
            .and_then(|r| install_tool(cat, env_root, name, &r, &iopts).map(|out| (r, out)));
        match step {
            Ok((_, out)) => emit_block(&mut first, install_rows(name, &out)),
            Err(e) => skip_or_fail(tool, name, e, &mut errors)?,
        }
    }
    summarize_all_errors(&errors)
}

/// all 循环容错：单工具失败时跳过续跑（WARN 加 skipped 行），单工具显式调用即时失败。
/// 返回 Err(()) 仅用于中断循环（调用方以 ? 传播），实际错误信息已在 errors 中。
fn skip_or_fail(
    tool_arg: &str,
    name: &str,
    e: String,
    errors: &mut Vec<String>,
) -> Result<(), String> {
    if tool_arg == "all" {
        eprintln!("[WARN] {name}: {e}（all 循环跳过继续）");
        errors.push(format!("{name}: {e}"));
        return Ok(());
    }
    Err(e)
}

/// all 循环收尾：有失败项则汇总报错（exit 非零），单工具路径恒 Ok。
fn summarize_all_errors(errors: &[String]) -> Result<(), String> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "all 循环 {} 项失败: {}",
            errors.len(),
            errors.join("; ")
        ))
    }
}

/// update：**对齐云端 catalog pin 安装**（D51 镜像默认通道：pin 驱动解析零 GitHub API，
/// 云端「最新」定义随此改指镜像与 catalog——上游滚版归数据面 omc 滚锁）；本机对照锁定
/// 走 D49 三态（一致 skip、落后/未装补装、领先如实报）；**不回写锁定**（D37 定案：
/// 版本锁定单源归数据面 omc；`ark pin` 留作临时本地锁）；`--force` 走真装。
fn cmd_update(cat: &Catalog, env_root: &Path, tool: &str, force: bool) -> Result<(), String> {
    let names = cat.select(tool)?;
    warn_if_manifest_missing(cat);
    // D51：默认解析 pin 驱动（GitHub 分支零 API 镜像直装；zig 等 index 分支无 pin 仍解析最新）
    let ropts = ResolveOptions::default();
    let iopts = InstallOptions {
        configure: true,
        update_lock: false,
        force,
    };
    let mut first = true;
    let mut errors: Vec<String> = Vec::new();
    for name in &names {
        let def = cat.tool(name)?;
        if ark::vsbuild::is_vsbuild(def) {
            eprintln!("[INFO] {name} 为 evergreen 引导器条目，不走 update（install 幂等）");
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("action", "skipped"),
                    kv("version", def.pin_version().unwrap_or("evergreen")),
                ],
            );
            continue;
        }
        // agent 类存量原地纳管（D07）：PATH 在位即跳过 update（升级走各 agent 自更新
        // 通道，或 install --force 显式装进 EnvRoot）；与 install 纳管判定同口径
        if def.category.as_deref() == Some("agent") && ark::toolver::find_on_path(name).is_some() {
            eprintln!(
                "[INFO] {name} 已在 PATH 安装，存量原地纳管跳过 update（agent 自更新或 install --force 装 EnvRoot）"
            );
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("action", "skipped"),
                    kv("version", def.pin_version().unwrap_or("-")),
                ],
            );
            continue;
        }
        if ark::rustup::is_rustup(def) {
            eprintln!("[INFO] {name} 为 rustup 引导器条目，不走 update（install 即 rustup update stable）");
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("action", "skipped"),
                    kv("version", def.pin_version().unwrap_or("evergreen")),
                ],
            );
            continue;
        }
        if ark::selfupdate::is_ark_self(def) {
            eprintln!("[INFO] {name} 为自管条目，不走 update（ark self update 三通道）");
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("action", "skipped"),
                    kv("version", "self-managed"),
                ],
            );
            continue;
        }
        if !ark::toolver::platform_managed(def) {
            eprintln!("[INFO] {name} 当前平台不适用（无本平台 exe 字段），跳过");
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("action", "skipped"),
                    kv("version", def.pin_version().unwrap_or("")),
                ],
            );
            continue;
        }
        // 版本锁定（hold）：update 拒绝（含 --force）
        if def.is_held() {
            eprintln!(
                "[INFO] {name} 已锁定（hold）：{}，不更新；解锁需删 catalog 的 hold 字段",
                def.pin_version().unwrap_or("")
            );
            emit_block(
                &mut first,
                vec![
                    kv("tool", name),
                    kv("action", "skipped"),
                    kv("version", def.pin_version().unwrap_or("")),
                ],
            );
            continue;
        }
        // 家族自研 CLI 委托腿（用户裁定 2026-09-19：自维护自升级 CLI 的 ark update 走
        // 其自身自升级通道，吃其独立域与锚校验回滚自证；其余非自研照旧镜像腿）：
        // 未装回落镜像安装腿首装；让位契约靠临时撤 ark-managed 落痕、完成（含失败）复痕。
        if ark::delegate::self_update_args(name).is_some() {
            match ark::delegate::run(name, def, env_root) {
                Ok(Some(out)) => {
                    if out.ok {
                        eprintln!("[OK] {name} 委托自升级完成: {}", out.via);
                    } else {
                        eprintln!(
                            "[WARN] {name} 委托自升级未成功: {}（详见上方家族 CLI 输出）",
                            out.via
                        );
                    }
                    // action 诚实裁定（对线 F4）：失败 failed；升级后探活在位且与升级前
                    // 不同 updated；其余（家族成功版本未动、或探活失败无法证变）skipped
                    let changed = matches!(
                        (&out.version_before, &out.version),
                        (Some(a), Some(b)) if a != b
                    );
                    let action = if !out.ok {
                        "failed"
                    } else if changed {
                        "updated"
                    } else {
                        "skipped"
                    };
                    let mut rows = vec![
                        kv("tool", name),
                        kv("action", action),
                        kv("channel", "self-update"),
                    ];
                    rows.push(kv("version", out.version.as_deref().unwrap_or("-")));
                    emit_block(&mut first, rows);
                    if !out.ok {
                        // 对线 F1：与镜像腿同走 skip_or_fail（all 计项、单工具即败，
                        // 退出码契约 1=失败两面一致）
                        skip_or_fail(
                            tool,
                            name,
                            format!("{name}: 委托自升级失败（{}）", out.via),
                            &mut errors,
                        )?;
                    }
                    continue;
                }
                Ok(None) => {
                    eprintln!("[INFO] {name} 家族 CLI 未装，回落镜像安装腿首装");
                }
                Err(e) => {
                    skip_or_fail(tool, name, e, &mut errors)?;
                    continue;
                }
            }
        }
        // PATH 存量已达锁定版纳管（REQ-0003，对线修正批）：自管位工具（hst 族
        // self-update 通道等）PATH 版本已达 pin 而 EnvRoot 未装时，视为已达态跳过
        // （避免 EnvRoot 双份副本与无谓的 resolve 网络调用）。位置在自管与 rustup 与
        // vsbuild 与平台与 hold 判定之后（对线裁定：更早会截获 tools.ark 等自管条目，
        // 翻转 version=self-managed 机读契约）；判据用 pin_drift 数值口径（与 D49 同尺，
        // 防形态差异静默失效）；PATH 落后不纳管（D49 真装语义不变）；--force 走真装。
        if !force {
            if let Some(pin) = def.pin_version() {
                if let Some(found) = ark::toolver::find_on_path(name) {
                    let env_installed = toolver::exe_path(def, env_root)
                        .map(|exe| exe.exists())
                        .unwrap_or(false);
                    if !env_installed
                        && toolver::pin_drift(
                            toolver::installed_version(&found, def).as_deref(),
                            pin,
                        ) == toolver::PinDrift::Current
                    {
                        eprintln!(
                            "[INFO] {name} 已在 PATH 装锁定版 {pin}（{}），纳管跳过（--force 走真装）",
                            found.display()
                        );
                        emit_block(
                            &mut first,
                            vec![
                                kv("tool", name),
                                kv("action", "skipped"),
                                kv("version", pin),
                            ],
                        );
                        continue;
                    }
                }
            }
        }
        let step = resolve_tool(name, def, &ropts).and_then(|r| {
            if def.pin_tag() == Some(r.tag.as_str()) {
                // D49 漂移收口：云端无新版（resolve==pin）时再对照本机 installed 三态——
                // 一致才 skip；落后/未装真装 pin 版；领先如实报（消提示归数据面滚锁）。
                let installed = toolver::exe_path(def, env_root)
                    .ok()
                    .and_then(|exe| toolver::installed_version(&exe, def));
                let pin = def.pin_version().unwrap_or("");
                match toolver::pin_drift(installed.as_deref(), pin) {
                    toolver::PinDrift::Current => {
                        eprintln!("[INFO] {name} 已是最新: {pin}");
                        emit_block(
                            &mut first,
                            vec![
                                kv("tool", name),
                                kv("action", "skipped"),
                                kv("version", pin),
                            ],
                        );
                        return Ok(());
                    }
                    toolver::PinDrift::Ahead => {
                        eprintln!(
                            "[INFO] {name} 已装 {} 领先锁定 {pin}，保持现状；消 status 漂移提示需数据面滚锁",
                            installed.as_deref().unwrap_or("")
                        );
                        emit_block(
                            &mut first,
                            vec![
                                kv("tool", name),
                                kv("action", "skipped"),
                                kv("version", pin),
                            ],
                        );
                        return Ok(());
                    }
                    // Behind（含未装）：补装 pin 版，落到下方 install_tool
                    _ => eprintln!(
                        "[INFO] {name} 本机 {} 落后锁定 {pin}，补装锁定版",
                        installed.as_deref().unwrap_or("未装")
                    ),
                }
            }
            match install_tool(cat, env_root, name, &r, &iopts) {
                Ok(out) => {
                    emit_block(&mut first, install_rows(name, &out));
                    Ok(())
                }
                Err(e) => Err(e),
            }
        });
        if let Err(e) = step {
            skip_or_fail(tool, name, e, &mut errors)?;
        }
    }
    summarize_all_errors(&errors)
}

/// status：locked / installed / path 三态对照，按七类 taxonomy 分组（catalog 已按类排序，
/// 组标题为 # 注释行，category 变化即出新组头）。
fn cmd_status(cat: &Catalog, env_root: &Path) -> Result<(), String> {
    render::header(&format!("环境根目录: {}", env_root.display()));
    let mut last_cat = String::new();
    let mut first = true;
    let mut drifted: Vec<String> = Vec::new();
    // 流式：每探完一个工具立即输出（探测要逐工具拉起 --version 子进程，整批探完才打印会被感知为卡顿）
    status::collect_status_with(cat, env_root, |row| {
        // D09-3 CTA 素材：漂移收集（D49 尾统一 pin_drift 数值口径，npm-tgz 不列）
        let extract = cat
            .tool(&row.name)
            .ok()
            .and_then(|d| d.extract().map(|s| s.to_string()));
        if toolver::status_drift_hint(
            extract.as_deref(),
            row.installed.as_deref(),
            row.locked.as_deref(),
        ) {
            drifted.push(row.name.clone());
        }
        if row.category != last_cat {
            render::header(&format!("[{}]", status::category_label(&row.category)));
            last_cat = row.category.clone();
        }
        emit_block(
            &mut first,
            vec![
                kv("tool", &row.name),
                kv("locked", row.locked.as_deref().unwrap_or("")),
                kv("installed", row.installed.as_deref().unwrap_or("-")),
                kv("path", if row.path { "true" } else { "false" }),
                kv(
                    "exe",
                    &row.exe
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ),
            ],
        );
        Ok(())
    })?;
    // D09-3 CTA：漂移下一步建议（stderr，不进 stdout 数据面——R013 冻结契约；agent 类漂移
    // 走 agent 自更新通道，不建议 ark update）
    let updatable: Vec<String> = drifted
        .into_iter()
        .filter(|n| cat.tool(n).ok().and_then(|d| d.category.clone()) != Some("agent".into()))
        .collect();
    if !updatable.is_empty() {
        eprintln!(
            "[HINT] 版本与锁定不一致，升级或滚锁: ark update {}",
            updatable.join(",")
        );
    }
    Ok(())
}

/// init：复制当前 exe 到用户程序目录，同步 catalog 到用户数据目录，注册用户 PATH（幂等；self-deploy 别名）。
/// `ark catalog [status|sync]`：运行态软件清单查看与云端刷新（D33）。
fn cmd_catalog(env_root: &Path, cat_path: &Path, cmd: Option<CatalogCmd>) -> Result<(), String> {
    match cmd.unwrap_or(CatalogCmd::Status) {
        CatalogCmd::Status => {
            let st = catalog::catalog_state(env_root, cat_path);
            render::emit(&[
                kv("path", &st.path.display().to_string()),
                kv("origin", st.origin),
                kv("local_sha256", st.local_sha.as_deref().unwrap_or("")),
                kv("cloud_sha256", st.cloud_sha.as_deref().unwrap_or("")),
                kv("synced", if st.synced { "true" } else { "false" }),
                kv(
                    "age_secs",
                    &st.age_secs
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ),
                kv("ttl_secs", &st.ttl_secs.to_string()),
                kv("offline", if st.offline { "true" } else { "false" }),
                kv("signature", st.signature.label()),
                kv("pubkey", catalog::CLOUD_CATALOG_PUBKEY_ID),
                // manifest 面（R016 六节新鲜度门）：在位/缺失、本地 sha、年龄、云端锚对比、签名态
                kv("manifest_path", &st.manifest.path.display().to_string()),
                kv(
                    "manifest_present",
                    if st.manifest.present { "true" } else { "false" },
                ),
                kv(
                    "manifest_local_sha256",
                    st.manifest.local_sha.as_deref().unwrap_or(""),
                ),
                kv(
                    "manifest_cloud_sha256",
                    st.manifest.cloud_sha.as_deref().unwrap_or(""),
                ),
                kv(
                    "manifest_synced",
                    if st.manifest.synced { "true" } else { "false" },
                ),
                kv(
                    "manifest_age_secs",
                    &st.manifest
                        .age_secs
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ),
                kv("manifest_signature", st.manifest.signature.label()),
                kv(
                    "manifest_cloud_error",
                    st.manifest.cloud_error.as_deref().unwrap_or(""),
                ),
                kv("cloud_error", st.cloud_error.as_deref().unwrap_or("")),
            ]);
            Ok(())
        }
        CatalogCmd::Sync => {
            let target = catalog::user_data_catalog_path();
            // 显式通道：跳过 TTL 判定直接比对（ARK_CATALOG_TTL 与 ARK_OFFLINE 只管自动刷新路径）
            let out = catalog::sync_to(env_root, &target, true, catalog::auto_ttl())?;
            if out.action() == "updated" {
                eprintln!("[OK] catalog 已刷新: {}", target.display());
            } else {
                eprintln!("[INFO] catalog 已是云端当前版: {}", target.display());
            }
            render::emit(&[
                kv("action", out.action()),
                kv("reason", out.reason()),
                kv("sha256", out.sha().unwrap_or("")),
                kv("path", &target.display().to_string()),
                kv("origin", "cloud"),
            ]);
            Ok(())
        }
    }
}

fn cmd_init(env_root: &Path) -> Result<(), String> {
    let out = ark::selfdeploy::self_deploy(env_root)?;
    let catalog = out
        .catalog
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "none".to_string());
    render::emit(&[
        kv("action", if out.copied { "deployed" } else { "current" }),
        kv("exe", &out.exe.display().to_string()),
        kv("bin_dir", &out.bin_dir.display().to_string()),
        kv("catalog", &catalog),
        kv(
            "path",
            if out.path_registered {
                "registered"
            } else {
                "exists"
            },
        ),
    ]);
    Ok(())
}

// ── 输出行构造（数据行统一收敛为 Vec<(key, value)>，经 render 层输出）──

fn kv(k: &str, v: &str) -> (String, String) {
    (k.to_string(), v.to_string())
}

/// 解析结果行：query 含 size/url，pin 回写只出 tool/tag/version/asset。
fn resolution_rows(r: &Resolution, full: bool) -> Vec<(String, String)> {
    let mut rows = vec![
        kv("tool", &r.tool),
        kv("tag", &r.tag),
        kv("version", &r.version),
        kv("asset", &r.asset_name),
    ];
    if full {
        rows.push(kv("size", &r.asset_size.to_string()));
        rows.push(kv("url", &r.asset_url));
    }
    rows
}

/// 安装结果行：tool/action/version/dir。
fn install_rows(name: &str, out: &InstallOutcome) -> Vec<(String, String)> {
    let mut rows = vec![
        kv("tool", name),
        kv("action", out.action.as_str()),
        kv("version", &out.version),
    ];
    if let Some(d) = &out.dir {
        rows.push(kv("dir", &d.display().to_string()));
    }
    rows
}

/// 输出一组行（多工具之间空行分隔）。
fn emit_block(first: &mut bool, rows: Vec<(String, String)>) {
    if !*first {
        render::blank();
    }
    *first = false;
    render::emit(&rows);
}

/// sha256 展示：截前 16 位加 ...，未回填则标注。
fn short_sha(sha: Option<&str>) -> String {
    match sha {
        Some(s) if !s.is_empty() => {
            let head: String = s.chars().take(16).collect();
            format!("{head}...")
        }
        _ => "(未回填)".to_string(),
    }
}

/// doctor 总判定的缺口计数（可装缺失：exe 字段在而未装，排除平台不适用空态）。
fn missing_total(srows: &[ark::status::StatusRow]) -> usize {
    srows
        .iter()
        .filter(|r| r.exe.is_some() && r.installed.is_none())
        .count()
}
