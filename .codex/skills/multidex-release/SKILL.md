---
name: multidex-release
description: Build and publish ModelCloud MultiDex packages for Linux x86_64, Linux ARM64, and macOS ARM64 after a PR merges to main.
---

# Publish MultiDex after a merge

Use this workflow when a PR has merged into ModelCloud/MultiDex `main` and its installable artifacts need updating.

Before building, confirm `sccache` and `mold` are installed so the build scripts enable the compiler
cache and fast linker; the scripts skip both silently when the packages are missing. See the
`multidex-build-acceleration` skill.

1. Fetch `origin/main` and record its full commit SHA. Build every package from that exact commit in a clean checkout. A version string alone is insufficient because several merges may share a workspace version.
2. Build native packages for `x86_64-unknown-linux-gnu` with `scripts/build_multidex_linux_amd64.sh` and `aarch64-unknown-linux-gnu` with `scripts/build_multidex_linux_aarch64.sh`. For `aarch64-apple-darwin`, use `scripts/build_codex_package.py --target aarch64-apple-darwin --variant multidex --cargo-profile multidex-release` with a package directory and archive output. On macOS, raise the shell's file descriptor limit (for example, `ulimit -n 4096`) and keep Cargo jobs below that limit; the default 256-descriptor limit can fail a parallel build. Set `STABLE_GIT_COMMIT` to the merged SHA for each build. Each package must contain `bin/multidex` and `bin/codex-code-mode-host`.
3. Stage the three archives as `multidex-package-<target>.tar.gz`, plus a `.sha256` file for each and a combined `SHA256SUMS`. Confirm the manifest in each archive names the matching target and `multidex` variant, and confirm every checksum before upload.
4. Publish the assets with `install-multidex.sh` to a commit-specific GitHub release tag such as `multidex-build-<version>-<shortsha>` when the workspace version already has a release. Keep existing version tags and their assets immutable. State the full source SHA in release notes and check that all three assets are downloadable after publishing.

For Linux x86_64, also publish `multidex-package-x86_64-unknown-linux-gnu-ubuntu24.04.tar.gz`
and `multidex-package-x86_64-unknown-linux-gnu-ubuntu26.04.tar.gz`, with checksums.
Build the 24.04 asset on Ubuntu 24.04 using `scripts/build_multidex_linux_amd64_ubuntu2404.sh`;
it links OpenSSL statically and checks that both binaries require at most glibc 2.39.
Use `scripts/package_multidex_linux_amd64_ubuntu2604.sh` to bundle OpenSSL with the
26.04 asset, then run both binaries on a clean Ubuntu 26.04 container. Run both
binaries from the 24.04 asset on a clean Ubuntu 24.04 container. Include the
OS-selecting `install-multidex.sh` in the release.

The Linux release workflow in `.github/workflows/multidex-release-linux.yml` is tied to version tags and covers only the two Linux targets. A new merged commit with the same version still needs this three-platform publication.

`scripts/install/install-multidex.sh` resolves `latest` by walking the published releases newest first and selecting the first one that ships the requested target, so a commit-specific release becomes installable as soon as its assets are uploaded. An explicit `--release` accepts either a version (`0.163.0`, `multidex-v0.163.0`) or a release tag (`multidex-build-0.163.0-18dbfada9`). Publish every platform before announcing a build: a release that is missing the asset for a target is skipped in favor of the next newest release that has it.
