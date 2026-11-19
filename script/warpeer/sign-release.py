#!/usr/bin/env python3
"""Sign the complete desktop asset set, verify it, then regenerate checksums."""
import base64
import hashlib
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

ARCHIVES = (
    "cyberterm2077-linux-x86_64.tar.gz",
    "cyberterm2077-windows-x86_64.zip",
)
DETACHED = (
    "cyberterm2077-macos-aarch64.dmg",
    "cyberterm2077-linux-x86_64.deb",
    "cyberterm2077-linux-x86_64.bin",
    "cyberterm2077-windows-x86_64-setup.exe",
)


def configuration():
    tag = os.environ.get("GIT_RELEASE_TAG", "")
    key = os.environ.get("WARPEER_UPDATE_PUBLIC_KEY", "")
    if not re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", tag):
        raise ValueError("Release tag must be a stable vMAJOR.MINOR.PATCH version")
    if not re.fullmatch(r"[0-9a-fA-F]{64}", key) or int(key, 16) == 0:
        raise ValueError("Set WARPEER_UPDATE_PUBLIC_KEY to the public key's 64 hex characters")
    return bytes.fromhex(key)


def sign(dist):
    public = configuration()
    private = base64.b64decode(os.environ["WARPEER_RELEASE_SIGNING_KEY"], validate=True)
    if len(private) != 64:
        raise ValueError("WARPEER_RELEASE_SIGNING_KEY must encode zipsign's 64-byte keypair")
    if subprocess.check_output(["zipsign", "--version"], text=True).strip() != "zipsign 0.2.2":
        raise ValueError("Expected zipsign 0.2.2")
    expected = {*ARCHIVES, *DETACHED}
    actual = {p.name for p in dist.iterdir() if not p.name.endswith(".sha256")}
    if actual != expected:
        raise ValueError(f"Unexpected release assets: missing={expected - actual}, extra={actual - expected}")
    with tempfile.TemporaryDirectory(prefix="warpeer-sign-") as temp:
        secret = Path(temp) / "release.key"
        pub = Path(temp) / "release.pub"
        secret.write_bytes(private)
        secret.chmod(0o600)
        pub.write_bytes(public)
        derived = Path(temp) / "derived.pub"
        subprocess.run(["zipsign", "gen-key", "--extract", str(secret), str(derived)], check=True)
        if derived.read_bytes() != public:
            raise ValueError("Signing secret does not match the embedded public key")
        for name in ARCHIVES:
            path = dist / name
            kind = "zip" if name.endswith(".zip") else "tar"
            subprocess.run(["zipsign", "sign", kind, str(path), str(secret)], check=True)
            subprocess.run(["zipsign", "verify", kind, str(path), str(pub)], check=True)
        for name in DETACHED:
            path = dist / name
            signature = dist / (name + ".sig")
            subprocess.run(["zipsign", "sign", "separate", str(path), str(secret),
                            "--output", str(signature), "--force"], check=True)
            subprocess.run(["zipsign", "verify", "separate", str(path), str(signature), str(pub)], check=True)
            expected.add(signature.name)
    (dist / "cyberterm2077-release.pub").write_bytes(public)
    expected.add("cyberterm2077-release.pub")
    # Hash the signed bytes, never the unsigned build artifacts.
    for name in sorted(expected):
        path = dist / name
        with path.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        (dist / (name + ".sha256")).write_text(f"{digest}  {name}\n")


if __name__ == "__main__":
    if sys.argv[1:] == ["--validate"]:
        configuration()
    elif not sys.argv[1:]:
        sign(Path("dist"))
    else:
        raise SystemExit("Usage: sign-release.py [--validate]")
