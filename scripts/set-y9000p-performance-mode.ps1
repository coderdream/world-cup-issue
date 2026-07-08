$ErrorActionPreference = "Stop"

# Windows-side performance profile for the Y9000P / 187 local ComfyUI node.
# Run from an elevated PowerShell when possible:
# powershell -NoProfile -ExecutionPolicy Bypass -File scripts\set-y9000p-performance-mode.ps1

$ultimateGuid = "e9a42b02-d5df-448d-aa00-03f14749eb61"
$highGuid = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"

$plans = powercfg /L
if ($plans -notmatch $ultimateGuid) {
  powercfg -duplicatescheme $ultimateGuid | Out-Null
}

$plans = powercfg /L
$target = if ($plans -match $ultimateGuid) { $ultimateGuid } else { $highGuid }

powercfg /S $target
powercfg /SETACVALUEINDEX $target SUB_PROCESSOR PROCTHROTTLEMIN 100
powercfg /SETACVALUEINDEX $target SUB_PROCESSOR PROCTHROTTLEMAX 100
powercfg /SETACVALUEINDEX $target SUB_PROCESSOR SYSCOOLPOL 1
powercfg /SETACVALUEINDEX $target SUB_PCIEXPRESS ASPM 0
powercfg /SETACVALUEINDEX $target 19cbb8fa-5279-450e-9fac-8a3d5fedd0c1 12bbebe6-58d6-4636-95bb-3217ef867c1a 0
powercfg /S $target

$pythonPath = "D:\AI\apps\ComfyUI\venv\Scripts\python.exe"
if (Test-Path $pythonPath) {
  $gpuPreferenceKey = "HKCU:\Software\Microsoft\DirectX\UserGpuPreferences"
  New-Item -Path $gpuPreferenceKey -Force | Out-Null
  New-ItemProperty -Path $gpuPreferenceKey -Name $pythonPath -Value "GpuPreference=2;" -PropertyType String -Force | Out-Null
}

Write-Host "Active Windows power plan:"
powercfg /GETACTIVESCHEME

Write-Host ""
Write-Host "NVIDIA GPU status:"
nvidia-smi --query-gpu=name,driver_version,pstate,power.draw,power.limit,clocks.gr,clocks.mem,utilization.gpu,memory.total,memory.used --format=csv

Write-Host ""
Write-Host "Note: RTX 3070 Laptop GPU under WDDM usually does not support nvidia-smi persistence mode or power-limit changes."
Write-Host "For the highest sustained performance, also set Lenovo Vantage/Legion mode to Performance and keep AC power connected."
