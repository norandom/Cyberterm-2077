#!/usr/bin/env python3
"""Package a native release-desktop build; run from the repository root."""
import hashlib
import json
import os
import plistlib
import re
import subprocess
from pathlib import Path
import shutil
import sys

platform = sys.argv[1]
if platform not in ("linux", "windows", "macos"):
    raise SystemExit("Expected linux, windows or macos")
arch = "aarch64" if platform == "macos" else "x86_64"
name = f"warpeer-{platform}-{arch}"
dist = Path("dist")
staging = Path("build") / name
if staging.exists():
    shutil.rmtree(staging)
staging.mkdir(parents=True)
dist.mkdir(exist_ok=True)
payload = staging
if platform == "macos":
    payload = staging / "Warpeer.app/Contents/MacOS"
    payload.mkdir(parents=True)
    resources = staging / "Warpeer.app/Contents/Resources"
else:
    resources = staging / "resources"
metadata = resources / "bundled/metadata"
metadata.mkdir(parents=True)
version = os.environ.get("GIT_RELEASE_TAG") or subprocess.check_output(["git", "rev-parse", "--short", "HEAD"], text=True).strip()
(metadata / "version.json").write_text(json.dumps({"warp_version": version, "product": "Warpeer"}) + "\n")
exe = "warpeer.exe" if platform == "windows" else "warpeer"
shutil.copy2(Path("target/release-desktop") / exe, payload / exe)
for filename in ("LICENSE-AGPL", "LICENSE-MIT", "NOTICE"):
    shutil.copy2(filename, staging / filename)
if platform == "windows":
    shutil.copy2("target/release-desktop/warpeer-update-helper.exe", staging / "warpeer-update-helper.exe")
    assets = Path("app/assets/windows/x64")
    for filename in ("conpty.dll", "dxcompiler.dll", "dxil.dll", "msvcp140.dll",
                     "vcruntime140.dll", "vcruntime140_1.dll"):
        shutil.copy2(assets / filename, staging / filename)
    (staging / "x64").mkdir()
    shutil.copy2(assets / "OpenConsole.exe", staging / "x64/OpenConsole.exe")
if platform == "macos":
    app = staging / "Warpeer.app"
    source = Path("app/src/bin/warpeer.rs").read_text()
    plist_xml = re.search(r'r#"(.*?)"#\.as_bytes', source, re.S).group(1).strip()
    info = plistlib.loads(plist_xml.encode())
    app_version = version.removeprefix("v") if re.fullmatch(r"v\d+\.\d+\.\d+", version) else "0.0.0"
    info.update(CFBundleShortVersionString=app_version, CFBundleVersion=app_version, CFBundleIconFile="warpeer.icns", LSMinimumSystemVersion="11.0")
    (app / "Contents/Info.plist").write_bytes(plistlib.dumps(info))
    shutil.copy2("app/channels/warpeer/icon/no-padding/icon.icns", resources / "warpeer.icns")
    plugin = Path("target/release-desktop/WarpDockTilePlugin.docktileplugin")
    if plugin.exists():
        (app / "Contents/PlugIns").mkdir()
        shutil.copytree(plugin, app / "Contents/PlugIns" / plugin.name)
        info["NSDockTilePlugIn"] = plugin.name
        (app / "Contents/Info.plist").write_bytes(plistlib.dumps(info))
    subprocess.run(["codesign", "--force", "--deep", "--sign", "-", str(app)], check=True)
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
    (staging / "Applications").symlink_to("/Applications")
    archive = dist / (name + ".dmg")
    subprocess.run(["hdiutil", "create", "-ov", "-volname", "Warpeer", "-srcfolder", str(staging), "-format", "UDZO", str(archive)], check=True)
else:
    archive = Path(shutil.make_archive(str(dist / name),
        "zip" if platform == "windows" else "gztar", staging.parent, name))
digest = hashlib.sha256(archive.read_bytes()).hexdigest()
(archive.parent / (archive.name + ".sha256")).write_text(f"{digest}  {archive.name}\n")
print(archive)
if platform == "linux":
    shutil.copy2(payload / exe, dist / (name + ".bin"))
    subprocess.run([sys.executable, "script/warpeer/package-deb.py", str(staging), version], check=True)
elif platform == "windows":
    nsis = shutil.which("makensis") or r"C:\Program Files (x86)\NSIS\makensis.exe"
    subprocess.run([nsis, "/DVERSION=" + version, "/DSOURCE=" + str(staging.resolve()),
                    "/DOUTPUT=" + str((dist / "warpeer-windows-x86_64-setup.exe").resolve()),
                    "script/warpeer/installer.nsi"], check=True)
