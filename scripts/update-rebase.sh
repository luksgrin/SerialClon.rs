#!/usr/bin/env bash
# Refresh the bundled restriction-enzyme database from REBASE.
#
# REBASE is maintained by Dr. Richard J. Roberts (New England Biolabs) and is free
# for academic use. The `striderc` flavour is the DNA Strider / Serial Cloner format:
#
#     Name,recognition_site_with_/,
#     #Name,recognition_site,cut_top,cut_bottom,     <- type IIs (cuts outside the site)
#
# Usage: scripts/update-rebase.sh
set -euo pipefail

URL="https://rebase.neb.com/rebase/link_striderc"
DEST="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/resources/RELibrary"
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

echo "Fetching $URL ..."
curl -fsSL --max-time 60 "$URL" -o "$TMP"

# Sanity-check before overwriting: header line plus a plausible number of enzymes.
grep -q 'REBASE version' "$TMP" || { echo "error: no REBASE header, refusing to overwrite" >&2; exit 1; }
n=$(grep -cE '^#?[A-Za-z0-9.]+,' "$TMP")
[ "$n" -ge 500 ] || { echo "error: only $n enzyme entries parsed, refusing to overwrite" >&2; exit 1; }

mv "$TMP" "$DEST"
trap - EXIT
echo "Updated $DEST"
sed -n '1p;8p' "$DEST"
echo "$n enzyme entries."
