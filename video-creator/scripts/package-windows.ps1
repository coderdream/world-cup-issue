$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$repoRoot = Resolve-Path (Join-Path $root "..")

Write-Host "[1/4] Clean regenerated build outputs"
$cleanTargets = @(
  (Join-Path $root "dist"),
  (Join-Path $root "target"),
  (Join-Path $root "release"),
  (Join-Path $root "src-tauri\target")
)
foreach ($target in $cleanTargets) {
  if (Test-Path $target) {
    Remove-Item -LiteralPath $target -Recurse -Force
  }
}

Write-Host "[2/4] Configure GNU Rust toolchain"
$env:RUSTUP_HOME = (Resolve-Path (Join-Path $repoRoot ".tooling\rustup")).Path
$env:CARGO_HOME = (Resolve-Path (Join-Path $repoRoot ".tooling\cargo")).Path
$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-gnu"
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "0"
$env:RUSTFLAGS = "-C debuginfo=0"

$mingw = "C:\Users\Administrator\scoop\apps\mingw\current\bin"
$rustLinker = (Resolve-Path (Join-Path $repoRoot ".tooling\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin")).Path
$env:PATH = "$mingw;$rustLinker;$env:PATH"

Write-Host "[3/4] Build frontend"
pnpm -C $root build

Write-Host "[4/4] Build Tauri NSIS package"
pnpm -C $root tauri build --ci --target x86_64-pc-windows-gnu
