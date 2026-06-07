<#
.SYNOPSIS
    Installs the palin CLI from the latest GitHub release.
.DESCRIPTION
    Detects the platform (Windows x64/ARM64), downloads the matching
    archive from GitHub Releases, and installs the binary to a location
    in PATH.
.EXAMPLE
    irm https://raw.githubusercontent.com/katrate/palimpsest/main/scripts/install.ps1 | iex
#>

$Repo = "katrate/palimpsest"
$Package = "palimpsest"
$Binary = "palin"

# ─── Detect platform ────────────────────────────────────────────────────
$Arch = switch ($env:PROCESSOR_ARCHITECTURE) {
  "AMD64" { "x86_64" }
  "ARM64" { "aarch64" }
  "x86"   { "x86_64" } # Assume 32-bit PowerShell on 64-bit Windows
  default { throw "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
}

$Target = "${Arch}-pc-windows-msvc"
$ArchiveName = "${Package}-${Target}.zip"
$ExtractedDir = "${Package}-${Target}"

# GitHub's permanent redirect — no API token or rate limit worries
$DownloadUrl = "https://github.com/${Repo}/releases/latest/download/${ArchiveName}"

# ─── Determine install directory ────────────────────────────────────────
$InstallDir = "${env:USERPROFILE}\.palin\bin"
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
$UserPaths = $UserPath -split ";"
$NeedsPathAdd = $UserPaths -notcontains $InstallDir

# ─── Download & extract ─────────────────────────────────────────────────
$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "palin-install-$([System.Guid]::NewGuid())"
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

try {
  $ArchivePath = Join-Path $TmpDir $ArchiveName
  Write-Host "✦ Downloading ${ArchiveName}..." -ForegroundColor Yellow
  Invoke-WebRequest -Uri $DownloadUrl -OutFile $ArchivePath -UseBasicParsing

  Write-Host "✦ Extracting..." -ForegroundColor Yellow
  Expand-Archive -Path $ArchivePath -DestinationPath $TmpDir -Force

  $ExtractedBinary = Join-Path $TmpDir "${ExtractedDir}\${Binary}.exe"

  Write-Host "✦ Installing to ${InstallDir}\${Binary}.exe ..." -ForegroundColor Yellow
  New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
  Copy-Item -Path $ExtractedBinary -Destination (Join-Path $InstallDir "${Binary}.exe") -Force

  # ─── Add to PATH if needed ───────────────────────────────────────────
  if ($NeedsPathAdd) {
    $NewPath = $UserPath.TrimEnd(";") + ";" + $InstallDir
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    # Update current session too
    $env:PATH = $env:PATH + ";" + $InstallDir
    Write-Host "  ⚠  Added ${InstallDir} to your user PATH." -ForegroundColor Cyan
    Write-Host "     You may need to restart your terminal for it to take effect." -ForegroundColor Cyan
  }

  Write-Host "✓ palin installed successfully at ${InstallDir}\${Binary}.exe" -ForegroundColor Green
  Write-Host ""
  Write-Host "  Run 'palin --help' to get started" -ForegroundColor Green

} finally {
  # Cleanup
  Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
}
