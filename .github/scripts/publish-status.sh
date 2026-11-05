#!/usr/bin/env bash
set -euo pipefail

version=${RELEASE_VERSION:?RELEASE_VERSION is required}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
status=$(curl --silent --show-error --retry 3 --max-time 30 \
  --header 'Cache-Control: no-cache' --output "$work/index" --write-out '%{http_code}' \
  https://index.crates.io/lo/gl/logloom)
case "$status" in
  200) published=$(jq -rs --arg version "$version" 'map(select(.vers == $version)) | first | .cksum // empty' "$work/index") ;;
  404) published='' ;;
  *) echo "Cannot check crates.io: HTTP $status." >&2; exit 1 ;;
esac
publish=true
if [[ -n $published ]]; then
  checksum=$(sha256sum "target/package/logloom-$version.crate" | cut -d ' ' -f 1)
  if [[ $published != "$checksum" ]]; then
    echo "$version is already published with different contents. Use a new version or the original release commit." >&2
    exit 1
  fi
  publish=false
  echo "logloom $version is already published with the same checksum; skipping upload."
fi
printf 'publish=%s\n' "$publish" >> "${GITHUB_OUTPUT:-/dev/stdout}"
