# One-line installer for the Candy dishwasher widget (Windows, x86_64):
#
#   irm https://raw.githubusercontent.com/cartudev/hon-dishwasher/main/packaging/bootstrap.ps1 | iex
#
# Downloads the latest release zip to %LOCALAPPDATA%\hon-dishwasher, creates the
# Python environment (pyhOn) and puts a launcher in the Startup folder.
$ErrorActionPreference = "Stop"

$repo = "https://github.com/cartudev/hon-dishwasher"
$zipUrl = "$repo/releases/latest/download/hon-dishwasher-windows-x86_64.zip"
$installDir = Join-Path $env:LOCALAPPDATA "hon-dishwasher"

if (-not [Environment]::Is64BitOperatingSystem) {
    Write-Error "This installer targets x86_64 Windows."
    exit 1
}

# Python is needed for the backend environment (pyhOn).
$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) {
    Write-Host "Python was not found. Install it with:" -ForegroundColor Yellow
    Write-Host "    winget install Python.Python.3.12"
    Write-Host "and run this installer again."
    exit 1
}

Write-Host "==> Downloading the latest release"
$tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ([guid]::NewGuid().ToString()))
try {
    Invoke-WebRequest -Uri $zipUrl -OutFile (Join-Path $tmp "app.zip") -UseBasicParsing

    Write-Host "==> Unpacking to $installDir"
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
    Expand-Archive -Path (Join-Path $tmp "app.zip") -DestinationPath $tmp -Force
    # The zip wraps a single directory; move its contents over.
    Copy-Item -Path (Join-Path $tmp "hon-dishwasher-windows\*") -Destination $installDir -Recurse -Force

    Set-Location $installDir

    Write-Host "==> Installing the Python environment (pyhOn)"
    if (-not (Test-Path ".venv\Scripts\python.exe")) {
        python -m venv .venv
        & .venv\Scripts\python.exe -m pip install --quiet -r requirements.txt
    }

    Write-Host "==> Adding the launcher to the Startup folder"
    $startup = [Environment]::GetFolderPath("Startup")
    $shortcutPath = Join-Path $startup "Candy dishwasher widget.lnk"
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = Join-Path $installDir "start-widget.bat"
    $shortcut.WorkingDirectory = $installDir
    $shortcut.WindowStyle = 7  # minimized
    $shortcut.Save()

    Write-Host ""
    Write-Host "==> Ready. Next steps:" -ForegroundColor Green
    Write-Host "    1. Run start-widget.bat (or just log out and back in; it autostarts)."
    Write-Host "    2. Open the tray icon's Settings and fill in your hOn account."
    Write-Host "    3. The first query takes ~30 s; after that the widget lists from its cache."
    Write-Host "    Re-run this installer to update to the latest release."
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
