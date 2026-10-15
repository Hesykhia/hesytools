# Makes a Dev tab access code for someone.
#
#   .\scripts\devcode.ps1 friend1
#
# Codes are signed with $HOME\.tauri\hesytools-dev.key (never commit it).
param([Parameter(Mandatory)] [string]$Name)
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)
cargo run -q -p hesytools-next --example devcode -- $Name
