#!/usr/bin/env python3
"""Native half of the desktop pipeline; also usable on a developer machine."""
import argparse
import os
import shutil
from pathlib import Path
import subprocess
import sys
import tempfile


def run(*args):
    subprocess.run(args, check=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("platform", choices=["linux", "windows", "macos"])
    parser.add_argument("--install-deps", action="store_true")
    parser.add_argument("--check-only", action="store_true")
    parser.add_argument("--smoke-installers", action="store_true")
    args = parser.parse_args()
    os.environ.setdefault("CARGO_BUILD_JOBS", "1" if args.platform == "macos" else "2")
    os.environ.update(CARGO_FULL_PROFILE="release-desktop", CARGO_BIN_NAME="warpeer", WARP_APP_NAME="Warpeer")
    os.environ.setdefault("GIT_RELEASE_TAG", "development")
    if args.platform == "macos":
        if args.install_deps:
            run("brew", "install", "protobuf", "pkg-config", "brotli")
        clang = subprocess.check_output(["xcrun", "--find", "clang"], text=True).strip()
        sdk = subprocess.check_output(["xcrun", "--sdk", "macosx", "--show-sdk-path"], text=True).strip()
        os.environ.update(CLANG_PATH=clang, LIBCLANG_PATH=str(Path(clang).parent.parent / "lib"),
                          SDKROOT=sdk, BINDGEN_EXTRA_CLANG_ARGS=f"-isysroot {sdk}", MACOSX_DEPLOYMENT_TARGET="11.0")
    elif args.platform == "windows":
        if args.install_deps:
            run("choco", "install", "protoc", "nsis", "--yes", "--no-progress")
        os.environ.setdefault("LIBCLANG_PATH", r"C:\Program Files\LLVM\bin")
        os.environ["PATH"] += os.pathsep + r"C:\ProgramData\chocolatey\bin"
    host = {"linux": "x86_64-unknown-linux-gnu", "windows": "x86_64-pc-windows-msvc", "macos": "aarch64-apple-darwin"}[args.platform]
    details = subprocess.check_output(["rustc", "-vV"], text=True)
    if f"host: {host}" not in details:
        raise SystemExit(f"Expected native Rust host {host}")
    # Dagger's Cargo cache is shared across concurrent calls. Standalone rustc
    # does not take Cargo's lock, so its test executable must be per invocation.
    with tempfile.TemporaryDirectory(prefix="warpeer-runbooks-") as temporary:
        tests = str(Path(temporary) / ("runbooks.exe" if args.platform == "windows" else "runbooks"))
        run("rustc", "--edition=2024", "--test", "app/src/local_runbooks/catalog.rs", "-o", tests)
        run(tests)
    run("cargo", "test", "--locked", "-p", "warpeer_update")
    if os.environ["GIT_RELEASE_TAG"].startswith("v"):
        run(sys.executable, "script/warpeer/sign-release.py", "--validate")
    if args.check_only:
        return
    run("cargo", "build", "--locked", "-p", "warp", "--bin", "warpeer", "--no-default-features", "--features", "warpeer", "--profile", "release-desktop")
    if args.platform == "windows":
        # The helper runs from a temporary directory without the app's VC runtime DLLs.
        helper_env = os.environ.copy()
        helper_env["RUSTFLAGS"] = helper_env.get("RUSTFLAGS", "") + " -C target-feature=+crt-static"
        # Explicit --target keeps CRT-static flags off host proc-macro crates.
        subprocess.run(["cargo", "build", "--locked", "-p", "warpeer_update", "--bin", "warpeer-update-helper", "--target", host, "--profile", "release-desktop"], env=helper_env, check=True)
        shutil.copy2(Path("target") / host / "release-desktop/warpeer-update-helper.exe",
                     "target/release-desktop/warpeer-update-helper.exe")
    run(sys.executable, "script/warpeer/package-desktop.py", args.platform)
    if args.smoke_installers and args.platform == "windows":
        import winreg
        # Ephemeral CI runner only: exercises installation, quoted uninstall
        # registration, required payload files, and uninstallation.
        with tempfile.TemporaryDirectory(prefix="warpeer-installer-test-") as temporary:
            installed = Path(temporary) / "Warpeer"
            setup = Path("dist/warpeer-windows-x86_64-setup.exe").resolve()
            try:
                run(str(setup), "/S", f"/D={installed}")
                for filename in ["warpeer.exe", "warpeer-update-helper.exe", "Uninstall.exe", "resources/bundled/metadata/version.json"]:
                    if not (installed / filename).is_file():
                        raise RuntimeError(f"Installer is missing {filename}")
                with winreg.OpenKey(winreg.HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Warpeer") as key:
                    command = winreg.QueryValueEx(key, "UninstallString")[0]
                    if command != f'"{installed / "Uninstall.exe"}"':
                        raise RuntimeError("Uninstall command is not correctly quoted")
            finally:
                if (installed / "Uninstall.exe").exists():
                    run(str(installed / "Uninstall.exe"), "/S", f"_?={installed}")
            if (installed / "warpeer.exe").exists():
                raise RuntimeError("Uninstaller left the executable behind")


if __name__ == "__main__":
    main()
