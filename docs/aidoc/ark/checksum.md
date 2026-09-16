# ark::checksum

checksum：sha256 校验源解析，语义对齐 helpers.ps1 的 Get-OfficialSha256。
优先级（R001 四、2）：pin 的 sha256 > 官方校验源三型——
1) cdn_index_url 的 HashiCorp SHA256SUMS 清单（shasums_url 来自解析结果）；
2) sums_asset 统一校验清单（{version}/{tag} 占位，按 sums_pattern 取行）；
3) asset_sha_suffix 逐资产后缀文件（如 <asset>.sha256）。

校验清单类资产每次强制重下（删旧再下），结果统一大写。

## Functions

- `expected_sha256` — 本次下载应遵循的 sha256 基准：pin 的 sha256 优先，但必须 **同 tag 且同 asset**
- `extract_hex64` — 从一行文本提取第一个 64 位 hex 并大写（对齐 pwsh 的 ([0-9a-fA-F]{64}) + ToUpperInvariant）。
- `official_sha256` — 官方校验源三型（对齐 Get-OfficialSha256 的分支顺序）。

