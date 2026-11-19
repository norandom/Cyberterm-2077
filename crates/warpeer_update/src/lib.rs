//! GitHub release updates for the portable Cyberterm 2077 distribution.
//! Blocking operations: callers must run check/install off the UI thread.
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use self_update::backends::github::Update;
use semver::Version;
#[cfg(any(target_os = "macos", test))]
mod macos;

pub const RELEASES_URL: &str = "https://github.com/norandom/Cyberterm-2077/releases";
pub const REPO_NAME: &str = "Cyberterm-2077";
pub const PRODUCT: &str = "Cyberterm 2077";
pub const BIN_NAME: &str = "cyberterm2077";
pub const APP_BUNDLE: &str = "Cyberterm 2077.app";
pub const BUILD_TAG: &str = match option_env!("GIT_RELEASE_TAG") {
    Some(tag) => tag,
    None => "development",
};

fn public_key(value: &str) -> Result<[u8; 32]> {
    ensure!(
        value.len() == 64 && value.is_ascii(),
        "Release signing key is not configured (expected 64 hex characters)"
    );
    let mut key = [0; 32];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16)
            .context("Invalid release public key")?;
    }
    ensure!(key != [0; 32], "Release signing key cannot be all zeroes");
    Ok(key)
}

pub fn newer(candidate: &str, current: &str) -> Result<bool> {
    let candidate = Version::parse(candidate.trim_start_matches('v'))?;
    let current = Version::parse(current.trim_start_matches('v'))?;
    Ok(candidate.pre.is_empty() && candidate > current)
}

fn platform() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("linux-x86_64"),
        ("windows", "x86_64") => Ok("windows-x86_64"),
        ("macos", "aarch64") => Ok("macos-aarch64"),
        _ => bail!("No signed Cyberterm 2077 releases for this platform"),
    }
}

fn install_root(exe: &Path, target: &str) -> Result<PathBuf> {
    if target.starts_with("macos") {
        let root = exe
            .ancestors()
            .nth(3)
            .context("Expected Cyberterm 2077.app/Contents/MacOS/cyberterm2077")?;
        ensure!(
            root.file_name().is_some_and(|n| n == APP_BUNDLE),
            "Install Cyberterm 2077.app before updating"
        );
        Ok(root.to_path_buf())
    } else {
        let root = exe.parent().context("Executable has no parent directory")?;
        let expected = format!("{BIN_NAME}-{target}");
        ensure!(
            root.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == expected || n.eq_ignore_ascii_case(BIN_NAME)),
            "Keep portable Cyberterm 2077 in its own cyberterm2077-platform-architecture directory before updating"
        );
        ensure!(
            !root.join(".package-managed").exists(),
            "This installation is managed by dpkg; install the newer .deb with your package manager"
        );
        // Never replace a shared directory such as /usr/bin or target/release.
        ensure!(
            root.join("resources/bundled/metadata/version.json")
                .is_file()
                && root.join("NOTICE").is_file(),
            "Self-update requires an extracted portable Cyberterm 2077 distribution; update package-managed or development builds manually"
        );
        Ok(root.to_path_buf())
    }
}

fn updater() -> Result<Update> {
    let key = public_key(option_env!("WARPEER_UPDATE_PUBLIC_KEY").unwrap_or(""))?;
    let current = Version::parse(BUILD_TAG.trim_start_matches('v'))
        .context("This development build has no release version; install a signed release first")?;
    let target = platform()?;
    let mut builder = Update::configure();
    builder
        .repo_owner("norandom")
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .target(target)
        .asset_identifier(if cfg!(windows) { ".zip" } else { ".tar.gz" })
        .current_version(current.to_string())
        .no_confirm(true)
        .show_output(false)
        .timeout(Duration::from_secs(300))
        .verifying_keys([key]);
    Ok(builder.build()?)
}

/// Only checks metadata. Installation always re-verifies the signed archive.
pub fn check() -> Result<Option<String>> {
    let releases = updater()?.get_latest_release()?;
    let Some(release) = releases.latest() else {
        return Ok(None);
    };
    if newer(release.version(), BUILD_TAG)? {
        Ok(Some(format!("v{}", release.version())))
    } else {
        Ok(None)
    }
}

/// Installs the complete distribution; failed directory swaps are rolled back.
pub fn install(tag: &str) -> Result<()> {
    install_at(tag, &std::env::current_exe()?.canonicalize()?)
}

/// Used by the Windows helper after the GUI has exited and released its DLLs.
pub fn install_at(tag: &str, exe: &Path) -> Result<()> {
    ensure!(
        newer(tag, BUILD_TAG)?,
        "Refusing a downgrade or prerelease update"
    );
    let target = platform()?;
    let root = install_root(exe, target)?;
    let key = public_key(option_env!("WARPEER_UPDATE_PUBLIC_KEY").unwrap_or(""))?;
    #[cfg(target_os = "macos")]
    return macos::install(tag, &root, key);
    #[cfg(not(target_os = "macos"))]
    {
        let archive_root = format!("{BIN_NAME}-{target}");
        let expected_tag = tag.to_owned();
        let previous_root = root.clone();
        Update::configure()
            .repo_owner("norandom")
            .repo_name(REPO_NAME)
            .bin_name(BIN_NAME)
            .target(target)
            .asset_identifier(if cfg!(windows) { ".zip" } else { ".tar.gz" })
            .current_version(BUILD_TAG.trim_start_matches('v'))
            .release_tag(tag)
            .bundle_path_in_archive(archive_root)
            .bundle_install_path(root)
            .verify_binary(move |root: &Path| {
                let verify = || -> Result<()> {
                    verify_payload(root, &expected_tag, target)?;
                    // The locally generated NSIS uninstaller carries install-path state.
                    // Preserve it across a full-directory update.
                    if target.starts_with("windows")
                        && previous_root.join("Uninstall.exe").is_file()
                    {
                        std::fs::copy(
                            previous_root.join("Uninstall.exe"),
                            root.join("Uninstall.exe"),
                        )?;
                    }
                    Ok(())
                };
                verify()
                    .map_err(|err| self_update::Error::verification_rejected(format!("{err:#}")))
            })
            .no_confirm(true)
            .show_output(false)
            .timeout(Duration::from_secs(300))
            .verifying_keys([key])
            .build()?
            .update()?;
        Ok(())
    }
}

/// Verify detached zipsign signatures (DMG, DEB, installer and raw binary).
pub fn verify_detached(path: &Path, signature: &Path, key: [u8; 32]) -> Result<()> {
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .context("Invalid asset filename")?;
    let keys = zipsign_api::verify::collect_keys([Ok(key)])?;
    let signatures = zipsign_api::verify::read_signatures(&mut std::fs::File::open(signature)?)?;
    let hash = zipsign_api::Prehash::calculate(&mut std::fs::File::open(path)?)?;
    zipsign_api::verify::find_match(&keys, &signatures, &hash, Some(name.as_bytes()))?;
    Ok(())
}

pub fn package_managed() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join(".package-managed")))
        .is_some_and(|p| p.exists())
}

#[cfg(windows)]
pub fn prepare_windows_helper() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let root = install_root(&exe, platform()?)?;
    let temp = tempfile::Builder::new()
        .prefix("cyberterm2077-update-")
        .tempdir()?;
    let helper = temp.path().join("warpeer-update-helper.exe");
    std::fs::copy(root.join("warpeer-update-helper.exe"), &helper)?;
    temp.keep();
    Ok(helper)
}

fn verify_payload(root: &Path, tag: &str, target: &str) -> Result<()> {
    let (metadata, binary) = if target.starts_with("macos") {
        (
            "Contents/Resources/bundled/metadata/version.json",
            "Contents/MacOS/cyberterm2077",
        )
    } else if target.starts_with("windows") {
        (
            "resources/bundled/metadata/version.json",
            "cyberterm2077.exe",
        )
    } else {
        ("resources/bundled/metadata/version.json", "cyberterm2077")
    };
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join(metadata))?)?;
    ensure!(
        value["product"] == PRODUCT && value["warp_version"] == tag,
        "Signed payload does not match the requested release"
    );
    ensure!(
        root.join(binary).is_file(),
        "Signed release is missing the executable"
    );
    #[cfg(target_os = "macos")]
    ensure!(
        std::process::Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(root)
            .status()?
            .success(),
        "App bundle code signature is invalid"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use super::*;

    #[test]
    fn signatures_reject_unsigned_wrong_key_tampering_and_renaming() {
        let temp = tempfile::tempdir().unwrap();
        let signing = ed25519_dalek::SigningKey::from_bytes(&[42; 32]);
        let key = signing.verifying_key().to_bytes();
        for extension in ["tar.gz", "zip"] {
            let name = format!("cyberterm2077-linux-x86_64.{extension}");
            let path = temp.path().join(&name);
            let mut input = Cursor::new(Vec::new());
            if extension == "zip" {
                let mut archive = zip::ZipWriter::new(&mut input);
                archive
                    .start_file("cyberterm2077", zip::write::SimpleFileOptions::default())
                    .unwrap();
                archive.write_all(b"executable payload").unwrap();
                archive.finish().unwrap();
            } else {
                let gzip =
                    flate2::write::GzEncoder::new(&mut input, flate2::Compression::default());
                let mut archive = tar::Builder::new(gzip);
                let mut header = tar::Header::new_gnu();
                header.set_size(18);
                header.set_mode(0o755);
                header.set_cksum();
                archive
                    .append_data(&mut header, "cyberterm2077", &b"executable payload"[..])
                    .unwrap();
                archive.into_inner().unwrap().finish().unwrap();
            }
            std::fs::write(&path, input.get_ref()).unwrap();
            assert!(self_update::verify_signature(&path, &[key]).is_err());
            input.set_position(0);
            let mut signed = Cursor::new(Vec::new());
            if extension == "zip" {
                zipsign_api::sign::copy_and_sign_zip(
                    &mut input,
                    &mut signed,
                    &[signing.clone()],
                    Some(name.as_bytes()),
                )
                .unwrap();
            } else {
                zipsign_api::sign::copy_and_sign_tar(
                    &mut input,
                    &mut signed,
                    &[signing.clone()],
                    Some(name.as_bytes()),
                )
                .unwrap();
            }
            std::fs::write(&path, signed.get_ref()).unwrap();
            self_update::verify_signature(&path, &[key]).unwrap();
            assert!(self_update::verify_signature(&path, &[[13; 32]]).is_err());
            let renamed = temp.path().join(format!("renamed.{extension}"));
            std::fs::copy(&path, &renamed).unwrap();
            assert!(self_update::verify_signature(&renamed, &[key]).is_err());
            signed.get_mut()[15] ^= 1;
            std::fs::write(&path, signed.get_ref()).unwrap();
            assert!(self_update::verify_signature(&path, &[key]).is_err());
        }
    }

    #[test]
    fn signed_payload_must_match_requested_version() {
        let temp = tempfile::tempdir().unwrap();
        let metadata = temp.path().join("resources/bundled/metadata");
        std::fs::create_dir_all(&metadata).unwrap();
        std::fs::write(
            metadata.join("version.json"),
            br#"{"product":"Cyberterm 2077","warp_version":"v1.0.0"}"#,
        )
        .unwrap();
        std::fs::write(temp.path().join(BIN_NAME), b"payload").unwrap();
        assert!(verify_payload(temp.path(), "v2.0.0", "linux-x86_64").is_err());
        // A valid metadata record must still carry a binary.
        std::fs::remove_file(temp.path().join(BIN_NAME)).unwrap();
        assert!(verify_payload(temp.path(), "v1.0.0", "linux-x86_64").is_err());
    }

    #[test]
    fn detached_signatures_authenticate_dmg_bytes_and_filename() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("cyberterm2077-macos-aarch64.dmg");
        let signature = temp.path().join("cyberterm2077-macos-aarch64.dmg.sig");
        let signing = ed25519_dalek::SigningKey::from_bytes(&[31; 32]);
        let key = signing.verifying_key().to_bytes();
        let bytes = b"disk image bytes";
        std::fs::write(&path, bytes).unwrap();
        let hash = zipsign_api::Prehash::calculate(&mut Cursor::new(bytes)).unwrap();
        let sig = zipsign_api::sign::gather_signature_data(
            &[signing],
            &hash,
            Some(b"cyberterm2077-macos-aarch64.dmg"),
        )
        .unwrap();
        std::fs::write(&signature, sig).unwrap();
        verify_detached(&path, &signature, key).unwrap();
        std::fs::write(&path, b"tampered disk image").unwrap();
        assert!(verify_detached(&path, &signature, key).is_err());
        std::fs::write(&path, bytes).unwrap();
        let renamed = temp.path().join("other.dmg");
        std::fs::rename(&path, &renamed).unwrap();
        assert!(verify_detached(&renamed, &signature, key).is_err());
    }
    #[test]
    fn only_newer_stable_versions() {
        assert!(newer("v1.10.0", "v1.9.0").unwrap());
        for version in ["v1.8.0", "v1.9.0", "v2.0.0-rc.1"] {
            assert!(!newer(version, "v1.9.0").unwrap());
        }
        assert!(newer("desktop-abc", "v1.0.0").is_err());
        assert!(newer("v1.0.0", "main").is_err());
    }
    #[test]
    fn missing_or_malformed_keys_fail_closed() {
        for key in ["", "xyz", &"0".repeat(64), &"é".repeat(32), &"g".repeat(64)] {
            assert!(public_key(key).is_err());
        }
        assert_eq!(public_key(&"ab".repeat(32)).unwrap(), [0xab; 32]);
    }
    #[test]
    fn refuses_shared_install_directories() {
        assert!(install_root(Path::new("/usr/bin/cyberterm2077"), "linux-x86_64").is_err());
        assert!(install_root(Path::new("/tmp/cyberterm2077"), "macos-aarch64").is_err());
        assert_eq!(
            install_root(
                Path::new("/Applications/Cyberterm 2077.app/Contents/MacOS/cyberterm2077"),
                "macos-aarch64"
            )
            .unwrap(),
            Path::new("/Applications/Cyberterm 2077.app")
        );
    }
}
