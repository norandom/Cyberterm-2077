"""Smoke-test actual DEB/NSIS packaging with a small disposable payload."""
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("package_deb", HERE / "package-deb.py")
deb = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deb)


class InstallerTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which("dpkg-shlibdeps"), "dpkg-dev required")
    def test_deb_contents_dependencies_and_package_manager_marker(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "portable"
            source.mkdir()
            shutil.copy2("/bin/true", source / "warpeer")
            package = root / "warpeer.deb"
            deb.package(source, "v1.2.3", package)
            version = subprocess.check_output(["dpkg-deb", "-f", str(package), "Version"], text=True).strip()
            self.assertEqual(version, "1.2.3")
            dependencies = subprocess.check_output(["dpkg-deb", "-f", str(package), "Depends"], text=True)
            self.assertIn("libc6", dependencies)
            self.assertIn("libegl1", dependencies)
            self.assertIn("libxkbcommon-x11-0", dependencies)
            unpack = root / "unpacked"
            subprocess.run(["dpkg-deb", "-x", str(package), str(unpack)], check=True)
            self.assertEqual((unpack / "usr/lib/warpeer/.package-managed").read_text(), "dpkg\n")
            self.assertTrue((unpack / "usr/bin/warpeer").resolve().is_file())
            self.assertTrue((unpack / "usr/share/applications/warpeer.desktop").is_file())

    @unittest.skipUnless(shutil.which("makensis"), "NSIS required")
    def test_windows_installer_compiles(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "warpeer.exe").write_bytes(b"placeholder")
            (root / "LICENSE-AGPL").write_text("test license")
            output = root / "warpeer-windows-x86_64-setup.exe"
            result = subprocess.run(["makensis", "-V2", "-DVERSION=v1.2.3", f"-DSOURCE={root}",
                            f"-DOUTPUT={output}", str(HERE / "installer.nsi")], check=True, capture_output=True, text=True)
            self.assertNotIn("warning 6000", result.stdout + result.stderr)
            self.assertGreater(output.stat().st_size, 0)


if __name__ == "__main__":
    unittest.main()
