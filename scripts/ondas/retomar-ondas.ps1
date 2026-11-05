# Inicia o Claude Code em segundo plano (sem janela) para retomar as ondas do Fenkra.
param([string]$Limite = "50%")
$raiz = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $raiz
$prompt = (Get-Content "$PSScriptRoot\prompt-ondas.md" -Raw -Encoding UTF8).Replace('{{LIMITE}}', $Limite)
$log = Join-Path $raiz "docs\backlog\execucao.log"
"=== inicio $(Get-Date -Format s) limite=$Limite ===" | Out-File $log -Append -Encoding utf8
$prompt | claude -p --permission-mode acceptEdits `
  --allowedTools "Bash(cargo:*)" "Bash(git:*)" "Bash(python:*)" "Bash(powershell:*)" "Read" "Edit" "Write" "Glob" "Grep" "Agent" `
  --output-format stream-json --verbose *>> $log
"=== fim $(Get-Date -Format s) codigo=$LASTEXITCODE ===" | Out-File $log -Append -Encoding utf8
