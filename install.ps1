<#
.SYNOPSIS
    Install the latest relayer-cli binary for Windows.
.DESCRIPTION
    Downloads the latest relayer-cli release from GitHub and places it in the current directory.
.EXAMPLE
    .\install.ps1
.EXAMPLE
    irm https://raw.githubusercontent.com/twilight-project/nyks-wallet/main/install.ps1 | iex
#>

& {
    $ErrorActionPreference = "Stop"

    $Repo = "twilight-project/nyks-wallet"
    $ApiUrl = "https://api.github.com/repos/$Repo/releases/latest"
    $Artifact = "nyks-wallet-windows-amd64.exe"
    $BinaryName = "relayer-cli.exe"

    Write-Host "Fetching latest release from $Repo..."

    try {
        $Release = Invoke-RestMethod -Uri $ApiUrl -Headers @{ "User-Agent" = "relayer-cli-installer" }
    } catch {
        Write-Host "Error: Failed to fetch release info from GitHub API: $_" -ForegroundColor Red
        return
    }

    $Asset = $Release.assets | Where-Object { $_.name -eq $Artifact }

    if (-not $Asset) {
        Write-Host "Error: Could not find asset '$Artifact' in latest release" -ForegroundColor Red
        return
    }

    $Tag = $Release.tag_name
    $DownloadUrl = $Asset.browser_download_url

    Write-Host "Downloading $Tag ($Artifact)..."

    try {
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $BinaryName -UseBasicParsing
    } catch {
        Write-Host "Error: Download failed: $_" -ForegroundColor Red
        return
    }

    Write-Host ""
    Write-Host "Installed $BinaryName $Tag to .\$BinaryName"
    Write-Host "Run .\$BinaryName --help to get started."
}
