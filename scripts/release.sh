#!/usr/bin/env bash
# release: tag main as vMAJOR.MINOR.PATCH and push the tag. release.yml then publishes the GitHub
# release (WHITEPAPER.pdf and PICASSOPAPER.pdf attached), Zenodo's webhook archives it with .zenodo.json and mints the
# version DOI, and pages.yml redeploys the site with the new version and changelog.
# Shape from marola-dev/marola and h0ffmann/ww3-gpu's scripts/release.sh; a DOI cannot be deleted,
# so this refuses anything but a clean main level with origin, and a version that does not go up.
#   just release                    # list what is unreleased and the bump it suggests; tags nothing
#   just release patch|minor|major  # bump the latest tag
#   just release 0.1.0 [--dry-run]
#   scripts/release.sh --self-test
set -euo pipefail

valid() { [[ "$1" =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; }

# bump <vX.Y.Z> <major|minor|patch>
bump() {
  local major minor patch
  IFS=. read -r major minor patch <<<"${1#v}"
  case "$2" in
    major) echo "v$((major + 1)).0.0" ;;
    minor) echo "v$major.$((minor + 1)).0" ;;
    patch) echo "v$major.$minor.$((patch + 1))" ;;
  esac
}

# newer <a> <b>: true when version a sorts strictly after b
newer() { [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)" = "$1" ]; }

# Commit subjects on stdin (the log's `type(scope)!: ...` shape); prints the bump they call for.
# Before 1.0 a breaking change bumps the minor (semver §4: 0.y.z may change anything).
suggest() {
  local prev="$1" subjects level=patch
  subjects="$(cat)"
  grep -qE '^feat(\([^)]*\))?!?: ' <<<"$subjects" && level=minor
  if grep -qE '^[a-z]+(\([^)]*\))?!: |BREAKING CHANGE' <<<"$subjects"; then
    [ "${prev%%.*}" = v0 ] && level=minor || level=major
  fi
  echo "$level"
}

if [ "${1:-}" = --self-test ]; then
  fail() { echo "release: self-test: $*" >&2; exit 1; }
  for t in v0.1.0 v1.0.0 v10.20.30; do valid "$t" || fail "rejected $t"; done
  for t in v0.1 0.1.0 v0.1.0-rc1 v01.2.3 v1.2.3.4; do ! valid "$t" || fail "accepted $t"; done
  [ "$(bump v0.0.0 minor)" = v0.1.0 ] || fail "0.0.0 + minor"
  [ "$(bump v0.1.9 patch)" = v0.1.10 ] || fail "0.1.9 + patch"
  [ "$(bump v1.4.2 minor)" = v1.5.0 ] || fail "1.4.2 + minor"
  [ "$(bump v1.4.2 major)" = v2.0.0 ] || fail "1.4.2 + major"
  newer v0.10.0 v0.9.0 || fail "0.10.0 not newer than 0.9.0"
  ! newer v0.1.0 v0.1.0 || fail "a version is newer than itself"
  ! newer v0.1.0 v0.2.0 || fail "0.1.0 newer than 0.2.0"
  [ "$(printf 'docs: x (#1)\nfix(vm): y (#2)\n' | suggest v0.1.0)" = patch ] || fail "fix/docs -> patch"
  [ "$(printf 'fix: y\nfeat(site): z (#3)\n' | suggest v0.1.0)" = minor ] || fail "feat -> minor"
  [ "$(printf 'feat(lint)!: drop ruff (#4)\n' | suggest v0.3.0)" = minor ] || fail "breaking before 1.0 -> minor"
  [ "$(printf 'feat(lint)!: drop ruff (#4)\n' | suggest v1.3.0)" = major ] || fail "breaking after 1.0 -> major"
  echo "release: self-test ok" >&2
  exit 0
fi

dry_run=0 want=""
for a in "$@"; do
  case "$a" in
    --dry-run) dry_run=1 ;;
    *) want="$a" ;;
  esac
done

cd "$(git rev-parse --show-toplevel)"
git fetch -q --tags origin main
prev="$(git tag -l 'v*' --sort=-v:refname | head -1)"
base="${prev:-v0.0.0}"
range="${prev:+$prev..}origin/main"
subjects="$(git log --first-parent --format=%s "$range")"
[ -n "$subjects" ] || { echo "release: nothing new on main since $prev; every release is a permanent DOI, so none is cut" >&2; exit 1; }

if [ -z "$want" ]; then
  level="$(suggest "$base" <<<"$subjects")"
  echo "Unreleased since ${prev:-the first commit} ($(wc -l <<<"$subjects") commits on main):"
  head -20 <<<"$subjects" | sed 's/^/  /'
  [ "$(wc -l <<<"$subjects")" -le 20 ] || echo "  …"
  echo
  echo "Suggested: just release $level   (→ $(bump "$base" "$level"); add --dry-run to check first)"
  exit 0
fi

case "$want" in
  major | minor | patch) tag="$(bump "$base" "$want")" ;;
  *) tag="v${want#v}" ;;
esac
valid "$tag" || { echo "release: '$want' is not major, minor, patch or MAJOR.MINOR.PATCH" >&2; exit 1; }
newer "$tag" "$base" || { echo "release: $tag does not come after ${prev:-v0.0.0}" >&2; exit 1; }
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  echo "release: tag $tag already exists" >&2
  exit 1
fi
[ "$(git branch --show-current)" = main ] || { echo "release: check out main first" >&2; exit 1; }
[ -z "$(git status --porcelain --untracked-files=no)" ] \
  || { echo "release: the working tree is dirty; commit or stash first" >&2; exit 1; }
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] \
  || { echo "release: main is not level with origin/main; pull or push first" >&2; exit 1; }
nix run ./tools#citation -- --check

echo "release: $tag at $(git log -1 --format='%h %s') (previous: ${prev:-none})" >&2
if [ "$dry_run" -eq 1 ]; then
  echo "+ git tag -a $tag -m 'picasso $tag'" >&2
  echo "+ git push origin $tag" >&2
  exit 0
fi
git tag -a "$tag" -m "picasso $tag"
git push origin "$tag"
echo "release: pushed $tag. release.yml publishes it and redeploys the site; Zenodo mints the DOI a few minutes later." >&2
