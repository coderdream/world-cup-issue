. "$PSScriptRoot\y9000p-comfyui-env.ps1"

if (!(Test-Path $script:Python310)) {
  throw "Python 3.10 runtime not found: $script:Python310"
}

if (!(Test-Path $script:ComfyRoot)) {
  git clone https://github.com/comfyanonymous/ComfyUI.git $script:ComfyRoot
} else {
  git -C $script:ComfyRoot pull --ff-only
}

if (!(Test-Path $script:ComfyPython)) {
  & $script:Python310 -m venv "$script:ComfyRoot\venv"
}

& $script:ComfyPython -m pip install --upgrade pip
& $script:ComfyPython -m pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121
& $script:ComfyPython -m pip install -r "$script:ComfyRoot\requirements.txt"
& $script:ComfyPython -c "import torch; print(torch.__version__); print(torch.cuda.is_available()); print(torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'NO_CUDA')"
