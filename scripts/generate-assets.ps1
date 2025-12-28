<#
Generate PNG exports and favicon from SVG assets (PowerShell)
Requires: ImageMagick (convert) or rsvg-convert available in PATH
Usage: powershell -File scripts\generate-assets.ps1
#>
Param()

Set-StrictMode -Version Latest

$RepoRoot = Split-Path -Parent $MyInvocation.MyCommand.Definition
$Assets = Join-Path $RepoRoot 'assets'
$Out = Join-Path $Assets 'exports'
if (-not (Test-Path $Out)) { New-Item -ItemType Directory -Path $Out | Out-Null }

$svgWord = Join-Path $Assets 'logo-wordmark.svg'
$svgIcon = Join-Path $Assets 'logo-icon.svg'

Write-Host "Generating PNG exports into $Out"

function Invoke-Convert($inFile, $outFile, $size) {
    if (Get-Command rsvg-convert -ErrorAction SilentlyContinue) {
        rsvg-convert -w $size.Split('x')[0] -h $size.Split('x')[1] $inFile -o $outFile
    } elseif (Get-Command convert -ErrorAction SilentlyContinue) {
        convert $inFile -background none -resize $size $outFile
    } else {
        throw "No SVG renderer found (rsvg-convert or ImageMagick 'convert')"
    }
}

try {
    Invoke-Convert $svgWord (Join-Path $Out 'logo-wordmark-360x68.png') '360x68'
    Invoke-Convert $svgWord (Join-Path $Out 'logo-wordmark-180x34.png') '180x34'
    Invoke-Convert $svgIcon (Join-Path $Out 'logo-icon-256.png') '256x256'
    Invoke-Convert $svgIcon (Join-Path $Out 'logo-icon-128.png') '128x128'
    Invoke-Convert $svgIcon (Join-Path $Out 'logo-icon-64.png') '64x64'
    Invoke-Convert $svgIcon (Join-Path $Out 'logo-icon-32.png') '32x32'

    if (Get-Command convert -ErrorAction SilentlyContinue) {
        convert (Join-Path $Out 'logo-icon-32.png') -define icon:auto-resize=16,32 (Join-Path $Assets 'favicon.ico')
        Write-Host "Created favicon.ico"
    } else {
        Write-Host "ImageMagick 'convert' not available; favicon not created."
    }
    Write-Host "Exports written to $Out"
} catch {
    Write-Error $_.Exception.Message
    exit 1
}
