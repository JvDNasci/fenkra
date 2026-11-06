# Gera o pacote de release do Fenkra para Windows x64.
#
#   powershell -ExecutionPolicy Bypass -File scripts\release.ps1
#
# Resultado: dist\Fenkra-<versão>-windows-x64\ e o .zip correspondente.
# Só as DLLs realmente usadas pelo executável são copiadas (fechamento de dependências via dumpbin).

param(
    [string]$OcctRoot = $(if ($env:OCCT_ROOT) { $env:OCCT_ROOT } else { "C:\dev\vcpkg\installed\x64-windows-release" }),
    [switch]$SkipBuild
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# versão do workspace
$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$name = "Fenkra-$version-windows-x64"
$dist = Join-Path $root "dist"
$out = Join-Path $dist $name
Write-Host "== Fenkra $version =="

# arquivos de licença obrigatórios (B151): sem eles o release não sai
$licencas = @("LICENSE", "NOTICE", "THIRD-PARTY-LICENSES")
$faltam = @($licencas | Where-Object { -not (Test-Path (Join-Path $root $_)) })
if ($faltam.Count -gt 0) { throw "arquivos de licença ausentes na raiz: $($faltam -join ', ')" }

$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
if (-not $SkipBuild) {
    cargo build --release -p fenkra-app -p fenkra-cli -p fenkra-mcp
    if ($LASTEXITCODE -ne 0) { throw "falha na compilação" }
}
# a pasta de build pode estar fora do projeto (.cargo\config.toml)
$targetDir = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).target_directory
$exe = Join-Path $targetDir "release\fenkra.exe"
if (-not (Test-Path $exe)) { throw "executável não encontrado: $exe" }
$conv = Join-Path $targetDir "release\fenkra-conv.exe"
if (-not (Test-Path $conv)) { throw "conversor não encontrado: $conv" }
# automação sem interface: linha de comando e servidor MCP, ao lado do programa para achar as DLLs
$automacao = @("fenkra-cli.exe", "fenkra-mcp.exe") | ForEach-Object { Join-Path $targetDir "release\$_" }
foreach ($a in $automacao) { if (-not (Test-Path $a)) { throw "programa não encontrado: $a" } }

# dumpbin do Visual Studio
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
$dumpbin = Get-ChildItem "$vs\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe" | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $dumpbin) { throw "dumpbin não encontrado (instale as ferramentas C++ do Visual Studio)" }

function Get-Deps([string]$file) {
    & $dumpbin.FullName /nologo /dependents $file |
        Where-Object { $_ -match '^\s+(\S+\.dll)\s*$' } |
        ForEach-Object { $Matches[1] }
}

# fechamento de dependências dentro da pasta bin do OCCT/vcpkg
$bin = Join-Path $OcctRoot "bin"
$needed = @{}
$queue = New-Object System.Collections.Queue
$queue.Enqueue($exe)
# o XCAF só é usado pelo processo filho: o fechamento precisa partir dos dois executáveis
$queue.Enqueue($conv)
foreach ($a in $automacao) { $queue.Enqueue($a) }
while ($queue.Count -gt 0) {
    $f = $queue.Dequeue()
    foreach ($d in Get-Deps $f) {
        $p = Join-Path $bin $d
        if ((Test-Path $p) -and -not $needed.ContainsKey($d.ToLower())) {
            $needed[$d.ToLower()] = $p
            $queue.Enqueue($p)
        }
    }
}

if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item $exe $out
Copy-Item $conv $out
foreach ($a in $automacao) { Copy-Item $a $out }
Copy-Item (Join-Path $root "python\fenkra") (Join-Path $out "python\fenkra") -Recurse -Exclude "__pycache__"
foreach ($p in $needed.Values) { Copy-Item $p $out }

# runtime C++: sem ele o app nem abre em PC que não tenha o redistribuível instalado
foreach ($dll in 'vcruntime140.dll', 'vcruntime140_1.dll', 'msvcp140.dll') {
    $p = Join-Path $env:WINDIR "System32\$dll"
    if (-not (Test-Path $p)) { throw "Runtime C++ ausente: $p" }
    Copy-Item $p $out
}

# visualizador do .fkv (B365): a página solta, com three.js dentro, que abre um .fkv por arrastar ou
# escolher o arquivo. Gerada pelo próprio programa, pois o modelo da página tem marcas a preencher.
$visualizador = Join-Path $out "Fenkra Visualizador.html"
$p = Start-Process -FilePath (Join-Path $out "fenkra.exe") -ArgumentList @("--gerar-visualizador", "`"$visualizador`"") -Wait -PassThru
if ($p.ExitCode -ne 0 -or -not (Test-Path $visualizador)) { throw "não consegui gerar Fenkra Visualizador.html" }
$html = Get-Content $visualizador -Raw -Encoding utf8
if ($html -match '/\*@(THREE|PACOTE|PACOTE_B|META)@\*/' -or $html -notmatch 'THREE') { throw "Fenkra Visualizador.html saiu com marcas não preenchidas" }

# licenças obrigatórias (LGPL-2.1: texto, aviso do OCCT e oferta de código-fonte)
foreach ($doc in $licencas) {
    $p = Join-Path $root $doc
    if (-not (Test-Path $p)) { throw "arquivo de licença ausente: $doc" }
    Copy-Item $p $out
}

# documentos opcionais que acompanham o pacote
foreach ($doc in @("CHANGELOG.md", "packaging\LEIA-ME.txt", "packaging\TERCEIROS.txt")) {
    $p = Join-Path $root $doc
    if (Test-Path $p) { Copy-Item $p $out }
}

# versões exatas do OCCT e das bibliotecas do vcpkg (a oferta de código-fonte do NOTICE aponta para este arquivo)
$build = @("Versões das bibliotecas nativas deste pacote (vcpkg.spdx.json)", "")
foreach ($lib in @("opencascade", "freetype", "libpng", "zlib", "bzip2", "brotli")) {
    $spdx = Join-Path $OcctRoot "share\$lib\vcpkg.spdx.json"
    if (-not (Test-Path $spdx)) { throw "vcpkg.spdx.json não encontrado para ${lib}: $spdx" }
    $pkg = (Get-Content $spdx -Raw | ConvertFrom-Json).packages | Select-Object -First 1
    $build += "$($pkg.name) $($pkg.versionInfo)  origem: $($pkg.downloadLocation)"
}
$build | Set-Content -Path (Join-Path $out "NOTICE-BUILD.txt") -Encoding utf8

# o pacote precisa conter os três arquivos de licença
foreach ($doc in $licencas + "Fenkra Visualizador.html") {
    if (-not (Test-Path (Join-Path $out $doc))) { throw "pacote sem $doc" }
}

$zip = Join-Path $dist "$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path "$out\*" -DestinationPath $zip

# SHA256SUMS do zip
$hash = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLower()
"$hash  $name.zip" | Set-Content -Path (Join-Path $dist "SHA256SUMS") -Encoding ascii

$size = [math]::Round(((Get-ChildItem $out | Measure-Object Length -Sum).Sum / 1MB), 1)
$zsize = [math]::Round(((Get-Item $zip).Length / 1MB), 1)
Write-Host "DLLs incluídas: $($needed.Count)"
Write-Host "Pasta: $out ($size MB)"
Write-Host "Zip:   $zip ($zsize MB)"
Write-Host "SHA256: $hash"

# atualiza o atalho da área de trabalho
& (Join-Path $PSScriptRoot "atalho.ps1")
