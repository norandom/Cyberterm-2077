"""Warpeer's Linux build and cross-platform release pipeline."""
import dagger
from typing import Annotated
from dagger import dag, function, object_type

Source = Annotated[dagger.Directory, dagger.Ignore([".git", "target", "dist", "build", ".dagger/sdk", ".dagger/.venv", "**/__pycache__", ".aws", ".codex", ".agents", ".env", ".env.*"])]


@object_type
class WarpeerCi:
    def base(self, source: dagger.Directory, cache: str = "target") -> dagger.Container:
        return (
            dag.container(platform=dagger.Platform("linux/amd64"))
            .from_("ubuntu:24.04")
            .with_env_variable("DEBIAN_FRONTEND", "noninteractive")
            .with_exec(["apt-get", "update"])
            .with_exec(["apt-get", "install", "-y", "--no-install-recommends",
                "ca-certificates", "curl", "git", "python3", "build-essential", "cmake",
                "pkg-config", "protobuf-compiler", "libssl-dev", "libfreetype-dev",
                "libexpat1-dev", "libgit2-dev", "libfontconfig1-dev", "libasound2-dev",
                "libclang-18-dev", "libxkbcommon-dev", "libwayland-dev", "libxcb1-dev",
                "libx11-dev", "libxi-dev", "libxcursor-dev", "dpkg-dev", "nsis"])
            # --no-install-recommends omits the well-known google/protobuf/*.proto
            # definitions required by the application's generated API clients.
            .with_exec(["apt-get", "install", "-y", "--no-install-recommends", "libprotobuf-dev"])
            .with_exec(["sh", "-ec", "curl -fsSL https://sh.rustup.rs -o /tmp/rustup.sh && sh /tmp/rustup.sh -y --profile minimal --default-toolchain none"])
            .with_env_variable("PATH", "/root/.cargo/bin:$PATH", expand=True)
            .with_env_variable("LIBCLANG_PATH", "/usr/lib/llvm-18/lib")
            .with_mounted_cache("/root/.cargo/registry", dag.cache_volume("warpeer-cargo-registry"))
            .with_mounted_cache("/root/.cargo/git", dag.cache_volume("warpeer-cargo-git"))
            # Install tools before adding sources so edits don't redownload Rust
            # or rebuild the signing CLI during local iteration.
            .with_file("/toolchain/rust-toolchain.toml", source.file("rust-toolchain.toml"))
            .with_workdir("/toolchain")
            .with_exec(["rustup", "show", "active-toolchain"])
            .with_exec(["cargo", "install", "zipsign", "--version", "=0.2.2", "--locked"])
            .with_directory("/src", source, exclude=[".git", "target", "dist", "build", ".dagger/sdk", ".dagger/.venv", "**/__pycache__"])
            .with_workdir("/src")
            .with_mounted_cache("/src/target", dag.cache_volume(f"warpeer-linux-amd64-{cache}"))
            .with_env_variable("CARGO_BUILD_JOBS", "2")
            .with_env_variable("GIT_RELEASE_TAG", "development")
        )

    @function
    async def checks(self, source: Source) -> str:
        """Run updater/runbook tests and real signature/installer regression tests."""
        return await (
            self.base(source, "checks")
            .with_exec(["python3", "script/warpeer/ci-native.py", "linux", "--check-only"])
            .with_exec(["python3", "script/warpeer/test-release.py"])
            .with_exec(["python3", "script/warpeer/test-installers.py"])
            .stdout()
        )

    @function
    def linux(self, source: Source, tag: str = "development", public_key: str = "", jobs: int = 2) -> dagger.Directory:
        """Build Linux DEB, raw binary and portable archive with the pinned Rust toolchain."""
        if jobs < 1:
            raise ValueError("jobs must be positive")
        return (
            self.base(source)
            .with_env_variable("CARGO_BUILD_JOBS", str(jobs))
            .with_env_variable("GIT_RELEASE_TAG", tag)
            .with_env_variable("WARPEER_UPDATE_PUBLIC_KEY", public_key)
            .with_exec(["python3", "script/warpeer/ci-native.py", "linux"])
            .directory("/src/dist")
        )

    @function
    def sign(self, source: Source, artifacts: dagger.Directory,
             tag: str, public_key: str, signing_key: dagger.Secret) -> dagger.Directory:
        """Sign a complete release asset set; never publishes it."""
        return (
            self.base(source)
            .with_directory("/src/dist", artifacts)
            .with_env_variable("GIT_RELEASE_TAG", tag)
            .with_env_variable("WARPEER_UPDATE_PUBLIC_KEY", public_key)
            .with_secret_variable("WARPEER_RELEASE_SIGNING_KEY", signing_key)
            .with_exec(["python3", "script/warpeer/sign-release.py"])
            .directory("/src/dist")
        )
