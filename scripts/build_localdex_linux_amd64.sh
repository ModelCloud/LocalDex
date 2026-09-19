#!/usr/bin/env bash
# Build a distributable LocalDex package for Linux x86_64 with fast, reusable
# compiler and linker settings. The output contains localdex and its required
# codex-code-mode-host companion binary.
set -euo pipefail

repo_root="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-unknown-linux-gnu"
target_dir="${CARGO_TARGET_DIR:-${repo_root}/codex-rs/target-localdex-release}"
dist_dir="${LOCALDEX_DIST_DIR:-${repo_root}/dist/localdex-${target}}"
package_version="${LOCALDEX_PACKAGE_VERSION:-$(python3 -c 'import sys, tomllib; print(tomllib.load(open(sys.argv[1], "rb"))["workspace"]["package"]["version"])' "${repo_root}/codex-rs/Cargo.toml")}"

export CARGO_TARGET_DIR="${target_dir}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$(nproc)}"
export STABLE_GIT_COMMIT="$(git -C "${repo_root}" rev-parse HEAD)"

if command -v sccache >/dev/null 2>&1; then
    export SCCACHE_DIR="${SCCACHE_DIR:-${repo_root}/.cache/sccache}"
    export SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-30G}"
    export RUSTC_WRAPPER="${RUSTC_WRAPPER:-sccache}"
    # A previous package build may already own the daemon socket.
    sccache --start-server >/dev/null 2>&1 || true
fi

if command -v clang >/dev/null 2>&1 && command -v mold >/dev/null 2>&1; then
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="clang"
    export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-C link-arg=-fuse-ld=mold"
fi

mkdir -p "${dist_dir}"
CODEX_REPO_ROOT="${repo_root}" python3 "${repo_root}/scripts/build_codex_package.py" \
    --target "${target}" \
    --variant localdex \
    --cargo-profile localdex-release \
    --package-version "${package_version}" \
    --package-dir "${dist_dir}/package" \
    --archive-output "${dist_dir}/localdex-${package_version}-${target}.tar.gz" \
    --force

printf 'LocalDex package: %s\n' "${dist_dir}/localdex-${package_version}-${target}.tar.gz"
