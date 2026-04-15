use std::io::Write;

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/twilight-project/nyks-wallet/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(serde::Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

fn artifact_name() -> Result<&'static str, String> {
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Ok("nyks-wallet-linux-amd64")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        Ok("nyks-wallet-linux-arm64")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Ok("nyks-wallet-macos-arm64")
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        Ok("nyks-wallet-windows-amd64.exe")
    } else {
        Err(format!(
            "Unsupported platform: {} / {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    }
}

fn parse_version(s: &str) -> Result<(u32, u32, u32), String> {
    let stripped = s
        .strip_prefix('v')
        .unwrap_or(s)
        .trim_end_matches("-relayer-cli");
    let parts: Vec<&str> = stripped.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("Invalid version: {s}"));
    }
    Ok((
        parts[0]
            .parse()
            .map_err(|_| format!("Bad major: {}", parts[0]))?,
        parts[1]
            .parse()
            .map_err(|_| format!("Bad minor: {}", parts[1]))?,
        parts[2]
            .parse()
            .map_err(|_| format!("Bad patch: {}", parts[2]))?,
    ))
}

pub(crate) async fn handle_update(check_only: bool) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .user_agent("relayer-cli-updater")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    println!("Checking for updates...");

    let release: GithubRelease = client
        .get(GITHUB_API_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse release JSON: {e}"))?;

    let remote = parse_version(&release.tag_name)?;
    let local = parse_version(CURRENT_VERSION)?;

    if remote <= local {
        println!("Already up to date (v{CURRENT_VERSION}).");
        return Ok(());
    }

    let remote_display = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .trim_end_matches("-relayer-cli");

    if check_only {
        println!("Update available: v{CURRENT_VERSION} -> v{remote_display}");
        return Ok(());
    }

    let expected_name = artifact_name()?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == expected_name)
        .ok_or_else(|| format!("No asset found for this platform: {expected_name}"))?;

    println!("Updating v{CURRENT_VERSION} -> v{remote_display} ({expected_name})...");

    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download: {e}"))?;

    println!("Downloaded {} bytes.", bytes.len());

    let tmp_path = std::env::temp_dir().join(format!("relayer-cli-update-{}", std::process::id()));

    {
        let mut file = std::fs::File::create(&tmp_path)
            .map_err(|e| format!("Failed to create temp file: {e}"))?;
        file.write_all(&bytes)
            .map_err(|e| format!("Failed to write temp file: {e}"))?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set permissions: {e}"))?;
    }

    self_replace::self_replace(&tmp_path).map_err(|e| format!("Failed to replace binary: {e}"))?;

    let _ = std::fs::remove_file(&tmp_path);

    println!("Updated to v{remote_display}. Restart the CLI to use the new version.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // parse_version
    // -----------------------------------------------------------------------

    #[test]
    fn parse_version_full_tag() {
        assert_eq!(parse_version("v0.1.2-relayer-cli"), Ok((0, 1, 2)));
    }

    #[test]
    fn parse_version_with_v_prefix_only() {
        assert_eq!(parse_version("v1.2.3"), Ok((1, 2, 3)));
    }

    #[test]
    fn parse_version_bare() {
        assert_eq!(parse_version("0.1.2"), Ok((0, 1, 2)));
    }

    #[test]
    fn parse_version_large_numbers() {
        assert_eq!(
            parse_version("v10.200.3000-relayer-cli"),
            Ok((10, 200, 3000))
        );
    }

    #[test]
    fn parse_version_too_few_parts() {
        assert!(parse_version("v1.2").is_err());
    }

    #[test]
    fn parse_version_too_many_parts() {
        assert!(parse_version("v1.2.3.4").is_err());
    }

    #[test]
    fn parse_version_non_numeric() {
        assert!(parse_version("v1.two.3").is_err());
    }

    #[test]
    fn parse_version_empty() {
        assert!(parse_version("").is_err());
    }

    #[test]
    fn parse_version_just_suffix() {
        assert!(parse_version("-relayer-cli").is_err());
    }

    // -----------------------------------------------------------------------
    // artifact_name
    // -----------------------------------------------------------------------

    #[test]
    fn artifact_name_returns_ok_on_supported_platform() {
        // We're compiling on a supported platform (macOS ARM64, Linux x86_64, or Windows x86_64),
        // so this should succeed. If CI adds an unsupported target, this test catches it.
        let result = artifact_name();
        assert!(
            result.is_ok(),
            "artifact_name() failed on this platform: {result:?}"
        );

        let name = result.unwrap();
        println!("name: {name}");
        assert!(
            name == "nyks-wallet-linux-amd64"
                || name == "nyks-wallet-linux-arm64"
                || name == "nyks-wallet-macos-arm64"
                || name == "nyks-wallet-windows-amd64.exe",
            "unexpected artifact name: {name}"
        );
    }

    #[test]
    fn artifact_name_matches_current_platform() {
        let name = artifact_name().unwrap();
        if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
            assert_eq!(name, "nyks-wallet-macos-arm64");
        } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
            assert_eq!(name, "nyks-wallet-linux-amd64");
        } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
            assert_eq!(name, "nyks-wallet-linux-arm64");
        } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
            assert_eq!(name, "nyks-wallet-windows-amd64.exe");
        }
    }

    // -----------------------------------------------------------------------
    // version comparison ordering
    // -----------------------------------------------------------------------

    #[test]
    fn version_ordering_newer_patch() {
        let old = parse_version("v0.1.1-relayer-cli").unwrap();
        let new = parse_version("v0.1.2-relayer-cli").unwrap();
        assert!(new > old);
    }

    #[test]
    fn version_ordering_newer_minor() {
        let old = parse_version("v0.1.9").unwrap();
        let new = parse_version("v0.2.0").unwrap();
        assert!(new > old);
    }

    #[test]
    fn version_ordering_newer_major() {
        let old = parse_version("v0.99.99").unwrap();
        let new = parse_version("v1.0.0").unwrap();
        assert!(new > old);
    }

    #[test]
    fn version_ordering_equal() {
        let a = parse_version("v0.1.2-relayer-cli").unwrap();
        let b = parse_version("0.1.2").unwrap();
        assert_eq!(a, b);
    }

    // -----------------------------------------------------------------------
    // CURRENT_VERSION is parseable
    // -----------------------------------------------------------------------

    #[test]
    fn current_version_is_valid() {
        let result = parse_version(CURRENT_VERSION);
        assert!(
            result.is_ok(),
            "CARGO_PKG_VERSION is not parseable: {result:?}"
        );
    }
}
