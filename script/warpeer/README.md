# Cyberterm 2077 builds, installers, and updates

The desktop workflow uses Dagger for Linux builds, checks, packaging, and release
signing. Native macOS and Windows builds use the same checked-in build driver
locally and in Actions; Apple's SDK/DMG tools and the MSVC toolchain need their
respective native hosts.

```sh
dagger call checks --source=.
dagger call linux --source=. export --path=./build/artifacts
# On the matching native host, with its build dependencies installed:
python3 script/warpeer/ci-native.py macos
python script/warpeer/ci-native.py windows
```

Use `--install-deps` on the native driver to install its Homebrew/Chocolatey
dependencies. Dagger supplies the Linux toolchain and system packages and caches
Cargo downloads and compilation. A Docker-compatible container engine is needed.
Use `dagger call linux --source=. --jobs=8 export --path=./build/artifacts` on a
machine with enough RAM for more parallel compilation. CI defaults to two jobs.
No Dagger Cloud account or subscription is required.

Release artifacts:

| Platform | Installer | Other downloads |
| --- | --- | --- |
| macOS ARM64 | compressed `.dmg` only | detached signature/checksum |
| Linux x86-64 | `.deb` | raw `.bin` and portable `.tar.gz` |
| Windows x86-64 | per-user `-setup.exe` | portable `.zip` |

Install Debian packages with `sudo apt install ./warpeer-linux-x86_64.deb`.
Their dependency list combines the built ELF's ABI requirements with the desktop
libraries loaded dynamically (EGL, Wayland, XKB). Portable archives
must be extracted into a dedicated application directory. Keep user files out
of that directory: updating replaces it in full. The raw Linux binary is for
manual deployment; use the portable archive for self-updating installs.

Settings → About offers **Check for updates** and **Update and restart**.
The app checks once a day, downloads only after an explicit update action, and
only accepts newer stable versions. It verifies signatures with an embedded
Ed25519 public key and checks the signed payload's version before replacing files.
macOS verifies the DMG before mounting it, then validates the copied app bundle.
Windows uses a helper outside the install directory and waits for the app to exit
so DLLs can be replaced. Cancellation of the normal unsaved-work/active-process
prompt cancels restart. Package-managed Debian installations offer a manual
download instead of modifying dpkg-owned files. Development builds have no stable
release identity and do not self-update.

## Configure free release signing

The workflow publishes stable `vMAJOR.MINOR.PATCH` tags only after all three
platform builds pass. Configure a GitHub environment named `release` with
appropriate tag restrictions and required reviewers before storing its key.

Generate a keypair outside the repository, keep a secure backup of the private
key, and configure GitHub (these commands require `gh` authentication):

```sh
cargo install zipsign --version '=0.2.2' --locked
umask 077
zipsign gen-key /secure/path/warpeer-release.key /secure/path/warpeer-release.pub
python3 -c 'from pathlib import Path; print(Path("/secure/path/warpeer-release.pub").read_bytes().hex())' |
  gh variable set WARPEER_UPDATE_PUBLIC_KEY --repo norandom/Cyberterm-2077
base64 -w0 /secure/path/warpeer-release.key |
  gh secret set WARPEER_RELEASE_SIGNING_KEY --env release --repo norandom/Cyberterm-2077
```

`base64 -w0` above is GNU/Linux syntax; on macOS use `base64 -i FILE | tr -d '\n'`.
The public key is a **repository variable**, available when compiling every
platform. The private key is a **release environment secret**, passed only into
the signing operation. Missing/mismatched keys abort release signing; the updater
never falls back to accepting unsigned bytes. Set `GIT_RELEASE_TAG` and
`WARPEER_UPDATE_PUBLIC_KEY` for local release-equivalent builds too.

ZIP/tar downloads carry embedded zipsign signatures. DMG, DEB, raw binary, and
Windows installer have `.sig` files. SHA-256 sidecars describe the final signed
bytes. Releases also include `warpeer-release.pub`; retain a trusted copy for
verifying subsequent releases. Example verification:

```sh
zipsign verify separate warpeer-macos-aarch64.dmg warpeer-macos-aarch64.dmg.sig warpeer-release.pub
zipsign verify tar warpeer-linux-x86_64.tar.gz warpeer-release.pub
```

Signing keys are not included in this checkout. No release is published by local
builds or Dagger calls; GitHub Actions publishes after signing. Bootstrap existing
unsigned/development installations by manually installing the first signed release.
Key rotation needs an overlap release trusting both keys; simply changing the
repository variable would prevent existing installations from updating.

These free signatures authenticate updates; they are **not** Apple Developer ID
or Windows Authenticode signatures. OS-trusted signing is deliberately out of
scope for this workflow. SmartScreen/Gatekeeper prompts can still appear. The
macOS bundle uses ad-hoc code signing to preserve bundle integrity, without
notarization. No paid service or account is configured.

## Standalone local DMG builder

Build the current `warpeer` terminal-only product for the host architecture:

```sh
./script/warpeer/build-dmg
```

The script always replaces `build/warpeer/Cyberterm2077.dmg`. The complete
Cargo target directory, staging bundle, and temporary disk image live in an
isolated temporary directory and are removed after every successful or failed
build. `build/` is intentionally ignored by Git. The default mode creates an
ad-hoc signed local artifact.

The installer window uses a native Finder background and positions Cyberterm 2077 and
Applications in a fixed layout. The disk image does not package background
artwork or a custom volume icon.

For direct distribution, install a Developer ID Application certificate and
authenticate `asc` on the build machine. Run:

```sh
WARPEER_SIGN_IDENTITY='Developer ID Application: Your Name (TEAMID)' \
./script/warpeer/build-dmg --distribution
```

Distribution mode enables hardened runtime, a secure timestamp, Apple
notarization through the ASC CLI, stapling, and Gatekeeper verification. An
Apple Development or Apple Distribution identity is not valid for direct DMG
distribution. The signing private key remains in the macOS Keychain; ASC
credentials remain in the user's ASC configuration and are never copied into
the app bundle or disk image.

The terminal-only profile skips the AI settings schema and generated license
aggregation used by Warp's full release bundle. Source licenses remain in the
checkout; use the upstream release pipeline for a public notarized artifact.

The generated application icon lives under `app/channels/warpeer/icon/`. Run
`./script/warpeer/build-icon` only after replacing its ImageGen-created
`source.png`.
