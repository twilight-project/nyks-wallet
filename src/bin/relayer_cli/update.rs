use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

/// Removes a temp file on drop, so orphaned update artifacts aren't left
/// behind in `$TMPDIR` when `handle_update` returns early via `?`.
struct TempFileGuard(PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/twilight-project/nyks-wallet/releases?per_page=100";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASE_TAG_SUFFIX: &str = "-relayer-cli";

#[derive(serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    assets: Vec<GithubAsset>,
}

#[derive(serde::Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

fn pick_latest_relayer_release(mut releases: Vec<GithubRelease>) -> Option<GithubRelease> {
    releases.retain(|r| {
        !r.draft
            && !r.prerelease
            && r.tag_name.ends_with(RELEASE_TAG_SUFFIX)
            && parse_version(&r.tag_name).is_ok()
    });
    releases.sort_by_key(|r| parse_version(&r.tag_name).unwrap_or((0, 0, 0)));
    releases.pop()
}

/// Returns the platform-specific suffix that release asset names end with.
///
/// Release assets are named like `nw_v0.1.9_relayer_cli_linux_amd64` (binary)
/// and `nw_v0.1.9_relayer_cli_linux_amd64.sha256` (checksum). Since the
/// version prefix changes per release, we match by trailing platform segment
/// instead of the full name.
fn artifact_suffix() -> Result<&'static str, String> {
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Ok("_linux_amd64")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        Ok("_linux_arm64")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Ok("_macos_arm64")
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        Ok("_windows_amd64.exe")
    } else {
        Err(format!(
            "Unsupported platform: {} / {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    }
}

fn find_binary_asset<'a>(assets: &'a [GithubAsset], suffix: &str) -> Option<&'a GithubAsset> {
    assets
        .iter()
        .find(|a| a.name.ends_with(suffix) && !a.name.ends_with(".sha256"))
}

fn find_checksum_asset<'a>(assets: &'a [GithubAsset], suffix: &str) -> Option<&'a GithubAsset> {
    let checksum_suffix = format!("{suffix}.sha256");
    assets.iter().find(|a| a.name.ends_with(&checksum_suffix))
}

fn parse_version(s: &str) -> Result<(u32, u32, u32), String> {
    let stripped = s
        .strip_prefix('v')
        .unwrap_or(s)
        .trim_end_matches(RELEASE_TAG_SUFFIX);
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
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    println!("Checking for updates...");

    let releases: Vec<GithubRelease> = client
        .get(GITHUB_API_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse release JSON: {e}"))?;

    let release = pick_latest_relayer_release(releases).ok_or_else(|| {
        format!("No release found with tag suffix '{RELEASE_TAG_SUFFIX}'.")
    })?;

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
        .trim_end_matches(RELEASE_TAG_SUFFIX);

    if check_only {
        println!("Update available: v{CURRENT_VERSION} -> v{remote_display}");
        return Ok(());
    }

    let suffix = artifact_suffix()?;
    let asset = find_binary_asset(&release.assets, suffix)
        .ok_or_else(|| format!("No asset found for this platform (suffix '{suffix}')."))?;

    println!(
        "Updating v{CURRENT_VERSION} -> v{remote_display} ({})...",
        asset.name
    );

    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download: {e}"))?;

    println!("Downloaded {} bytes.", bytes.len());

    // --- Verify checksum --------------------------------------------------------

    let checksum_asset = find_checksum_asset(&release.assets, suffix);

    if let Some(checksum_asset) = checksum_asset {
        println!("Verifying checksum...");

        let checksum_text = client
            .get(&checksum_asset.browser_download_url)
            .send()
            .await
            .map_err(|e| format!("Failed to download checksum: {e}"))?
            .text()
            .await
            .map_err(|e| format!("Failed to read checksum: {e}"))?;

        let expected = checksum_text
            .split_whitespace()
            .next()
            .ok_or_else(|| "Checksum file is empty".to_string())?;

        let actual = hex::encode(Sha256::digest(&bytes));

        if actual != expected {
            return Err(format!(
                "Checksum mismatch!\n  Expected: {expected}\n  Actual:   {actual}"
            ));
        }

        println!("Checksum verified.");
    } else {
        println!("Note: no checksum file in release, skipping verification.");
    }

    // --- Write and replace -------------------------------------------------------

    let tmp_path =
        std::env::temp_dir().join(format!("relayer-cli-update-{}", std::process::id()));
    let _tmp_guard = TempFileGuard(tmp_path.clone());

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
    // artifact_suffix
    // -----------------------------------------------------------------------

    #[test]
    fn artifact_suffix_returns_ok_on_supported_platform() {
        let result = artifact_suffix();
        assert!(
            result.is_ok(),
            "artifact_suffix() failed on this platform: {result:?}"
        );

        let suffix = result.unwrap();
        assert!(
            suffix == "_linux_amd64"
                || suffix == "_linux_arm64"
                || suffix == "_macos_arm64"
                || suffix == "_windows_amd64.exe",
            "unexpected artifact suffix: {suffix}"
        );
    }

    #[test]
    fn artifact_suffix_matches_current_platform() {
        let suffix = artifact_suffix().unwrap();
        if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
            assert_eq!(suffix, "_macos_arm64");
        } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
            assert_eq!(suffix, "_linux_amd64");
        } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
            assert_eq!(suffix, "_linux_arm64");
        } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
            assert_eq!(suffix, "_windows_amd64.exe");
        }
    }

    // -----------------------------------------------------------------------
    // find_binary_asset / find_checksum_asset
    // -----------------------------------------------------------------------

    fn asset(name: &str) -> GithubAsset {
        GithubAsset {
            name: name.to_string(),
            browser_download_url: format!("https://example/{name}"),
        }
    }

    #[test]
    fn find_binary_asset_picks_binary_not_checksum() {
        let assets = vec![
            asset("nw_v0.1.9_relayer_cli_linux_amd64"),
            asset("nw_v0.1.9_relayer_cli_linux_amd64.sha256"),
            asset("nw_v0.1.9_relayer_cli_linux_arm64"),
        ];
        let picked = find_binary_asset(&assets, "_linux_amd64").expect("should find asset");
        assert_eq!(picked.name, "nw_v0.1.9_relayer_cli_linux_amd64");
    }

    #[test]
    fn find_binary_asset_windows_matches_exe_not_sha256() {
        let assets = vec![
            asset("nw_v0.1.9_relayer_cli_windows_amd64.exe"),
            asset("nw_v0.1.9_relayer_cli_windows_amd64.exe.sha256"),
        ];
        let picked = find_binary_asset(&assets, "_windows_amd64.exe").expect("should find asset");
        assert_eq!(picked.name, "nw_v0.1.9_relayer_cli_windows_amd64.exe");
    }

    #[test]
    fn find_binary_asset_missing_returns_none() {
        let assets = vec![asset("nw_v0.1.9_relayer_cli_linux_amd64")];
        assert!(find_binary_asset(&assets, "_macos_arm64").is_none());
    }

    #[test]
    fn find_checksum_asset_picks_matching_checksum() {
        let assets = vec![
            asset("nw_v0.1.9_relayer_cli_linux_amd64"),
            asset("nw_v0.1.9_relayer_cli_linux_amd64.sha256"),
            asset("nw_v0.1.9_relayer_cli_linux_arm64.sha256"),
        ];
        let picked = find_checksum_asset(&assets, "_linux_amd64").expect("should find checksum");
        assert_eq!(picked.name, "nw_v0.1.9_relayer_cli_linux_amd64.sha256");
    }

    #[test]
    fn find_checksum_asset_windows_matches_exe_sha256() {
        let assets = vec![
            asset("nw_v0.1.9_relayer_cli_windows_amd64.exe"),
            asset("nw_v0.1.9_relayer_cli_windows_amd64.exe.sha256"),
        ];
        let picked =
            find_checksum_asset(&assets, "_windows_amd64.exe").expect("should find checksum");
        assert_eq!(picked.name, "nw_v0.1.9_relayer_cli_windows_amd64.exe.sha256");
    }

    #[test]
    fn find_checksum_asset_missing_returns_none() {
        let assets = vec![asset("nw_v0.1.9_relayer_cli_linux_amd64")];
        assert!(find_checksum_asset(&assets, "_linux_amd64").is_none());
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

    // -----------------------------------------------------------------------
    // pick_latest_relayer_release
    // -----------------------------------------------------------------------

    fn make_release(tag: &str, draft: bool, prerelease: bool) -> GithubRelease {
        GithubRelease {
            tag_name: tag.to_string(),
            draft,
            prerelease,
            assets: vec![],
        }
    }

    #[test]
    fn picks_highest_relayer_cli_tag_ignoring_other_components() {
        let releases = vec![
            make_release("v0.1.9-relayer-cli", false, false),
            make_release("v0.1.8-relayer-cli", false, false),
            make_release("v0.1.1", false, false),
            make_release("v0.0.4-relayer-deployer", false, false),
            make_release("v0.0.2-validator-wallet", false, false),
            make_release("v0.1.4-realyer-cli", false, false), // typo, must be skipped
        ];
        let picked = pick_latest_relayer_release(releases).expect("should pick a release");
        assert_eq!(picked.tag_name, "v0.1.9-relayer-cli");
    }

    #[test]
    fn skips_drafts_and_prereleases() {
        let releases = vec![
            make_release("v0.2.0-relayer-cli", true, false),  // draft
            make_release("v0.1.9-relayer-cli", false, true),  // prerelease
            make_release("v0.1.8-relayer-cli", false, false), // stable
        ];
        let picked = pick_latest_relayer_release(releases).expect("should pick a release");
        assert_eq!(picked.tag_name, "v0.1.8-relayer-cli");
    }

    #[test]
    fn returns_none_when_no_relayer_cli_release() {
        let releases = vec![
            make_release("v0.1.1", false, false),
            make_release("v0.0.4-relayer-deployer", false, false),
        ];
        assert!(pick_latest_relayer_release(releases).is_none());
    }

    #[test]
    fn returns_none_on_empty_input() {
        assert!(pick_latest_relayer_release(vec![]).is_none());
    }

    // -----------------------------------------------------------------------
    // TempFileGuard
    // -----------------------------------------------------------------------

    #[test]
    fn temp_file_guard_removes_file_on_drop() {
        let path = std::env::temp_dir().join(format!(
            "relayer-cli-update-guard-test-{}",
            std::process::id()
        ));
        std::fs::write(&path, b"scratch").expect("write scratch file");
        assert!(path.exists(), "precondition: scratch file exists");

        {
            let _guard = TempFileGuard(path.clone());
        }

        assert!(!path.exists(), "guard should remove file on drop");
    }

    #[test]
    fn temp_file_guard_missing_file_is_noop() {
        let path = std::env::temp_dir().join(format!(
            "relayer-cli-update-guard-missing-{}",
            std::process::id()
        ));
        assert!(!path.exists(), "precondition: path does not exist");

        // Must not panic even when the file was never created or was already
        // consumed by self_replace.
        let _guard = TempFileGuard(path);
    }
}
