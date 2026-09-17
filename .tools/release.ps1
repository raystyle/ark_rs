#Requires -Version 7
<#
release.ps1 - ark 本地编译打包发布命令面（REQ-0007 批二，对齐 build-release 标准三段式）。
三段式：本脚本承担第一二段（本地编译打包加 gh release 直发）；第三段自动播种由
r2-seed workflow 于 release published 事件触发（版本段加 stable 段双段同灌）。

发布形：gh release create --draft 挂资产传齐后 gh release edit --draft=false 发布
（published 即齐备信号，r2-seed 齐备闸直过；终态非 draft 不违标准禁 draft 形）。

用法（pwsh 7）：
  pwsh -NoProfile -File .tools/release.ps1 -Tag v1.3.1                    # 全链发布
  pwsh -NoProfile -File .tools/release.ps1 -Tag v1.3.1 -DryRun            # 全链演练止步于发布步
  pwsh -NoProfile -File .tools/release.ps1 -Tag v1.3.1 -SkipTagCheck -DryRun  # 未打 tag 排练

前置：HEAD 即 tag 指向提交（-SkipTagCheck 豁免排练）；x86_64-w64-mingw32-gcc 与
rustup target x86_64-pc-windows-gnu 在位；lan-mac mesh 可达（mac 实机构建腿，-SkipMac 跳过）；
gh 已登录。产物形：三平台裸二进制加逐件 .sha256 边车（包形归 REQ-0008 批三）。
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Tag,
    [switch]$DryRun,
    [switch]$SkipTagCheck,
    [switch]$SkipMac
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot/..").Path
Set-Location $root
$ver = $Tag.TrimStart('v')

function Fail([string]$msg) { Write-Host "[FAIL] $msg" -ForegroundColor Red; exit 1 }
function RunOk([string]$what) { if ($LASTEXITCODE -ne 0) { Fail $what } }

# 0 tag 锚闸：HEAD 即 tag 指向提交（排练可豁免）
if (-not $SkipTagCheck) {
    $at = @(git tag --points-at HEAD)
    if ($at -notcontains $Tag) { Fail "HEAD 不在 tag $Tag 指向提交（排练加 -SkipTagCheck）" }
    # 0a 工作树洁净闸：脏树构建产物与 tag 源不一致且无迹可查（codex 批二 G1）
    $dirty = @(git status --porcelain --untracked-files=no)
    if ($dirty.Count -gt 0) { Fail "工作树有未提交改动（$($dirty.Count) 件），先提交或排练加 -SkipTagCheck" }
    # 0b 发布预检（照 hst 1b 族规，codex 批二 F1）：gh 已登、远端 tag 已推且指本 sha、
    # tag 无既有 Release——gh release create 遇远程无 tag 会自动钉默认分支 HEAD，锚链失保
    gh auth status *> $null
    if ($LASTEXITCODE -ne 0) { Fail 'precheck: gh 未登录（先 gh auth login）' }
    $remoteTag = (git ls-remote --tags origin "refs/tags/$Tag" 2>$null | Out-String).Trim()
    if ($remoteTag -eq '') { Fail "precheck: 远端无 tag $Tag，先 git push origin $Tag（gh 会自动钉错 HEAD）" }
    if ($remoteTag -notmatch [regex]::Escape((git rev-parse HEAD).Trim())) {
        Fail "precheck: 远端 tag $Tag 指向别处，须自本 HEAD 重推"
    }
    gh release view $Tag *> $null
    if ($LASTEXITCODE -eq 0) {
        Fail "precheck: release $Tag 已存在，恢复路径 gh release delete $Tag --yes 后重跑，或 gh release upload $Tag <资产> --clobber"
    }
    Write-Host '[OK] precheck: gh 登录、远端 tag 锚本 sha、无既有 release'
}

# 1 版本一致性闸：tag 对载体 manifest（Cargo.toml 唯一权威，不一致即止红）
$cargoVer = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' |
    Select-Object -First 1).Matches[0].Groups[1].Value
if ($cargoVer -ne $ver) { Fail "版本一致性闸红：Cargo.toml=$cargoVer 对 tag=$ver 不一致" }
Write-Host "[OK] 版本一致性闸：Cargo.toml $cargoVer 与 $Tag 一致"

# 2 测试闸先行（本地；v* tag 不再触发 CI，本闸即发布前最后门禁——md 四件与 aidoc 同入，
# codex 批二 G2：v1.3.0 的 aidoc 漂移正是无本地门禁兜底的那一类）
Write-Host '== 测试闸：cargo test 加 md 四件套加 aidoc =='
cargo test --release --locked
RunOk '测试闸红（cargo test）'
uv run --script .tools/mdcharlint.py .
RunOk '测试闸红（mdcharlint）'
uv run --script .tools/md-ref-scan.py
RunOk '测试闸红（md-ref-scan）'
uv run --script .tools/md-heading-scan.py
RunOk '测试闸红（md-heading-scan）'
rumdl check .
RunOk '测试闸红（rumdl）'
cargo aidoc --check --strict
RunOk '测试闸红（aidoc 漂移）'

# 3 本地编译（linux 本职加 win-gnu 交叉；mac 实机腿见第 4 步）
foreach ($t in @('x86_64-unknown-linux-gnu', 'x86_64-pc-windows-gnu')) {
    Write-Host "== 构建 $t =="
    cargo build --release --locked --target $t
    RunOk "构建 $t 红"
}

# 4 mac 实机构建（lan-mac mesh；rsync 源码树免管远端 git 态）
$macBin = 'ark-aarch64-apple-darwin'
if (-not $SkipMac) {
    Write-Host '== mac 实机构建（lan-mac）=='
    rsync -a --delete --exclude target --exclude .git --exclude dist ./ 'lan-mac:~/ark-release-build/'
    RunOk 'rsync 源码树到 lan-mac 红'
    ssh lan-mac 'cd ~/ark-release-build && cargo build --release --locked --target aarch64-apple-darwin'
    RunOk 'lan-mac 构建红'
    scp -q "lan-mac:~/ark-release-build/target/aarch64-apple-darwin/release/ark" "/tmp/$macBin"
    RunOk '回传 mac 产物红'
}

# 5 打包与逐件边车（sha256sum 原生格式：小写 hex 空两格名；裸件形，包形归批三）
$dist = Join-Path $root 'dist'
if (Test-Path $dist) { Remove-Item -Recurse -Force $dist }
New-Item -ItemType Directory -Path $dist | Out-Null
$assets = @(
    @{ src = "target/x86_64-unknown-linux-gnu/release/ark"; name = 'ark-x86_64-unknown-linux-gnu' },
    @{ src = 'target/x86_64-pc-windows-gnu/release/ark.exe'; name = 'ark-x86_64-pc-windows-gnu.exe' },
    @{ src = "/tmp/$macBin"; name = $macBin }
)
foreach ($a in $assets) {
    if (-not (Test-Path $a.src)) { Fail "产物缺 $($a.src)（mac 腿见 -SkipMac）" }
    Copy-Item $a.src (Join-Path $dist $a.name)
    $hex = (Get-FileHash (Join-Path $dist $a.name) -Algorithm SHA256).Hash.ToLower()
    "$hex  $($a.name)" | Set-Content (Join-Path $dist "$($a.name).sha256") -Encoding ascii
}

# 6 冒烟：每件 --version 对 tag（win 走 WSL interop 直调 PE；mac 在实机跑）
foreach ($a in $assets) {
    if ($a.name -eq $macBin) {
        if ($SkipMac) { continue }
        scp -q (Join-Path $dist $a.name) "lan-mac:/tmp/$macBin"
        RunOk "上载冒烟件 $($a.name) 红"
        # scp 不保执行位，先 chmod 再跑（2026-09-17 演练实录：permission denied）
        $out = @(ssh lan-mac "chmod +x /tmp/$macBin && /tmp/$macBin --version")
    } else {
        $out = @(& (Join-Path $dist $a.name) --version)
    }
    # 冒烟逐字对（codex 批二 G4）：子串匹配会把 1.3.10 判成 1.3.1 命中，改首行前缀逐字
    $first = ($out | Select-Object -First 1).Trim()
    if ($LASTEXITCODE -ne 0 -or $first -notmatch "^ark $([regex]::Escape($ver))(\s|$)") {
        Fail "冒烟红 $($a.name)：$($out -join ' ')"
    }
    Write-Host "[OK] 冒烟 $($a.name)：$($out -join ' ')"
}

# 7 GitHub 产物发布：draft 挂资产传齐后 edit --draft=false 加 --latest 直发
Write-Host '== gh 直发（draft 传齐再发布，published 即齐备信号）=='
$files = @(Get-ChildItem $dist | Sort-Object Name | ForEach-Object { $_.FullName })
if ($DryRun) {
    Write-Host "[DryRun] gh release create $Tag（6 件）--draft 后 gh release edit $Tag --draft=false --latest"
} else {
    gh release create $Tag @files --draft --title "ark $Tag" `
        --notes "正式版 $Tag（release.ps1 三段式发布）。安装：ark self update --stable；agent 发现通道：ark --llms。"
    RunOk 'gh release create 红'
    gh release edit $Tag --draft=false --latest
    RunOk 'gh release edit 发布红'
    Write-Host "[OK] 已发布 $Tag（r2-seed 于 published 事件双段同灌，收执见 Actions r2-seed run）"
}

Write-Host '== 逐件摘要 =='
Get-ChildItem "$dist/*.sha256" | ForEach-Object { Get-Content $_ }
