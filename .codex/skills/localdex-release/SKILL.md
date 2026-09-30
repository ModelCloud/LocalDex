---
name: localdex-release
description: Build and publish ModelCloud LocalDex packages for Linux x86_64, Linux ARM64, and macOS ARM64 after a PR merges to main.
---

# Publish LocalDex after a merge

Use this workflow when a PR has merged into ModelCloud/LocalDex `main` and its installable artifacts need updating.

1. Fetch `origin/main` and record its full commit SHA. Build every package from that exact commit in a clean checkout. A version string alone is insufficient because several merges may share a workspace version.
2. Build native packages for `x86_64-unknown-linux-gnu` with `scripts/build_localdex_linux_amd64.sh` and `aarch64-unknown-linux-gnu` with `scripts/build_localdex_linux_aarch64.sh`. For `aarch64-apple-darwin`, use `scripts/build_codex_package.py --target aarch64-apple-darwin --variant localdex --cargo-profile localdex-release` with a package directory and archive output. On macOS, raise the shell's file descriptor limit (for example, `ulimit -n 4096`) and keep Cargo jobs below that limit; the default 256-descriptor limit can fail a parallel build. Set `STABLE_GIT_COMMIT` to the merged SHA for each build. Each package must contain `bin/localdex` and `bin/codex-code-mode-host`.
3. Stage the three archives as `localdex-package-<target>.tar.gz`, plus a `.sha256` file for each and a combined `SHA256SUMS`. Confirm the manifest in each archive names the matching target and `localdex` variant, and confirm every checksum before upload.
4. Publish the assets with `install-localdex.sh` to a commit-specific GitHub release tag such as `localdex-build-<version>-<shortsha>` when the workspace version already has a release. Keep existing version tags and their assets immutable. State the full source SHA in release notes and check that all three assets are downloadable after publishing.

The Linux release workflow in `.github/workflows/localdex-release-linux.yml` is tied to version tags and covers only the two Linux targets. A new merged commit with the same version still needs this three-platform publication.
