//! Authenticate the DMG before allowing hdiutil to parse or mount it.
use super::*;
use std::{fs, process::Command};

struct Mount(PathBuf);
impl Drop for Mount {
    fn drop(&mut self) {
        let _ = Command::new("/usr/bin/hdiutil")
            .arg("detach")
            .arg(&self.0)
            .status();
    }
}

pub(super) fn install(tag: &str, root: &Path, key: [u8; 32]) -> Result<()> {
    ensure!(
        !root.starts_with("/Volumes") && !root.to_string_lossy().contains("AppTranslocation"),
        "Move Warpeer.app to Applications and launch it from there before updating"
    );
    let downloads = tempfile::tempdir()?;
    let name = "warpeer-macos-aarch64.dmg";
    let dmg = downloads.path().join(name);
    let signature = downloads.path().join(format!("{name}.sig"));
    let base = format!("https://github.com/norandom/warpeer/releases/download/{tag}");
    for path in [&dmg, &signature] {
        let filename = path.file_name().unwrap().to_str().unwrap();
        self_update::Download::from_url(&format!("{base}/{filename}"))
            .timeout(Duration::from_secs(300))
            .download_to(&mut fs::File::create(path)?)?;
    }
    super::verify_detached(&dmg, &signature, key)?;
    let mountpoint = downloads.path().join("mount");
    fs::create_dir(&mountpoint)?;
    ensure!(
        Command::new("/usr/bin/hdiutil")
            .args(["attach", "-readonly", "-nobrowse", "-mountpoint"])
            .arg(&mountpoint)
            .arg(&dmg)
            .status()?
            .success(),
        "Unable to mount verified DMG"
    );
    let _mount = Mount(mountpoint.clone());
    let parent = root.parent().context("App bundle has no parent")?;
    let staging = tempfile::Builder::new()
        .prefix(".warpeer-update-")
        .tempdir_in(parent)?;
    let new_app = staging.path().join("Warpeer.app");
    ensure!(
        Command::new("/usr/bin/ditto")
            .arg(mountpoint.join("Warpeer.app"))
            .arg(&new_app)
            .status()?
            .success(),
        "Unable to copy app bundle"
    );
    super::verify_payload(&new_app, tag, "macos-aarch64")?;
    let backup = staging.path().join("previous.app");
    fs::rename(root, &backup)
        .context("Cannot replace app; check Applications folder permissions")?;
    if let Err(error) = fs::rename(&new_app, root) {
        if let Err(rollback) = fs::rename(&backup, root) {
            let recovery = staging.keep();
            bail!(
                "Update failed: {error}; restore failed: {rollback}. Previous app retained at {}",
                recovery.display()
            );
        }
        return Err(error.into());
    }
    Ok(())
}
