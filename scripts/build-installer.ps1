param(
    [string]$Version,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Push-Location $Root

try {
    if (-not $Version) {
        $match = Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
        if (-not $match) {
            throw "Versi tidak ditemukan di Cargo.toml"
        }
        $Version = $match.Matches[0].Groups[1].Value
    }

    $isccCandidates = @(
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
    )
    $iscc = $isccCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $iscc) {
        throw "Inno Setup 6 tidak ditemukan. Pasang dari https://jrsoftware.org/isdl.php"
    }

    if (-not $SkipBuild) {
        & cargo build --release
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build --release gagal"
        }
    }

    New-Item -ItemType Directory -Force -Path "release" | Out-Null

    & $iscc "/DAppVersion=$Version" "installer\nodeui.iss"
    if ($LASTEXITCODE -ne 0) {
        throw "Inno Setup gagal membuat installer"
    }

    Copy-Item "target\release\nui.exe" "release\NodeUI-$Version-windows-x64.exe" -Force

    $staging = Join-Path $env:TEMP "nodeui-portable-$PID"
    if (Test-Path $staging) {
        Remove-Item -Recurse -Force $staging
    }
    New-Item -ItemType Directory -Force -Path $staging | Out-Null
    Copy-Item "target\release\nui.exe" $staging
    Copy-Item "README.md" $staging
    Copy-Item "LICENSE" $staging
    Copy-Item "examples" $staging -Recurse
    Compress-Archive -Path (Join-Path $staging "*") -DestinationPath "release\NodeUI-$Version-portable.zip" -Force
    Remove-Item -Recurse -Force $staging

    Write-Output ""
    Write-Output "Installer : release\NodeUI-$Version-setup.exe"
    Write-Output "Portable  : release\NodeUI-$Version-portable.zip"
    Write-Output "Binary    : release\NodeUI-$Version-windows-x64.exe"
}
finally {
    Pop-Location
}
