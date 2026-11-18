"""Exercise the real signing CLI using disposable keys and payloads."""
import base64
import hashlib
import importlib.util
import io
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

spec = importlib.util.spec_from_file_location("sign_release", Path(__file__).with_name("sign-release.py"))
signer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(signer)


@unittest.skipUnless(shutil.which("zipsign"), "zipsign 0.2.2 required")
class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.dist = self.root / "dist"
        self.dist.mkdir()
        private, public = self.root / "key", self.root / "pub"
        subprocess.run(["zipsign", "gen-key", str(private), str(public)], check=True)
        self.public = public
        self.env = patch.dict(os.environ, GIT_RELEASE_TAG="v1.2.3",
            WARPEER_UPDATE_PUBLIC_KEY=public.read_bytes().hex(),
            WARPEER_RELEASE_SIGNING_KEY=base64.b64encode(private.read_bytes()).decode())
        self.env.start()
        self.addCleanup(self.env.stop)
        for name in signer.ARCHIVES:
            if name.endswith(".zip"):
                with zipfile.ZipFile(self.dist / name, "w") as archive:
                    archive.writestr("warpeer.exe", b"test payload")
            else:
                with tarfile.open(self.dist / name, "w:gz") as archive:
                    entry = tarfile.TarInfo("warpeer")
                    entry.size = 12
                    archive.addfile(entry, io.BytesIO(b"test payload"))
        for name in signer.DETACHED:
            (self.dist / name).write_bytes(b"test installer payload")

    def test_sign_verify_hash_and_tamper(self):
        signer.sign(self.dist)
        for path in self.dist.iterdir():
            if not path.name.endswith(".sha256"):
                digest = hashlib.sha256(path.read_bytes()).hexdigest()
                self.assertEqual((self.dist / (path.name + ".sha256")).read_text(), f"{digest}  {path.name}\n")
        dmg = self.dist / "warpeer-macos-aarch64.dmg"
        dmg.write_bytes(b"tampered")
        result = subprocess.run(["zipsign", "verify", "separate", str(dmg), str(dmg) + ".sig", str(self.public)], capture_output=True)
        self.assertNotEqual(result.returncode, 0)

    def test_wrong_key_does_not_sign(self):
        os.environ["WARPEER_UPDATE_PUBLIC_KEY"] = "ab" * 32
        with self.assertRaisesRegex(ValueError, "does not match"):
            signer.sign(self.dist)

    def test_incomplete_release_rejected(self):
        (self.dist / signer.ARCHIVES[0]).unlink()
        with self.assertRaisesRegex(ValueError, "Unexpected release assets"):
            signer.sign(self.dist)

    def test_nonstable_tags_rejected(self):
        for tag in ["main", "v1.0.0-rc.1", "v01.2.3", "v1.2.3; echo bad"]:
            os.environ["GIT_RELEASE_TAG"] = tag
            with self.assertRaises(ValueError):
                signer.configuration()


if __name__ == "__main__":
    unittest.main()
