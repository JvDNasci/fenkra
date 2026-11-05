@echo off
rem Duplo clique: inicia a execucao das ondas sem janela. Acompanhe em docs\backlog\execucao.log
set LIMITE=%1
if "%LIMITE%"=="" set LIMITE=50%%
start "" /min powershell -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File "%~dp0retomar-ondas.ps1" -Limite "%LIMITE%"
