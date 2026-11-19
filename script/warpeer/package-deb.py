#!/usr/bin/env python3
"""Build a per-machine Debian package from the portable Linux distribution."""
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

ICON = Path(__file__).resolve().parents[2] / "app/assets/bundled/png/warpeer.png"


def package(source, version, destination):
    deb_version = version.removeprefix("v")
    if not re.fullmatch(r"\d+\.\d+\.\d+", deb_version):
        deb_version = "0.0.0+" + re.sub(r"[^a-zA-Z0-9.+]", ".", version)
    with tempfile.TemporaryDirectory(prefix="cyberterm2077-deb-") as tmp:
        work = Path(tmp)
        root = work / "root"
        payload = root / "usr/lib/cyberterm2077"
        shutil.copytree(source, payload)
        (payload / ".package-managed").write_text("dpkg\n")
        (root / "usr/bin").mkdir(parents=True)
        (root / "usr/bin/cyberterm2077").symlink_to("../lib/cyberterm2077/cyberterm2077")
        desktop = root / "usr/share/applications"
        desktop.mkdir(parents=True)
        icons = root / "usr/share/icons/hicolor/512x512/apps"
        icons.mkdir(parents=True)
        shutil.copy2(ICON, icons / "cyberterm2077.png")
        (desktop / "cyberterm2077.desktop").write_text(
            "[Desktop Entry]\nType=Application\nName=Cyberterm 2077\n"
            "Comment=Task-oriented terminal for Markdown runbooks\nExec=cyberterm2077\nIcon=cyberterm2077\n"
            "Terminal=false\nCategories=System;TerminalEmulator;\nStartupWMClass=com.matrixme.warpeer\n")
        (work / "debian").mkdir()
        (work / "debian/control").write_text("Source: cyberterm2077\n\nPackage: cyberterm2077\nArchitecture: amd64\n")
        dependencies = subprocess.check_output(
            ["dpkg-shlibdeps", "-O", "-e" + str(payload / "cyberterm2077")], cwd=work, text=True
        ).strip().removeprefix("shlibs:Depends=")
        # EGL/Wayland/XKB libraries are loaded at runtime and don't all appear
        # in ELF NEEDED entries. Keep the desktop runtime dependencies from
        # resources/linux/debian/app/control.template as well as derived ABI minima.
        runtime = ["fontconfig", "libegl1", "libwayland-client0", "libwayland-egl1",
                   "libx11-6", "libxcb1", "libxcursor1", "libxi6", "libxkbcommon-x11-0", "zlib1g"]
        detected = {item.strip().split()[0] for item in dependencies.split(",") if item.strip()}
        dependencies = ", ".join([dependencies] + [name for name in runtime if name not in detected])
        control = root / "DEBIAN"
        control.mkdir()
        (control / "control").write_text(
            f"Package: cyberterm2077\nVersion: {deb_version}\nArchitecture: amd64\n"
            "Maintainer: norandom <11855163+norandom@users.noreply.github.com>\n"
            f"Depends: {dependencies}\nRecommends: libgl1-mesa-dri\nSection: utils\nPriority: optional\n"
            "Conflicts: warpeer\nReplaces: warpeer\n"
            "Homepage: https://github.com/norandom/Cyberterm-2077\nDescription: Cyberterm 2077 native terminal\n")
        destination.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["dpkg-deb", "--root-owner-group", "--build", str(root), str(destination)], check=True)


if __name__ == "__main__":
    package(Path(sys.argv[1]), sys.argv[2], Path("dist/cyberterm2077-linux-x86_64.deb"))
