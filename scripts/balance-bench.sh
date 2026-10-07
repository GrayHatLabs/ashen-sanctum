#!/usr/bin/env bash
# The balance benchmark (ASHEN_BENCH, snapshot.rs bench()): every hero, every act's test start.
cd "$(dirname "$0")/.."
for act in ${ACTS:-1 2 3 4 5 6}; do
  for h in ${HEROES:-sorceress vampire inventor valkyrie berserker reaper druid inquisitor}; do
    ASHEN_BENCH=1 ASHEN_ACT=$act ASHEN_CLASS=$h "$HOME/.cache/ashensanctum-target/release/ashensanctum" --selftest 2>&1 | grep -E "^bench|panicked"
  done
done
