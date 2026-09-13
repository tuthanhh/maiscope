#!/usr/bin/env bash
# Put songs/ back into <version>/<song>/ from the manifest flatten-songs.sh wrote.
#
#   scripts/unflatten-songs.sh            # dry run (default)
#   scripts/unflatten-songs.sh --apply    # actually move back
#
# songs/.songs-layout.tsv is the only record of which maimai version each song
# came from — maidata.txt does not carry one. Without that file this script has
# nothing to work from and there is no way to rebuild the grouping.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SONGS_DIR="${SONGS_DIR:-$WORKSPACE_DIR/songs}"
MANIFEST="$SONGS_DIR/.songs-layout.tsv"

APPLY=0
for arg in "$@"; do
  case "$arg" in
    --apply) APPLY=1 ;;
    -h | --help)
      sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unflatten-songs: unknown argument '$arg' (try --help)" >&2
      exit 2
      ;;
  esac
done

[ -e "$MANIFEST" ] || {
  echo "unflatten-songs: no manifest at $MANIFEST" >&2
  echo "                 Nothing to restore from — the version grouping is not" >&2
  echo "                 recoverable from the song directories alone." >&2
  exit 1
}

# ── read the manifest ────────────────────────────────────────────────────────
versions=()
songs=()
flattened=()
while IFS=$'\t' read -r version song flat; do
  case "$version" in '#'* | '') continue ;; esac
  versions+=("$version")
  songs+=("$song")
  flattened+=("$flat")
done <"$MANIFEST"

total=${#songs[@]}
[ "$total" -gt 0 ] || {
  echo "unflatten-songs: manifest lists no songs" >&2
  exit 1
}

# ── validate ─────────────────────────────────────────────────────────────────
# A song the manifest names but that is not at the top level was already moved
# back, renamed, or deleted. Skip it rather than fail: a partial flatten that
# died mid-run must still be repairable by this script.
present=()
missing=0
for i in "${!songs[@]}"; do
  if [ -d "$SONGS_DIR/${flattened[$i]}" ]; then
    present+=("$i")
  else
    missing=$((missing + 1))
  fi
done

fatal=0
for i in "${present[@]}"; do
  dest="$SONGS_DIR/${versions[$i]}/${songs[$i]}"
  if [ -e "$dest" ]; then
    echo "unflatten-songs: FATAL: destination already exists: ${versions[$i]}/${songs[$i]}" >&2
    fatal=1
  fi
done
[ "$fatal" -eq 0 ] || exit 1

echo "unflatten-songs: $((${#present[@]})) of $total songs to restore"
[ "$missing" -gt 0 ] && echo "                 $missing listed but not found at the top level — skipped"

if [ "$APPLY" -eq 0 ]; then
  echo
  echo "  DRY RUN — nothing moved. First 10:"
  shown=0
  for i in "${present[@]}"; do
    [ "$shown" -ge 10 ] && break
    printf '    %s\n      -> %s/%s\n' "${flattened[$i]}" "${versions[$i]}" "${songs[$i]}"
    shown=$((shown + 1))
  done
  echo
  echo "  re-run with --apply to move back"
  exit 0
fi

moved=0
for i in "${present[@]}"; do
  mkdir -p "$SONGS_DIR/${versions[$i]}"
  mv -n "$SONGS_DIR/${flattened[$i]}" "$SONGS_DIR/${versions[$i]}/${songs[$i]}"
  moved=$((moved + 1))
  if [ $((moved % 200)) -eq 0 ]; then
    echo "  moved $moved/${#present[@]}"
  fi
done

# The manifest describes a flattened tree. Once everything in it is back under a
# version directory it no longer describes reality, so retire it — leaving it
# would make the next flatten-songs.sh run abort on a stale record.
if [ "$moved" -eq "$total" ]; then
  rm -f "$MANIFEST"
  echo "unflatten-songs: moved $moved songs; removed the manifest"
else
  echo "unflatten-songs: moved $moved songs; kept $MANIFEST ($missing entry/entries unresolved)"
fi
