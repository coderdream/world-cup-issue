$ErrorActionPreference = "Stop"

$env:HTTP_PROXY = "http://127.0.0.1:1080"
$env:HTTPS_PROXY = "http://127.0.0.1:1080"
$env:ALL_PROXY = "socks5://127.0.0.1:1080"
$env:PIP_CACHE_DIR = "D:\AI\pip-cache"
$env:HF_HOME = "D:\AI\huggingface"
$env:TRANSFORMERS_CACHE = "D:\AI\huggingface\transformers"
$env:PYTHONUTF8 = "1"
$env:PYTHONIOENCODING = "utf-8"
$env:TERM = "dumb"

$script:ComfyRoot = "D:\AI\apps\ComfyUI"
$script:ComfyPython = "D:\AI\apps\ComfyUI\venv\Scripts\python.exe"
$script:Python310 = "D:\AI\runtimes\Python310\python.exe"

$torchLib = "D:\AI\apps\ComfyUI\venv\Lib\site-packages\torch\lib"
if (Test-Path $torchLib) {
  $env:PATH = "$torchLib;$env:PATH"
}

$torchaudioLib = "D:\AI\apps\ComfyUI\venv\Lib\site-packages\torchaudio\lib"
if (Test-Path $torchaudioLib) {
  $env:PATH = "$torchaudioLib;$env:PATH"
}

New-Item -ItemType Directory -Force `
  "D:\AI\apps", `
  "D:\AI\models", `
  "D:\AI\outputs", `
  "D:\AI\workflows", `
  "D:\AI\downloads", `
  "D:\AI\logs", `
  "D:\AI\runtimes", `
  "D:\AI\pip-cache", `
  "D:\AI\huggingface" | Out-Null
