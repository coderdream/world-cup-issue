$ErrorActionPreference = "Stop"

$stats = curl.exe --noproxy 127.0.0.1 -s http://127.0.0.1:8188/system_stats
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($stats)) {
  throw "ComfyUI system_stats request failed."
}

$stats
