#!/usr/bin/env bash
# Balance survey: every hero through every act's test start for ASHEN_MINUTES (default 12) game minutes.
# Prints one row per run: hero, act, minutes to each boss kill (act boss last), deaths, damage taken, kills.
cd "$(dirname "$0")/.."
MIN=${MIN:-12}
for act in ${ACTS:-1 2 3 4 5 6}; do
  for h in ${HEROES:-sorceress vampire inventor valkyrie berserker reaper druid inquisitor}; do
    out=$(ASHEN_ACT=$act ASHEN_CLASS=$h ASHEN_MINUTES=$MIN "$HOME/.cache/ashensanctum-target/release/ashensanctum" --selftest 2>&1)
    bosses=$(echo "$out" | grep -o "\[ *[0-9]* min\] boss down in [A-Z' ]*" | head -4 | sed -E 's/\[ *([0-9]+) min\] boss down in (THE )?/\1m:/' | cut -c1-22 | paste -sd'|' -)
    deaths=$(echo "$out" | grep -o "deaths: [0-9]*" | tail -1 | cut -d' ' -f2)
    dmg=$(echo "$out" | grep -o "damage_taken: [0-9.]*" | tail -1 | cut -d' ' -f2 | cut -d. -f1)
    kills=$(echo "$out" | grep -o "kills=[0-9]*" | tail -1 | cut -d= -f2)
    panic=$(echo "$out" | grep -c panicked)
    printf "act%s %-10s deaths=%-3s dmg=%-7s kills=%-4s panic=%s  %s\n" "$act" "$h" "$deaths" "$dmg" "$kills" "$panic" "$bosses"
  done
done
