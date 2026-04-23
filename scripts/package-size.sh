#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

bytes() {
  if stat -f '%z' "$1" >/dev/null 2>&1; then
    stat -f '%z' "$1"
  else
    stat -c '%s' "$1"
  fi
}

human() {
  local value="$1"
  awk -v bytes="$value" 'BEGIN {
    split("B KiB MiB GiB", units)
    size = bytes
    unit = 1
    while (size >= 1024 && unit < 4) {
      size = size / 1024
      unit++
    }
    if (unit == 1) {
      printf "%d %s", size, units[unit]
    } else {
      printf "%.1f %s", size, units[unit]
    }
  }'
}

cargo package --no-verify --allow-dirty >/dev/null
crate_file="$(find target/package -maxdepth 1 -name 'docsite-to-md-*.crate' -type f | sort | tail -n 1)"
crate_bytes="$(bytes "$crate_file")"

unpacked_dir="$(mktemp -d)"
trap 'rm -rf "$unpacked_dir"' EXIT
tar -xzf "$crate_file" -C "$unpacked_dir"
unpacked_kib="$(du -sk "$unpacked_dir" | awk '{print $1}')"
unpacked_bytes="$((unpacked_kib * 1024))"

cargo build --release --locked >/dev/null
release_binary="target/release/docsite-to-md"
release_bytes="$(bytes "$release_binary")"

printf '| Metric | Value | Source |\n'
printf '| --- | ---: | --- |\n'
printf '| Crate compressed size | %s | `%s` |\n' "$(human "$crate_bytes")" "$crate_file"
printf '| Crate unpacked source size | %s | package archive contents |\n' "$(human "$unpacked_bytes")"
printf '| Release binary size | %s | `%s` |\n' "$(human "$release_bytes")" "$release_binary"

if [[ "${1:-}" == "--with-browser" ]]; then
  cargo build --release --locked --features browser >/dev/null
  browser_bytes="$(bytes "$release_binary")"
  printf '| Release binary size with browser feature | %s | `%s --features browser` |\n' "$(human "$browser_bytes")" "cargo build --release --locked"
fi
