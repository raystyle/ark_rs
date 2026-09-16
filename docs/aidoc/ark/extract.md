# ark::extract

extract：解压/安装分派，对齐 helpers.ps1 Install-ToolVersion 的 switch（1148-1242 行）。
zip/targz 展平顶层单包裹目录；copy/single 单 exe 落目录；gsudo 只取 x64；
7z-extra 用 bootstrap 7zr.exe 解压并 shim 7za.exe 成 7z.exe 后清空目录其余文件；
7zsfx 同步等待退出码；msi 走 msiexec /qn；rmux 跑资产内官方 install.ps1；

## Functions

- `extract_asset` — 解压/安装主分派（对齐 pwsh switch ($d.Extract)）。
- `extract_targz` — tar.gz 解压（flate2 + tar crate）。
- `extract_tarxz` — tar.xz 解压（xz2 + tar crate）。
- `extract_zip` — zip 解压（zip crate，防路径穿越：拒绝越出目标的条目）。
- `flatten_single_wrapper` — 展平顶层单包裹目录（如 age zip 的 age/、python 的 python/ 包裹层）：

