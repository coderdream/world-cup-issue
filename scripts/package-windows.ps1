$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

$tool = Join-Path $root ".tooling"
$gnuBin = Join-Path $tool "rustup\toolchains\stable-x86_64-pc-windows-gnu\bin"
$keyPath = Join-Path $tool "updater\worldcupissue.key"

if (-not (Test-Path $keyPath)) {
  throw "Missing updater signing key: $keyPath"
}

$env:RUSTUP_HOME = Join-Path $tool "rustup"
$env:CARGO_HOME = Join-Path $tool "cargo"
$env:TEMP = Join-Path $tool "tmp"
$env:TMP = Join-Path $tool "tmp"
$env:PATH = "$gnuBin;C:\Users\Administrator\.cargo\bin;C:\Users\Administrator\scoop\apps\mingw\current\bin;$env:PATH"
$env:HTTP_PROXY = "http://127.0.0.1:1080"
$env:HTTPS_PROXY = "http://127.0.0.1:1080"
$env:CARGO_HTTP_PROXY = "http://127.0.0.1:1080"
$env:CARGO_HTTP_TIMEOUT = "600"
$env:CARGO_NET_RETRY = "10"
$env:CARGO_REGISTRIES_CRATES_IO_PROTOCOL = "sparse"
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "0"
$env:RUSTFLAGS = "-C debuginfo=0"
$env:NODE_OPTIONS = "--max-old-space-size=2048"
$env:ESBUILD_WORKER_THREADS = "0"
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content -Path $keyPath -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""

pnpm tauri build --ci --target x86_64-pc-windows-gnu
node scripts\write-update-manifest.mjs
