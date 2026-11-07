# Captura o app real abrindo um arquivo do kart. Uso:
#   capturar.ps1 -arquivo QUADRO.fkp -saida quadro.png [-bancada Estrutural] [-camera "35,25,1"] [-extra "FENKRA_EXIBIR=zebra;FENKRA_DEMO_TEMA=Claro"]
param([string]$arquivo, [string]$saida, [string]$bancada = "", [string]$camera = "35,25,1", [string]$extra = "", [int]$espera = 30)
$raiz = Split-Path -Parent $MyInvocation.MyCommand.Path
$tmp = Join-Path $env:TEMP ("kart14_app_" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force $tmp | Out-Null
$png = Join-Path $raiz ("capturas\" + $saida)
$env:APPDATA = $tmp
$env:FENKRA_DEMO_FOTO = $png
$env:FENKRA_DEMO_CAMERA = $camera
$env:FENKRA_DEMO_FOTO_ESPERA = "$($espera - 6)"
if ($arquivo) { $env:FENKRA_DEMO_ARQUIVO = Join-Path $raiz ("modelo\" + $arquivo) }
if ($bancada) { $env:FENKRA_BANCADA = $bancada }
foreach ($par in $extra.Split(";")) {
    if ($par -match "^([^=]+)=(.*)$") { Set-Item -Path "env:$($Matches[1])" -Value $Matches[2] }
}
$p = Start-Process C:\cargo-target\kart14\release\fenkra.exe -PassThru -WindowStyle Minimized
if (-not $p.WaitForExit($espera * 1000)) { Stop-Process -Id $p.Id -Force; "tempo esgotado: $saida" } else { "ok: $saida" }
Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
