# Builds a signed installer and publishes it as a GitHub release, which is
# what the in-app updater picks up.
#
#   .\scripts\release.ps1 -Version 5.1.0 -Notes "- Thing one`n- Thing two"
#   .\scripts\release.ps1 -Version 5.1.0 -NotesFile notes.md
#
# Needs: the signing key at $HOME\.tauri\hesytools.key (keep a backup, without
# it existing installs can't be updated), and gh logged in as Hesykhia.

param(
    [Parameter(Mandatory)] [string]$Version,
    [string]$Notes,
    [string]$NotesFile,
    [switch]$Draft
)
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

$key = "$HOME\.tauri\hesytools.key"
if (-not (Test-Path $key)) { throw "Signing key not found at $key" }
if ($NotesFile) { $Notes = Get-Content $NotesFile -Raw }
if (-not $Notes) { throw "Give release notes with -Notes or -NotesFile" }
$tag = "v$Version"
$utf8 = New-Object System.Text.UTF8Encoding($false)

# Bump the version (first match only) in all three places, keeping formatting.
function Set-FirstMatch($path, $pattern, $replacement) {
    $text = [IO.File]::ReadAllText((Resolve-Path $path))
    $new = ([regex]$pattern).Replace($text, $replacement, 1)
    if ($new -eq $text -and $text -notmatch [regex]::Escape($replacement)) { throw "No version found in $path" }
    [IO.File]::WriteAllText((Resolve-Path $path), $new, $utf8)
}
Set-FirstMatch src-tauri\tauri.conf.json '"version":\s*"[^"]+"' "`"version`": `"$Version`""
Set-FirstMatch package.json '"version":\s*"[^"]+"' "`"version`": `"$Version`""
Set-FirstMatch src-tauri\Cargo.toml '(?m)^version = "[^"]+"' "version = `"$Version`""

# A running copy from target\release would block the build.
Get-Process hesytools -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -like "*\target\release\*" } | Stop-Process -Force

$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
pnpm tauri build
if ($LASTEXITCODE -ne 0) { throw "Build failed" }

$bundle = "target\release\bundle\nsis"
$setup = "$bundle\hesytools_${Version}_x64-setup.exe"
$sig = Get-Content "$setup.sig" -Raw

# The manifest the updater reads from releases/latest/download/latest.json
$manifest = [ordered]@{
    version   = $Version
    notes     = $Notes.Trim()
    pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    platforms = @{
        "windows-x86_64" = @{
            signature = $sig.Trim()
            url       = "https://github.com/Hesykhia/hesytools/releases/download/$tag/hesytools_${Version}_x64-setup.exe"
        }
    }
}
[IO.File]::WriteAllText((Join-Path (Resolve-Path $bundle) "latest.json"), ($manifest | ConvertTo-Json -Depth 5), $utf8)

git add src-tauri\tauri.conf.json package.json src-tauri\Cargo.toml Cargo.lock
git commit -m "v$Version"
git -c credential.helper= -c "credential.helper=!gh auth git-credential" push

$notesPath = Join-Path (Resolve-Path $bundle) "notes.md"
[IO.File]::WriteAllText($notesPath, $Notes, $utf8)
$ghArgs = @($tag, $setup, "$bundle\latest.json", "-R", "Hesykhia/hesytools", "--title", "hesytools $tag", "--notes-file", $notesPath)
if ($Draft) { $ghArgs += "--draft" }
gh release create @ghArgs
Write-Host "Released $tag"
