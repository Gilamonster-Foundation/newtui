#!/usr/bin/env bash
# Decide how much of the gate a push needs. Reads the pre-push stdin protocol
# (`<local ref> <local sha> <remote ref> <remote sha>` per line) and prints one
# line: `skip: <reason>` or `full: <reason>`. It always exits 0; the hook acts on
# the word.
#
# The rule is narrow on purpose. A push skips the gate only when it sends no
# code: a branch delete, or changes that are all Markdown files nothing reads.
# Anything else, and anything this script cannot determine, is `full`. CI still
# runs every job on every PR, so a wrong `skip` costs a red PR, never a merge.
#
# A Markdown file is read when Rust includes it (`include_str!`, which is how the
# README examples become doctests and CATALOG.md is checked against the
# exports) or a test, script, justfile, workflow or workspace member names its
# path. That set is derived from the tree on each run, so it cannot drift from a
# hand-kept list. A new reference needs a code change, and that change is not
# Markdown, so it runs the full gate itself.
#
# PREPUSH_BASE names the branch a new remote branch is compared against.
set -uo pipefail

base="${PREPUSH_BASE:-origin/main}"
zero=0000000000000000000000000000000000000000

gated_docs() {
  {
    git grep -ohE 'include_(str|bytes)!\("[^"]+\.md"\)' -- '*.rs' |
      sed -E 's/^[^"]*"//; s/"\)$//'
    git grep -ohE '[A-Za-z0-9_./-]+\.md' -- tests scripts justfile .github \
      'newtui-*/src' 'newtui-*/tests' 'newtui-*/python' 'newtui-*/python-tests'
  } | sed -E 's|^(\.\./)+||' | sort -u
}

changed=""
seen=0
while read -r _ local_sha _ remote_sha; do
  [ "$local_sha" = "$zero" ] && continue # delete: no commits go out
  seen=1
  if [ "$remote_sha" = "$zero" ]; then
    from=$(git merge-base "$local_sha" "$base" 2>/dev/null) ||
      { echo "full: no merge-base with $base for a new branch"; exit 0; }
  else
    from=$remote_sha
  fi
  files=$(git diff --name-only "$from" "$local_sha" 2>/dev/null) ||
    { echo "full: cannot diff ${from:0:12}..${local_sha:0:12}"; exit 0; }
  changed+="$files"$'\n'
done

if [ "$seen" = 0 ]; then
  echo "skip: nothing but a branch delete"
  exit 0
fi

changed=$(printf '%s' "$changed" | sed '/^$/d' | sort -u)
if [ -z "$changed" ]; then
  echo "skip: no file changes"
  exit 0
fi

gated=$(gated_docs)
while IFS= read -r f; do
  case "$f" in
    *.md) ;;
    *) echo "full: $f is not Markdown"; exit 0 ;;
  esac
  if printf '%s\n' "$gated" | grep -qxF "$f"; then
    echo "full: $f is read by code"
    exit 0
  fi
done <<<"$changed"

echo "skip: only Markdown nothing reads ($(printf '%s\n' "$changed" | wc -l | tr -d ' ') files)"
