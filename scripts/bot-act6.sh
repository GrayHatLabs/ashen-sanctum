#!/usr/bin/env bash
# Every hero through Act 6 (from the Act 6 test start).
cd "$(dirname "$0")/.."
for h in sorceress vampire inventor valkyrie berserker reaper druid inquisitor; do
  bash scripts/bot-hero.sh "$h" 6
done
