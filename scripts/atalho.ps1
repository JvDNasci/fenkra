# Cria ou atualiza o atalho "Fenkra" na área de trabalho, apontando para o pacote mais novo em dist.
#
#   powershell -ExecutionPolicy Bypass -File scripts\atalho.ps1

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$versao = (Select-String -Path (Join-Path $root "Cargo.toml") -Pattern '^version\s*=\s*"(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$pasta = Join-Path $root "dist\Fenkra-$versao-windows-x64"
$exe = Join-Path $pasta "fenkra.exe"
if (-not (Test-Path $exe)) { throw "pacote não encontrado: $exe" }

$destino = Join-Path ([Environment]::GetFolderPath("Desktop")) "Fenkra.lnk"
$shell = New-Object -ComObject WScript.Shell
$atalho = $shell.CreateShortcut($destino)
$atalho.TargetPath = $exe
$atalho.WorkingDirectory = $pasta
$atalho.IconLocation = "$exe,0"
$atalho.Description = "Fenkra CAD $versao"
$atalho.Save()
Write-Host "Atalho: $destino -> $exe"
