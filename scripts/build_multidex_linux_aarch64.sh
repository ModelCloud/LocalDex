#!/usr/bin/env bash
# Build a distributable MultiDex package for ARM64 Linux with native tools.
# The package includes multidex and its codex-code-mode-host companion.
set -euo pipefail

repo_root="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
target="aarch64-unknown-linux-gnu"
target_dir="${CARGO_TARGET_DIR:-${repo_root}/codex-rs/target-multidex-release-aarch64}"
dist_dir="${MULTIDEX_DIST_DIR:-${repo_root}/dist/multidex-${target}}"
cargo_bin="${MULTIDEX_CARGO:-cargo}"
package_version="${MULTIDEX_PACKAGE_VERSION:-$(python3 -c 'import sys, tomllib; print(tomllib.load(open(sys.argv[1], "rb"))["workspace"]["package"]["version"])' "${repo_root}/codex-rs/Cargo.toml")}"

export CARGO_TARGET_DIR="${target_dir}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$(nproc)}"
export STABLE_GIT_COMMIT="$(git -C "${repo_root}" rev-parse HEAD)"

if command -v sccache >/dev/null 2>&1; then
    export SCCACHE_DIR="${SCCACHE_DIR:-${repo_root}/.cache/sccache}"
    export SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-30G}"
    export RUSTC_WRAPPER="${RUSTC_WRAPPER:-sccache}"
    sccache --start-server >/dev/null 2>&1 || true
fi

if [ "$(uname -m)" = "aarch64" ] \
    && command -v clang >/dev/null 2>&1 \
    && command -v mold >/dev/null 2>&1; then
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="clang"
    export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-C link-arg=-fuse-ld=mold"
fi

mkdir -p "${dist_dir}"
CODEX_REPO_ROOT="${repo_root}" python3 "${repo_root}/scripts/build_codex_package.py" \
    --cargo "${cargo_bin}" \
    --target "${target}" \
    --variant multidex \
    --cargo-profile multidex-release \
    --package-version "${package_version}" \
    --package-dir "${dist_dir}/package" \
    --archive-output "${dist_dir}/multidex-${package_version}-${target}.tar.gz" \
    --force

printf 'MultiDex package: %s\n' "${dist_dir}/multidex-${package_version}-${target}.tar.gz"
