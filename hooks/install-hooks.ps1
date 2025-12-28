param(
    [switch]$Enable,
    [switch]$Disable
)

# Installs or removes hooks from .git/hooks
try {
    $repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Definition
    $gitHooks = Join-Path $repoRoot ".git/hooks"
    if (-not (Test-Path $gitHooks)) {
        Write-Error ".git/hooks not found. Run this script from the repository root."; exit 1
    }

    $srcShell = Join-Path $repoRoot "hooks/pre-push"
    $srcPS = Join-Path $repoRoot "hooks/pre-push.ps1"
    $dstShell = Join-Path $gitHooks "pre-push"
    $dstPS = Join-Path $gitHooks "pre-push.ps1"

    if ($Enable) {
        Copy-Item -Path $srcShell -Destination $dstShell -Force
        Copy-Item -Path $srcPS -Destination $dstPS -Force
        # Ensure shell script is usable on environments that respect file modes
        try { icacls $dstShell /grant Everyone:RX | Out-Null } catch { }
        Write-Host "Installed hooks into .git/hooks (pre-push)." -ForegroundColor Green
        Write-Host "To disable: powershell -File hooks/install-hooks.ps1 -Disable" -ForegroundColor Yellow
        exit 0
    }

    if ($Disable) {
        if (Test-Path $dstShell) { Remove-Item -Path $dstShell -Force }
        if (Test-Path $dstPS) { Remove-Item -Path $dstPS -Force }
        Write-Host "Removed pre-push hooks from .git/hooks." -ForegroundColor Green
        exit 0
    }

    Write-Host "Usage: powershell -File hooks/install-hooks.ps1 -Enable`n       powershell -File hooks/install-hooks.ps1 -Disable"
    exit 1
} catch {
    Write-Error $_.Exception.Message
    exit 1
}
