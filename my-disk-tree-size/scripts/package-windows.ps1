$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$repoRoot = Resolve-Path (Join-Path $root "..")
$env:RUSTUP_HOME = Join-Path $repoRoot ".tooling\rustup"
$env:CARGO_HOME = Join-Path $repoRoot ".tooling\cargo"
$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-gnu"
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "0"
$env:RUSTFLAGS = "-C debuginfo=0"
$env:HTTP_PROXY = "http://127.0.0.1:1080"
$env:HTTPS_PROXY = "http://127.0.0.1:1080"
$env:PATH = (Join-Path $repoRoot ".tooling\rustup\toolchains\stable-x86_64-pc-windows-gnu\bin") + ";C:\Users\Administrator\.cargo\bin;C:\Users\Administrator\scoop\apps\mingw\current\bin;" + $env:PATH

pnpm tauri build --ci --target x86_64-pc-windows-gnu
