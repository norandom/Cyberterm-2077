#!/usr/bin/env python3
"""Package a native release-desktop build; run from the repository root."""
import hashlib
from pathlib import Path
import shutil
import sys

platform = sys.argv[1]
if platform not in ("linux", "windows"):
    raise SystemExit("Expected linux or windows")
name = f"warpeer-{platform}-x86_64"
dist = Path("dist")
staging = Path("build") / name
if staging.exists():
    shutil.rmtree(staging)
staging.mkdir(parents=True)
dist.mkdir(exist_ok=True)
exe = "warpeer.exe" if platform == "windows" else "warpeer"
shutil.copy2(Path("target/release-desktop") / exe, staging / exe)
for filename in ("LICENSE-AGPL", "LICENSE-MIT", "NOTICE"):
    shutil.copy2(filename, staging / filename)
if platform == "windows":
    assets = Path("app/assets/windows/x64")
    for filename in ("conpty.dll", "dxcompiler.dll", "dxil.dll", "msvcp140.dll",
                     "vcruntime140.dll", "vcruntime140_1.dll"):
        shutil.copy2(assets / filename, staging / filename)
    (staging / "x64").mkdir()
    shutil.copy2(assets / "OpenConsole.exe", staging / "x64/OpenConsole.exe")
archive = Path(shutil.make_archive(str(dist / name),
    "zip" if platform == "windows" else "gztar", staging.parent, name))
digest = hashlib.sha256(archive.read_bytes()).hexdigest()
(archive.parent / (archive.name + ".sha256")).write_text(f"{digest}  {archive.name}\n")
print(archive)
