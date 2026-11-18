param([switch]$SkipTests)
$ErrorActionPreference='Stop'
$savedFlags=$env:CARGO_ENCODED_RUSTFLAGS
$savedLib=$env:LIB
Push-Location $PSScriptRoot
try {
    if(-not $env:LIB) {
        $vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        if(Test-Path -LiteralPath $vswhere) {
            $vs=& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
            if($vs) {
                $tool=Get-ChildItem -LiteralPath (Join-Path $vs 'VC\Tools\MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1
                $vcLib=@((Join-Path $tool.FullName 'lib\x64'),(Join-Path $tool.FullName 'lib\onecore\x64')) | Where-Object {Test-Path -LiteralPath $_} | Select-Object -First 1
                $kitRoot=Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Lib'
                $kit=Get-ChildItem -LiteralPath $kitRoot -Directory | Where-Object {(Test-Path -LiteralPath (Join-Path $_.FullName 'um\x64')) -and (Test-Path -LiteralPath (Join-Path $_.FullName 'ucrt\x64'))} | Sort-Object Name -Descending | Select-Object -First 1
                if($vcLib -and $kit){$env:LIB=@($vcLib,(Join-Path $kit.FullName 'um\x64'),(Join-Path $kit.FullName 'ucrt\x64')) -join ';'}
            }
        }
    }
    if(-not $SkipTests) {
        cargo test --locked --workspace
        if($LASTEXITCODE -ne 0){throw 'Tests failed.'}
    }
    $flags=@('-C','control-flow-guard=yes',"--remap-path-prefix=$PSScriptRoot=Azr")
    if($env:USERPROFILE){$flags+="--remap-path-prefix=$env:USERPROFILE=USER"}
    $env:CARGO_ENCODED_RUSTFLAGS=$flags -join [char]31
    cargo build --locked --release --workspace
    if($LASTEXITCODE -ne 0){throw 'Release build failed.'}
    $version=([regex]::Match((Get-Content -LiteralPath 'Cargo.toml' -Raw),'(?m)^version = "([^"]+)"')).Groups[1].Value
    if(-not $version){throw 'Workspace version missing.'}
    $run=Join-Path $PSScriptRoot ('dist\build-'+[guid]::NewGuid().ToString('N'))
    $stage=Join-Path $run ('Azr-'+$version)
    New-Item -ItemType Directory -Force $stage | Out-Null
    Copy-Item -LiteralPath 'target\release\azr.exe','target\release\azr-guard.exe','README.md','README.vi.md','LICENSE.md' -Destination $stage
    Copy-Item -LiteralPath 'assets\fonts','assets\brand','legal','docs' -Destination $stage -Recurse
    foreach($readme in @('README.md','README.vi.md')) {
        $readmePath=Join-Path $stage $readme
        (Get-Content -LiteralPath $readmePath -Raw -Encoding UTF8).Replace('assets/brand/','brand/') | Set-Content -LiteralPath $readmePath -Encoding UTF8
    }
    Get-ChildItem -LiteralPath $stage -File -Recurse | ForEach-Object {
        [pscustomobject]@{File=$_.FullName.Substring($stage.Length+1);SHA256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash}
    } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stage 'SHA256.json') -Encoding UTF8
    $archive=Join-Path $run ('Azr-'+$version+'-portable.zip')
    Compress-Archive -LiteralPath $stage -DestinationPath $archive
    Write-Output ('Portable package: '+$archive)
} finally {
    $env:CARGO_ENCODED_RUSTFLAGS=$savedFlags
    $env:LIB=$savedLib
    Pop-Location
}
