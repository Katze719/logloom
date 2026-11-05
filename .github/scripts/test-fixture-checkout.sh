#!/usr/bin/env bash
set -euo pipefail

repo=$(git rev-parse --show-toplevel)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Exercise Git's Windows-style conversion on any CI host, using an isolated index.
mkdir -p "$work/source/tests" "$work/checkout"
cp "$repo/.gitattributes" "$work/source/.gitattributes"
cp -R "$repo/tests/fixtures" "$work/source/tests/fixtures"
git -C "$work/source" -c init.defaultBranch=main init --quiet
git -C "$work/source" -c core.autocrlf=false add .
git -C "$work/source" -c core.autocrlf=true checkout-index \
  --all --prefix="$work/checkout/"
git -C "$work/source" ls-files -z tests/fixtures > "$work/fixtures"

while IFS= read -r -d '' fixture; do
  cmp "$work/source/$fixture" "$work/checkout/$fixture"
done < "$work/fixtures"

echo 'Fixture bytes survive core.autocrlf=true checkouts.'
