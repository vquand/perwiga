#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
module_dir=$(cd -- "$script_dir/.." && pwd)
snapshot="$module_dir/data/items.json"
asset_dir="$module_dir/assets/items"
asset_root="https://endfield-assets.fffdan.com/vfs/Bundle/file/assets/beyond/dynamicassets/gameplay/ui/sprites"
temporary_dir=$(mktemp -d)
trap 'rm -rf -- "$temporary_dir"' EXIT

mkdir -p -- "$asset_dir"
mapfile -t icon_ids < <(jq -r '.items[].client_icon_id' "$snapshot" | LC_ALL=C sort -u)

download_icon() {
  local icon_id=$1
  if [[ ! "$icon_id" =~ ^[a-z0-9_-]+$ ]]; then
    printf 'Refusing unsafe client icon ID: %s\n' "$icon_id" >&2
    return 1
  fi

  local target="$asset_dir/$icon_id.webp"
  if [[ -e "$target" ]]; then
    return 0
  fi

  local candidate="$temporary_dir/$icon_id.webp"
  if ! curl --fail --location --silent --show-error --retry 3 \
    "$asset_root/itemicon/$icon_id.png" --output "$candidate"; then
    curl --fail --location --silent --show-error --retry 3 \
      "$asset_root/itemiconbig/$icon_id.png" --output "$candidate"
  fi
  if [[ $(LC_ALL=C dd if="$candidate" bs=1 count=4 2>/dev/null) != "RIFF" ]] ||
    [[ $(LC_ALL=C dd if="$candidate" bs=1 skip=8 count=4 2>/dev/null) != "WEBP" ]]; then
    printf 'Downloaded asset is not WebP: %s\n' "$icon_id" >&2
    return 1
  fi

  mv -- "$candidate" "$target"
}
export -f download_icon
export asset_dir asset_root temporary_dir

ready_before=$(find "$asset_dir" -maxdepth 1 -type f -name '*.webp' | wc -l)
printf '%s\n' "${icon_ids[@]}" | xargs -r -P 8 -n 1 bash -c 'download_icon "$1"' _
ready_after=$(find "$asset_dir" -maxdepth 1 -type f -name '*.webp' | wc -l)
downloaded=$((ready_after - ready_before))
skipped=$ready_before

printf 'Item thumbnails ready: %d downloaded, %d already present, %d total\n' \
  "$downloaded" "$skipped" "${#icon_ids[@]}"
