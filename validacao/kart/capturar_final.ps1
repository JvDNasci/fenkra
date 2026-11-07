# Capturas finais do kart pintado, para o diário. Uso: capturar_final.ps1 <pasta_do_diario>
param([string]$diario)
foreach ($f in "SC-QUADRO", "SC-EIXO", "SC-DIRECAO", "SC-MOTOR", "SC-FREIO", "SC-POSTO", "SC-CARROCERIA", "KART-PLANO", "KART.fka", "QUADRO", "EX-08", "RD-04", "RD-02", "PO-01", "PR-06", "CH-08", "CH-01", "TR-01", "EX-01") {
    python capturar.py $diario $f pintado
}
python capturar.py $diario DW-KART
