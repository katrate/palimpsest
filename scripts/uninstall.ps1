<#
.SYNOPSIS
    Uninstalls the palin CLI and optionally removes user data.
.DESCRIPTION
    Removes the palin binary from %USERPROFILE%\.palin\bin and
    cleans up the user PATH entry. User snapshot data is kept
    unless -RemoveData is specified.
.EXAMPLE
    irm https://raw.githubusercontent.com/katrate/palimpsest/main/scripts/uninstall.ps1 | iex
.EXAMPLE
    irm https://raw.githubusercontent.com/katrate/palimpsest/main/scripts/uninstall.ps1 | iex -RemoveData
#>

param(
  [switch]$RemoveData
)

$Binary = "palin"
$InstallDir = "${env:USERPROFILE}\.palin"
$BinDir = "${InstallDir}\bin"
$BinaryPath = "${BinDir}\${Binary}.exe"
$DataDir = "${env:USERPROFILE}\palimpsest"

Write-Host "==> Uninstalling palin..." -ForegroundColor Yellow

# Remove the binary and install directory
if (Test-Path $BinaryPath) {
  Remove-Item -Path $BinaryPath -Force
  Write-Host "  Removed ${BinaryPath}" -ForegroundColor Green
} else {
  Write-Host "  Binary not found at ${BinaryPath}" -ForegroundColor DarkYellow
}

# Remove the .palin directory if empty
if (Test-Path $InstallDir) {
  Remove-Item -Path $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
  Write-Host "  Removed ${InstallDir}" -ForegroundColor Green
}

# Clean up PATH entry
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -like "*${BinDir}*") {
  $NewPath = ($UserPath -split ";" | Where-Object { $_ -ne $BinDir }) -join ";"
  [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
  $env:PATH = ($env:PATH -split ";" | Where-Object { $_ -ne $BinDir }) -join ";"
  Write-Host "  Removed ${BinDir} from your user PATH." -ForegroundColor Green
}

Write-Host "OK palin uninstalled." -ForegroundColor Green

# Handle data directory
if (Test-Path $DataDir) {
  if ($RemoveData) {
    Remove-Item -Path $DataDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "  Removed snapshot data at ${DataDir}" -ForegroundColor Yellow
  } else {
    Write-Host ""
    Write-Host "  NOTE: Your palimpsest data (snapshots, history) is still at:" -ForegroundColor Cyan
    Write-Host "    ${DataDir}" -ForegroundColor Cyan
    Write-Host "  To remove it too, re-run with:  iex ((irm ...) + ' -RemoveData')" -ForegroundColor Cyan
  }
}
