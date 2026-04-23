#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

pages="${1:-100}"
concurrency="${DOCSITE_BENCH_CONCURRENCY:-8}"
tmp_root="$(mktemp -d /tmp/docsite-to-md-live-benchmark.XXXXXX)"

cleanup() {
  rm -rf "$tmp_root"
}
trap cleanup EXIT

cargo build --release --locked >/dev/null

printf '| Framework | Live target | Max pages | Exported pages | Wall time | Throughput |\n'
printf '| --- | --- | ---: | ---: | ---: | ---: |\n'

run_target() {
  local framework="$1"
  local label="$2"
  local url="$3"
  local scope_prefix="$4"
  local slug="$5"
  local result="$tmp_root/$slug.json"
  local timefile="$tmp_root/$slug.time"
  local output_dir="$tmp_root/$slug-output"

  if [[ -n "$scope_prefix" ]]; then
    /usr/bin/time -p target/release/docsite-to-md export "$url" \
      --scope-prefix "$scope_prefix" \
      --max-pages "$pages" \
      --concurrency "$concurrency" \
      --output-dir "$output_dir" >"$result" 2>"$timefile"
  else
    /usr/bin/time -p target/release/docsite-to-md export "$url" \
      --max-pages "$pages" \
      --concurrency "$concurrency" \
      --output-dir "$output_dir" >"$result" 2>"$timefile"
  fi

  local exported
  local seconds
  local throughput
  exported="$(jq '.pages | length' "$result")"
  seconds="$(awk '/^real /{print $2}' "$timefile")"
  throughput="$(awk -v pages="$exported" -v seconds="$seconds" 'BEGIN { printf "%.2f", pages / seconds }')"

  printf '| `%s` | %s | %s | %s | %.2f s | %s pages/s |\n' \
    "$framework" "$label" "$pages" "$exported" "$seconds" "$throughput"
}

run_target "GitBookModern" "[GitBook docs](https://docs.gitbook.com/)" "https://docs.gitbook.com/" "" "gitbook-modern"
run_target "Docusaurus" "[React Native docs](https://reactnative.dev/docs/getting-started)" "https://reactnative.dev/docs/getting-started" "/docs" "docusaurus-react-native"
run_target "MkDocsMaterial" "[MkDocs Material](https://squidfunk.github.io/mkdocs-material/)" "https://squidfunk.github.io/mkdocs-material/" "/mkdocs-material" "mkdocs-material"
run_target "VitePress" "[VitePress docs](https://vitepress.dev/)" "https://vitepress.dev/" "/" "vitepress"
run_target "Nextra" "[Nextra docs](https://nextra.site/)" "https://nextra.site/" "/" "nextra"
