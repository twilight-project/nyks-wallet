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

    # Filter to stable releases whose tag ends with '-relayer-cli' and which have
    # both the platform binary and its .sha256 uploaded. Requiring the checksum
    # ensures we skip releases whose workflow is still building — the release tag
    # exists the moment it is created, but assets are uploaded 30–40 minutes later
    # when the matrix build finishes. Without this check we'd error out with
    # "could not find a relayer-cli asset" during that window. The workflow
    # uploads the binary first and the checksum second, so a present checksum
    # means the binary is fully uploaded too.
    $RelayerReleases = $Releases | Where-Object {
        -not $_.draft -and -not $_.prerelease -and $_.tag_name -match '-relayer-cli$' -and
        ($_.assets | Where-Object { $_.name.EndsWith($PlatformSuffix) -and -not $_.name.EndsWith(".sha256") }) -and
        ($_.assets | Where-Object { $_.name.EndsWith("$PlatformSuffix.sha256") })
    }

    if (-not $RelayerReleases) {
        Write-Host "Error: no published relayer-cli release with assets for platform '$PlatformSuffix'. A newer release may still be building." -ForegroundColor Red
        return
    }

    $Release = $RelayerReleases | Sort-Object -Property @{
        Expression = {
            $v = $_.tag_name -replace '^v', '' -replace '-relayer-cli$', ''
            try { [version]$v } catch { [version]"0.0.0" }
        }
    } -Descending | Select-Object -First 1

    $Tag = $Release.tag_name

    $Asset = $Release.assets | Where-Object {
        $_.name.EndsWith($PlatformSuffix) -and -not $_.name.EndsWith(".sha256")
    } | Select-Object -First 1

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

        # Fetch raw checksum file content. The .sha256 asset is served as
        # application/octet-stream, so Invoke-WebRequest's $Response.Content
        # is byte[] under PowerShell 5.1 and string under PowerShell 7+.
        # Casting [string] on a byte[] yields space-separated decimals (e.g.
        # "52 49 ..." for "41..."), so decode bytes explicitly when needed.
        $Expected = $null
        try {
            $ChecksumResponse = Invoke-WebRequest `
                -Uri $ChecksumAsset.browser_download_url `
                -Headers @{ "User-Agent" = "relayer-cli-installer" } `
                -UseBasicParsing
            if ($ChecksumResponse.Content -is [byte[]]) {
                $ChecksumText = [System.Text.Encoding]::UTF8.GetString($ChecksumResponse.Content)
            } else {
                $ChecksumText = [string]$ChecksumResponse.Content
            }
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
