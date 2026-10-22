# Gera o pacote de release do SolidCadCraft para Windows x64.
#
#   powershell -ExecutionPolicy Bypass -File scripts\release.ps1
#
# Resultado: dist\SolidCadCraft-<versão>-windows-x64\ e o .zip correspondente.
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
$name = "SolidCadCraft-$version-windows-x64"
$dist = Join-Path $root "dist"
$out = Join-Path $dist $name
Write-Host "== SolidCadCraft $version =="

if (-not $SkipBuild) {
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    cargo build --release -p scc-app
    if ($LASTEXITCODE -ne 0) { throw "falha na compilação" }
}
$exe = Join-Path $root "target\release\solidcadcraft.exe"
if (-not (Test-Path $exe)) { throw "executável não encontrado: $exe" }

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
foreach ($p in $needed.Values) { Copy-Item $p $out }

# documentos que acompanham o pacote
Copy-Item (Join-Path $root "CHANGELOG.md") $out
Copy-Item (Join-Path $root "packaging\LEIA-ME.txt") $out
Copy-Item (Join-Path $root "packaging\TERCEIROS.txt") $out

$zip = Join-Path $dist "$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path "$out\*" -DestinationPath $zip

$size = [math]::Round(((Get-ChildItem $out | Measure-Object Length -Sum).Sum / 1MB), 1)
$zsize = [math]::Round(((Get-Item $zip).Length / 1MB), 1)
Write-Host "DLLs incluídas: $($needed.Count)"
Write-Host "Pasta: $out ($size MB)"
Write-Host "Zip:   $zip ($zsize MB)"
