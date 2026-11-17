# Warpeer

Warpeer is a terminal-only desktop build derived from the open-source
[Warp](https://github.com/warpdotdev/warp) client. It keeps Warp's fast GPU-rendered terminal
and removes accounts, cloud sync, hosted AI, and other hosted-service features.

## Features

- GPU-accelerated terminal rendering with IME support
- Tabs, vertical tabs, split panes, and pane layout controls
- Command history, completions, global search, and code review
- Bundled Ghostty color themes and configurable terminal motion effects
- Tab privacy with biometric unlock and pane previews
- Worktree agent control center for local CLI agents

Warpeer never sends telemetry or crash reports, does not auto-update, does not run a local HTTP
server, and does not require sign-in. It stores its data in its own `.warpeer` and application
support directories, separate from Warp.

## Desktop builds

This fork tracks [Warp upstream](https://github.com/warpdotdev/warp) directly,
while preserving Warpeer's terminal-only product profile. The current integration
includes Warp through `0f81fad401df526e440083f95efa287a2e02f5b3` (2026-11-17).

GitHub Actions builds Intel/AMD x86-64 Windows and Linux and Apple Silicon macOS.
Download artifacts from a successful [Desktop builds run](https://github.com/norandom/warpeer/actions/workflows/build-desktop.yml).
Each artifact includes a SHA-256 checksum. Extract the whole archive, then run
`warpeer.exe` on Windows or `./warpeer` on Linux. On macOS, open the DMG and copy
Warpeer to Applications. The macOS app is ad-hoc signed, not notarized; Windows
builds are unsigned. Linux builds target Ubuntu 24.04 (glibc 2.39 or newer) and
need a graphical session, fontconfig, ALSA, X11/Wayland and a working GPU driver.

## Local runbooks

Use **Add local runbook folder** and **Browse local runbooks** in the command
palette to open Markdown documents from your own folders. Shell blocks have an
explicit Run action. There is no account or synchronization requirement.
See [local runbooks](docs/local-runbooks.md) for configuration and scan limits.

## Build from source

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.100.0). Native build
dependencies are listed in `.github/workflows/build-desktop.yml`; Linux users
with Nix can enter the provided development environment with `nix develop`.

```sh
CARGO_FULL_PROFILE=release-desktop CARGO_BIN_NAME=warpeer WARP_APP_NAME=Warpeer \
  cargo build --locked -p warp --bin warpeer --no-default-features --features warpeer --profile release-desktop
python3 script/warpeer/package-desktop.py linux # or windows / macos
```

The desktop profile disables LTO to reduce build memory use. It uses the
terminal-only Warpeer features, with no login or trial mode. Before sending
changes, check the product profile:

```sh
./script/verify-warpeer-profile
```

## License

Warpeer is a modified version of Warp. The `warpui_core` and `warpui` crates are licensed under
the [MIT license](LICENSE-MIT). All other code is licensed under the
[GNU Affero General Public License v3.0](LICENSE-AGPL). See [NOTICE](NOTICE) for attribution
details.

Warpeer is an independent project. It is not affiliated with, endorsed by, or supported by
Warp (Denver Technologies, Inc.). "Warp" is a trademark of its respective owner.
