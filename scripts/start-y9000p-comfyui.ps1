. "$PSScriptRoot\y9000p-comfyui-env.ps1"

if (!(Test-Path $script:ComfyPython)) {
  throw "ComfyUI venv python not found: $script:ComfyPython"
}

Set-Location $script:ComfyRoot
& $script:ComfyPython main.py --listen 0.0.0.0 --port 8188 --output-directory "D:\AI\outputs\ComfyUI"
