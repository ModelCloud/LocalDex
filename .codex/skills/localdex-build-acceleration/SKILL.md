---
name: localdex-build-acceleration
description: Speed up LocalDex Rust builds and test runs by installing the sccache compiler cache and mold linker before building. Use before release packaging or repeated cargo/just iteration when builds are slow.
---

# Accelerate LocalDex Rust builds

`sccache` and `mold` are not Rust dependencies, so nothing in the workspace requires them — but the
LocalDex build scripts only use them when they resolve on `PATH` and otherwise skip both silently.
Install them before any significant build, release packaging, or repeated test iteration.

Without them a full rebuild is link- and I/O-bound rather than CPU-bound: the `localdex-release`
profile sets `codegen-units = 256`, so many cores stay busy while the default `ld` serializes the
link. The tell is `sys` time far exceeding `user` time. Adding both turns an ~8 minute full rebuild
into seconds for subsequent incremental builds.

## Install

```bash
apt-get install -y sccache mold
```

`clang` must also be installed: the scripts enable the mold linker only when **both** `clang` and
`mold` resolve. Confirm all three before starting a build:

```bash
command -v sccache mold clang
```

## How the build picks them up

`scripts/build_localdex_linux_amd64.sh` and `scripts/build_localdex_linux_aarch64.sh` detect both
tools and configure them automatically:

- `sccache` becomes `RUSTC_WRAPPER`, caching under `<repo>/.cache/sccache` (30G default), and the
  script starts the server.
- `clang` + `mold` add `-C link-arg=-fuse-ld=mold` with the matching
  `CARGO_TARGET_<TARGET>_UNKNOWN_LINUX_GNU_LINKER`.

No script change is needed; installing the packages is the whole fix.

## Interactive cargo and just runs

The `just` recipes and a bare `cargo` invocation do not set these, so export the same environment
yourself when iterating outside the build scripts:

```bash
export RUSTC_WRAPPER=sccache
export SCCACHE_DIR="$PWD/.cache/sccache"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-fuse-ld=mold"
```

Keep `RUSTFLAGS` stable across runs. Cargo keys its cache on it, so changing it — or setting the
linker flags on some builds and not others — forces a full recompile and discards the benefit.

## Verify

```bash
sccache --show-stats
```

A cold run reports all cache misses; that is expected. Rebuild after touching a source file and
confirm hits appear and the wall time drops.
