# LocalDex fork guidance

Preserve upstream behavior while keeping LocalDex provider routing, Responses continuation,
compaction safeguards, and portable endpoint tool support. Prefer adapting fork features to
upstream abstractions so later merges remain small.

Upstream owns conversation/context construction and formatting, including instructions,
history items, and compaction payloads. LocalDex adds local endpoint support; do not fork
those formats. Keep endpoint compatibility adaptations at the provider/transport boundary.

## Artifacts after a merge

After each PR merges into ModelCloud/LocalDex `main`, publish installable packages for Linux
x86_64, Linux ARM64, and macOS ARM64 from the same merged commit. Follow
[.codex/skills/localdex-release/SKILL.md](.codex/skills/localdex-release/SKILL.md).
The workspace version can remain unchanged across merges, so check the artifact's source
commit as well as its version.

Before substantial Rust builds, follow
[.codex/skills/localdex-build-acceleration/SKILL.md](.codex/skills/localdex-build-acceleration/SKILL.md).
