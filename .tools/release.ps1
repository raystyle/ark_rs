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
  pwsh -NoProfile -File .tools/release.ps1 -Tag v1.4.0 -IncludePackages        # REQ-0008 双挂窗形（包形并挂）

前置：HEAD 即 tag 指向提交且远端 tag 已推（git push origin <tag>；-SkipTagCheck 豁免排练，
豁免面含发布预检 0b）；x86_64-w64-mingw32-gcc 与 rustup target x86_64-pc-windows-gnu 在位；
lan-mac mesh 可达（mac 实机构建腿，-SkipMac 跳过）；
gh 已登录。产物形：三平台裸二进制加逐件 .sha256 边车；-IncludePackages 加挂包形
（REQ-0008 双挂窗形：单顶层目录 ark-<净triple> 直放二进制加 README 加 LICENSE，win zip 他 tar.gz）。
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Tag,
    [switch]$DryRun,
    [switch]$SkipTagCheck,
    [switch]$SkipMac,
    # REQ-0008 窗口：双挂窗开启（裸件加包形并挂）；缺省裸件单形 = 2.2.0 终版形
    [switch]$IncludePackages
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
}
# 0a 工作树洁净闸：脏树（含未跟踪新文件——忘 git add 的最常见漏形态）构建产物与 tag 源
# 不一致且无迹可查；dist/ 与 __pycache__ 已 ignore 不误伤（codex 批二 G1 加 G2 收紧）
$dirty = @(git status --porcelain)
if ($dirty.Count -gt 0) { Fail "工作树不洁净（$($dirty.Count) 件，含未跟踪），先提交或排练加 -SkipTagCheck" }
if (-not $SkipTagCheck) {
    # 0b 发布预检（照 hst 1b 族规，codex 批二 F1）：gh 已登、远端 tag 已推且指本 sha、
    # tag 无既有 Release——gh release create 遇远程无 tag 会自动钉默认分支 HEAD，锚链失保
    gh auth status *> $null
    if ($LASTEXITCODE -ne 0) { Fail 'precheck: gh 未登录（先 gh auth login）' }
    $remoteTag = git ls-remote --tags origin "refs/tags/$Tag" 2>$null
    if ($LASTEXITCODE -ne 0) { Fail 'precheck: 远端不可达（git ls-remote 非零），查网络或 origin' }
    $remoteTag = ($remoteTag | Out-String).Trim()
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

# 5 打包与逐件边车（sha256sum 原生格式：小写 hex 空两格名；裸件形）
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

# 5b 包形构建（REQ-0008 窗口，-IncludePackages 才出）：单顶层目录 ark-<净triple> 直放
# 二进制（win 形 ark.exe）加 README 加 LICENSE，win zip 他 tar.gz，逐包边车。
# 暂存目录在系统 temp（不进 dist 防 gh 连目录上传）。
if ($IncludePackages) {
    if (-not (Test-Path LICENSE)) { Fail '包形契约要 LICENSE（仓库根缺件）' }
    $pkgStage = Join-Path ([System.IO.Path]::GetTempPath()) 'ark-release-pkg'
    if (Test-Path $pkgStage) { Remove-Item -Recurse -Force $pkgStage }
    New-Item -ItemType Directory -Path $pkgStage | Out-Null
    foreach ($a in $assets) {
        $triple = ($a.name -replace '^ark-', '') -replace '\.exe$', ''
        $ext = if ($triple -like '*windows*') { 'zip' } else { 'tar.gz' }
        $inner = if ($triple -like '*windows*') { 'ark.exe' } else { 'ark' }
        $dirName = "ark-$triple"
        New-Item -ItemType Directory -Path (Join-Path $pkgStage $dirName) | Out-Null
        Copy-Item (Join-Path $dist $a.name) (Join-Path $pkgStage "$dirName/$inner")
        Copy-Item README.md (Join-Path $pkgStage $dirName)
        Copy-Item LICENSE (Join-Path $pkgStage $dirName)
        $pkgName = "$dirName.$ext"
        if ($ext -eq 'zip') {
            Compress-Archive -Path (Join-Path $pkgStage $dirName) -DestinationPath (Join-Path $dist $pkgName)
        } else {
            Push-Location $pkgStage; tar czf (Join-Path $dist $pkgName) $dirName; Pop-Location
            if ($LASTEXITCODE -ne 0) { Fail "tar czf $pkgName 红" }
        }
        $hex = (Get-FileHash (Join-Path $dist $pkgName) -Algorithm SHA256).Hash.ToLower()
        "$hex  $pkgName" | Set-Content (Join-Path $dist "$pkgName.sha256") -Encoding ascii
        Write-Host "[OK] 包形 $pkgName"
    }
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
    # 冒烟逐字对（codex 批二 G4）：子串匹配会把 1.3.10 判成 1.3.1 命中，改首行前缀逐字；
    # ssh stdout 须干净（远端 shell 横幅打 stdout 会误红，stderr 无碍）
    $first = ($out | Select-Object -First 1).Trim()
    if ($LASTEXITCODE -ne 0 -or $first -notmatch "^ark $([regex]::Escape($ver))(\s|$)") {
        Fail "冒烟红 $($a.name)：$($out -join ' ')"
    }
    Write-Host "[OK] 冒烟 $($a.name)：$($out -join ' ')"
}

# 6b 包形冒烟（-IncludePackages 才跑）：解包取内层（拼名契约 ark-<净triple>/ark(.exe)），
# 内容等价断言（包内二进制 sha 与裸件逐字等）加本平台可跑件直跑 --version
if ($IncludePackages) {
    # 验证目录在仓根（dist-pkg-verify，gitignore dist-pkg*/ 已收）：WSL interop 起 PE
    # 在 /tmp 实测 ENOENT（仓根路径正常，2026-09-17 窗一版演练实录）
    $pkgVerify = Join-Path $root 'dist-pkg-verify'
    if (Test-Path $pkgVerify) { Remove-Item -Recurse -Force $pkgVerify }
    New-Item -ItemType Directory -Path $pkgVerify | Out-Null
    foreach ($a in $assets) {
        $triple = ($a.name -replace '^ark-', '') -replace '\.exe$', ''
        $ext = if ($triple -like '*windows*') { 'zip' } else { 'tar.gz' }
        $inner = if ($triple -like '*windows*') { 'ark.exe' } else { 'ark' }
        $pkgName = "ark-$triple.$ext"
        $vdir = Join-Path $pkgVerify $triple
        New-Item -ItemType Directory -Path $vdir | Out-Null
        if ($ext -eq 'zip') {
            Expand-Archive -Path (Join-Path $dist $pkgName) -DestinationPath $vdir
        } else {
            tar xzf (Join-Path $dist $pkgName) -C $vdir
            if ($LASTEXITCODE -ne 0) { Fail "包形解包红 $pkgName" }
        }
        $innerPath = Join-Path $vdir "ark-$triple/$inner"
        if (-not (Test-Path $innerPath)) { Fail "包内拼名未命中 $innerPath（契约 ark-<target>/ark）" }
        $ha = (Get-FileHash $innerPath -Algorithm SHA256).Hash.ToLower()
        $hb = (Get-FileHash (Join-Path $dist $a.name) -Algorithm SHA256).Hash.ToLower()
        if ($ha -ne $hb) { Fail "包内二进制与裸件 sha 不等 $pkgName" }
        # interop/exec 都要执行位：Expand-Archive 不保 +x（zip 实测 ENOENT 根因）
        chmod +x $innerPath
        if ($triple -like '*windows*') {
            $out = @(& $innerPath --version)
        } elseif ($triple -like '*linux*') {
            $out = @(& $innerPath --version)
        } else {
            continue  # mac 内层与裸件同物（sha 等已断言），实机冒烟走裸件面
        }
        $first = ($out | Select-Object -First 1).Trim()
        if ($LASTEXITCODE -ne 0 -or $first -notmatch "^ark $([regex]::Escape($ver))(\s|$)") {
            Fail "包形冒烟红 $pkgName：$($out -join ' ')"
        }
        Write-Host "[OK] 包形冒烟 $pkgName：$first"
    }
}

# 7 GitHub 产物发布：draft 挂资产传齐后 edit --draft=false 加 --latest 直发
Write-Host '== gh 直发（draft 传齐再发布，published 即齐备信号）=='
$files = @(Get-ChildItem $dist -File | Sort-Object Name | ForEach-Object { $_.FullName })
if ($DryRun) {
    Write-Host "[DryRun] gh release create $Tag（$($files.Count) 件）--draft 后 gh release edit $Tag --draft=false --latest"
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
