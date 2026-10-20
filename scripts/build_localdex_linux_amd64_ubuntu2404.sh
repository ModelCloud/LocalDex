#!/usr/bin/env bash
# Build a LocalDex package on Ubuntu 24.04 with OpenSSL linked into both
# executables. The glibc ceiling is checked before the archive is published.
set -euo pipefail

repo_root="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-unknown-linux-gnu"
dist_dir="${LOCALDEX_DIST_DIR:-${repo_root}/dist/localdex-ubuntu2404}"

if [[ "$(. /etc/os-release; printf '%s' "$VERSION_ID")" != "24.04" ]]; then
    echo "Build this package on Ubuntu 24.04." >&2
    exit 1
fi

export OPENSSL_STATIC=1
export OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
export OPENSSL_INCLUDE_DIR=/usr/include
export LOCALDEX_DIST_DIR="${dist_dir}"

"${repo_root}/scripts/build_localdex_linux_amd64.sh"

for binary in "${dist_dir}/package/bin/localdex" "${dist_dir}/package/bin/codex-code-mode-host"; do
    [[ -x "${binary}" ]] || { echo "Missing ${binary}" >&2; exit 1; }
    if readelf -d "${binary}" | grep -Eq 'NEEDED.*lib(ssl|crypto)\.so'; then
        echo "${binary} still requires system OpenSSL" >&2
        exit 1
    fi
    objdump -T "${binary}" | python3 -c '
import re
import sys
versions = [tuple(map(int, match.groups())) for line in sys.stdin
            if (match := re.search(r"GLIBC_(\d+)\.(\d+)", line))]
if max(versions, default=(0, 0)) > (2, 39):
    raise SystemExit(f"Binary requires GLIBC {max(versions)} > 2.39")
'
done

version="${LOCALDEX_PACKAGE_VERSION:-}"
if [[ -z "${version}" ]]; then
    version="$(python3 -c 'import tomllib,sys; print(tomllib.load(open(sys.argv[1], "rb"))["workspace"]["package"]["version"])' "${repo_root}/codex-rs/Cargo.toml")"
fi
archive="${dist_dir}/localdex-${version}-${target}.tar.gz"
asset="${dist_dir}/localdex-package-${target}-ubuntu24.04.tar.gz"
cp "${archive}" "${asset}"
(cd "${dist_dir}" && sha256sum "$(basename "${asset}")" > "$(basename "${asset}").sha256")
printf 'Ubuntu 24.04 package: %s\n' "${asset}"
