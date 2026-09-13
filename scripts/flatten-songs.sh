#!/usr/bin/env bash
# Flatten songs/<version>/<song>/ down to songs/<song>/, recording the layout it
# came from so it can be put back.
#
#   scripts/flatten-songs.sh                 # dry run (default) — prints the plan
#   scripts/flatten-songs.sh --apply         # actually move
#   scripts/flatten-songs.sh --apply --prefix   # name them "01. maimai - Song"
#
# WHY A MANIFEST: maidata.txt carries &title, &artist, &wholebpm and &lv_*, but
# nothing that names the maimai version. The version exists *only* as the parent
# directory name, so flattening is the one operation here that destroys
# information. Every run writes songs/.songs-layout.tsv first, and
# scripts/unflatten-songs.sh replays it. Delete that file and the grouping is
# gone for good.
#
# WHY mv AND NOT cp: the collection is ~16GB. A rename within one filesystem is
# metadata-only and instant; a copy would need 16GB free and rewrite every byte.
# The script refuses to run if songs/ turns out to span filesystems, because then
# mv silently degrades into exactly that copy.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SONGS_DIR="${SONGS_DIR:-$WORKSPACE_DIR/songs}"
MANIFEST="$SONGS_DIR/.songs-layout.tsv"

APPLY=0
PREFIX=0
for arg in "$@"; do
  case "$arg" in
    --apply) APPLY=1 ;;
    --prefix) PREFIX=1 ;;
    -h | --help)
      sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "flatten-songs: unknown argument '$arg' (try --help)" >&2
      exit 2
      ;;
  esac
done

[ -d "$SONGS_DIR" ] || {
  echo "flatten-songs: no such directory: $SONGS_DIR" >&2
  exit 1
}

# ── discover ─────────────────────────────────────────────────────────────────
# A song directory is one that contains maidata.txt; a version directory is one
# that contains song directories. Keying off the file rather than off the "NN. "
# naming convention means a renamed or added version folder still works, and an
# already-flattened tree is recognised rather than mangled.
mapfile -t -d '' VERSION_DIRS < <(
  find "$SONGS_DIR" -mindepth 1 -maxdepth 1 -type d \
    ! -name '.*' ! -exec test -e '{}/maidata.txt' \; -print0 | sort -z
)

if [ ${#VERSION_DIRS[@]} -eq 0 ]; then
  echo "flatten-songs: nothing to do — no version directories under $SONGS_DIR"
  echo "               (already flattened, or the tree is not <version>/<song>/)"
  exit 0
fi

# Collect every (version, song) pair up front. Nothing moves until the whole plan
# has been validated: a collision found halfway through would leave the tree in a
# state that is neither nested nor flat.
versions=()
songs=()
targets=()

for vdir in "${VERSION_DIRS[@]}"; do
  version="$(basename "$vdir")"
  while IFS= read -r -d '' sdir; do
    song="$(basename "$sdir")"
    if [ "$PREFIX" -eq 1 ]; then
      target="$version - $song"
    else
      target="$song"
    fi
    versions+=("$version")
    songs+=("$song")
    targets+=("$target")
  done < <(find "$vdir" -mindepth 1 -maxdepth 1 -type d -print0 | sort -z)
done

total=${#songs[@]}
if [ "$total" -eq 0 ]; then
  echo "flatten-songs: found ${#VERSION_DIRS[@]} version directories but no songs inside them"
  exit 0
fi

# ── validate ─────────────────────────────────────────────────────────────────
fatal=0

# A tab or newline in a directory name would corrupt the TSV manifest, and the
# manifest is the only record of the version each song came from.
for i in "${!songs[@]}"; do
  case "${versions[$i]}${songs[$i]}" in
    *$'\t'* | *$'\n'*)
      echo "flatten-songs: FATAL: tab or newline in name: ${versions[$i]}/${songs[$i]}" >&2
      fatal=1
      ;;
  esac
done

# Two songs flattening onto one name would have the second mv swallow the first
# into it as a subdirectory. Today the collection has zero collisions; this is
# the check that keeps that from becoming a silent data loss later.
collisions="$(printf '%s\n' "${targets[@]}" | sort | uniq -d)"
if [ -n "$collisions" ]; then
  echo "flatten-songs: FATAL: ${#targets[@]} songs collapse onto duplicate names:" >&2
  while IFS= read -r dup; do
    echo "  '$dup' claimed by:" >&2
    for i in "${!targets[@]}"; do
      [ "${targets[$i]}" = "$dup" ] && echo "    ${versions[$i]}/${songs[$i]}" >&2
    done
  done <<<"$collisions"
  echo "  re-run with --prefix to keep the version in the name" >&2
  fatal=1
fi

# A target that already exists at the top level and is not itself one of the
# songs being moved.
for t in "${targets[@]}"; do
  if [ -e "$SONGS_DIR/$t" ]; then
    echo "flatten-songs: FATAL: '$t' already exists directly under songs/" >&2
    fatal=1
  fi
done

if [ -e "$MANIFEST" ]; then
  echo "flatten-songs: FATAL: $MANIFEST already exists." >&2
  echo "               A previous flatten was not unflattened. Run" >&2
  echo "               scripts/unflatten-songs.sh --apply first, or delete it to" >&2
  echo "               abandon the old record." >&2
  fatal=1
fi

# mv across filesystems silently becomes a full 16GB copy.
songs_fs="$(df --output=source "$SONGS_DIR" | tail -1)"
for vdir in "${VERSION_DIRS[@]}"; do
  vfs="$(df --output=source "$vdir" | tail -1)"
  if [ "$vfs" != "$songs_fs" ]; then
    echo "flatten-songs: FATAL: '$vdir' is on $vfs but songs/ is on $songs_fs;" >&2
    echo "               mv would copy ~16GB instead of renaming." >&2
    fatal=1
  fi
done

[ "$fatal" -eq 0 ] || exit 1

# ── warn (not fatal) ─────────────────────────────────────────────────────────
# A song directory holding another song directory. songs/27. CiRCLE PLUS/
# HyperdrivE/HyperdrivE/ is one today: the outer has the full asset set and the
# inner only a stray maidata.txt. Flattening moves the outer as one unit and the
# leftover rides along inside it, which is harmless but worth seeing.
for i in "${!songs[@]}"; do
  nested="$(find "$SONGS_DIR/${versions[$i]}/${songs[$i]}" -mindepth 1 -maxdepth 1 -type d -print -quit)"
  if [ -n "$nested" ]; then
    echo "flatten-songs: note: ${versions[$i]}/${songs[$i]} contains a nested directory" >&2
    echo "                     $(basename "$nested")/ — moved along with it, not unpacked" >&2
  fi
done

# ── report / execute ─────────────────────────────────────────────────────────
echo "flatten-songs: $total songs in ${#VERSION_DIRS[@]} versions -> $SONGS_DIR/"
if [ "$PREFIX" -eq 1 ]; then
  echo "               naming: '<version> - <song>'"
else
  echo "               naming: '<song>' (version recorded in the manifest only)"
fi

if [ "$APPLY" -eq 0 ]; then
  echo
  echo "  DRY RUN — nothing moved. First 10 of $total:"
  for i in "${!songs[@]}"; do
    [ "$i" -ge 10 ] && break
    printf '    %s/%s\n      -> %s\n' "${versions[$i]}" "${songs[$i]}" "${targets[$i]}"
  done
  echo
  echo "  re-run with --apply to move, and keep $(basename "$MANIFEST") to undo it"
  exit 0
fi

# Manifest first: if the process dies mid-move, the record of where everything
# came from already exists, and unflatten skips pairs it cannot find.
{
  printf '# written by scripts/flatten-songs.sh on %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf '# version\tsong\tflattened_name\n'
  for i in "${!songs[@]}"; do
    printf '%s\t%s\t%s\n' "${versions[$i]}" "${songs[$i]}" "${targets[$i]}"
  done
} >"$MANIFEST"

moved=0
for i in "${!songs[@]}"; do
  mv -n "$SONGS_DIR/${versions[$i]}/${songs[$i]}" "$SONGS_DIR/${targets[$i]}"
  moved=$((moved + 1))
  if [ $((moved % 200)) -eq 0 ]; then
    echo "  moved $moved/$total"
  fi
done

# Only rmdir — a version directory with anything left in it is a surprise worth
# surfacing rather than deleting.
for vdir in "${VERSION_DIRS[@]}"; do
  if ! rmdir "$vdir" 2>/dev/null; then
    echo "flatten-songs: note: kept '$(basename "$vdir")' — not empty after the move" >&2
  fi
done

echo "flatten-songs: moved $moved songs; layout recorded in $MANIFEST"
echo "               undo with: scripts/unflatten-songs.sh --apply"
