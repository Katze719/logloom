#!/usr/bin/env bash
set -euo pipefail

version=${RELEASE_VERSION:?RELEASE_VERSION is required}
artifacts=$(realpath "${1:?Usage: release-assets.sh ARTIFACTS OUTPUT}")
output=$(realpath -m "${2:?Usage: release-assets.sh ARTIFACTS OUTPUT}")
mkdir -- "$output"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

verify_artifact() {
  sed 's/\r$//' "$artifact/SHA256SUMS" > "$work/SHA256SUMS"
  # Our packagers emit basenames, optionally prefixed with ./.
  while IFS= read -r line || [[ -n $line ]]; do
    if [[ ! $line =~ ^[0-9a-f]{64}\ \ (\./)?[a-zA-Z0-9_.-]+$ ]]; then
      echo "Invalid checksum entry in $artifact." >&2
      exit 1
    fi
  done < "$work/SHA256SUMS"
  (cd "$artifact" && sha256sum --check --strict "$work/SHA256SUMS")
}

copy_asset() {
  local name=$1 destination=$2 checksum
  checksum=$(sha256sum "$artifact/$name" | cut -d ' ' -f 1)
  if ! grep -Fxq -- "$checksum  $name" "$work/SHA256SUMS" &&
     ! grep -Fxq -- "$checksum  ./$name" "$work/SHA256SUMS"; then
    echo "Asset is missing from its checksum manifest: $artifact/$name." >&2
    exit 1
  fi
  cp -- "$artifact/$name" "$destination"
}

for target in x86_64-unknown-linux-musl aarch64-unknown-linux-musl; do
  case "$target" in
    x86_64-*) deb_arch=amd64; rpm_arch=x86_64 ;;
    aarch64-*) deb_arch=arm64; rpm_arch=aarch64 ;;
  esac
  artifact="$artifacts/logloom-$target"
  verify_artifact
  copy_asset logloom "$output/logloom-$version-$target"
  for name in "logloom-$version-$target.tar.gz" "logloom_$version-1_$deb_arch.deb" "logloom-$version-1.$rpm_arch.rpm"; do
    copy_asset "$name" "$output/$name"
  done
done

target=x86_64-pc-windows-msvc
artifact="$artifacts/logloom-$target"
verify_artifact
copy_asset logloom.exe "$output/logloom-$version-$target.exe"
copy_asset "logloom-$version-windows-x64.msi" "$output/logloom-$version-windows-x64.msi"
mkdir "$work/zip"
for name in logloom.exe LICENSE README.md; do
  copy_asset "$name" "$work/zip/$name"
done
(cd "$work/zip" && zip -q "$output/logloom-$version-$target.zip" logloom.exe LICENSE README.md)
cp -- "$work/zip/LICENSE" "$output/LICENSE"
(cd "$output" && sha256sum ./* > SHA256SUMS)
