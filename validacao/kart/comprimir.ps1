# Copia as capturas úteis do diário para capturas/ em JPEG. Uso: comprimir.ps1 <pasta_do_diario>
param([string]$diario)
Add-Type -AssemblyName System.Drawing
$codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq "image/jpeg" }
$par = New-Object System.Drawing.Imaging.EncoderParameters 1
$par.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter([System.Drawing.Imaging.Encoder]::Quality, [long]72)
$destino = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) "capturas"
$nomes = "kart_geral_colorido", "kart_lateral_colorido", "kart_subconjuntos_pintado", "sc-direcao_pronta_colorido", "sc-eixo_pronta_colorido", "sc-quadro_pronta_colorido", "quadro_topo", "ch-08_painel_aba_pintado", "ex-12_falhou", "po-04_planificado", "dw-kart_folha_colorido", "dw-quadro_folha", "tr-01_pronta", "po-01_pronta_pintado", "pr-06_pronta"
foreach ($n in $nomes) {
    $f = Get-ChildItem $diario -Filter "*_$n.png" | Select-Object -Last 1
    if (-not $f) { "sem $n"; continue }
    $img = [System.Drawing.Image]::FromFile($f.FullName)
    $img.Save((Join-Path $destino "$n.jpg"), $codec, $par)
    $img.Dispose()
}
