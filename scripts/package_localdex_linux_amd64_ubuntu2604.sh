#!/usr/bin/env bash
# Add an isolated OpenSSL runtime to a Linux x86_64 LocalDex package built on
# Ubuntu 26.04. Run this with an OpenSSL 3.5+ runtime that supports the binary.
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "Usage: $0 BASE_ARCHIVE OPENSSL_LIB_DIR OUTPUT_DIR" >&2
    exit 2
fi

base_archive="$(realpath "$1")"
openssl_dir="$(realpath "$2")"
output_dir="$(realpath -m "$3")"
target="x86_64-unknown-linux-gnu"

command -v patchelf >/dev/null || { echo "patchelf is required" >&2; exit 1; }
for library in libssl.so.3 libcrypto.so.3; do
    [[ -f "${openssl_dir}/${library}" ]] || { echo "Missing ${library}" >&2; exit 1; }
done

work_dir="$(mktemp -d)"
trap 'rm -rf "${work_dir}"' EXIT
mkdir -p "${work_dir}/package/lib" "${output_dir}"
tar -xzf "${base_archive}" -C "${work_dir}/package"

version="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "${work_dir}/package/codex-package.json")"
manifest_target="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["target"])' "${work_dir}/package/codex-package.json")"
[[ "${manifest_target}" == "${target}" ]] || { echo "Wrong package target" >&2; exit 1; }

cp "${openssl_dir}/libssl.so.3" "${openssl_dir}/libcrypto.so.3" "${work_dir}/package/lib/"
for binary in "${work_dir}/package/bin/localdex" "${work_dir}/package/bin/codex-code-mode-host"; do
    [[ -x "${binary}" ]] || { echo "Missing ${binary}" >&2; exit 1; }
    patchelf --set-rpath '$ORIGIN/../lib' "${binary}"
    [[ "$(patchelf --print-rpath "${binary}")" == '$ORIGIN/../lib' ]] || exit 1
done

asset="${output_dir}/localdex-package-${target}-ubuntu26.04.tar.gz"
tar -C "${work_dir}/package" -czf "${asset}" .
(cd "${output_dir}" && sha256sum "$(basename "${asset}")" > "$(basename "${asset}").sha256")
printf 'Ubuntu 26.04 package: %s (LocalDex %s)\n' "${asset}" "${version}"
