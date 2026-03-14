# Run Trunk with rag-ui on PATH so "sh" resolves to sh.cmd (Windows fix for Trunk's built-in hooks)
$uiDir = $PSScriptRoot
$env:PATH = "$uiDir;$env:PATH"
if (Test-Path Env:NO_COLOR) { Remove-Item Env:NO_COLOR }
Set-Location $uiDir
trunk serve --port 8081
