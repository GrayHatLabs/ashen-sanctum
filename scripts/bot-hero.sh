#!/usr/bin/env bash
# The selftest bot as one hero in each act: scripts/bot-hero.sh <hero> [acts...]
cd "$(dirname "$0")/.."
hero=$1; shift
for act in ${*:-1 2 3 4 5}; do
  printf 'act %s %-10s ' "$act" "$hero"
  ASHEN_ACT=$act ASHEN_CLASS=$hero "$HOME/.cache/ashensanctum-target/release/ashensanctum" --selftest 2>&1 | grep -E "selftest|panicked" | sed -E 's/ stats=.*//; s/selftest ok: [0-9]+ ticks in [^,]+, avg draw [^,]+, //'
done
