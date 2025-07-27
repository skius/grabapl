#!/bin/bash

set -o xtrace

# Exit if there are unstaged or staged changes
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "❌ You have unstaged or staged but uncommitted changes. Please commit or stash them first."
  exit 1
fi

old="$1"
new="$2"

if [ -z "$old" ] || [ -z "$new" ]; then
  echo "Usage: $0 <old_version> <new_version>"
  exit 1
fi

find . -name Cargo.toml -exec sed -i "s/\"$old\"/\"$new\"/g" {} +

echo "✅ Version updated from $old to $new."
