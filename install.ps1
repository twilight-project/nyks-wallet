<#
.SYNOPSIS
    Install the latest relayer-cli binary for Windows.
.DESCRIPTION
    Downloads the latest relayer-cli release from GitHub and places it in the current directory.
    Verifies the download against a SHA256 checksum if available. Filters releases by the
    '-relayer-cli' tag suffix so unrelated tags (validator-wallet, relayer-deployer, etc.)
    are ignored.
.EXAMPLE
    .\install.ps1
.EXAMPLE
    irm https://raw.githubusercontent.com/twilight-project/nyks-wallet/main/install.ps1 | iex
#>

& {
    $ErrorActionPreference = "Stop"

    $Repo = "twilight-project/nyks-wallet"
    $ApiUrl = "https://api.github.com/repos/$Repo/releases?per_page=100"
    $PlatformSuffix = "_windows_amd64.exe"
    $BinaryName = "relayer-cli.exe"

    Write-Host "Fetching releases from $Repo..."

    try {
        $Releases = Invoke-RestMethod -Uri $ApiUrl -Headers @{ "User-Agent" = "relayer-cli-installer" }
    } catch {
        Write-Host "Error: Failed to fetch releases from GitHub API: $_" -ForegroundColor Red
        return
    }

    # Filter to stable releases whose tag ends with '-relayer-cli' and pick the one
    # with the highest semver. GitHub's default ordering is creation date, which we
    # don't fully trust, so sort by parsed version instead.
    $RelayerReleases = $Releases | Where-Object {
        -not $_.draft -and -not $_.prerelease -and $_.tag_name -match '-relayer-cli$'
    }

    if (-not $RelayerReleases) {
        Write-Host "Error: no release found with tag suffix '-relayer-cli'" -ForegroundColor Red
        return
    }

    $Release = $RelayerReleases | Sort-Object -Property @{
        Expression = {
            $v = $_.tag_name -replace '^v', '' -replace '-relayer-cli$', ''
            try { [version]$v } catch { [version]"0.0.0" }
        }
    } -Descending | Select-Object -First 1

    $Tag = $Release.tag_name

    # Binary asset: ends with platform suffix, not a .sha256 file.
    $Asset = $Release.assets | Where-Object {
        $_.name.EndsWith($PlatformSuffix) -and -not $_.name.EndsWith(".sha256")
    } | Select-Object -First 1

    if (-not $Asset) {
        Write-Host "Error: Could not find a relayer-cli asset for platform '$PlatformSuffix' in release $Tag" -ForegroundColor Red
        return
    }

    $DownloadUrl = $Asset.browser_download_url

    Write-Host "Downloading $Tag ($($Asset.name))..."

    try {
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $BinaryName -UseBasicParsing
    } catch {
        Write-Host "Error: Download failed: $_" -ForegroundColor Red
        return
    }

    # --- Verify checksum ---

    $ChecksumAsset = $Release.assets | Where-Object {
        $_.name -eq "$($Asset.name).sha256"
    } | Select-Object -First 1

    if ($ChecksumAsset) {
        Write-Host "Verifying checksum..."

        # Fetch raw checksum file content. Use Invoke-WebRequest rather than
        # Invoke-RestMethod because the .sha256 asset is served as
        # application/octet-stream, which Invoke-RestMethod may decode into a
        # byte[] instead of a string.
        $Expected = $null
        try {
            $ChecksumResponse = Invoke-WebRequest `
                -Uri $ChecksumAsset.browser_download_url `
                -Headers @{ "User-Agent" = "relayer-cli-installer" } `
                -UseBasicParsing
            $ChecksumText = [string]$ChecksumResponse.Content
            $Expected = ($ChecksumText -split '\s+')[0].ToUpper()
        } catch {
            Write-Host "Warning: could not download checksum, skipping verification: $_" -ForegroundColor Yellow
        }

        if ($Expected) {
            $Actual = (Get-FileHash $BinaryName -Algorithm SHA256).Hash

            if ($Actual -ne $Expected) {
                Write-Host "Error: checksum mismatch!" -ForegroundColor Red
                Write-Host "  Expected: $Expected" -ForegroundColor Red
                Write-Host "  Actual:   $Actual" -ForegroundColor Red

                # Best-effort cleanup. If the file can't be removed (locked,
                # antivirus, permissions), warn but still abort — we must NOT
                # fall through to the success path with a bad binary on disk.
                try {
                    Remove-Item $BinaryName -Force
                } catch {
                    Write-Host "Warning: failed to remove corrupt binary at .\$BinaryName — delete it manually." -ForegroundColor Yellow
                }
                return
            }

            Write-Host "Checksum verified."
        }
    } else {
        Write-Host "Note: no checksum file found in release, skipping verification"
    }

    Write-Host ""
    Write-Host "Installed $BinaryName $Tag to .\$BinaryName"
    Write-Host "Run .\$BinaryName --help to get started."
}
