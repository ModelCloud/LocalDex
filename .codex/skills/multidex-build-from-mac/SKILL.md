---
name: multidex-build-from-mac
description: Build ModelCloud MultiDex packages for Linux amd64 (Ubuntu 24.04 and 26.04), Linux arm64, and macOS arm64 entirely from an Apple Silicon Mac using smolvm VMs. Use when a release must be built without dedicated Linux builders, or when a Mac is the only available build host.
---

# Build every MultiDex target from one Apple Silicon Mac

This is the supported path when no dedicated Linux builder is available. It builds all four
assets from a single Mac and keeps the Rust toolchain pinned identically across platforms.

Never build these targets with qemu user emulation (`docker --platform linux/amd64` on an arm64
host). qemu mis-executes the `aws-lc-sys` assembly and fails with `SIGSEGV` during the C/asm
compile. Also avoid `smolvm ... --net-backend tsi` for builds: TSI throughput stalls cargo
downloads. Use `virtio-net`.

## Pin Rust to the repo's declared version

`codex-rs/rust-toolchain.toml` declares the channel the workspace expects. Read it, do not guess:

```bash
git show <SHA>:codex-rs/rust-toolchain.toml   # channel = "1.99.1" at 721a79a74
```

All hosts and both guest architectures must report the **same** rustc commit. Verify explicitly:

```bash
rustc +1.99.1 --version    # 1.99.1 (58cf8f9c3 2026-10-13)
```

A toolchain directory can exist yet be stale. If `rustc +1.99.1` prints a different version than
the directory name suggests, reinstall it (`rustup toolchain uninstall 1.99.1-<host>` then
`rustup toolchain install 1.99.1-<host> --profile minimal`).

On macOS, `~/.local/bin/rustc` and `~/.local/bin/cargo` must be rustup shims. If they are direct
symlinks into `~/.rustup/toolchains/stable-*`, they bypass both `rust-toolchain.toml` and
`RUSTUP_TOOLCHAIN`, silently building with the wrong compiler:

```bash
ln -sf /opt/homebrew/bin/rustup ~/.local/bin/rustc
ln -sf /opt/homebrew/bin/rustup ~/.local/bin/cargo
```

Build scripts should also prepend the pinned toolchain's own bin directory so that `cargo <sub>`
works regardless of shim resolution:

```bash
export PATH="$HOME/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:$PATH"
```

## Dependencies

Common to every Linux guest:

```bash
apt-get install -y build-essential pkg-config libssl-dev clang mold patchelf \
                   cmake python3 curl git ca-certificates xz-utils libcap-dev
```

`libcap-dev` is required: `codex-bwrap`'s build script probes `libcap` via pkg-config for Linux
targets and fails hard without it. `sccache` and `mold` are picked up automatically when present
(see the `multidex-build-acceleration` skill).

## Target 1 — macOS arm64 (build directly on the Mac)

Native, no VM:

```bash
export CARGO_TARGET_DIR=$repo/codex-rs/target-multidex-release-darwin
export STABLE_GIT_COMMIT=<full-sha>
export CODEX_REPO_ROOT=$repo
ulimit -n 4096      # raise before parallel builds; the default 256 fails
python3 scripts/build_codex_package.py \
  --target aarch64-apple-darwin --variant multidex \
  --cargo-profile multidex-release --package-version 0.164.0 \
  --package-dir "$dist/package" \
  --archive-output "$dist/multidex-0.164.0-aarch64-apple-darwin.tar.gz" --force
cp "$dist/multidex-0.164.0-aarch64-apple-darwin.tar.gz" \
   "$dist/multidex-package-aarch64-apple-darwin.tar.gz"
(cd "$dist" && shasum -a 256 multidex-package-aarch64-apple-darwin.tar.gz \
   > multidex-package-aarch64-apple-darwin.tar.gz.sha256)
```

Run long builds inside `tmux`, not a bare background job; macOS has no `setsid` and a job dies
with its parent shell.

## Target 2 — Linux arm64 (smolvm arm64 guest)

Create a persistent arm64 VM with a host directory mounted for output:

```bash
smolvm machine create --name mdlinux --net --net-backend virtio-net \
  -I ubuntu:24.04 --cpus 12 --mem 28672 --storage 60 --overlay 40 \
  -v /path/to/artifacts:/out
smolvm machine start --name mdlinux
smolvm machine exec --name mdlinux -- <command>
```

Clone inside the guest rather than bind-mounting a host git worktree: a worktree's `.git` points
at host-absolute paths and `git rev-parse` fails in the guest.

```bash
git clone https://github.com/ModelCloud/MultiDex.git /root/src
cd /root/src && git checkout <full-sha> && git rev-parse HEAD
export PATH=/root/.cargo/bin:$PATH
scripts/build_multidex_linux_aarch64.sh
```

Then stage `multidex-package-aarch64-unknown-linux-gnu.tar.gz` plus its `.sha256`.

## Target 3 — Linux amd64, Ubuntu 24.04 asset (smolvm Rosetta guest)

Rosetta runs genuine x86_64 code (verify with `uname -m` and `file`: `ELF 64-bit ... x86-64`)
and does not suffer the qemu assembly crashes. `machine create` has no platform flag, so use
`machine run` with `--oci-platform`:

```bash
smolvm machine run --net --net-backend virtio-net --rosetta \
  --oci-platform linux/amd64 --image ubuntu:24.04 \
  -v /path/to/artifacts:/out -- bash /out/build_x86.sh
```

Inside that guest, then:

```bash
export CARGO_TARGET_DIR=/work/codex-rs/target-multidex-release
export MULTIDEX_DIST_DIR=/out/x86/ubuntu2404
scripts/build_multidex_linux_amd64_ubuntu2404.sh
```

That script hard-requires Ubuntu 24.04, links OpenSSL statically, and fails the build if either
binary needs glibc newer than 2.39 or still depends on system `libssl`/`libcrypto`.

## Target 4 — Linux amd64, Ubuntu 26.04 asset

Built from the generic amd64 archive plus a bundled OpenSSL 3.5 runtime, patched to a relative
rpath:

```bash
scripts/build_multidex_linux_amd64.sh        # generic base archive
scripts/package_multidex_linux_amd64_ubuntu2604.sh \
  "$dist/amd64-base/multidex-0.164.0-x86_64-unknown-linux-gnu.tar.gz" \
  /path/to/openssl-3.5.3/lib \
  "$dist/ubuntu2604"
```

The OpenSSL directory must contain `libssl.so.3` and `libcrypto.so.3` for x86-64 (an x86-64 build
host's `/fast/openssl-3.5.3/lib` works; a bare or `/lib`-nested path is a common mistake — check
the files exist before invoking the script). `patchelf` is required.

## Verify every archive before publishing

Each archive must report the right variant/target and contain both executables:

```bash
python3 - <<'PY'
import tarfile, json
p = "multidex-package-<target>.tar.gz"
with tarfile.open(p) as t:
    names = t.getnames()
    manifest = next(n for n in names if n.endswith("codex-package.json"))
    d = json.load(t.extractfile(manifest))
    print(d["variant"], d["target"], d["version"])
    print(any(n.endswith("bin/multidex") for n in names))
    print(any(n.endswith("bin/codex-code-mode-host") for n in names))
PY
sha256sum -c multidex-package-<target>.tar.gz.sha256
```

Publish each asset as soon as it verifies. `install-multidex.sh` resolves `latest` by walking
releases newest-first and picking the first that ships the requested target, so a partial release
is safe to publish and becomes usable per-platform as assets land. Publish a draft release, upload
assets, then flip it live once all four are present.

## Quick reference

| Target | Where | Script | Asset |
|---|---|---|---|
| macOS arm64 | Mac, native | `scripts/build_codex_package.py --variant multidex` | `multidex-package-aarch64-apple-darwin.tar.gz` |
| Linux arm64 | smolvm arm64 | `scripts/build_multidex_linux_aarch64.sh` | `multidex-package-aarch64-unknown-linux-gnu.tar.gz` |
| Linux amd64 24.04 | smolvm Rosetta | `scripts/build_multidex_linux_amd64_ubuntu2404.sh` | `multidex-package-x86_64-unknown-linux-gnu-ubuntu24.04.tar.gz` |
| Linux amd64 26.04 | smolvm Rosetta | `scripts/build_multidex_linux_amd64.sh` + `scripts/package_multidex_linux_amd64_ubuntu2604.sh` | `multidex-package-x86_64-unknown-linux-gnu-ubuntu26.04.tar.gz` |
