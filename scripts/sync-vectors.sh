#!/usr/bin/env bash
# Re-sync the vendored test vectors from a specification checkout, and record
# source HEAD as a baseline. This overwrites the vendored directory; it is not
# a read-only check. Review source working-tree changes and update lock provenance
# separately if needed. See test_vectors/SOURCE.md.
set -euo pipefail

SPEC="${1:-../HAPS}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$SPEC/test_vectors/v0.4"

[[ -d "$SRC" ]] || { echo "no vectors at $SRC" >&2; exit 1; }

if diff -rq "$SRC" "$HERE/test_vectors/v0.4" >/dev/null 2>&1; then
  echo "vendored vectors are in sync with $SPEC"
  exit 0
fi

echo "drift detected against $SPEC:"
diff -rq "$SRC" "$HERE/test_vectors/v0.4" || true
rm -rf "$HERE/test_vectors/v0.4"
cp -R "$SRC" "$HERE/test_vectors/v0.4"
COMMIT="$(cd "$SPEC" && git rev-parse HEAD)"
PROVENANCE_TMP="$(mktemp "$HERE/test_vectors/SOURCE.md.XXXXXX")"
sed "s/copied from commit: .*/copied from commit: ${COMMIT}/" "$HERE/test_vectors/SOURCE.md" > "$PROVENANCE_TMP"
mv "$PROVENANCE_TMP" "$HERE/test_vectors/SOURCE.md"
echo "re-synced at ${COMMIT}; review 'git diff' before committing"
