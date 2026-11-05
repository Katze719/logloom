#!/usr/bin/env bash
set -euo pipefail

scripts=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
expect_failure() {
  if "$@" > "$work/error" 2>&1; then
    echo "Expected failure: $*" >&2
    exit 1
  fi
}

# Exercise Cargo metadata, annotated tags, and changed tag targets.
mkdir -p "$work/repo/src"
cd "$work/repo"
printf '[package]\nname="logloom"\nversion="0.1.2"\nedition="2024"\n' > Cargo.toml
touch src/lib.rs
cargo generate-lockfile --offline --quiet
git -c init.defaultBranch=main init --quiet
git config user.name 'Release test'
git config user.email 'release@example.invalid'
git add Cargo.toml Cargo.lock src/lib.rs
git commit --quiet -m First
git tag -a v0.1.2 -m Release
env -u RELEASE_VERSION GITHUB_OUTPUT="$work/outputs" bash "$scripts/check-release.sh" v0.1.2
grep -Fxq 'version=0.1.2' "$work/outputs"
grep -Fxq "sha=$(git rev-parse HEAD)" "$work/outputs"
expect_failure env RELEASE_VERSION=0.1.2 bash "$scripts/check-release.sh" v0.1.3
expect_failure env RELEASE_VERSION=0.1.2-rc.1 bash "$scripts/check-release.sh" v0.1.2-rc.1
git commit --quiet --allow-empty -m Second
expect_failure env RELEASE_VERSION=0.1.2 bash "$scripts/check-release.sh" v0.1.2
grep -q 'no longer points' "$work/error"

# Check duplicate publishing and registry errors without network access.
mkdir -p "$work/bin" target/package
cat > "$work/bin/curl" <<'CURL'
#!/usr/bin/env bash
set -euo pipefail
while (($#)); do
  case "$1" in
    --output) output=$2; shift 2 ;;
    *) shift ;;
  esac
done
[[ $TEST_HTTP_STATUS != network-error ]] || exit 6
cp "$TEST_INDEX" "$output"
printf '%s' "$TEST_HTTP_STATUS"
CURL
chmod +x "$work/bin/curl"
export PATH="$work/bin:$PATH" TEST_INDEX="$work/index" TEST_HTTP_STATUS=200
export RELEASE_VERSION=0.1.2 GITHUB_OUTPUT="$work/outputs"
printf 'crate contents' > target/package/logloom-0.1.2.crate
digest=$(sha256sum target/package/logloom-0.1.2.crate | cut -d ' ' -f 1)
printf '{"vers":"0.1.2","cksum":"%s"}\n' "$digest" > "$TEST_INDEX"
bash "$scripts/publish-status.sh"
grep -Fxq 'publish=false' "$GITHUB_OUTPUT"
printf '{"vers":"0.1.2","cksum":"different"}\n' > "$TEST_INDEX"
expect_failure bash "$scripts/publish-status.sh"
grep -q 'different contents' "$work/error"
printf '{"vers":"0.1.1","cksum":"older"}\n' > "$TEST_INDEX"
bash "$scripts/publish-status.sh"
grep -Fxq 'publish=true' "$GITHUB_OUTPUT"
for status in 503 network-error; do
  expect_failure env TEST_HTTP_STATUS="$status" bash "$scripts/publish-status.sh"
done
: > "$GITHUB_OUTPUT"
TEST_HTTP_STATUS=404 bash "$scripts/publish-status.sh"
grep -Fxq 'publish=true' "$GITHUB_OUTPUT"
printf 'invalid json' > "$TEST_INDEX"
expect_failure bash "$scripts/publish-status.sh"

# Package all architectures and verify the final payload and Windows ZIP.
for target in x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-pc-windows-msvc; do
  artifact="$work/artifacts/logloom-$target"
  mkdir -p "$artifact"
  case "$target" in
    x86_64-unknown-*) deb=amd64; rpm=x86_64 ;;
    aarch64-unknown-*) deb=arm64; rpm=aarch64 ;;
    x86_64-pc-*)
      names=(logloom.exe logloom-0.1.2-windows-x64.msi logloom.wixpdb)
      ;;
  esac
  if [[ $target != x86_64-pc-* ]]; then
    names=(logloom "logloom-0.1.2-$target.tar.gz" "logloom_0.1.2-1_$deb.deb" "logloom-0.1.2-1.$rpm.rpm")
  fi
  for name in LICENSE README.md "${names[@]}"; do
    printf '%s: %s\n' "$target" "$name" > "$artifact/$name"
  done
  (cd "$artifact" && sha256sum ./* > SHA256SUMS)
  sed 's/$/\r/' "$artifact/SHA256SUMS" > "$work/manifest"
  cp "$work/manifest" "$artifact/SHA256SUMS"
done
bash "$scripts/release-assets.sh" "$work/artifacts" "$work/assets" > /dev/null
files=("$work/assets/"*)
[[ ${#files[@]} == 13 ]]
(cd "$work/assets" && sha256sum --check --strict SHA256SUMS > /dev/null)
cmp "$work/artifacts/logloom-x86_64-unknown-linux-musl/logloom" "$work/assets/logloom-0.1.2-x86_64-unknown-linux-musl"
unzip -p "$work/assets/logloom-0.1.2-x86_64-pc-windows-msvc.zip" logloom.exe > "$work/unzipped.exe"
cmp "$work/artifacts/logloom-x86_64-pc-windows-msvc/logloom.exe" "$work/unzipped.exe"
[[ ! -e $work/assets/logloom.wixpdb ]]
artifact="$work/artifacts/logloom-x86_64-unknown-linux-musl"
cp "$artifact/logloom" "$work/original-binary"
printf 'tampered' > "$artifact/logloom"
expect_failure bash "$scripts/release-assets.sh" "$work/artifacts" "$work/tampered"
grep -q FAILED "$work/error"
cp "$work/original-binary" "$artifact/logloom"
(cd "$artifact" && sha256sum LICENSE > SHA256SUMS)
expect_failure bash "$scripts/release-assets.sh" "$work/artifacts" "$work/unlisted"
grep -q 'missing from its checksum manifest' "$work/error"
printf '%s  ../outside\n' "$digest" > "$artifact/SHA256SUMS"
expect_failure bash "$scripts/release-assets.sh" "$work/artifacts" "$work/unsafe"
grep -q 'Invalid checksum' "$work/error"
echo 'Release checks passed.'
