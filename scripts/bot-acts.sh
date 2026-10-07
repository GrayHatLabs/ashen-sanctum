#!/usr/bin/env bash
# The selftest bot in every act with every hero (crash hunting). Needs a release build.
# Usage: scripts/bot-acts.sh [acts...]   (default: 1 2 3 4 5 6)
set -e
cd "$(dirname "$0")/.."
BIN="$HOME/.cache/ashensanctum-target/release/ashensanctum"
ACTS="${*:-1 2 3 4 5 6}"
for act in $ACTS; do
  for hero in sorceress vampire inventor valkyrie berserker reaper druid inquisitor; do
    printf 'act %s %-10s ' "$act" "$hero"
    ASHEN_ACT=$act ASHEN_CLASS=$hero "$BIN" --selftest 2>&1 | grep -E "selftest|panicked" | sed -E 's/ stats=.*//; s/selftest ok: [0-9]+ ticks in [^,]+, avg draw [^,]+, //'
  done
done
