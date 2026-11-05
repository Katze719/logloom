#!/usr/bin/env bash
set -euo pipefail

tag=${1:?Usage: check-release.sh TAG}
version=${RELEASE_VERSION:-$(cargo metadata --locked --no-deps --format-version 1 | jq -er '.packages[0].version')}
if [[ ! $version =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ || $tag != "v$version" ]]; then
  echo "Release tag must match the stable Cargo version exactly: v$version." >&2
  exit 1
fi
sha=$(git rev-parse --verify HEAD)
if [[ $sha != "$(git rev-parse --verify "refs/tags/$tag^{commit}")" ]]; then
  echo "The tag no longer points to the checked-out commit; refusing to release." >&2
  exit 1
fi
printf 'version=%s\ntag=%s\nsha=%s\n' "$version" "$tag" "$sha" >> "${GITHUB_OUTPUT:-/dev/stdout}"
