# Check for Cargo
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Error: Rust/Cargo is not installed."
    exit 1
}

# Build release
Write-Host "Building vex-controller..."
cargo build --release

if (-not $?) {
    Write-Host "Build failed."
    exit 1
}

$targetDir = "$PSScriptRoot\target\release"
$exePath = "$targetDir\vex-controller.exe"

if (-not (Test-Path $exePath)) {
    Write-Host "Error: Executable not found at $exePath"
    exit 1
}

# Create Startup Shortcut
$startupPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup"
$shortcutPath = "$startupPath\VexController.lnk"
$wshShell = New-Object -ComObject WScript.Shell
$shortcut = $wshShell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $exePath
$shortcut.Arguments = "--daemon"
$shortcut.WorkingDirectory = $targetDir
$shortcut.Description = "VEX IQ Gen 2 Controller Driver"
$shortcut.Save()

Write-Host "Installation complete! The driver will start automatically when you log in."
Write-Host "To start it immediately, run: $exePath --daemon"
