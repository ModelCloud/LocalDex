# MultiDex fork guidance

Preserve upstream behavior while keeping MultiDex provider routing, Responses continuation,
compaction safeguards, and portable endpoint tool support. Prefer adapting fork features to
upstream abstractions so later merges remain small.

Upstream owns conversation/context construction and formatting, including instructions,
history items, and compaction payloads. MultiDex adds local endpoint support; do not fork
those formats. Keep endpoint compatibility adaptations at the provider/transport boundary.

## Artifacts after a merge

After each PR merges into ModelCloud/MultiDex `main`, publish installable packages for Linux
x86_64, Linux ARM64, and macOS ARM64 from the same merged commit. Follow
[.codex/skills/multidex-release/SKILL.md](.codex/skills/multidex-release/SKILL.md).
For Linux x86_64, publish both Ubuntu 24.04 and 26.04 assets. The 24.04 build
must be compiled against glibc 2.39 or older and both assets must include or
statically link a compatible OpenSSL runtime.
The workspace version can remain unchanged across merges, so check the artifact's source
commit as well as its version.

Before substantial Rust builds, follow
[.codex/skills/multidex-build-acceleration/SKILL.md](.codex/skills/multidex-build-acceleration/SKILL.md).

When building from an Apple Silicon Mac (or without dedicated Linux builders), follow
[.codex/skills/multidex-build-from-mac/SKILL.md](.codex/skills/multidex-build-from-mac/SKILL.md).
It pins Rust to the channel declared in `codex-rs/rust-toolchain.toml`, requires the same compiler
on macOS and both Linux architectures, and covers the amd64 Ubuntu 24.04/26.04 and arm64 builds.

That skill is also the canonical reference for the `smolvm` VM commands used to build every
non-native target, including the flag set that breaks builds when omitted (`--net-backend
virtio-net`, an explicit `--mem`, `--net`, and `--oci-platform` with `--rosetta` for amd64).
Before publishing, it requires installing every asset from the published release and confirming the
reported SHA matches the source commit: a release missing
`multidex-package-x86_64-unknown-linux-gnu-ubuntu24.04.tar.gz` silently makes
`install-multidex.sh` on Ubuntu 24.04 fall back to an older release.
