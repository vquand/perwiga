#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
module_dir="$(cd -- "${script_dir}/.." && pwd)"
snapshot_path="${module_dir}/data/weapons.json"
output_dir="${module_dir}/assets/weapons"

command -v curl >/dev/null
command -v jq >/dev/null
command -v file >/dev/null

mkdir -p "${output_dir}"

jq -r '.weapons[] | [.source_key, .thumbnail_source_url] | @tsv' "${snapshot_path}" |
while IFS=$'\t' read -r source_key source_url; do
    destination="${output_dir}/${source_key}.png"
    if [[ -e "${destination}" ]]; then
        continue
    fi

    temporary_file="$(mktemp)"
    downloaded=false
    for attempt in 1 2 3 4 5 6; do
        if curl --fail --location --silent --show-error \
            --user-agent "Perwiga local asset curator/1.0" \
            --output "${temporary_file}" "${source_url}"; then
            downloaded=true
            break
        fi
        sleep "$((attempt * 3))"
    done
    if [[ "${downloaded}" != true ]]; then
        printf 'Unable to download %s after bounded retries\n' "${source_key}" >&2
        exit 1
    fi
    if [[ "$(file --brief --mime-type "${temporary_file}")" != "image/png" ]]; then
        printf 'Unexpected content for %s\n' "${source_key}" >&2
        exit 1
    fi
    mv --no-clobber "${temporary_file}" "${destination}"
    sleep 1
done

actual_count="$(find "${output_dir}" -maxdepth 1 -type f -name '*.png' | wc -l)"
expected_count="$(jq '.weapons | length' "${snapshot_path}")"
if [[ "${actual_count}" != "${expected_count}" ]]; then
    printf 'Expected %s weapon thumbnails, found %s\n' "${expected_count}" "${actual_count}" >&2
    exit 1
fi

printf 'Verified %s weapon thumbnails in %s\n' "${actual_count}" "${output_dir}"
