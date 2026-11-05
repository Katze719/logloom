#!/usr/bin/env bash
set -euo pipefail

target=${1:?Usage: package-linux.sh TARGET}
case "$target" in
  x86_64-unknown-linux-musl) export LOGLOOM_ARCH=amd64; rpm_arch=x86_64 ;;
  aarch64-unknown-linux-musl) export LOGLOOM_ARCH=arm64; rpm_arch=aarch64 ;;
  *) echo "Unsupported Linux target: $target" >&2; exit 1 ;;
esac

export LOGLOOM_VERSION
LOGLOOM_VERSION=$(cargo metadata --locked --no-deps --format-version 1 |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["packages"][0]["version"])')
export LOGLOOM_BINARY="target/$target/release/logloom"
dist="target/dist/$target"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Static PIE may have a dynamic section for relocations, but must have neither
# an interpreter nor a shared-library dependency. Save output before grepping
# so pipefail cannot hide a failure behind SIGPIPE.
readelf --file-header "$LOGLOOM_BINARY"
readelf --program-headers "$LOGLOOM_BINARY" > "$work/program-headers"
readelf --dynamic "$LOGLOOM_BINARY" > "$work/dynamic"
if grep -q INTERP "$work/program-headers" || grep -q '(NEEDED)' "$work/dynamic"; then
  echo "The Linux binary must be statically linked with no dynamic loader." >&2
  exit 1
fi
"$LOGLOOM_BINARY" --version
"$LOGLOOM_BINARY" validate --strict CHANGELOG.md

mkdir -p "$dist" "$work/archive"
install -m 755 "$LOGLOOM_BINARY" "$dist/logloom"
install -m 644 LICENSE README.md "$dist/"
cp "$dist/logloom" "$dist/LICENSE" "$dist/README.md" "$work/archive/"
tar -czf "$dist/logloom-$LOGLOOM_VERSION-$target.tar.gz" -C "$work/archive" logloom LICENSE README.md
nfpm package --config .github/packaging/nfpm.yaml --packager deb --target "$dist/"
nfpm package --config .github/packaging/nfpm.yaml --packager rpm --target "$dist/"

# Check metadata and the exact binary that each package will install.
for deb in "$dist/"*.deb; do
  test "$(dpkg-deb --field "$deb" Architecture)" = "$LOGLOOM_ARCH"
  test -z "$(dpkg-deb --field "$deb" Depends)"
  dpkg-deb --extract "$deb" "$work/deb"
  cmp "$LOGLOOM_BINARY" "$work/deb/usr/bin/logloom"
  "$work/deb/usr/bin/logloom" validate --strict CHANGELOG.md
done
for rpm in "$dist/"*.rpm; do
  test "$(rpm --query --package --queryformat '%{ARCH}' "$rpm")" = "$rpm_arch"
  rpm --query --package --requires "$rpm" > "$work/rpm-requires"
  # rpmlib capabilities describe the package format, not runtime dependencies.
  if grep -q -v '^rpmlib(' "$work/rpm-requires"; then
    cat "$work/rpm-requires" >&2
    echo "Unexpected RPM runtime dependency." >&2
    exit 1
  fi
  rpm2cpio "$rpm" > "$work/rpm.cpio"
  mkdir "$work/rpm"
  (cd "$work/rpm" && cpio --extract --make-directories --no-absolute-filenames < "$work/rpm.cpio")
  cmp "$LOGLOOM_BINARY" "$work/rpm/usr/bin/logloom"
  "$work/rpm/usr/bin/logloom" validate --strict CHANGELOG.md
done

(cd "$dist" && sha256sum logloom LICENSE README.md ./*.tar.gz ./*.deb ./*.rpm > SHA256SUMS)
