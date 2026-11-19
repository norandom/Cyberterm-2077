# Contributing

Thanks for your interest in Cyberterm 2077.

- Open an issue before starting a large change so we can agree on the approach.
- Keep Cyberterm 2077 terminal-only: changes must not add accounts, hosted services or
  telemetry, and updates must only install when the user asks.
- Run `./script/format`, `cargo clippy --workspace --all-targets --tests -- -D warnings`, and
  `./script/verify-warpeer-profile` before opening a pull request.
- Contributions are accepted under the license of the files they modify (AGPL-3.0, or MIT for
  `crates/warpui_core` and `crates/warpui`).

General terminal fixes that are not specific to Cyberterm 2077 are best contributed to upstream
[Warp](https://github.com/warpdotdev/warp) so both projects benefit.
