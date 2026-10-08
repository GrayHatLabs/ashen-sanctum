#!/usr/bin/env bash
# The all-act systems' balance (extras.rs): every hero against each act's arena and a rival.
#   ACTS="1 3 6" HEROES="sorceress druid" TRIALS=rival bash scripts/trials.sh
cd "$(dirname "$0")/.."
B="$HOME/.cache/ashensanctum-target/release/ashensanctum"
for act in ${ACTS:-1 2 3 4 5 6}; do
  for h in ${HEROES:-sorceress vampire inventor valkyrie berserker reaper druid inquisitor}; do
    [ "${TRIALS:-arena rival}" != "rival" ] && ASHEN_ACT=$act ASHEN_CLASS=$h ASHEN_TRIAL=arena "$B" --selftest 2>&1 | grep '^trial'
    ASHEN_ACT=$act ASHEN_CLASS=$h ASHEN_TRIAL=rival ASHEN_RIVAL=$(( (act * 3 + ${#h}) % 8 )) "$B" --selftest 2>&1 | grep '^trial'
  done
done
